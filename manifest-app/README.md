# Manifest App

A native macOS application for Manifest, built with GPUI (Zed's GPU-accelerated UI framework).

## Structure

```
manifest-app/
├── app/              # Main application entry point
├── feature_panel/    # Left sidebar with feature tree
├── terminal/         # Terminal emulation (alacritty-based)
├── terminal_view/    # Terminal UI with tabs
└── manifest_client/  # HTTP client for Manifest API
```

## Build & Run

```bash
cargo build           # Debug build
cargo run             # Run the application
cargo build --release # Release build
```

## Requirements

- macOS (GPUI is macOS-only currently)
- Rust 2024 edition
- Zed's GPUI crate (linked via path dependency)

## Dependencies

- **gpui** - GPU-accelerated UI framework from Zed
- **alacritty_terminal** - Terminal emulation
- **ureq** - HTTP client for Manifest server communication
