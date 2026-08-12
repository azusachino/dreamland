import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";

export default defineConfig({
  plugins: [react(), tailwindcss()],
  clearScreen: false,
  server: {
    host: "127.0.0.1",
    port: 1420,
    strictPort: true,
  },
  build: {
    rollupOptions: {
      output: {
        manualChunks(id) {
          if (!id.includes("node_modules")) return;
          if (id.includes("@mui")) return "mui";
          if (id.includes("@tanstack")) return "query";
          if (id.includes("react") || id.includes("scheduler")) return "react";
          if (id.includes("@tauri-apps")) return "tauri";
        },
      },
    },
  },
});
