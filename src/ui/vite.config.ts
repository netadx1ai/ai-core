import react from "@vitejs/plugin-react";
import { defineConfig } from "vite";

// https://vite.dev/config/
export default defineConfig({
    plugins: [react()],

    // Environment variables configuration
    envPrefix: "VITE_",

    // Development server configuration
    server: {
        port: 5173,
        host: true,
        cors: true,
        proxy: {
            // Proxy API calls to avoid CORS issues in development
            "/api": {
                target: "http://localhost:8801",
                changeOrigin: true,
                rewrite: (path) => path.replace(/^\/api/, "/v1"),
            },
            // Direct proxies to AI-CORE services for health checks
            "/health/federation": {
                target: "http://localhost:8801",
                changeOrigin: true,
                rewrite: () => "/health",
            },
            "/health/intent-parser": {
                target: "http://localhost:8802",
                changeOrigin: true,
                rewrite: () => "/health",
            },
            "/health/mcp-manager": {
                target: "http://localhost:8804",
                changeOrigin: true,
                rewrite: () => "/health",
            },
            "/health/mcp-proxy": {
                target: "http://localhost:8803",
                changeOrigin: true,
                rewrite: () => "/health",
            },
        },
    },

    // Build configuration
    build: {
        outDir: "dist",
        sourcemap: true,
        rollupOptions: {
            output: {
                manualChunks: {
                    vendor: ["react", "react-dom"],
                    router: ["react-router-dom"],
                    ui: ["@heroicons/react"],
                },
            },
        },
    },

    // CSS configuration
    css: {
        devSourcemap: true,
    },

    // Define global constants
    define: {
        __APP_VERSION__: JSON.stringify(process.env.npm_package_version || "1.0.0"),
        __BUILD_TIME__: JSON.stringify(new Date().toISOString()),
    },

    // Resolve configuration
    resolve: {
        alias: {
            "@": "/src",
        },
    },
});
