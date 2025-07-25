import { createRoot } from 'react-dom/client';
import AlgoliaSearch from './AlgoliaSearch';
import ChatBot from './ChatBot';

// Constants from original custom.js
const PANEL_W = 380;
const OPEN = 'baml-ai-open';

// Helper functions from original custom.js
const css = (s: string) => {
  const st = document.createElement('style');
  st.textContent = s;
  document.head.appendChild(st);
};

const whenReady = (f: () => void) =>
  document.readyState === 'loading'
    ? document.addEventListener('DOMContentLoaded', f)
    : f();

// Highlight functionality from original custom.js
function navigateToDoc(hit: any, q: string) {
  localStorage.setItem(
    'baml-hl',
    JSON.stringify({ url: hit.u, text: q, sel: hit.sel }),
  );

  fetch(hit.u, { credentials: 'same-origin' })
    .then((r) => r.text())
    .then((html) => {
      const temp = new DOMParser().parseFromString(html, 'text/html');
      const fresh = temp.querySelector('main') || temp.body;
      const live = document.querySelector('main') || document.body;
      if (fresh && live) {
        live.replaceWith(fresh);
        history.pushState({ pjax: true }, '', hit.u);
        highlightFromStore();
        window.dispatchEvent(new Event('resize'));
      }
    })
    .catch(() => window.location.assign(hit.u));
}

function highlightFromStore() {
  const data = JSON.parse(localStorage.getItem('baml-hl') || 'null');
  if (!data || !location.pathname.endsWith(data.url)) return;
  localStorage.removeItem('baml-hl');

  whenReady(() => {
    const scope =
      document.querySelector(data.sel) ||
      document.querySelector('main') ||
      document.body;
    if (!scope) return;
    const walker = document.createTreeWalker(scope, NodeFilter.SHOW_TEXT);
    const re = new RegExp(
      data.text.replace(/[.*+?^${}()|[\]\\]/g, '\\$&'),
      'gi',
    );
    let added = 0;
    while (walker.nextNode() && added < 20) {
      const n = walker.currentNode as Text;
      if (re.test(n.textContent || '')) {
        const span = document.createElement('span');
        span.innerHTML = (n.textContent || '').replace(
          re,
          (m) => `<mark class="ai-hl">${m}</mark>`,
        );
        n.replaceWith(...span.childNodes);
        added++;
      }
    }
    const first = document.querySelector('.ai-hl');
    first?.scrollIntoView({ behavior: 'smooth', block: 'center' });
  });
}

// Updated global CSS to integrate with Algolia search styling
css(`
body.${OPEN}{padding-right:${PANEL_W}px;transition:padding-right .3s cubic-bezier(.4,0,.2,1);overflow-x:hidden;}
/* Smoothly slide the "On this page" TOC out instead of popping it off‑screen */
.fern-toc,#fern-toc{
  transition:transform .3s cubic-bezier(.4,0,.2,1),opacity .3s;
}
body.${OPEN} .fern-toc,
body.${OPEN} #fern-toc{
  transform:translateX(100%);
  opacity:0;
  pointer-events:none;
}
/* Hide right‑hand "On this page" TOC when the AI panel is open */
body.${OPEN} nav[aria-label="On this page"],
body.${OPEN} [data-toc],body.${OPEN} .fern-toc,body.${OPEN} #fern-toc{
  display:none !important;
}

/* Cleaned up styling */

/* Search result highlighting */
.ai-hl{background:#fff7a8;padding:0 2px;border-radius:4px;animation:ai-blink 1.6s ease-in-out 2;}
@keyframes ai-blink{
  0%,100%{background:#fff7a8;}
  50%{background:#ffe949;}
}
.goto-doc{color:#7c3aed;text-decoration:underline;font-weight:600;}
`);

// Search interface integration with Algolia
function initializeSearchInterface() {
  const obs = new MutationObserver(() => {
    const old = document.querySelector(
      '[data-search], .fern-search, [class*="search"]',
    );
    if (!old) return;

    // Build custom search interface with Algolia integration
    const wrap = document.createElement('div');
    wrap.id = 'baml-search-wrap';
    wrap.style.cssText = 'max-width: 640px; width: 100%; position: relative;';

    const algoliaContainer = document.createElement('div');
    algoliaContainer.id = 'baml-algolia-container';
    algoliaContainer.style.cssText = 'width: 100%; position: relative;';

    wrap.append(algoliaContainer);

    // Hide original search and replace with custom
    if (old.parentNode) {
      (old as HTMLElement).style.display = 'none';
      old.parentNode.insertBefore(wrap, old);
    }

    // Render Algolia search component
    const algoliaRoot = createRoot(algoliaContainer);

    // AI functionality callback
    const handleAskAI = (query: string) => {
      console.log('Ask AI clicked with query:', query);

      // Open the AI chatbot first
      initChatbot();

      // Store AI context for the chatbot with just the query
      localStorage.setItem(
        'baml-ai-context',
        JSON.stringify({
          query: query,
          timestamp: Date.now(),
        }),
      );
    };

    // Initialize React chatbot with sidebar panel functionality
    let chatbotRoot: any = null;
    let isOpen = false;
    const AUTO_MOUNT_SIDEBAR = false; // Changed to false to prevent auto-opening

    const setOpen = (flag: boolean) => {
      isOpen = flag;
      document.body.classList.toggle(OPEN, flag);

      if (chatbotRoot) {
        chatbotRoot.render(
          <ChatBot isOpen={flag} onClose={() => setOpen(false)} />,
        );
      }

      setTimeout(() => window.dispatchEvent(new Event('resize')), 10);
    };

    const toggleChatbot = () => {
      if (!chatbotRoot) {
        const rootElement = document.createElement('div');
        rootElement.id = 'fern-chatbot-root';
        document.body.appendChild(rootElement);
        chatbotRoot = createRoot(rootElement);
        // Initial render with closed state
        chatbotRoot.render(
          <ChatBot isOpen={false} onClose={() => setOpen(false)} />,
        );
      }
      setOpen(!isOpen); // Toggle the current state
    };

    algoliaRoot.render(
      <AlgoliaSearch onAskAI={handleAskAI} onToggleAI={toggleChatbot} />,
    );

    const initChatbot = () => {
      if (!chatbotRoot) {
        const rootElement = document.createElement('div');
        rootElement.id = 'fern-chatbot-root';
        document.body.appendChild(rootElement);
        chatbotRoot = createRoot(rootElement);
        // Initial render with closed state
        chatbotRoot.render(
          <ChatBot isOpen={false} onClose={() => setOpen(false)} />,
        );
      }
      setOpen(true); // Always open when called from Ask AI
    };

    // Don't auto-mount sidebar - only open when explicitly requested
    // if (AUTO_MOUNT_SIDEBAR) {
    //   initChatbot();
    // }

    obs.disconnect();
  });
  obs.observe(document.body, { subtree: true, childList: true });
}

// Global initialization function that Fern can call
declare global {
  interface Window {
    initFernChatbot: (options?: { apiEndpoint?: string }) => void;
  }
}

window.initFernChatbot = (options = {}) => {
  // Initialize search interface integration
  initializeSearchInterface();

  // Handle highlight functionality
  highlightFromStore();

  window.addEventListener('popstate', (e) => {
    if ((e.state as any)?.pjax) highlightFromStore();
  });
};

// Auto-initialize if running in a browser environment
if (typeof window !== 'undefined') {
  whenReady(() => {
    window.initFernChatbot();
  });
}
