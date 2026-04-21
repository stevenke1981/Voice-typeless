import type { Config } from "tailwindcss";

export default {
  content: ["./src/**/*.{html,js,svelte,ts}"],
  darkMode: "class",
  theme: {
    extend: {
      colors: {
        vtl: {
          teal: "#00E6C8",
          indigo: "#5B4EFF",
          green: "#22FFAA",
          "bg-dark": "#0F0F12",
          "bg-dark-2": "#1F1F25",
          "bg-light": "#FAFAFC",
          "text-dark": "#E5E5E8",
          "text-light": "#1A1A1F",
          gray: "#A0A0A8",
          border: "#4A4A52",
        },
      },
      fontFamily: {
        sans: ["Inter", "system-ui", "sans-serif"],
        mono: ["JetBrains Mono", "Fira Code", "monospace"],
      },
    },
  },
  plugins: [],
} satisfies Config;
