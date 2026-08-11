# Application Demonstration

Dreamland opens as a Tauri desktop window containing a React/Vite gallery.

## Main Window

- The toolbar contains tag search, refresh, and **Settings** actions.
- The gallery displays preview images, dimensions, tags, ratings, and download
  buttons in a responsive grid.
- Latest and tag-search feeds use page controls; popular feeds use Yande's
  fixed day/week/month windows.
- Settings edits the local download directory, content policy, and connection
  resilience. The Yande endpoint is owned by its adapter and is not exposed.

## Application Flow

1. Tauri loads the React frontend and Rust loads the saved configuration.
2. The active feed invokes a typed Rust site command for the current page or
   popularity window.
3. Image cards render the returned metadata and lazy-loaded previews.
4. **Download** invokes Rust to fetch, name, and save the full image.
5. Settings are persisted in the platform user configuration directory.
