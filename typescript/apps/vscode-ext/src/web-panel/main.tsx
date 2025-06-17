console.log('main.tsx');
import React from 'react';
import { createRoot } from 'react-dom/client';
import App from './App';
// import { AppStateProvider } from './shared/AppStateContext'
import '@baml/ui/globals.css';

// Create a root.
const container = document.getElementById('root');
if (!container) {
  throw new Error('No container found');
}
const root = createRoot(container);

// Initial render: Render your app inside the AppStateProvider.
root.render(
  <React.StrictMode>
    {/* <AppStateProvider> */}
    <App />
    {/* </AppStateProvider> */}
  </React.StrictMode>,
);
