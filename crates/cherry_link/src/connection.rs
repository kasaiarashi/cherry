use crate::protocol::{BuildConfiguration, BuildStatus, JsonRpcRequest, JsonRpcResponse, LogMessage, PlayState};
use crate::CherryLinkEvent;
use anyhow::{anyhow, Result};
use futures::channel::mpsc;
use futures::{AsyncReadExt, AsyncWriteExt, FutureExt, StreamExt};
use gpui::{Context, Task};
use smol::net::TcpStream;
use std::sync::atomic::{AtomicU64, Ordering};

static REQUEST_ID: AtomicU64 = AtomicU64::new(1);

/// Connection state for CherryLink
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ConnectionState {
    #[default]
    Disconnected,
    Connecting,
    Connected,
}

/// Internal message for outgoing requests
#[derive(Debug)]
struct OutgoingMessage {
    json: String,
}

/// Incoming notification from the server
#[derive(Debug)]
enum IncomingNotification {
    LogMessage(LogMessage),
    PlayStateChanged(PlayState),
    BuildStatusChanged(BuildStatus),
    Disconnected,
}

/// CherryLink connection entity
pub struct CherryLinkConnection {
    state: ConnectionState,
    port: u16,
    play_state: PlayState,
    build_status: BuildStatus,
    selected_config: BuildConfiguration,
    outgoing_tx: Option<mpsc::UnboundedSender<OutgoingMessage>>,
    _connection_task: Option<Task<()>>,
    _notification_task: Option<Task<()>>,
}

impl CherryLinkConnection {
    pub fn new(port: u16) -> Self {
        Self {
            state: ConnectionState::Disconnected,
            port,
            play_state: PlayState::default(),
            build_status: BuildStatus::default(),
            selected_config: BuildConfiguration::default(),
            outgoing_tx: None,
            _connection_task: None,
            _notification_task: None,
        }
    }

    pub fn is_connected(&self) -> bool {
        self.state == ConnectionState::Connected
    }

    pub fn connection_state(&self) -> ConnectionState {
        self.state
    }

    pub fn play_state(&self) -> PlayState {
        self.play_state
    }

    pub fn build_status(&self) -> BuildStatus {
        self.build_status
    }

    pub fn selected_configuration(&self) -> BuildConfiguration {
        self.selected_config
    }

    pub fn set_selected_configuration(&mut self, config: BuildConfiguration, cx: &mut Context<Self>) {
        self.selected_config = config;
        cx.notify();
    }

    /// Connect to the CherryLink server
    pub fn connect(&mut self, cx: &mut Context<Self>) {
        if self.state != ConnectionState::Disconnected {
            return;
        }

        self.state = ConnectionState::Connecting;
        cx.notify();

        let port = self.port;
        let (outgoing_tx, outgoing_rx) = mpsc::unbounded();
        let (notification_tx, notification_rx) = mpsc::unbounded();
        self.outgoing_tx = Some(outgoing_tx);

        // Spawn the background connection task
        let connection_task = cx.background_executor().spawn(async move {
            let result = Self::connection_loop(port, outgoing_rx, notification_tx.clone()).await;
            if let Err(e) = result {
                log::error!("CherryLink connection error: {}", e);
            }
            // Signal disconnection
            let _ = notification_tx.unbounded_send(IncomingNotification::Disconnected);
        });

        // Spawn foreground task to process notifications
        let notification_task = cx.spawn(async move |this, mut cx| {
            Self::process_notifications(notification_rx, this, &mut cx).await;
        });

        self._connection_task = Some(Task::from(connection_task));
        self._notification_task = Some(notification_task);

        // Optimistically set connected (will be corrected if connection fails)
        self.state = ConnectionState::Connected;
        cx.emit(CherryLinkEvent::Connected);
        cx.notify();
    }

    /// Process incoming notifications on the foreground thread
    async fn process_notifications(
        mut rx: mpsc::UnboundedReceiver<IncomingNotification>,
        this: gpui::WeakEntity<Self>,
        cx: &mut gpui::AsyncApp,
    ) {
        while let Some(notification) = rx.next().await {
            let result = this.update(cx, |this, cx| {
                match notification {
                    IncomingNotification::LogMessage(msg) => {
                        cx.emit(CherryLinkEvent::LogReceived(msg));
                        cx.notify();
                    }
                    IncomingNotification::PlayStateChanged(state) => {
                        this.play_state = state;
                        cx.emit(CherryLinkEvent::PlayStateChanged(state));
                        cx.notify();
                    }
                    IncomingNotification::BuildStatusChanged(status) => {
                        this.build_status = status;
                        cx.emit(CherryLinkEvent::BuildStatusChanged(status));
                        cx.notify();
                    }
                    IncomingNotification::Disconnected => {
                        this.state = ConnectionState::Disconnected;
                        this.outgoing_tx = None;
                        cx.emit(CherryLinkEvent::Disconnected);
                        cx.notify();
                    }
                }
            });

            if result.is_err() {
                break;
            }
        }
    }

    /// Main connection loop (runs on background thread)
    async fn connection_loop(
        port: u16,
        mut outgoing_rx: mpsc::UnboundedReceiver<OutgoingMessage>,
        notification_tx: mpsc::UnboundedSender<IncomingNotification>,
    ) -> Result<()> {
        let addr = format!("127.0.0.1:{}", port);
        log::info!("CherryLink: Connecting to {}...", addr);

        let mut stream = TcpStream::connect(&addr).await?;
        log::info!("CherryLink: Connected to {}", addr);

        // Perform handshake
        Self::perform_handshake(&mut stream).await?;

        // Clone stream for reading (we'll use the original for writing)
        let (mut reader, mut writer) = stream.split();

        // Use select to handle both reading and writing
        loop {
            futures::select! {
                // Handle outgoing messages
                msg = outgoing_rx.next() => {
                    match msg {
                        Some(msg) => {
                            log::info!("CherryLink: connection_loop - received message from queue, sending...");
                            if let Err(e) = Self::send_raw_message(&mut writer, &msg.json).await {
                                log::error!("CherryLink: Write error: {}", e);
                                break;
                            }
                            log::info!("CherryLink: connection_loop - message sent successfully");
                        }
                        None => {
                            log::info!("CherryLink: Outgoing channel closed");
                            break;
                        }
                    }
                }
                // Handle incoming messages
                result = Self::read_message(&mut reader).fuse() => {
                    match result {
                        Ok(response) => {
                            if let Some(notification) = Self::parse_notification(&response) {
                                if notification_tx.unbounded_send(notification).is_err() {
                                    break;
                                }
                            }
                        }
                        Err(e) => {
                            log::error!("CherryLink: Read error: {}", e);
                            break;
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Parse a response into a notification if applicable
    fn parse_notification(response: &JsonRpcResponse) -> Option<IncomingNotification> {
        // Only handle notifications (no id)
        if response.id.is_some() {
            return None;
        }

        let method = response.method.as_ref()?;
        let params = response.params.as_ref();

        match method.as_str() {
            "logging/message" => {
                if let Some(p) = params {
                    if let Ok(log_msg) = serde_json::from_value::<LogMessage>(p.clone()) {
                        log::debug!("CherryLink: Received log message: {} - {}", log_msg.category, log_msg.message);
                        return Some(IncomingNotification::LogMessage(log_msg));
                    } else {
                        log::error!("CherryLink: Failed to parse logging/message params: {:?}", p);
                    }
                } else {
                    log::warn!("CherryLink: logging/message notification has no params");
                }
            }
            "play/stateChanged" => {
                if let Some(p) = params {
                    if let Some(state_str) = p.get("state").and_then(|v| v.as_str()) {
                        let state = match state_str {
                            "playing" => PlayState::Playing,
                            "paused" => PlayState::Paused,
                            "simulating" => PlayState::Simulating,
                            _ => PlayState::Stopped,
                        };
                        return Some(IncomingNotification::PlayStateChanged(state));
                    }
                }
            }
            "build/status" => {
                if let Some(p) = params {
                    if let Some(status_str) = p.get("status").and_then(|v| v.as_str()) {
                        let status = match status_str {
                            "compiling" => BuildStatus::Compiling,
                            "success" => BuildStatus::Success,
                            "failed" => BuildStatus::Failed,
                            _ => BuildStatus::Idle,
                        };
                        return Some(IncomingNotification::BuildStatusChanged(status));
                    }
                }
            }
            _ => {
                log::debug!("CherryLink: Unknown notification: {}", method);
            }
        }
        None
    }

    /// Perform initial handshake with server
    async fn perform_handshake(stream: &mut TcpStream) -> Result<()> {
        let request = JsonRpcRequest::new(
            REQUEST_ID.fetch_add(1, Ordering::SeqCst).to_string(),
            "connection/initialize".to_string(),
            serde_json::json!({
                "clientVersion": env!("CARGO_PKG_VERSION"),
                "clientName": "cherry",
                "capabilities": ["blueprint", "logging", "play", "build"]
            }),
        );

        let json = serde_json::to_string(&request)?;
        Self::send_raw_message(stream, &json).await?;

        // Read handshake response
        let response = Self::read_message(stream).await?;
        log::info!("CherryLink: Handshake complete. Server info: {:?}", response.result);

        Ok(())
    }

    /// Send a raw message with 4-byte length prefix
    async fn send_raw_message<W: AsyncWriteExt + Unpin>(writer: &mut W, message: &str) -> Result<()> {
        let bytes = message.as_bytes();
        let len = bytes.len() as u32;

        log::debug!("CherryLink: send_raw_message - sending {} bytes", len);
        // Write 4-byte big-endian length prefix
        writer.write_all(&len.to_be_bytes()).await?;
        writer.write_all(bytes).await?;
        writer.flush().await?;
        log::debug!("CherryLink: send_raw_message - sent successfully");

        Ok(())
    }

    /// Read a message with 4-byte length prefix
    async fn read_message<R: AsyncReadExt + Unpin>(reader: &mut R) -> Result<JsonRpcResponse> {
        // Read 4-byte length prefix
        let mut len_buf = [0u8; 4];
        reader.read_exact(&mut len_buf).await?;
        let len = u32::from_be_bytes(len_buf) as usize;

        if len == 0 || len > 10 * 1024 * 1024 {
            return Err(anyhow!("Invalid message length: {}", len));
        }

        // Read message body
        let mut msg_buf = vec![0u8; len];
        reader.read_exact(&mut msg_buf).await?;

        let response: JsonRpcResponse = serde_json::from_slice(&msg_buf)?;
        Ok(response)
    }

    /// Send a JSON-RPC request (fire and forget for now)
    fn send_request(&self, method: &str, params: serde_json::Value) {
        if let Some(tx) = &self.outgoing_tx {
            let request = JsonRpcRequest::new(
                REQUEST_ID.fetch_add(1, Ordering::SeqCst).to_string(),
                method.to_string(),
                params,
            );

            if let Ok(json) = serde_json::to_string(&request) {
                log::info!("CherryLink: Sending request: {}", method);
                if let Err(e) = tx.unbounded_send(OutgoingMessage { json }) {
                    log::error!("CherryLink: Failed to send request {}: {:?}", method, e);
                }
            } else {
                log::error!("CherryLink: Failed to serialize request for method: {}", method);
            }
        } else {
            log::warn!("CherryLink: Cannot send request '{}' - not connected (outgoing_tx is None)", method);
        }
    }

    /// Disconnect from the CherryLink server
    pub fn disconnect(&mut self, cx: &mut Context<Self>) {
        self.outgoing_tx = None;
        self._connection_task = None;
        self._notification_task = None;
        self.state = ConnectionState::Disconnected;
        cx.emit(CherryLinkEvent::Disconnected);
        cx.notify();
    }

    /// Send a play start request
    pub fn play_start(&mut self, cx: &mut Context<Self>) {
        if !self.is_connected() {
            return;
        }
        self.send_request("play/start", serde_json::json!({}));
        // Optimistically update state
        self.play_state = PlayState::Playing;
        cx.emit(CherryLinkEvent::PlayStateChanged(self.play_state));
        cx.notify();
    }

    /// Send a play stop request
    pub fn play_stop(&mut self, cx: &mut Context<Self>) {
        if !self.is_connected() {
            return;
        }
        self.send_request("play/stop", serde_json::json!({}));
        self.play_state = PlayState::Stopped;
        cx.emit(CherryLinkEvent::PlayStateChanged(self.play_state));
        cx.notify();
    }

    /// Send a play pause request
    pub fn play_pause(&mut self, cx: &mut Context<Self>) {
        if !self.is_connected() {
            return;
        }
        self.send_request("play/pause", serde_json::json!({}));
        self.play_state = PlayState::Paused;
        cx.emit(CherryLinkEvent::PlayStateChanged(self.play_state));
        cx.notify();
    }

    /// Send a play resume request
    pub fn play_resume(&mut self, cx: &mut Context<Self>) {
        if !self.is_connected() {
            return;
        }
        self.send_request("play/resume", serde_json::json!({}));
        self.play_state = PlayState::Playing;
        cx.emit(CherryLinkEvent::PlayStateChanged(self.play_state));
        cx.notify();
    }

    /// Send a live coding build request
    pub fn build_live_coding(&mut self, cx: &mut Context<Self>) {
        if !self.is_connected() {
            return;
        }
        self.send_request("build/liveCoding", serde_json::json!({}));
        self.build_status = BuildStatus::Compiling;
        cx.emit(CherryLinkEvent::BuildStatusChanged(self.build_status));
        cx.notify();
    }

    /// Subscribe to logging
    pub fn subscribe_logging(&mut self, _cx: &mut Context<Self>) {
        if !self.is_connected() {
            return;
        }
        self.send_request("logging/subscribe", serde_json::json!({}));
    }

    /// Unsubscribe from logging
    pub fn unsubscribe_logging(&mut self, _cx: &mut Context<Self>) {
        if !self.is_connected() {
            return;
        }
        self.send_request("logging/unsubscribe", serde_json::json!({}));
    }
}
