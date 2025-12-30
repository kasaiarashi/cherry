# Cherry: Remaining Features to Implement

## Current Status
The basic structure is in place with three new crates:
- `cherry_link` - Connection management (stub implementation)
- `unreal_toolbar` - Toolbar UI components
- `unreal_panel` - Log panel UI

The project compiles successfully.

---

## High Priority

### 1. TCP Connection Implementation
**Location**: `crates/cherry_link/src/connection.rs`

Currently stubbed. Needs:
- [ ] TCP socket connection to `127.0.0.1:21567`
- [ ] Connection state machine (Disconnected → Connecting → Connected)
- [ ] Reconnection logic with backoff
- [ ] Async read/write loops using `cx.spawn`

### 2. JSON-RPC 2.0 Protocol
**Location**: `crates/cherry_link/src/protocol.rs`

Protocol types exist but need:
- [ ] Message framing (4-byte big-endian length prefix)
- [ ] Request/response correlation by ID
- [ ] Notification handling (no response expected)
- [ ] Methods to implement:
  - `play/start`, `play/stop`, `play/pause`, `play/resume`
  - `build/liveCoding`
  - `logging/subscribe`, `logging/unsubscribe`
  - `project/getInfo`

### 3. Toolbar Title Bar Integration
**Location**: `crates/title_bar/src/title_bar.rs`

The toolbar renders but needs:
- [ ] Conditional display when UE project detected
- [ ] Proper positioning below title bar
- [ ] Theme integration

### 4. Log Panel Scrolling
**Location**: `crates/unreal_panel/src/panel.rs`

Currently renders all logs inline:
- [ ] Implement virtual list for performance (10k+ logs)
- [ ] Auto-scroll to bottom when new logs arrive (turn on/off, default is off)
- [ ] Scroll position preservation when user scrolls up

---

## Medium Priority

### 5. Settings Integration
**Location**: `crates/cherry_link/src/settings.rs`

Settings struct exists but needs:
- [ ] Register with Zed's Settings system (`settings::Settings` trait)
- [ ] Settings UI in preferences
- [ ] Settings fields:
  - `enabled: bool`
  - `port: u16`
  - `auto_connect: bool`
  - `default_configuration: BuildConfiguration`

### 6. Project Detection
**New file**: `crates/cherry_link/src/project_detection.rs`

- [ ] Detect `.uproject` files in workspace
- [ ] Parse .uproject for project name
- [ ] Emit event when UE project opened
- [ ] Auto-connect based on settings

### 7. Log Filtering
**Location**: `crates/unreal_panel/src/panel.rs`

- [ ] Filter by verbosity (Log, Warning, Error, Display)
- [ ] Filter by category (LogTemp, LogBlueprintUserMessages, etc.)
- [ ] Search/filter text box
- [ ] Persist filter state

### 8. Build Status Display
- [ ] Show build progress in toolbar
- [ ] Build errors in panel
- [ ] Clickable errors to navigate to source

---

## Lower Priority

### 9. Debug Integration
**Location**: `crates/unreal_toolbar/src/toolbar.rs`

Debug button exists but action is empty:
- [ ] Define debug workflow (attach to UE process?)
- [ ] Integration with Zed's debugger panel
- [ ] Breakpoint sync with UE

### 10. UE Plugin Rename (ZedLink → CherryLink)
**Location**: External repo `/Users/krishnateja/Developer/Work/ue5-zed/ZedLink/`

- [ ] Rename plugin directory
- [ ] Update `ZedLink.uplugin` → `CherryLink.uplugin`
- [ ] Rename classes: `FZedLinkServer` → `FCherryLinkServer`
- [ ] Update port file path

### 11. Additional Branding
- [ ] Menu items (View → Unreal Panel, etc.)
- [ ] Keyboard shortcuts for PIE commands
- [ ] App icon update
- [ ] Splash screen / about dialog

### 12. Polish
- [ ] Status bar indicators
- [ ] Notifications for connection/build events
- [ ] Sounds for build complete/errors (optional)
- [ ] Performance profiling

---

## Technical Debt

- [ ] Clean up unused imports in all Unreal crates
- [ ] Add unit tests for protocol parsing
- [ ] Add integration tests with mock UE server
- [ ] Documentation for public APIs
- [ ] Error messages for common failure cases

---

## Reference Implementation

The `zed-unreal-helper` binary contains working TCP/JSON-RPC code:
- `/Users/krishnateja/Developer/Work/ue5-zed/zed-unreal/zed-unreal-helper/src/connection.rs`
- `/Users/krishnateja/Developer/Work/ue5-zed/zed-unreal/zed-unreal-helper/src/protocol.rs`

This can be adapted for the GPUI async model.
