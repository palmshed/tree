import { defineConfig } from "vite";

export default defineConfig({
  server: {
    proxy: {
      "/repositories": "http://127.0.0.1:8080",
      "/health": "http://127.0.0.1:8080",
      "/users": "http://127.0.0.1:8080",
      "/api": "http://127.0.0.1:8080",
    },
  },
});
