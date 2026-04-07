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
        // Orrbeam brand — definitive palette, see branding/BRAND.md
        // This is the ORRBEAM app chrome. The sunshine/moonlight
        // sub-brands below remain the per-pane semantic colors.
        brand: {
          DEFAULT: "#F8D808",   // Beacon Gold
          primary: "#F8D808",   // Beacon Gold
          highlight: "#887808", // Brass Helm
          mid: "#787808",       // Worn Brass
          deep: "#786808",      // Aged Brass
          shadow: "#685808",    // Dark Patina
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
