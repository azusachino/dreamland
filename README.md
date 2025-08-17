# Dreamland Image Viewer

A cross-platform image gallery application built with Rust, Dioxus, and supporting image APIs like yande.re.

## Features

- **Image Gallery**: Browse images in a responsive grid layout
- **API Integration**: Fetches images from JSON APIs (yande.re/post.json format)
- **Download Management**: Download high-quality images to local storage
- **Settings Panel**: Configure download locations and API endpoints
- **Pagination**: Navigate through multiple pages of images
- **Cross-Platform**: Built with Dioxus for desktop applications

## Project Structure

```
src/
├── main.rs           # Main application entry point
├── api.rs            # API client for fetching and downloading images
├── config.rs         # Configuration management
└── ui/
    ├── mod.rs        # UI module exports
    ├── header.rs     # Top header with controls and settings
    ├── gallery.rs    # Main image gallery component
    └── settings.rs   # Settings dialog component
```

## Architecture

### State Management
The application uses Dioxus's context system to manage global state:
- Image list and loading states
- User configuration (download path, API URL)
- Current page for pagination

### API Integration
- Fetches image metadata from JSON APIs
- Downloads high-quality images to configurable local directories
- Supports pagination for large image sets

### UI Components
- **Header**: Load button, settings toggle
- **Gallery**: Responsive grid of image cards with download buttons
- **Settings**: Modal dialog for configuration
- **Pagination**: Previous/Next page navigation

## Configuration

The application stores settings in:
- `~/.config/dreamland/config.json` on Linux/macOS
- User config directory on Windows

Default settings:
```json
{
  "download_path": "~/Downloads/dreamland_images",
  "api_url": "https://yande.re/post.json",
  "images_per_page": 20
}
```

## Usage

1. **Start the application**: `cargo run`
2. **Load images**: Click "Load Images" to fetch from the configured API
3. **Browse gallery**: Scroll through the image grid
4. **Download images**: Click download button on any image card
5. **Configure settings**: Click "Settings" to change download path or API URL
6. **Navigate pages**: Use Previous/Next buttons for pagination

## Dependencies

- **dioxus**: Modern reactive UI framework for Rust
- **dioxus-desktop**: Desktop renderer for Dioxus apps
- **reqwest**: HTTP client for API requests and downloads
- **serde**: JSON serialization/deserialization
- **dirs**: Cross-platform directory utilities
- **tokio**: Async runtime for HTTP operations

## API Format

The application expects JSON APIs that return arrays of objects with these fields:
```json
[
  {
    "id": 123456,
    "tags": "tag1 tag2 tag3",
    "width": 1920,
    "height": 1080,
    "file_url": "https://example.com/full_image.jpg",
    "sample_url": "https://example.com/sample.jpg",
    "preview_url": "https://example.com/preview.jpg",
    "rating": "s",
    "score": 42,
    "md5": "abcdef123456",
    "file_size": 1048576
  }
]
```

This format is compatible with popular image board APIs like yande.re, danbooru, and similar services.
