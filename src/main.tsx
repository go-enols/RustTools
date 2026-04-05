import React from 'react';
import ReactDOM from 'react-dom/client';
import App from './App';
import './styles/index.css';
import './styles/hub.css';

// Initialize theme from localStorage
const savedSettings = localStorage.getItem('settings');
if (savedSettings) {
  try {
    const { theme } = JSON.parse(savedSettings);
    if (theme === 'light') {
      document.documentElement.setAttribute('data-theme', 'light');
    }
  } catch {}
}

ReactDOM.createRoot(document.getElementById('root')!).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>
);
