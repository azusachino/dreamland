# Application Demonstration

Dreamland opens as a Tauri desktop window containing a React/Vite gallery.

## Main Window

- The dark toolbar contains **Load Images** and **Settings** actions.
- The gallery displays preview images, dimensions, tags, ratings, and download
  buttons in a responsive grid.
- Previous/Next controls page through API results.
- Settings edits the API URL and local download directory.

## Application Flow

1. Tauri loads the React frontend and Rust loads the saved configuration.
2. **Load Images** invokes the Rust API command for the current page.
3. Image cards render the returned metadata and lazy-loaded previews.
4. **Download** invokes Rust to fetch, name, and save the full image.
5. Settings are persisted in the platform user configuration directory.
