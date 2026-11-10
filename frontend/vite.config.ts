import react from "@vitejs/plugin-react";
import { defineConfig } from "vite";

// In dev, API calls go to the Rust backend on :3000.
export default defineConfig({
  plugins: [react()],
  server: {
    proxy: {
      "/api": "http://localhost:3000",
      "/health": "http://localhost:3000",
    },
  },
});
