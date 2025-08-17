# Development Guide

## Quick Start

1. **Install Rust**: https://rustup.rs/
2. **Clone and Build**:
   ```bash
   git clone <repository-url>
   cd dreamland
   chmod +x build.sh
   ./build.sh
   ```
3. **Run Application**:
   ```bash
   cargo run
   ```

## Development Commands

- `cargo check` - Fast compilation check
- `cargo test` - Run unit tests
- `cargo run` - Build and run the application
- `cargo build --release` - Optimized production build

## Key Features Implemented

### ✅ Core Application Structure
- Dioxus desktop application setup
- State management with shared context
- Modular component architecture

### ✅ API Integration
- HTTP client for fetching JSON data
- Image download functionality
- Support for yande.re API format
- Error handling for network operations

### ✅ User Interface Components
- **Header**: Title, load button, settings toggle
- **Gallery**: Responsive image grid with lazy loading
- **Image Cards**: Preview, metadata, download buttons
- **Settings Dialog**: Configuration management
- **Pagination**: Page navigation controls

### ✅ Configuration Management
- Persistent settings storage
- Cross-platform config directories
- User-configurable download paths and API URLs

### ✅ File Management
- Automatic directory creation
- Organized download structure
- File naming based on image MD5 hashes

## Architecture Overview

```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   UI Components │────│   App State     │────│  Configuration  │
│   (Dioxus)      │    │   (Reactive)    │    │   (JSON File)   │
└─────────────────┘    └─────────────────┘    └─────────────────┘
         │                        │                        │
         │                        │                        │
         ▼                        ▼                        ▼
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   API Client    │    │  Image Gallery  │    │ Download Manager│
│   (Reqwest)     │    │   (Grid Layout) │    │ (File System)   │
└─────────────────┘    └─────────────────┘    └─────────────────┘
```

## Testing

The application includes unit tests for:
- JSON deserialization of image data
- Configuration loading and saving
- API response parsing

Run tests with:
```bash
cargo test
```

## Future Enhancements

- [ ] Image caching for better performance
- [ ] Advanced filtering and search
- [ ] Multiple API source support
- [ ] Thumbnail generation
- [ ] Batch download operations
- [ ] Custom themes and styling
- [ ] Export/import settings

## Troubleshooting

### Common Issues

1. **Network Timeouts**: Check internet connection and API availability
2. **Permission Errors**: Ensure write access to download directory
3. **Compilation Errors**: Update Rust toolchain with `rustup update`

### Debug Mode

Run with debug logging:
```bash
RUST_LOG=debug cargo run
```

### Configuration Reset

Delete config file to reset to defaults:
```bash
rm ~/.config/dreamland/config.json  # Linux/macOS
```