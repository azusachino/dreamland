# Application Demonstration

This document shows the key components and functionality of the Dreamland Image Viewer application.

## Main Application Window

The application consists of three main sections:

### 1. Header Section (src/ui/header.rs)
```
┌─────────────────────────────────────────────────────────────────┐
│ Dreamland Image Viewer                    [Load Images] [Settings] │
└─────────────────────────────────────────────────────────────────┘
```

Features:
- Application title on the left
- "Load Images" button to fetch from API (shows "Loading..." when active)
- "Settings" button to open configuration dialog
- Dark header with white text for visual contrast

### 2. Gallery Section (src/ui/gallery.rs)
```
┌─────────────┬─────────────┬─────────────┬─────────────┐
│ ┌─────────┐ │ ┌─────────┐ │ ┌─────────┐ │ ┌─────────┐ │
│ │ Preview │ │ │ Preview │ │ │ Preview │ │ │ Preview │ │
│ │ Image   │ │ │ Image   │ │ │ Image   │ │ │ Image   │ │
│ │         │ │ │         │ │ │         │ │ │         │ │
│ └─────────┘ │ └─────────┘ │ └─────────┘ │ └─────────┘ │
│ Tags: ...   │ Tags: ...   │ Tags: ...   │ Tags: ...   │
│ Rating: s   │ Rating: q   │ Rating: e   │ Rating: s   │
│ [Download]  │ [Download]  │ [Download]  │ [Download]  │
├─────────────┼─────────────┼─────────────┼─────────────┤
│     ...     │     ...     │     ...     │     ...     │
└─────────────┴─────────────┴─────────────┴─────────────┘
```

Features:
- Responsive grid layout (auto-adjusts columns based on window width)
- Image cards with preview thumbnails
- Resolution display overlay (e.g., "1920x1080")
- Truncated tags display
- Rating information
- Individual download buttons for each image
- Lazy loading for performance

### 3. Pagination Controls
```
                [Previous] Page 2 [Next]
```

### 4. Settings Dialog (src/ui/settings.rs)
```
┌─────────────────────────────────────────────────────┐
│                    Settings                         │
│                                                     │
│ API URL:                                            │
│ ┌─────────────────────────────────────────────────┐ │
│ │ https://yande.re/post.json                      │ │
│ └─────────────────────────────────────────────────┘ │
│                                                     │
│ Download Path:                                      │
│ ┌─────────────────────────────────────────────────┐ │
│ │ /home/user/Downloads/dreamland_images           │ │
│ └─────────────────────────────────────────────────┘ │
│                                                     │
│                               [Cancel] [Save]       │
└─────────────────────────────────────────────────────┘
```

## Application Flow

1. **Startup**: 
   - Loads saved configuration from config file
   - Shows empty gallery with message "Click 'Load Images' to fetch images from the API"

2. **Loading Images**:
   - User clicks "Load Images"
   - Button changes to "Loading..." and becomes disabled
   - HTTP request made to configured API URL
   - Images populate in gallery grid

3. **Browsing Images**:
   - User scrolls through gallery
   - Can click pagination buttons to load more pages
   - Each image shows preview, metadata, and download option

4. **Downloading Images**:
   - User clicks "Download" on any image card
   - High-quality image downloaded to configured path
   - Creates directory if it doesn't exist
   - Console shows download progress

5. **Settings Management**:
   - User clicks "Settings" to open dialog
   - Can modify API URL and download path
   - Settings saved to config file when "Save" clicked
   - Changes applied immediately

## Technical Implementation

### State Management
- Global app state managed through Dioxus context
- Reactive updates when data changes
- Async operations for API calls and downloads

### Error Handling
- Network errors logged to console
- Failed downloads reported with error messages
- Invalid configuration falls back to defaults

### Performance Features
- Image lazy loading for smooth scrolling
- Async HTTP requests don't block UI
- Minimal memory usage with preview images

### Cross-Platform Support
- Works on Windows, macOS, and Linux
- Uses platform-appropriate config directories
- Responsive design adapts to different screen sizes

This application provides a complete solution for browsing and downloading images from JSON APIs with a modern, responsive user interface.