use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// JSON-RPC 2.0 request
#[derive(Debug, Clone, Serialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: &'static str,
    pub id: String,
    pub method: String,
    pub params: serde_json::Value,
}

impl JsonRpcRequest {
    pub fn new(id: String, method: String, params: serde_json::Value) -> Self {
        Self {
            jsonrpc: "2.0",
            id,
            method,
            params,
        }
    }
}

/// JSON-RPC 2.0 response
#[derive(Debug, Clone, Deserialize)]
pub struct JsonRpcResponse {
    #[allow(dead_code)]
    pub jsonrpc: String,
    pub id: Option<String>,
    pub result: Option<serde_json::Value>,
    pub error: Option<JsonRpcError>,
    pub method: Option<String>,
    pub params: Option<serde_json::Value>,
}

/// JSON-RPC 2.0 error
#[derive(Debug, Clone, Deserialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
}

/// JSON-RPC notification (no response expected)
#[derive(Debug, Clone)]
pub struct Notification {
    pub method: String,
    pub params: Option<serde_json::Value>,
}

/// Play state from CherryLink/ZedLink
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum PlayState {
    #[default]
    Stopped,
    Playing,
    Paused,
    Simulating,
}

impl std::fmt::Display for PlayState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PlayState::Stopped => write!(f, "Stopped"),
            PlayState::Playing => write!(f, "Playing"),
            PlayState::Paused => write!(f, "Paused"),
            PlayState::Simulating => write!(f, "Simulating"),
        }
    }
}

/// Build status from CherryLink/ZedLink
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum BuildStatus {
    #[default]
    Idle,
    Compiling,
    Success,
    Failed,
    Cancelled,
}

impl std::fmt::Display for BuildStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BuildStatus::Idle => write!(f, "Idle"),
            BuildStatus::Compiling => write!(f, "Compiling"),
            BuildStatus::Success => write!(f, "Success"),
            BuildStatus::Failed => write!(f, "Failed"),
            BuildStatus::Cancelled => write!(f, "Cancelled"),
        }
    }
}

/// Build configuration for Unreal Engine
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default, JsonSchema)]
pub enum BuildConfiguration {
    #[default]
    DevelopmentEditor,
    DebugEditor,
    Shipping,
    DebugGame,
    Development,
    Debug,
}

impl BuildConfiguration {
    pub fn all() -> &'static [BuildConfiguration] {
        &[
            BuildConfiguration::DevelopmentEditor,
            BuildConfiguration::DebugEditor,
            BuildConfiguration::Shipping,
            BuildConfiguration::DebugGame,
            BuildConfiguration::Development,
            BuildConfiguration::Debug,
        ]
    }
}

impl std::fmt::Display for BuildConfiguration {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BuildConfiguration::DevelopmentEditor => write!(f, "Development Editor"),
            BuildConfiguration::DebugEditor => write!(f, "Debug Editor"),
            BuildConfiguration::Shipping => write!(f, "Shipping"),
            BuildConfiguration::DebugGame => write!(f, "DebugGame"),
            BuildConfiguration::Development => write!(f, "Development"),
            BuildConfiguration::Debug => write!(f, "Debug"),
        }
    }
}

/// Log message from CherryLink/ZedLink
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LogMessage {
    pub category: String,
    pub verbosity: String,
    pub message: String,
    #[serde(default)]
    pub timestamp: String,
    #[serde(default)]
    pub frame: u64,
    #[serde(default)]
    pub source_file: Option<String>,
    #[serde(default)]
    pub source_line: Option<i32>,
}

/// Build error from CherryLink/ZedLink
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BuildError {
    pub file: String,
    pub line: i32,
    pub column: i32,
    pub message: String,
    pub severity: String,
}
