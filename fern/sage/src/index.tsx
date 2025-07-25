import React from 'react';
import { createRoot } from 'react-dom/client';
import { Provider } from 'jotai';
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

// Docs data from original custom.js
const docs = [
  {
    t: 'Getting Started with BAML',
    u: '/guide/introduction/what-is-baml',
    x: 'Overview of core concepts',
    sel: 'h1',
  },
  {
    t: 'Python Installation',
    u: '/guide/installation-language/python',
    x: 'Quick‑start for Python devs',
    sel: 'article',
  },
  {
    t: 'TypeScript Installation',
    u: '/guide/installation-language/typescript',
    x: 'Install & set up with TS',
    sel: 'article',
  },
  {
    t: 'Functions',
    u: '/ref/baml/function',
    x: 'Reference for BAML functions',
  },
  { t: 'Classes', u: '/ref/baml/class', x: 'Data structures & classes' },
];

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

// Add global CSS from original custom.js with sidebar panel styling
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
#baml-search-wrap{display:flex;align-items:center;max-width:640px;width:100%;position:relative;}
#baml-search-input{flex:1;padding:10px 46px 10px 16px;border:1.5px solid #e2e8f0;border-radius:10px;font-size:14px;outline:none;}
#baml-ai-btn{position:absolute;right:3px;top:4px;bottom:4px;display:flex;align-items:center;gap:6px;background:#7c3aed;border:none;border-radius:6px;color:#fff;font-weight:600;font-size:13px;line-height:1;padding:0 12px;cursor:pointer;transition:background .2s,transform .2s;}
#baml-ai-btn:hover{background:#6320df;}
#baml-ai-btn.open{background:#edf2ff;color:#7c3aed;border:1px solid #7c3aed;}
#baml-ai-btn span{color:inherit;}
#baml-ai-btn svg{width:16px;height:16px;stroke:currentColor;fill:none;stroke-width:2;}
.baml-search-dd{position:absolute;inset:auto 0 0 0;transform:translateY(100%);background:#fff;border:1px solid #e5e7eb;border-radius:10px;box-shadow:0 12px 32px rgba(0,0,0,.12);z-index:1500;max-height:420px;overflow-y:auto;display:none;}
.baml-search-dd.open{display:block;}
.baml-item{padding:12px 18px;display:block;text-decoration:none;color:#111827;transition:background .15s;}
.baml-item:hover{background:#f9fafb;}
.baml-item+.baml-item{border-top:1px solid #f3f4f6;}
.baml-item .title{font-weight:600;font-size:14px;display:block;}
.baml-item .descr{font-size:12.5px;color:#6b7280;}
.baml-item.ai{background:#f8f5ff;color:#7c3aed;}
.baml-item.ai:hover{background:#f3efff;}
mark{background:transparent;color:#ec4899;font-weight:700;}
.ai-hl{background:#fff7a8;padding:0 2px;border-radius:4px;animation:ai-blink 1.6s ease-in-out 2;}
@keyframes ai-blink{
  0%,100%{background:#fff7a8;}
  50%{background:#ffe949;}
}
.goto-doc{color:#7c3aed;text-decoration:underline;font-weight:600;}
`);

// Search interface integration
function initializeSearchInterface() {
  const obs = new MutationObserver(() => {
    const old = document.querySelector(
      '[data-search], .fern-search, [class*="search"]',
    );
    if (!old) return;

    // Build custom search interface
    const wrap = document.createElement('div');
    wrap.id = 'baml-search-wrap';
    const input = document.createElement('input');
    input.id = 'baml-search-input';
    input.placeholder = 'Search BAML docs…';
    const aiBtn = document.createElement('button');
    aiBtn.id = 'baml-ai-btn';
    aiBtn.innerHTML =
      '<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M12 4v16m8-8H4"/></svg><span>Ask</span>';
    const dd = document.createElement('div');
    dd.className = 'baml-search-dd';
    wrap.append(input, aiBtn, dd);

    // Hide original search and replace with custom
    if (old.parentNode) {
      (old as HTMLElement).style.display = 'none';
      old.parentNode.insertBefore(wrap, old);
    }

    // Initialize React chatbot with sidebar panel functionality
    let chatbotRoot: any = null;
    let isOpen = false;
    const AUTO_MOUNT_SIDEBAR = true;

    const setOpen = (flag: boolean) => {
      isOpen = flag;
      document.body.classList.toggle(OPEN, flag);
      aiBtn.classList.toggle('open', flag);
      (aiBtn.querySelector('span') as HTMLElement).textContent = flag
        ? 'Close'
        : 'Ask';
      (aiBtn.querySelector('svg') as SVGElement).innerHTML = flag
        ? '<path d="M18 6L6 18M6 6l12 12"/>'
        : '<path d="M12 4v16m8-8H4"/>';

      if (chatbotRoot) {
        chatbotRoot.render(
          <Provider>
            <ChatBot isOpen={flag} onClose={() => setOpen(false)} />
          </Provider>,
        );
      }

      setTimeout(() => window.dispatchEvent(new Event('resize')), 10);
    };

    const initChatbot = () => {
      if (!chatbotRoot) {
        const rootElement = document.createElement('div');
        rootElement.id = 'fern-chatbot-root';
        document.body.appendChild(rootElement);
        chatbotRoot = createRoot(rootElement);
        // Initial render with closed state
        chatbotRoot.render(
          <Provider>
            <ChatBot isOpen={false} onClose={() => setOpen(false)} />
          </Provider>,
        );
      }
      setOpen(true);
    };

    // Auto-mount sidebar if AUTO_MOUNT_SIDEBAR is true
    if (AUTO_MOUNT_SIDEBAR) {
      initChatbot();
    }

    // Handle AI button clicks
    aiBtn.addEventListener('click', () => {
      const q = input.value.trim();

      // Toggle close if already open
      if (isOpen) {
        setOpen(false);
        return;
      }

      // Opening the chatbot
      initChatbot();
      if (!q) {
        input.focus();
        return;
      }

      // TODO: Send initial query to chatbot when that functionality is implemented
    });

    // Search dropdown functionality
    const hi = (s: string, q: string) =>
      s.replace(new RegExp(`(${q})`, 'ig'), '<mark>$1</mark>');

    const render = (q: string) => {
      dd.innerHTML = '';
      if (!q) {
        dd.classList.remove('open');
        return;
      }

      const ask = document.createElement('div');
      ask.className = 'baml-item ai';
      ask.innerHTML = `Ask "${hi(q, q)}"`;
      ask.onclick = () => {
        initChatbot();
        dd.classList.remove('open');
      };
      dd.append(ask);

      for (const d of docs
        .filter(
          (d) => d.t.toLowerCase().includes(q) || d.x.toLowerCase().includes(q),
        )
        .slice(0, 10) || [{ t: `No docs for "${q}"`, u: '#', x: '' }]) {
        const a = document.createElement('a');
        a.href = d.u;
        a.className = 'baml-item';
        a.innerHTML = `<span class="title">${hi(d.t, q)}</span>${d.x ? `<span class="descr">${hi(d.x, q)}</span>` : ''}`;
        dd.append(a);
      }
      dd.classList.add('open');
    };

    let tm: any;
    input.addEventListener('input', (e) => {
      clearTimeout(tm);
      tm = setTimeout(
        () => render((e.target as HTMLInputElement).value.trim().toLowerCase()),
        100,
      );
    });

    input.addEventListener('focus', () => {
      input.value.trim() && render(input.value.trim().toLowerCase());
    });

    document.addEventListener('click', (e) => {
      if (!wrap.contains(e.target as Node)) dd.classList.remove('open');
    });

    input.addEventListener('keydown', (e) => {
      if (e.key === 'Escape') dd.classList.remove('open');
    });

    obs.disconnect();
  });
  obs.observe(document.body, { subtree: true, childList: true });
}

// Global initialization function that Fern can call
declare global {
  interface Window {
    initFernChatbot: (options?: { apiEndpoint?: string }) => void;
    navigateToDoc: (hit: { u: string; t: string; sel?: string }, q: string) => void;
  }
}

// Expose navigateToDoc globally
window.navigateToDoc = navigateToDoc;

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
