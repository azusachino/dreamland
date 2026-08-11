import React from "react";
import ReactDOM from "react-dom/client";
import { HashRouter, Route, Routes } from "react-router-dom";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { isTauri } from "@tauri-apps/api/core";
import App from "./App";
import "./styles.css";

if (isTauri()) {
  document.documentElement.dataset.platform = /Mac/.test(navigator.platform) ? "macos" : "native";
}

const queryClient = new QueryClient();
const root = document.getElementById("root");

if (!root) {
  throw new Error("Root element is missing");
}

ReactDOM.createRoot(root).render(
  <React.StrictMode>
    <QueryClientProvider client={queryClient}>
      <HashRouter>
        <Routes>
          <Route path="*" element={<App />} />
        </Routes>
      </HashRouter>
    </QueryClientProvider>
  </React.StrictMode>,
);
