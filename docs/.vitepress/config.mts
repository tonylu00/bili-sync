import { defineConfig } from "vitepress";
import taskLists from "markdown-it-task-lists";

// https://vitepress.dev/reference/site-config
export default defineConfig({
	title: "bili-sync",
	description: "由 Rust & Tokio 驱动的哔哩哔哩同步工具",
	lang: "zh-Hans",
	base: "/bili-sync-01/", // GitHub仓库名
	sitemap: {
		hostname: "https://qq1582185982.github.io/bili-sync-01",
	},
	lastUpdated: true,
	cleanUrls: true,
	metaChunk: true,
	ignoreDeadLinks: true, // 忽略死链接检查
	themeConfig: {
		outline: {
			label: "页面导航",
			level: "deep",
		},
		// https://vitepress.dev/reference/default-theme-config
		nav: [
			{ text: "主页", link: "/" },
			{
				        text: "v2.7.1",
				items: [
					{
						text: "程序更新",
						link: "https://github.com/qq1582185982/bili-sync-01/releases",
					},
					{
						text: "文档更新",
						link: "https://github.com/qq1582185982/bili-sync-01/commits/main",
					},
				],
			},
		],
		sidebar: [
			{
				text: "简介",
				items: [
					{ text: "什么是 bili-sync？", link: "/introduction" },
					{ text: "快速开始", link: "/quick-start" },
					{ text: "功能展示", link: "/features" },
				],
			},
			{
				text: "细节",
				items: [
					{ text: "配置文件", link: "/configuration" },
					{ text: "命令行参数", link: "/args" },
					{ text: "工作原理", link: "/design" },
				],
			},
			{
				text: "参考",
				items: [
					{ text: "获取收藏夹信息", link: "/favorite" },
					{
						text: "获取视频合集/视频列表信息",
						link: "/collection",
					},
					{ text: "获取投稿信息", link: "/submission" },
				],
			},
			{
				text: "技术文档",
				collapsed: false,
				items: [
					{ text: "🛡️ 删除任务队列系统", link: "/README_DELETE_TASK_QUEUE" },
					{ text: "⚙️ 系统配置智能队列", link: "/SYSTEM_CONFIG_QUEUE_SUMMARY" },
					{ text: "📊 队列管理功能说明", link: "/QUEUE_FEATURE_SUMMARY" },
					{ text: "🎊 删除功能实现总结", link: "/FEATURE_SUMMARY" },
					{ text: "📝 配置迁移指南", link: "/MIGRATION_GUIDE" },
				],
			},
			{
				text: "其它",
				items: [
					{ text: "常见问题", link: "/question" },
					{ text: "管理页", link: "/frontend" },
					{ text: "部署指南", link: "/deploy-guide" },
				],
			},
			{
				text: "更新日志",
				items: [
					{ text: "🚀 综合更新 v2.7.0 (2025-06-03)", link: "/comprehensive-update-2025-06-03" },
					{ text: "🔧 智能合并bug修复", link: "/bangumi-merge-fix" },
					{ text: "v2.6.2 (2025年6月)", link: "/update-log-2025-06" },
					{ text: "2024年6月更新", link: "/update-log-2024-06" },
				],
			},
		],
		socialLinks: [
			{ icon: "github", link: "https://github.com/qq1582185982/bili-sync-01" },
		],
		search: {
			provider: "local",
		},
		notFound: {
			title: "你来到了没有知识的荒原",
			quote: "这里什么都没有",
			linkText: "返回首页",
		},
		docFooter: {
			prev: "上一页",
			next: "下一页",
		},
		lastUpdated: {
			text: "上次更新于",
		},
		returnToTopLabel: "回到顶部",
		sidebarMenuLabel: "菜单",
		darkModeSwitchLabel: "主题",
		lightModeSwitchTitle: "切换到浅色模式",
		darkModeSwitchTitle: "切换到深色模式",
	},
	markdown: {
		config: (md) => {
			md.use(taskLists);
		},
		theme: {
			light: "github-light",
			dark: "github-dark",
		},
	},
	head: [
		["link", { rel: "icon", type: "image/svg+xml", href: "/icon.svg" }],
		["link", { rel: "icon", type: "image/png", href: "/icon.png" }],
	],
});
