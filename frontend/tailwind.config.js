/** @type {import('tailwindcss').Config} */
export default {
  content: ["./index.html", "./src/**/*.{js,ts,jsx,tsx}"],
  darkMode: "class",
  theme: {
    extend: {
      colors: {
        bg: {
          DEFAULT: "#f5f5f7",
          dark: "#000000",
        },
        surface: {
          DEFAULT: "#ffffff",
          dark: "#1c1c1e",
          hover: {
            DEFAULT: "#fafafa",
            dark: "#2c2c2e",
          },
        },
        brand: {
          primary: "#007aff",
          success: "#34c759",
          warning: "#ff9500",
          danger: "#ff3b30",
          purple: "#af52de",
          teal: "#5ac8fa",
          indigo: "#5856d6",
          pink: "#ff375f",
        },
      },
    },
  },
  plugins: [],
};
