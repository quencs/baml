// @ts-check
import { themes as prismThemes } from 'prism-react-renderer';

/** @type {import('@docusaurus/types').Config} */
const config = {
  title: 'Learn BAML',
  tagline: 'Build reliable AI applications with structured outputs',
  favicon: 'img/favicon.ico',

  url: 'https://learn.boundaryml.com',
  baseUrl: '/',

  headTags: [
    {
      tagName: 'link',
      attributes: {
        rel: 'preconnect',
        href: 'https://fonts.googleapis.com',
      },
    },
    {
      tagName: 'link',
      attributes: {
        rel: 'preconnect',
        href: 'https://fonts.gstatic.com',
        crossorigin: 'anonymous',
      },
    },
    {
      tagName: 'link',
      attributes: {
        rel: 'stylesheet',
        href: 'https://fonts.googleapis.com/css2?family=Inter:wght@400;500;600;700&display=swap',
      },
    },
    {
      tagName: 'link',
      attributes: {
        rel: 'stylesheet',
        href: 'https://fonts.googleapis.com/css2?family=Geist+Mono:wght@400;500&display=swap',
      },
    },
  ],

  organizationName: 'Boundary',
  projectName: 'baml',

  onBrokenLinks: 'warn',

  markdown: {
    hooks: {
      onBrokenMarkdownLinks: 'warn',
    },
  },

  i18n: {
    defaultLocale: 'en',
    locales: ['en'],
  },

  plugins: [
    [
      '@signalwire/docusaurus-plugin-llms-txt',
      {
        markdown: {
          enableFiles: true,
          includeDocs: true,
          includeBlog: false,
          includePages: false,
        },
        llmsTxt: {
          siteTitle: 'Learn BAML',
          siteDescription: 'Build reliable AI applications with structured outputs',
          includeDocs: true,
        },
        ui: {
          copyPageContent: {
            buttonLabel: 'Copy Page',
            display: {
              docs: true,
            },
          },
        },
      },
    ],
  ],

  themes: ['@signalwire/docusaurus-theme-llms-txt'],

  presets: [
    [
      'classic',
      /** @type {import('@docusaurus/preset-classic').Options} */
      ({
        docs: {
          sidebarPath: './sidebars.js',
          routeBasePath: '/',
          editUrl: 'https://github.com/BoundaryML/baml/tree/main/typescript/apps/learn-baml/',
        },
        blog: false,
        theme: {
          customCss: './src/css/custom.css',
        },
      }),
    ],
  ],

  themeConfig:
    /** @type {import('@docusaurus/preset-classic').ThemeConfig} */
    ({
      image: 'img/baml-social-card.png',
      navbar: {
        title: 'Learn BAML',
        logo: {
          alt: 'BAML Logo',
          src: 'img/logo.svg',
          href: '/',
        },
        items: [
          // Tour is custom pages, not docs - use regular link
          {
            to: '/tour',
            position: 'left',
            label: 'Tour',
          },
          {
            type: 'docSidebar',
            sidebarId: 'tutorialsSidebar',
            position: 'left',
            label: 'Tutorials',
          },
          {
            type: 'docSidebar',
            sidebarId: 'howToSidebar',
            position: 'left',
            label: 'How-to',
          },
          {
            type: 'docSidebar',
            sidebarId: 'conceptsSidebar',
            position: 'left',
            label: 'Concepts',
          },
          {
            type: 'docSidebar',
            sidebarId: 'referenceSidebar',
            position: 'left',
            label: 'Reference',
          },
          {
            type: 'docSidebar',
            sidebarId: 'cookbookSidebar',
            position: 'left',
            label: 'Cookbook',
          },
          // Right side items
          {
            type: 'custom-askAiButton',
            position: 'right',
          },
          {
            href: 'https://promptfiddle.com',
            label: 'Playground',
            position: 'right',
          },
          {
            href: 'https://github.com/BoundaryML/baml',
            label: 'GitHub',
            position: 'right',
          },
        ],
      },
      footer: {
        style: 'dark',
        links: [
          {
            title: 'Learn',
            items: [
              { label: 'Tour', to: '/tour' },
              { label: 'Tutorials', to: '/tutorials/getting-started' },
              { label: 'Concepts', to: '/concepts/type-system' },
            ],
          },
          {
            title: 'Community',
            items: [
              { label: 'Discord', href: 'https://discord.gg/boundaryml' },
              { label: 'GitHub', href: 'https://github.com/BoundaryML/baml' },
            ],
          },
          {
            title: 'More',
            items: [
              { label: 'Playground', href: 'https://promptfiddle.com' },
              { label: 'API Reference', to: '/reference/baml-syntax' },
            ],
          },
        ],
        copyright: `Copyright © ${new Date().getFullYear()} BoundaryML, Inc.`,
      },
      prism: {
        theme: prismThemes.github,
        darkTheme: prismThemes.dracula,
        additionalLanguages: ['bash', 'json', 'typescript', 'python'],
      },
      colorMode: {
        defaultMode: 'light',
        disableSwitch: false,
        respectPrefersColorScheme: true,
      },
    }),
};

export default config;
