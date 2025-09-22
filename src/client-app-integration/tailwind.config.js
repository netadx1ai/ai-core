/** @type {import('tailwindcss').Config} */
module.exports = {
    content: ["./index.html", "./src/**/*.{js,ts,jsx,tsx}"],
    theme: {
        extend: {
            colors: {
                primary: {
                    50: "#eff6ff",
                    500: "#3b82f6",
                    600: "#2563eb",
                    700: "#1d4ed8",
                },
                secondary: {
                    500: "#64748b",
                    600: "#475569",
                    700: "#334155",
                },
                success: {
                    500: "#10b981",
                    600: "#059669",
                },
                error: {
                    500: "#ef4444",
                    600: "#dc2626",
                },
                warning: {
                    500: "#f59e0b",
                    600: "#d97706",
                },
            },
            fontFamily: {
                sans: ["Inter", "system-ui", "sans-serif"],
            },
            animation: {
                "spin-slow": "spin 2s linear infinite",
                "pulse-fast": "pulse 1s ease-in-out infinite",
                "bounce-subtle": "bounce 2s infinite",
            },
            boxShadow: {
                glow: "0 0 20px rgba(59, 130, 246, 0.3)",
                "glow-success": "0 0 20px rgba(16, 185, 129, 0.3)",
                "glow-error": "0 0 20px rgba(239, 68, 68, 0.3)",
            },
        },
    },
    plugins: [],
};
