(() => {
  const PANEL_W = 380;
  const OPEN = 'baml-ai-open';

  /* quick helpers */
  const css = (s) => {
    const st = document.createElement('style');
    st.textContent = s;
    document.head.appendChild(st);
  };
  const whenReady = (f) =>
    document.readyState === 'loading'
      ? document.addEventListener('DOMContentLoaded', f)
      : f();

  /* --------------------------------------------------------------------- */
  /* Global CSS                                                            */
  /* --------------------------------------------------------------------- */
  css(`
  body.${OPEN}{padding-right:${PANEL_W}px;transition:padding-right .3s cubic-bezier(.4,0,.2,1);overflow-x:hidden;}
  /* Smoothly slide the “On this page” TOC out instead of popping it off‑screen */
  .fern-toc,#fern-toc{
    transition:transform .3s cubic-bezier(.4,0,.2,1),opacity .3s;
  }
  body.${OPEN} .fern-toc,
  body.${OPEN} #fern-toc{
    transform:translateX(100%);
    opacity:0;
    pointer-events:none;
  }
  /* Hide right‑hand “On this page” TOC when the AI panel is open */
  body.${OPEN} nav[aria-label="On this page"],
  body.${OPEN} [data-toc],
  body.${OPEN} .fern-toc,
  body.${OPEN} #fern-toc{
    display:none !important;
  }
  #baml-ai-panel{position:fixed;right:0;width:${PANEL_W}px;background:#fff;border-left:1px solid #e2e8f0;box-shadow:-4px 0 32px rgba(0,0,0,.08);transform:translateX(100%);transition:transform .3s cubic-bezier(.4,0,.2,1);z-index:2000;display:flex;flex-direction:column;font-family:Inter,system-ui,-apple-system,BlinkMacSystemFont,sans-serif;}
  #baml-ai-panel.open{transform:translateX(0);}
  #baml-ai-panel>header{display:flex;align-items:center;justify-content:space-between;height:56px;padding:0 20px;font-size:15px;font-weight:600;background:#7c3aed;color:#fff;}
  #baml-ai-panel button.close{background:none;border:none;font-size:26px;color:#fff;cursor:pointer;opacity:.75;}
  #baml-ai-panel button.close:hover{opacity:1;}
  
  .baml-bubble{max-width:75%;padding:10px 14px;border-radius:14px;font-size:14px;line-height:1.5;margin-bottom:6px;box-shadow:0 2px 6px rgba(0,0,0,.06);}
  .baml-me{align-self:flex-end;background:#7c3aed;color:#fff;}
  .baml-ai{align-self:flex-start;background:#f3f4f6;color:#111827;}
  
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
  /* inline highlight for AI‑navigated sections */
  .ai-hl{background:#fff7a8;padding:0 2px;border-radius:4px;animation:ai-blink 1.6s ease-in-out 2;}
  @keyframes ai-blink{
    0%,100%{background:#fff7a8;}
    50%{background:#ffe949;}
  }
  /* link shown inside AI reply */
  .goto-doc{color:#7c3aed;text-decoration:underline;font-weight:600;}
  `);

  /* --------------------------------------------------------------------- */
  /* Main                                                                  */
  /* --------------------------------------------------------------------- */
  whenReady(() => {
    /* ------------------------------------------------------------ */
    /* Chat helpers                                                */
    /* ------------------------------------------------------------ */
    const sendQuery = (panel, q) => {
      const chatMain = panel.querySelector('main');
      const me = Object.assign(document.createElement('div'), {
        className: 'baml-bubble baml-me',
        textContent: q,
      });
      const ai = Object.assign(document.createElement('div'), {
        className: 'baml-bubble baml-ai',
        textContent: '…thinking',
      });
      chatMain.append(me, ai);
      chatMain.scrollTop = chatMain.scrollHeight;
      return ai; /* give back reference so caller can update it */
    };

    /* wait for Fern search bar */
    const obs = new MutationObserver(() => {
      const old = document.querySelector(
        '[data-search], .fern-search, [class*="search"]',
      );
      if (!old) return;

      /* build custom search */
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
      old.style.display = 'none';
      old.parentNode.insertBefore(wrap, old);

      /* build AI panel */
      const panel = document.createElement('div');
      panel.id = 'baml-ai-panel';
      panel.innerHTML = `
        <header>BAML AI<button class="close" aria-label="Close">×</button></header>
        <main style="flex:1;overflow-y:auto;padding:18px;display:flex;flex-direction:column;"></main>
        <form id="baml-chat-form" style="display:flex;border-top:1px solid #e5e7eb;">
          <input placeholder="Type a question…" style="flex:1;padding:14px;border:none;font-size:14px;outline:none;">
          <button type="submit" style="border:none;padding:0 20px;background:#7c3aed;color:#fff;font-weight:600;cursor:pointer;">Send</button>
        </form>`;
      document.body.appendChild(panel);

      /* position panel below header */
      const measure = () => {
        const h = document.querySelector('header, .fern-header');
        const top = h ? h.getBoundingClientRect().bottom : 0;
        panel.style.top = `${top}px`;
        panel.style.height = `calc(100vh - ${top}px)`;
      };
      measure();
      window.addEventListener('resize', measure);
      window.addEventListener('scroll', measure, { passive: true });

      /* open/close */
      const setOpen = (flag) => {
        panel.classList.toggle('open', flag);
        document.body.classList.toggle(OPEN, flag);
        aiBtn.classList.toggle('open', flag);
        aiBtn.querySelector('span').textContent = flag ? 'Close' : 'Ask';
        aiBtn.querySelector('svg').innerHTML = flag
          ? '<path d="M18 6L6 18M6 6l12 12"/>'
          : '<path d="M12 4v16m8-8H4"/>';
        setTimeout(() => window.dispatchEvent(new Event('resize')), 10);
      };
      const toggle = () => setOpen(!panel.classList.contains('open'));

      /* ask button in bar */
      aiBtn.addEventListener('click', () => {
        const isOpen = panel.classList.contains('open');
        const q = input.value.trim();

        /* toggle close regardless of field contents */
        if (isOpen) {
          setOpen(false);
          return;
        }

        /* opening */
        setOpen(true);
        if (!q) {
          input.focus();
          return;
        }

        const aiBubble = sendQuery(panel, q);
        input.focus();

        /* map query → doc hit */
        const hit = docs.find((d) =>
          d.t.toLowerCase().includes(q.toLowerCase()),
        );

        setTimeout(() => {
          if (hit) {
            aiBubble.innerHTML = `Here’s what you need: <a class="goto-doc" href="${hit.u}">open the “${hit.t}” docs&nbsp;»</a>`;
            navigateToDoc(hit, q);
          } else {
            aiBubble.textContent = `I couldn’t find anything on “${q}” yet.`;
          }
          panel.querySelector('main').scrollTop =
            panel.querySelector('main').scrollHeight;
        }, 800); /* think for effect */

        /* — simple doc‑navigation demo — */
        // const hit = docs.find((d) =>
        //   d.t.toLowerCase().includes(q.toLowerCase()),
        // );
        // if (hit) {
        //   localStorage.setItem(
        //     'baml-hl',
        //     JSON.stringify({ url: hit.u, text: q }),
        //   );
        //   setTimeout(() => {
        //     window.location.href = hit.u;
        //   }, 500); /* allow animation to finish */
        // }
      });

      /* chat form */
      panel.querySelector('#baml-chat-form').addEventListener('submit', (e) => {
        e.preventDefault();
        const q = e.target.querySelector('input').value.trim();
        if (q) {
          sendQuery(panel, q);
          e.target.querySelector('input').value = '';
        }
      });
      panel
        .querySelector('.close')
        .addEventListener('click', () => setOpen(false));

      /* quick ask inside dropdown */
      const quickAsk = (q) => {
        setOpen(true);
        sendQuery(panel, q);
        dd.classList.remove('open');

        /* answer + navigation for quick‑ask */
        const aiBubble = sendQuery(panel, q);
        const hit = docs.find((d) =>
          d.t.toLowerCase().includes(q.toLowerCase()),
        );
        setTimeout(() => {
          if (hit) {
            aiBubble.innerHTML = `Jumping to: <a class="goto-doc" href="${hit.u}">${hit.t}</a>`;
            navigateToDoc(hit, q);
          } else {
            aiBubble.textContent = `No docs found for “${q}”.`;
          }
        }, 800);
      };

      /* search data + logic */
      /* minimal doc db – extend as needed */
      const docs = [
        {
          t: 'Getting Started with BAML',
          u: '/guide/introduction/what-is-baml',
          x: 'Overview of core concepts',
          sel: 'h1' /* selector where highlight should start */,
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
      const hi = (s, q) =>
        s.replace(new RegExp(`(${q})`, 'ig'), '<mark>$1</mark>');
      const render = (q) => {
        dd.innerHTML = '';
        if (!q) {
          dd.classList.remove('open');
          return;
        }
        const ask = document.createElement('div');
        ask.className = 'baml-item ai';
        ask.innerHTML = `Ask “${hi(q, q)}”`;
        ask.onclick = () => quickAsk(q);
        dd.append(ask);
        for (const d of docs
          .filter(
            (d) =>
              d.t.toLowerCase().includes(q) || d.x.toLowerCase().includes(q),
          )
          .slice(0, 10) || [{ t: `No docs for “${q}”`, u: '#', x: '' }]) {
          const a = document.createElement('a');
          a.href = d.u;
          a.className = 'baml-item';
          a.innerHTML = `<span class="title">${hi(d.t, q)}</span>${d.x ? `<span class="descr">${hi(d.x, q)}</span>` : ''}`;
          dd.append(a);
        }
        dd.classList.add('open');
      };
      let tm;
      input.addEventListener('input', (e) => {
        clearTimeout(tm);
        tm = setTimeout(() => render(e.target.value.trim().toLowerCase()), 100);
      });
      input.addEventListener('focus', () => {
        input.value.trim() && render(input.value.trim().toLowerCase());
      });
      document.addEventListener('click', (e) => {
        !wrap.contains(e.target) && dd.classList.remove('open');
      });
      input.addEventListener('keydown', (e) => {
        e.key === 'Escape' && dd.classList.remove('open');
      });

      obs.disconnect();
    });
    obs.observe(document.body, { subtree: true, childList: true });
  });

  /* ------------------------------------------------------------------ */
  /* Highlight previously‑requested text after navigation               */
  /* ------------------------------------------------------------------ */
  highlightFromStore();

  /* ------------------------------------------------------------------ */
  /* Helper: navigate without full reload & trigger highlight later     */
  /* ------------------------------------------------------------------ */
  function navigateToDoc(hit, q) {
    /* save highlight data */
    localStorage.setItem(
      'baml-hl',
      JSON.stringify({ url: hit.u, text: q, sel: hit.sel }),
    );

    /* PJAX fetch → swap main content to avoid full reload  */
    fetch(hit.u, { credentials: 'same-origin' })
      .then((r) => r.text())
      .then((html) => {
        const temp = new DOMParser().parseFromString(html, 'text/html');
        const fresh = temp.querySelector('main') || temp.body;
        const live = document.querySelector('main') || document.body;
        live.replaceWith(fresh);
        history.pushState({ pjax: true }, '', hit.u);
        highlightFromStore();
        window.dispatchEvent(new Event('resize')); /* let layouts recalc */
      })
      .catch(() => window.location.assign(hit.u));
  }

  window.addEventListener('popstate', (e) => {
    if (e.state?.pjax) highlightFromStore();
  });

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
        const n = walker.currentNode;
        if (re.test(n.textContent)) {
          const span = document.createElement('span');
          span.innerHTML = n.textContent.replace(
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
})();
