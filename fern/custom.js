(() => {
  // Only create chat bubble if window width > 700px
  if (window.innerWidth <= 700) {
    console.log('Window width <= 700px, not showing chat bubble');
    return;
  }

  // Create the chat bubble
  const chatBubble = document.createElement('div');
  chatBubble.style.position = 'fixed';
  chatBubble.style.bottom = '20px';
  chatBubble.style.right = '20px';
  chatBubble.style.width = '60px';
  chatBubble.style.height = '60px';
  chatBubble.style.backgroundColor = '#6025d1';
  chatBubble.style.borderRadius = '50%';
  chatBubble.style.boxShadow = '0 4px 8px rgba(0, 0, 0, 0.2)';
  chatBubble.style.cursor = 'pointer';
  chatBubble.style.zIndex = '1000';
  chatBubble.style.transition = 'all 0.3s ease';
  chatBubble.style.display = 'flex';
  chatBubble.style.alignItems = 'center';
  chatBubble.style.justifyContent = 'center';

  // Add magical neon border with animation
  chatBubble.style.border = '2px solid transparent';
  chatBubble.style.backgroundClip = 'padding-box';

  // Create a pseudo-element for the animated border
  const borderAnimation = document.createElement('div');
  borderAnimation.style.position = 'absolute';
  borderAnimation.style.top = '-4px';
  borderAnimation.style.left = '-4px';
  borderAnimation.style.right = '-4px';
  borderAnimation.style.bottom = '-4px';
  borderAnimation.style.borderRadius = '50%';
  borderAnimation.style.zIndex = '-1';
  borderAnimation.style.background =
    'linear-gradient(45deg, #ff00cc, #6025d1, #00ccff, #6025d1)';
  borderAnimation.style.backgroundSize = '200% 200%';
  borderAnimation.style.filter = 'blur(2px)';
  borderAnimation.style.opacity = '0.7';
  borderAnimation.style.transition = 'opacity 0.3s ease';

  // Add animation using CSS keyframes
  const style = document.createElement('style');
  style.textContent = `
    @keyframes magicBorder {
      0% { background-position: 0% 50%; }
      50% { background-position: 100% 50%; }
      100% { background-position: 0% 50%; }
    }
  `;
  document.head.appendChild(style);

  borderAnimation.style.animation = 'magicBorder 20s ease infinite';
  chatBubble.appendChild(borderAnimation);

  // Add chat icon
  const chatIcon = document.createElement('div');
  chatIcon.innerHTML = `
  <svg xmlns="http://www.w3.org/2000/svg" width="32" height="32" viewBox="0 0 24 24" fill="none" stroke="white" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="lucide lucide-bot"><path d="M12 8V4H8"/><rect width="16" height="12" x="4" y="8" rx="2"/><path d="M2 14h2"/><path d="M20 14h2"/><path d="M15 13v2"/><path d="M9 13v2"/></svg>
  `;

  chatBubble.appendChild(chatIcon);

  // Create tooltip
  const tooltip = document.createElement('div');
  tooltip.textContent = 'Ask BAML AI Agent';
  tooltip.style.position = 'absolute';
  tooltip.style.right = '70px';
  tooltip.style.backgroundColor = 'rgba(0, 0, 0, 0.7)';
  tooltip.style.color = 'white';
  tooltip.style.padding = '8px 12px';
  tooltip.style.borderRadius = '4px';
  tooltip.style.fontSize = '14px';
  tooltip.style.whiteSpace = 'nowrap';
  tooltip.style.opacity = '0';
  tooltip.style.visibility = 'hidden';
  tooltip.style.transition = 'opacity 0.3s, visibility 0.3s';
  chatBubble.appendChild(tooltip);

  // Create sparkle container
  const sparkleContainer = document.createElement('div');
  sparkleContainer.style.position = 'absolute';
  sparkleContainer.style.width = '40px';
  sparkleContainer.style.height = '40px';
  sparkleContainer.style.top = '5px';
  sparkleContainer.style.left = '5px';
  sparkleContainer.style.pointerEvents = 'none';
  sparkleContainer.style.opacity = '0';
  sparkleContainer.style.transition = 'opacity 0.3s';
  chatBubble.appendChild(sparkleContainer);

  // Function to create a sparkle
  function createSparkle() {
    const sparkle = document.createElement('div');
    sparkle.style.position = 'absolute';
    sparkle.style.width = '3px';
    sparkle.style.height = '3px';
    sparkle.style.borderRadius = '30%';
    sparkle.style.backgroundColor = '#ffffff';
    sparkle.style.boxShadow = '0 0 5px 1px rgba(255, 255, 255, 0.8)';

    // Random position
    sparkle.style.left = `${Math.random() * 100}%`;
    sparkle.style.top = `${Math.random() * 100}%`;

    // Animation
    sparkle.animate(
      [
        { transform: 'scale(0)', opacity: 0 },
        { transform: 'scale(1)', opacity: 1 },
        { transform: 'scale(0)', opacity: 0 },
      ],
      {
        duration: 1200 + Math.random() * 800,
        easing: 'ease-out',
      },
    );

    sparkleContainer.appendChild(sparkle);

    // Remove after animation
    setTimeout(() => {
      sparkle.remove();
    }, 1000);
  }

  // Sparkle interval reference
  let sparkleInterval;

  // Add hover effects
  chatBubble.addEventListener('mouseover', () => {
    chatBubble.style.backgroundColor = '#6025d1';
    chatBubble.style.transform = 'scale(1.05)';
    tooltip.style.opacity = '1';
    tooltip.style.visibility = 'visible';

    // Enhance the magical border on hover
    borderAnimation.style.opacity = '1';
    borderAnimation.style.filter = 'blur(4px)';

    // Show sparkle container
    sparkleContainer.style.opacity = '1';

    // Create sparkles periodically
    sparkleInterval = setInterval(createSparkle, 300);
  });

  chatBubble.addEventListener('mouseout', () => {
    chatBubble.style.backgroundColor = '#6025d1';
    chatBubble.style.transform = 'scale(1)';
    tooltip.style.opacity = '0';
    tooltip.style.visibility = 'hidden';

    // Reduce the magical border effect when not hovering
    borderAnimation.style.opacity = '0.7';
    borderAnimation.style.filter = 'blur(5px)';

    // Hide sparkle container
    sparkleContainer.style.opacity = '0';

    // Stop creating new sparkles
    clearInterval(sparkleInterval);
  });

  // Add click event
  chatBubble.addEventListener('click', () => {
    window.open('https://boundaryml.com/chat', '_blank', 'noopener,noreferrer');
  });

  // Function to append to body when it's available
  function appendToBody() {
    if (document.body) {
      document.body.appendChild(chatBubble);
      console.log('Chat bubble added to DOM');
    } else {
      // If body isn't available yet, try again shortly
      setTimeout(appendToBody, 50);
    }
  }

  // Start the process
  appendToBody();

  // Also listen for window resize to hide/show based on width
  window.addEventListener('resize', () => {
    if (window.innerWidth <= 700) {
      chatBubble.style.display = 'none';
    } else {
      chatBubble.style.display = 'flex';
    }
  });
})();

// Custom Search Bar Implementation
(() => {
  function initCustomSearch() {
    // Wait for Fern to load completely
    const observer = new MutationObserver((mutations, obs) => {
      // Look for Fern's search bar container
      const searchContainer = document.querySelector(
        '[data-search], .fern-search, [class*="search"]',
      );
      const headerElement = document.querySelector(
        'header, [role="banner"], .fern-header',
      );

      if (headerElement) {
        // Hide the default Fern search bar
        const defaultSearch = document.querySelector(
          '[data-search], .fern-search, [class*="search"]',
        );
        if (defaultSearch) {
          defaultSearch.style.display = 'none';
        }

        // Create custom search container
        const customSearchContainer = document.createElement('div');
        customSearchContainer.id = 'custom-search-container';
        customSearchContainer.style.cssText = `
          position: relative;
          display: flex;
          align-items: center;
          max-width: 540px;
          width: 100%;
          margin: 24px auto 18px auto;
        `;

        // Create search input
        const searchInput = document.createElement('input');
        searchInput.type = 'text';
        searchInput.placeholder = 'Search BAML docs...';
        searchInput.id = 'custom-search-input';
        searchInput.style.cssText = `
          width: 100%;
          min-width: 0;
          max-width: 540px;
          padding: 8px 48px 8px 18px;
          border: 1.5px solid #e2e8f0;
          border-radius: 10px 0 0 10px;
          background: rgba(255, 255, 255, 0.98);
          font-size: 14px;
          outline: none;
          transition: box-shadow 0.18s, border 0.18s;
          box-shadow: 0 2px 12px rgba(96, 37, 209, 0.04);
          backdrop-filter: blur(10px);
          color: #222;
          border-right: none;
          flex: 1 1 auto;
        `;
        // Create Ask AI button (icon only, circular, with tooltip)
        let aiMode = false;
        const askAiBtn = document.createElement('button');
        askAiBtn.type = 'button';
        askAiBtn.style.cssText = `
          width: 40px;
          height: 40px;
          border-radius: 0 10px 10px 0;
          border: 1.5px solid #e2e8f0;
          border-left: none;
          background: #fff;
          color: #6025d1;
          font-size: 18px;
          display: flex;
          align-items: center;
          justify-content: center;
          cursor: pointer;
          transition: background 0.15s, color 0.15s, border 0.15s, box-shadow 0.15s;
          outline: none;
          position: relative;
        `;
        // Add icon (sparkle/magic/chatbot)
        askAiBtn.innerHTML = `<svg width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M12 2v2"/><path d="M12 20v2"/><path d="M5.22 5.22l1.42 1.42"/><path d="M17.36 17.36l1.42 1.42"/><path d="M2 12h2"/><path d="M20 12h2"/><path d="M5.22 18.78l1.42-1.42"/><path d="M17.36 6.64l1.42-1.42"/><circle cx="12" cy="12" r="5"/></svg>`;
        // Tooltip
        askAiBtn.title = 'Ask BAML AI';
        function updateAiBtnStyle() {
          if (aiMode) {
            askAiBtn.style.background = '#6025d1';
            askAiBtn.style.color = '#fff';
            askAiBtn.style.border = '1.5px solid #6025d1';
            askAiBtn.style.borderLeft = 'none';
            askAiBtn.style.boxShadow = '0 0 0 2px #e9e4fa';
          } else {
            askAiBtn.style.background = '#fff';
            askAiBtn.style.color = '#6025d1';
            askAiBtn.style.border = '1.5px solid #e2e8f0';
            askAiBtn.style.borderLeft = 'none';
            askAiBtn.style.boxShadow = 'none';
          }
        }
        updateAiBtnStyle();
        askAiBtn.addEventListener('click', () => {
          aiMode = !aiMode;
          updateAiBtnStyle();
          // Show/hide chat in side panel
          if (aiMode) {
            chatContainer.style.display = '';
          } else {
            chatContainer.style.display = 'none';
          }
        });
        // Create a flex row for search input and Ask AI button
        const searchInputGroup = document.createElement('div');
        searchInputGroup.style.cssText =
          'display: flex; align-items: stretch; width: 100%; position: relative;';
        searchInputGroup.appendChild(searchInput);
        searchInputGroup.appendChild(askAiBtn);
        // Place search icon absolutely inside the input group (move to left)
        const searchIcon = document.createElement('div');
        searchIcon.innerHTML = `
          <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <circle cx="11" cy="11" r="8"></circle>
            <path d="m21 21-4.35-4.35"></path>
          </svg>
        `;
        searchIcon.style.cssText = `
          position: absolute;
          left: 14px;
          top: 50%;
          transform: translateY(-50%);
          color: #64748b;
          pointer-events: none;
        `;
        // Adjust input padding to make room for icon
        searchInput.style.paddingLeft = '38px';
        searchInputGroup.appendChild(searchIcon);
        // Add the input group to the custom search container
        customSearchContainer.appendChild(searchInputGroup);

        // Create results dropdown
        const resultsDropdown = document.createElement('div');
        resultsDropdown.id = 'custom-search-results';
        resultsDropdown.style.cssText = `
          position: absolute;
          top: 100%;
          left: 0;
          right: 0;
          background: white;
          border: 1px solid #e2e8f0;
          border-radius: 6px;
          box-shadow: 0 10px 25px rgba(0, 0, 0, 0.1);
          max-height: 400px;
          overflow-y: auto;
          z-index: 1000;
          display: none;
          margin-top: 4px;
        `;

        // --- Side Panel Implementation ---
        // Create side panel
        const sidePanel = document.createElement('div');
        sidePanel.id = 'custom-search-side-panel';
        sidePanel.style.cssText = `
          position: fixed;
          top: 60px;
          right: 40px;
          width: 340px;
          height: auto;
          max-height: 80vh;
          background: #fff;
          box-shadow: 0 8px 32px rgba(0,0,0,0.18);
          border-radius: 16px;
          border: 1px solid #e2e8f0;
          z-index: 2000;
          display: none;
          flex-direction: column;
          padding: 0 0 0 0;
          transition: transform 0.3s cubic-bezier(.4,0,.2,1), opacity 0.2s;
          overflow: hidden;
        `;
        // --- Top Bar (Context Selector Dropdown + Draggable + Close) ---
        const topBar = document.createElement('div');
        topBar.style.cssText = `
          display: flex;
          align-items: center;
          justify-content: space-between;
          padding: 0 0 0 0;
          background: linear-gradient(90deg, #e9e4fa 0%, #f5f3fa 100%);
          border-bottom: 1px solid #e2e8f0;
          min-height: 44px;
          height: 44px;
          width: 100%;
          position: relative;
          cursor: move;
          user-select: none;
        `;
        // Grab handle + Title
        const grabTitle = document.createElement('div');
        grabTitle.style.cssText = `
          display: flex;
          align-items: center;
          gap: 8px;
          font-size: 16px;
          font-weight: 600;
          color: #6025d1;
          padding: 0 12px;
          white-space: nowrap;
          overflow: hidden;
          text-overflow: ellipsis;
        `;
        grabTitle.innerHTML = `<svg width="18" height="18" style="margin-right:2px;opacity:0.7;" viewBox="0 0 20 20"><rect x="4" y="7" width="2" height="6" rx="1" fill="#b7a7e6"/><rect x="9" y="7" width="2" height="6" rx="1" fill="#b7a7e6"/><rect x="14" y="7" width="2" height="6" rx="1" fill="#b7a7e6"/></svg>BAML Search`;
        // Context dropdown (no AI)
        const languages = [
          { name: 'Python', value: 'python' },
          { name: 'TypeScript', value: 'typescript' },
          { name: 'Ruby', value: 'ruby' },
          { name: 'Go', value: 'go' },
        ];
        let selectedLanguage = languages[0].value;
        const contextDropdown = document.createElement('select');
        contextDropdown.style.cssText = `
          margin-left: 8px;
          padding: 4px 18px 4px 8px;
          border-radius: 6px;
          border: 1px solid #d1c4e9;
          background: #fff;
          color: #6025d1;
          font-size: 13px;
          font-weight: 500;
          outline: none;
          cursor: pointer;
          min-width: 80px;
          max-width: 120px;
        `;
        for (const lang of languages) {
          const opt = document.createElement('option');
          opt.value = lang.value;
          opt.textContent = lang.name;
          contextDropdown.appendChild(opt);
        }
        contextDropdown.value = selectedLanguage;
        contextDropdown.addEventListener('change', () => {
          selectedLanguage = contextDropdown.value;
        });
        // Close button
        const closeBtn = document.createElement('button');
        closeBtn.textContent = '×';
        closeBtn.setAttribute('aria-label', 'Close search panel');
        closeBtn.style.cssText = `
          background: none;
          border: none;
          font-size: 22px;
          color: #bbb;
          cursor: pointer;
          padding: 0 16px 0 8px;
          line-height: 1;
          transition: color 0.15s;
          margin-left: 8px;
          height: 44px;
          display: flex;
          align-items: center;
        `;
        closeBtn.addEventListener('mouseenter', () => {
          closeBtn.style.color = '#6025d1';
        });
        closeBtn.addEventListener('mouseleave', () => {
          closeBtn.style.color = '#bbb';
        });
        closeBtn.addEventListener('click', () => {
          sidePanel.style.display = 'none';
        });
        // Assemble top bar
        grabTitle.appendChild(contextDropdown);
        topBar.appendChild(grabTitle);
        topBar.appendChild(closeBtn);
        sidePanel.appendChild(topBar);
        // Remove info area for minimalism
        // Results area
        const sidePanelResults = document.createElement('div');
        sidePanelResults.id = 'custom-search-side-panel-results';
        sidePanelResults.style.cssText = `
          flex: 1 1 auto;
          overflow-y: auto;
          padding: 0 12px 12px 12px;
          max-height: 250px;
        `;
        sidePanel.appendChild(sidePanelResults);
        // --- Chat Interface ---
        const chatContainer = document.createElement('div');
        chatContainer.style.cssText = `
          border-top: 1px solid #e2e8f0;
          padding: 10px 12px 12px 12px;
          background: inherit;
          display: flex;
          flex-direction: column;
          gap: 6px;
        `;
        // Only show chat if AI mode
        chatContainer.style.display = 'none';
        // Chat messages area
        const chatMessages = document.createElement('div');
        chatMessages.style.cssText = `
          min-height: 40px;
          max-height: 120px;
          overflow-y: auto;
          margin-bottom: 6px;
          font-size: 14px;
          color: #222;
          background: #f8f8fa;
          border-radius: 6px;
          padding: 8px;
        `;
        chatContainer.appendChild(chatMessages);
        // Chat input row
        const chatInputRow = document.createElement('div');
        chatInputRow.style.cssText = 'display: flex; gap: 6px;';
        const chatInput = document.createElement('input');
        chatInput.type = 'text';
        chatInput.placeholder = 'Ask a question...';
        chatInput.style.cssText = `
          flex: 1 1 auto;
          padding: 6px 10px;
          border: 1px solid #e2e8f0;
          border-radius: 6px;
          font-size: 14px;
          outline: none;
        `;
        const chatSend = document.createElement('button');
        chatSend.textContent = 'Send';
        chatSend.style.cssText = `
          padding: 6px 14px;
          background: #6025d1;
          color: #fff;
          border: none;
          border-radius: 6px;
          font-size: 14px;
          cursor: pointer;
          transition: background 0.15s;
        `;
        chatSend.addEventListener('mouseenter', () => {
          chatSend.style.background = '#7d3cf6';
        });
        chatSend.addEventListener('mouseleave', () => {
          chatSend.style.background = '#6025d1';
        });
        chatInputRow.appendChild(chatInput);
        chatInputRow.appendChild(chatSend);
        chatContainer.appendChild(chatInputRow);
        sidePanel.appendChild(chatContainer);
        // Chat logic
        function appendChatMessage(text, isUser) {
          const msg = document.createElement('div');
          msg.textContent = text;
          msg.style.cssText = `
            margin-bottom: 4px;
            padding: 4px 8px;
            border-radius: 4px;
            background: ${isUser ? '#e9e4fa' : '#f1f5f9'};
            color: #222;
            align-self: ${isUser ? 'flex-end' : 'flex-start'};
            max-width: 90%;
            word-break: break-word;
          `;
          chatMessages.appendChild(msg);
          chatMessages.scrollTop = chatMessages.scrollHeight;
        }
        function sendChat() {
          const value = chatInput.value.trim();
          if (!value) return;
          appendChatMessage(value, true);
          chatInput.value = '';
          setTimeout(() => {
            appendChatMessage(
              'This is a placeholder response from BAML AI.',
              false,
            );
          }, 600);
        }
        chatSend.addEventListener('click', sendChat);
        chatInput.addEventListener('keydown', (e) => {
          if (e.key === 'Enter') sendChat();
        });
        // --- End Chat Interface ---
        document.body.appendChild(sidePanel);
        // --- End Side Panel Implementation ---

        // Assemble the search component
        // customSearchContainer.appendChild(searchInput); // Removed as it's now in searchInputGroup
        // customSearchContainer.appendChild(askAiBtn); // Removed as it's now in searchInputGroup
        // customSearchContainer.appendChild(searchIcon); // Removed as it's now in searchInputGroup
        // customSearchContainer.appendChild(resultsDropdown); // Remove dropdown from DOM

        // Insert into header (try different insertion strategies)
        if (searchContainer) {
          searchContainer.parentNode.insertBefore(
            customSearchContainer,
            searchContainer,
          );
        } else if (headerElement) {
          // If no search container found, append to header
          headerElement.appendChild(customSearchContainer);
        }

        // Add dark mode support
        const isDarkMode = () =>
          document.documentElement.classList.contains('dark') ||
          document.body.classList.contains('dark') ||
          getComputedStyle(document.body).backgroundColor === 'rgb(11, 13, 14)';

        const updateSearchTheme = () => {
          if (isDarkMode()) {
            searchInput.style.background = 'rgba(30, 30, 30, 0.95)';
            searchInput.style.border = '1px solid #2e2e2e';
            searchInput.style.color = '#ffffff';
            sidePanel.style.background = '#1a1a1a';
            sidePanel.style.border = '1px solid #2e2e2e';
            sidePanelResults.style.background = '#1a1a1a';
            sidePanelResults.style.color = '#fff';
          } else {
            searchInput.style.background = 'rgba(255, 255, 255, 0.95)';
            searchInput.style.border = '1px solid #e2e8f0';
            searchInput.style.color = '#000000';
            sidePanel.style.background = '#fff';
            sidePanel.style.border = '1px solid #e2e8f0';
            sidePanelResults.style.background = '#fff';
            sidePanelResults.style.color = '#000';
          }
        };

        updateSearchTheme();

        // Watch for theme changes
        const themeObserver = new MutationObserver(updateSearchTheme);
        themeObserver.observe(document.documentElement, {
          attributes: true,
          attributeFilter: ['class'],
        });
        themeObserver.observe(document.body, {
          attributes: true,
          attributeFilter: ['class'],
        });

        // Search functionality
        let searchTimeout;
        const searchResults = [];

        // Mock search data - you can replace this with actual search implementation
        const searchData = [
          {
            title: 'Getting Started with BAML',
            url: '/guide/introduction/what-is-baml',
            excerpt: 'Learn the basics of BAML and how to get started',
          },
          {
            title: 'Python Installation',
            url: '/guide/installation-language/python',
            excerpt: 'Install BAML for Python projects',
          },
          {
            title: 'TypeScript Installation',
            url: '/guide/installation-language/typescript',
            excerpt: 'Install BAML for TypeScript/JavaScript projects',
          },
          {
            title: 'Functions',
            url: '/ref/baml/function',
            excerpt: 'Define and use BAML functions',
          },
          {
            title: 'Classes',
            url: '/ref/baml/class',
            excerpt: 'Define data structures with BAML classes',
          },
          {
            title: 'LLM Clients',
            url: '/ref/baml/client-llm',
            excerpt: 'Configure LLM providers and clients',
          },
          {
            title: 'Prompt Engineering',
            url: '/examples/prompt-engineering',
            excerpt: 'Best practices for prompt engineering',
          },
        ];

        // Replace performSearch and displayResults to update sidePanelResults
        const performSearch = (query) => {
          if (!query.trim()) {
            sidePanelResults.innerHTML = '';
            return;
          }
          const filtered = searchData.filter(
            (item) =>
              item.title.toLowerCase().includes(query.toLowerCase()) ||
              item.excerpt.toLowerCase().includes(query.toLowerCase()),
          );
          displayResults(filtered, query);
        };
        const displayResults = (results, query) => {
          sidePanelResults.innerHTML = '';
          if (results.length === 0) {
            const noResults = document.createElement('div');
            noResults.textContent = `No results for "${query}"`;
            noResults.style.cssText = `
              padding: 32px 0 0 0;
              text-align: center;
              color: #a0aec0;
              font-size: 15px;
            `;
            sidePanelResults.appendChild(noResults);
          } else {
            for (const result of results) {
              const resultItem = document.createElement('a');
              resultItem.href = result.url;
              resultItem.style.cssText = `
                display: block;
                padding: 12px 0 10px 0;
                text-decoration: none;
                border-bottom: 1px solid #f1f5f9;
                transition: background 0.15s;
                color: inherit;
                border-radius: 4px;
                margin-bottom: 2px;
                padding-left: 18px;
              `;
              resultItem.innerHTML = `
                <div style="font-weight: 500; font-size: 15px; color: #6025d1; margin-bottom: 2px;">${result.title}</div>
                <div style="font-size: 12px; color: #7b8494; line-height: 1.4;">${result.excerpt}</div>
              `;
              resultItem.addEventListener('mouseenter', () => {
                resultItem.style.background = isDarkMode()
                  ? '#23232a'
                  : '#f3f4f6';
              });
              resultItem.addEventListener('mouseleave', () => {
                resultItem.style.background = 'transparent';
              });
              sidePanelResults.appendChild(resultItem);
            }
          }
        };

        // Search input event listeners
        searchInput.addEventListener('input', (e) => {
          clearTimeout(searchTimeout);
          searchTimeout = setTimeout(() => {
            performSearch(e.target.value);
          }, 150);
        });

        searchInput.addEventListener('focus', () => {
          searchInput.style.borderColor = '#6025d1';
          searchInput.style.boxShadow = '0 0 0 3px rgba(96, 37, 209, 0.1)';
          if (searchInput.value.trim()) {
            performSearch(searchInput.value);
          }
          // Show side panel
          sidePanel.style.display = 'flex';
        });

        searchInput.addEventListener('blur', () => {
          searchInput.style.borderColor = isDarkMode() ? '#2e2e2e' : '#e2e8f0';
          searchInput.style.boxShadow = 'none';
        });

        // Hide side panel when clicking outside
        document.addEventListener('mousedown', (e) => {
          if (
            sidePanel.style.display === 'flex' &&
            !sidePanel.contains(e.target) &&
            !customSearchContainer.contains(e.target)
          ) {
            sidePanel.style.display = 'none';
          }
        });
        // Hide side panel on Escape
        document.addEventListener('keydown', (e) => {
          if (e.key === 'Escape' && sidePanel.style.display === 'flex') {
            sidePanel.style.display = 'none';
            searchInput.blur();
          }
        });
        // Keyboard navigation (optional: highlight in sidePanelResults)
        let selectedIndex = -1;
        searchInput.addEventListener('keydown', (e) => {
          const items = sidePanelResults.querySelectorAll('a');
          if (e.key === 'ArrowDown') {
            e.preventDefault();
            selectedIndex = Math.min(selectedIndex + 1, items.length - 1);
            updateSelection(items);
          } else if (e.key === 'ArrowUp') {
            e.preventDefault();
            selectedIndex = Math.max(selectedIndex - 1, -1);
            updateSelection(items);
          } else if (e.key === 'Enter') {
            e.preventDefault();
            if (selectedIndex >= 0 && items[selectedIndex]) {
              items[selectedIndex].click();
            }
          } else if (e.key === 'Escape') {
            sidePanel.style.display = 'none';
            searchInput.blur();
          }
        });
        const updateSelection = (items) => {
          for (let index = 0; index < items.length; index++) {
            const item = items[index];
            if (index === selectedIndex) {
              item.style.backgroundColor = isDarkMode() ? '#2a2a2a' : '#f8fafc';
            } else {
              item.style.backgroundColor = 'transparent';
            }
          }
        };
        // Click outside to close
        document.addEventListener('click', (e) => {
          if (
            !customSearchContainer.contains(e.target) &&
            !sidePanel.contains(e.target)
          ) {
            sidePanel.style.display = 'none';
          }
        });
        // --- Draggable Side Panel ---
        let isDragging = false;
        let dragOffsetX = 0;
        let dragOffsetY = 0;
        // Make the whole topBar draggable
        topBar.addEventListener('mousedown', (e) => {
          isDragging = true;
          const rect = sidePanel.getBoundingClientRect();
          dragOffsetX = e.clientX - rect.left;
          dragOffsetY = e.clientY - rect.top;
          document.body.style.userSelect = 'none';
        });
        document.addEventListener('mousemove', (e) => {
          if (isDragging) {
            sidePanel.style.left = `${e.clientX - dragOffsetX}px`;
            sidePanel.style.top = `${e.clientY - dragOffsetY}px`;
            sidePanel.style.right = 'auto';
          }
        });
        document.addEventListener('mouseup', () => {
          isDragging = false;
          document.body.style.userSelect = '';
        });
        // --- End Draggable Side Panel ---
        obs.disconnect();
      }
    });

    observer.observe(document.body, {
      childList: true,
      subtree: true,
    });

    // Also try immediate execution if page is already loaded
    if (document.readyState === 'complete') {
      setTimeout(() => observer.disconnect(), 100);
    }
  }

  // Initialize when DOM is ready
  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', initCustomSearch);
  } else {
    initCustomSearch();
  }
})();
