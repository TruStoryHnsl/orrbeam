import type { Config } from "tailwindcss";

export default {
  content: ["./index.html", "./src/**/*.{ts,tsx}"],
  theme: {
    extend: {
      colors: {
        surface: {
          0: "#0a0a0a",
          1: "#141414",
          2: "#1e1e1e",
          3: "#282828",
          4: "#323232",
        },
        sunshine: {
          DEFAULT: "#ff8c00",
          dim: "#cc7000",
          bright: "#ffaa33",
        },
        moonlight: {
          DEFAULT: "#6366f1",
          dim: "#4f46e5",
          bright: "#818cf8",
        },
        accent: "#ff8c00",
      },
    },
  },
  plugins: [],
} satisfies Config;
