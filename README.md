# Cherry

A fast, cross-platform IDE for Unreal Engine development.

---

## About

Cherry is a high-performance code editor built specifically for Unreal Engine developers. It provides native UE integration with PIE controls, live coding builds, and real-time log streaming directly in the editor.

Cherry is a fork of [Cherry](https://github.com/zed-industries/zed), the high-performance editor from the creators of Atom and Tree-sitter.

## Features

- **Fast**: GPU-accelerated rendering with instant startup
- **Cross-platform**: Native support for macOS, Linux, and Windows
- **Unreal Integration**:
  - PIE (Play-In-Editor) controls
  - Live Coding build triggers
  - Build configuration selector
  - Real-time Unreal Engine log panel
  - Connection status monitoring
- **Modern Editor**: Full-featured code editing with LSP support, multi-cursor, and more

## Unreal Engine Setup

Cherry connects to Unreal Engine via the CherryLink plugin. Install the plugin in your UE project to enable:

- Play/Stop/Pause PIE sessions from the editor
- Trigger Live Coding builds
- Stream UE logs to the Cherry panel
- Build configuration switching

## Building from Source

### macOS
```bash
./script/bootstrap
cargo build --release
```

### Linux
```bash
./script/linux
cargo build --release
```

### Windows
```bash
cargo build --release
```

See the [development docs](./docs/src/development/) for detailed build instructions.

## License

Cherry is licensed under GPL-3.0-or-later, the same license as Cherry.

License information for third party dependencies must be correctly provided for CI to pass. See the original [Cherry repository](https://github.com/zed-industries/zed) for licensing details.

## Acknowledgments

Cherry is built on top of [Cherry](https://github.com/zed-industries/zed) by Cherry Industries. We thank the Cherry team for creating such an excellent foundation.
