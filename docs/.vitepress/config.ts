import { defineConfig } from 'vitepress';
import { withMermaid } from 'vitepress-plugin-mermaid';

const repository = 'https://github.com/ashllll/log-analyzer_rust';

function normalizeBase(value: string | undefined): string {
  const base = value?.trim() || '/log-analyzer_rust/';
  const normalized = base.replace(/^\/+|\/+$/g, '');
  return normalized ? `/${normalized}/` : '/';
}

export default withMermaid(
  defineConfig(({ command }) => ({
    lang: 'zh-CN',
    title: 'Log Analyzer',
    description: '面向大规模日志导入、搜索、分析与实时监控的本地桌面工具。',
    base: command === 'serve' ? normalizeBase(process.env.DOCS_BASE || '/') : normalizeBase(process.env.DOCS_BASE),
    cleanUrls: true,
    lastUpdated: true,
    sitemap: {
      hostname: 'https://ashllll.github.io/log-analyzer_rust/'
    },
    head: [
      ['link', { rel: 'icon', href: '/favicon.svg', type: 'image/svg+xml' }],
      ['meta', { name: 'theme-color', content: '#0b0f14' }],
      ['meta', { property: 'og:type', content: 'website' }],
      ['meta', { property: 'og:site_name', content: 'Log Analyzer Documentation' }],
      ['meta', { property: 'og:title', content: 'Log Analyzer — 本地日志分析工作台' }],
      ['meta', { property: 'og:description', content: '离线优先，面向大规模日志的导入、搜索、过滤与实时监控。' }]
    ],
    themeConfig: {
      logo: '/favicon.svg',
      siteTitle: 'Log Analyzer',
      nav: [
        { text: '指南', link: '/guide/getting-started' },
        { text: '架构', link: '/architecture/overview' },
        { text: '开发', link: '/development/setup' },
        { text: '运维', link: '/operations/ci' },
        {
          text: '项目',
          items: [
            { text: '更新日志', link: `${repository}/blob/main/CHANGELOG.md` },
            { text: '发行版本', link: `${repository}/releases` },
            { text: '问题反馈', link: `${repository}/issues` }
          ]
        }
      ],
      sidebar: {
        '/guide/': [
          {
            text: '开始使用',
            items: [
              { text: '快速开始', link: '/guide/getting-started' },
              { text: '功能概览', link: '/guide/features' }
            ]
          },
          {
            text: '用户指南',
            items: [
              { text: '工作区与导入', link: '/guide/workspaces' },
              { text: '搜索与过滤', link: '/guide/search' },
              { text: '关键词与持续监听', link: '/guide/keywords-watch' }
            ]
          }
        ],
        '/architecture/': [
          {
            text: '系统架构',
            items: [
              { text: '架构总览', link: '/architecture/overview' },
              { text: '搜索链路', link: '/architecture/search' },
              { text: '导入链路', link: '/architecture/import' },
              { text: 'IPC 与状态同步', link: '/architecture/ipc' }
            ]
          },
          {
            text: '深入阅读',
            items: [
              { text: 'CAS 存储架构', link: '/architecture/CAS_ARCHITECTURE' },
              { text: '分布式工作区评估', link: '/architecture/DISTRIBUTED_WORKSPACE_ASSESSMENT' }
            ]
          }
        ],
        '/development/': [
          {
            text: '开发者指南',
            items: [
              { text: '环境与启动', link: '/development/setup' },
              { text: '项目结构', link: '/development/structure' },
              { text: '测试与质量', link: '/development/testing' },
              { text: '贡献指南', link: '/CONTRIB' },
              { text: '领域词汇', link: '/CONTEXT' }
            ]
          }
        ],
        '/operations/': [
          {
            text: '交付与运维',
            items: [
              { text: 'CI 与 GitHub 工作流', link: '/operations/ci' },
              { text: '发布流程', link: '/operations/release' },
              { text: '故障排查', link: '/operations/troubleshooting' },
              { text: '完整运行手册', link: '/RUNBOOK' }
            ]
          }
        ]
      },
      socialLinks: [{ icon: 'github', link: repository }],
      search: {
        provider: 'local',
        options: {
          translations: {
            button: { buttonText: '搜索文档', buttonAriaLabel: '搜索文档' },
            modal: {
              noResultsText: '没有找到相关结果',
              resetButtonTitle: '清除查询',
              footer: { selectText: '选择', navigateText: '切换', closeText: '关闭' }
            }
          }
        }
      },
      editLink: {
        pattern: `${repository}/edit/main/docs/:path`,
        text: '在 GitHub 上编辑此页'
      },
      lastUpdated: { text: '最后更新', formatOptions: { dateStyle: 'medium', timeStyle: 'short' } },
      docFooter: { prev: '上一篇', next: '下一篇' },
      outline: { label: '本页目录', level: [2, 3] },
      darkModeSwitchLabel: '外观',
      sidebarMenuLabel: '目录',
      returnToTopLabel: '返回顶部',
      externalLinkIcon: true,
      footer: {
        message: '离线优先，数据留在本地。',
        copyright: 'Apache-2.0 Licensed · Log Analyzer'
      }
    },
    mermaid: {
      theme: 'base',
      themeVariables: {
        primaryColor: '#162d2a',
        primaryTextColor: '#dffbf3',
        primaryBorderColor: '#35d0a0',
        lineColor: '#688078',
        secondaryColor: '#171d26',
        tertiaryColor: '#0b0f14'
      }
    }
  }))
);
