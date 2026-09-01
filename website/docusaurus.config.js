// @ts-check
// `@type` JSDoc annotations allow editor autocompletion and type checking
// (when paired with `@ts-check`).
// There are various equivalent ways to declare your Docusaurus config.
// See: https://docusaurus.io/docs/api/docusaurus-config

import {themes as prismThemes} from 'prism-react-renderer';
import docsSystemPlugin, {ecosystemFooterGroup, ecosystemNavbarItems} from '@beyond10x/docs-system/docusaurus';

const organizationName = 'beyond10x';
const projectName = 'metaharness';

/** @type {import('@docusaurus/types').Config} */
const config = {
  title: 'metaharness',
  tagline: 'One interface to many agent harnesses — observable, steerable, hermetic.',
  favicon: 'img/favicon.ico',

  future: {
    v4: true,
  },

  // GitHub Pages: https://beyond10x.github.io/metaharness/
  url: `https://${organizationName}.github.io`,
  baseUrl: `/${projectName}/`,
  organizationName,
  projectName,
  deploymentBranch: 'gh-pages',
  trailingSlash: false,

  onBrokenLinks: 'throw',
  plugins: [docsSystemPlugin],

  markdown: {
    hooks: {
      onBrokenMarkdownLinks: 'throw',
    },
  },

  i18n: {
    defaultLocale: 'en',
    locales: ['en'],
  },

  presets: [
    [
      'classic',
      /** @type {import('@docusaurus/preset-classic').Options} */
      ({
        docs: {
          sidebarPath: './sidebars.js',
          routeBasePath: '/docs',
          editUrl: `https://github.com/${organizationName}/${projectName}/tree/main/website/`,
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
      image: 'img/docusaurus-social-card.jpg',
      colorMode: {
        respectPrefersColorScheme: true,
      },
      navbar: {
        title: 'metaharness',
        logo: {
          alt: 'metaharness',
          src: 'img/logo.svg',
        },
        items: [
          ...ecosystemNavbarItems(),
          {
            type: 'docSidebar',
            sidebarId: 'docsSidebar',
            position: 'left',
            label: 'Docs',
          },
          {
            href: `https://github.com/${organizationName}/${projectName}`,
            label: 'GitHub',
            position: 'right',
          },
        ],
      },
      footer: {
        style: 'dark',
        links: [
          ecosystemFooterGroup(),
          {
            title: 'Docs',
            items: [
              {label: 'What it is', to: '/docs/'},
              {label: 'Quickstart', to: '/docs/quickstart'},
              {label: 'CLI reference', to: '/docs/reference/cli'},
            ],
          },
          {
            title: 'Design',
            items: [
              {label: 'Protocol', to: '/docs/protocol/events'},
              {label: 'Hermetic contract', to: '/docs/hermetic'},
              {label: 'Control seam', to: '/docs/control-seam'},
            ],
          },
          {
            title: 'More',
            items: [
              {
                label: 'GitHub',
                href: `https://github.com/${organizationName}/${projectName}`,
              },
              {label: 'Status', to: '/docs/status'},
            ],
          },
        ],
        copyright: `metaharness — pre-v1. Built ${new Date().getFullYear()}.`,
      },
      prism: {
        theme: prismThemes.github,
        darkTheme: prismThemes.dracula,
        additionalLanguages: ['rust', 'bash', 'toml', 'json'],
      },
    }),
};

export default config;
