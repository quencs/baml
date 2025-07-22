// Immediately execute function
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
          max-width: 400px;
          margin: 0 auto;
        `;

        // Create search input
        const searchInput = document.createElement('input');
        searchInput.type = 'text';
        searchInput.placeholder = 'Search BAML docs...';
        searchInput.id = 'custom-search-input';
        searchInput.style.cssText = `
          width: 100%;
          padding: 8px 40px 8px 16px;
          border: 1px solid #e2e8f0;
          border-radius: 6px;
          background: rgba(255, 255, 255, 0.95);
          font-size: 14px;
          outline: none;
          transition: all 0.2s ease;
          backdrop-filter: blur(10px);
        `;

        // Create search icon
        const searchIcon = document.createElement('div');
        searchIcon.innerHTML = `
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <circle cx="11" cy="11" r="8"></circle>
            <path d="m21 21-4.35-4.35"></path>
          </svg>
        `;
        searchIcon.style.cssText = `
          position: absolute;
          right: 12px;
          top: 50%;
          transform: translateY(-50%);
          color: #64748b;
          pointer-events: none;
        `;

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

        // Assemble the search component
        customSearchContainer.appendChild(searchInput);
        customSearchContainer.appendChild(searchIcon);
        customSearchContainer.appendChild(resultsDropdown);

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
            resultsDropdown.style.background = '#1a1a1a';
            resultsDropdown.style.border = '1px solid #2e2e2e';
          } else {
            searchInput.style.background = 'rgba(255, 255, 255, 0.95)';
            searchInput.style.border = '1px solid #e2e8f0';
            searchInput.style.color = '#000000';
            resultsDropdown.style.background = 'white';
            resultsDropdown.style.border = '1px solid #e2e8f0';
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

        const performSearch = (query) => {
          if (!query.trim()) {
            resultsDropdown.style.display = 'none';
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
          resultsDropdown.innerHTML = '';

          if (results.length === 0) {
            const noResults = document.createElement('div');
            noResults.textContent = `No results found for "${query}"`;
            noResults.style.cssText = `
              padding: 16px;
              text-align: center;
              color: #64748b;
              font-size: 14px;
            `;
            resultsDropdown.appendChild(noResults);
          } else {
            for (const result of results) {
              const resultItem = document.createElement('a');
              resultItem.href = result.url;
              resultItem.style.cssText = `
                display: block;
                padding: 12px 16px;
                text-decoration: none;
                border-bottom: 1px solid #f1f5f9;
                transition: background-color 0.2s ease;
                color: inherit;
              `;

              resultItem.innerHTML = `
                <div style="font-weight: 500; margin-bottom: 4px; color: #6025d1;">${result.title}</div>
                <div style="font-size: 12px; color: #64748b; line-height: 1.4;">${result.excerpt}</div>
              `;

              resultItem.addEventListener('mouseenter', () => {
                resultItem.style.backgroundColor = isDarkMode()
                  ? '#2a2a2a'
                  : '#f8fafc';
              });

              resultItem.addEventListener('mouseleave', () => {
                resultItem.style.backgroundColor = 'transparent';
              });

              resultsDropdown.appendChild(resultItem);
            }
          }

          resultsDropdown.style.display = 'block';
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
        });

        searchInput.addEventListener('blur', () => {
          searchInput.style.borderColor = isDarkMode() ? '#2e2e2e' : '#e2e8f0';
          searchInput.style.boxShadow = 'none';
          // Delay hiding results to allow clicks
          setTimeout(() => {
            resultsDropdown.style.display = 'none';
          }, 150);
        });

        // Keyboard navigation
        let selectedIndex = -1;

        searchInput.addEventListener('keydown', (e) => {
          const items = resultsDropdown.querySelectorAll('a');

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
            resultsDropdown.style.display = 'none';
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
          if (!customSearchContainer.contains(e.target)) {
            resultsDropdown.style.display = 'none';
          }
        });

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
