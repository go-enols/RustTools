/** @type {import('tailwindcss').Config} */
export default {
  content: ['./index.html', './src/**/*.{js,ts,jsx,tsx}'],
  theme: {
    extend: {
      colors: {
        bg: {
          primary: '#0F0F0F',
          sidebar: '#1A1A1A',
          card: '#242424',
        },
        accent: {
          primary: '#00D9FF',
          secondary: '#FF6B35',
          success: '#00FF88',
        },
        text: {
          primary: '#FFFFFF',
          secondary: '#8A8A8A',
        },
        border: '#333333',
      },
      fontFamily: {
        sans: ['Inter', 'Microsoft YaHei', 'PingFang SC', 'system-ui', 'sans-serif'],
        mono: ['JetBrains Mono', 'monospace'],
      },
    },
  },
  plugins: [],
};
