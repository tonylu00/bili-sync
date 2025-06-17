---
# https://vitepress.dev/reference/default-theme-home-page
layout: home

title: bili-sync
titleTemplate: 由 Rust & Tokio 驱动的哔哩哔哩同步工具

hero:
  name: "bili-sync"
  text: "由 Rust & Tokio 驱动的哔哩哔哩同步工具"
  tagline: "v2.7.3 - 配置系统全面升级，支持热重载和智能文件名处理"
  actions:
    - theme: brand
      text: 什么是 bili-sync？
      link: /introduction
    - theme: alt
      text: 快速开始
      link: /quick-start
    - theme: alt
      text: GitHub
      link: https://github.com/qq1582185982/bili-sync-01
  image:
    src: /logo.webp
    alt: bili-sync

features:
  - icon: 🔥
    title: 配置热重载系统
    details: 配置完全基于数据库，支持实时热重载，无需重启即可生效
  - icon: 📝
    title: 智能文件名处理
    details: 自动处理所有特殊字符，支持全角转换，确保跨平台兼容性
  - icon: 🚀
    title: 任务队列优化
    details: 配置保存通过任务队列处理，彻底解决数据库锁定问题
  - icon: 🎯
    title: 初始设置向导
    details: 友好的初始设置界面，轻松配置 Cookie 和基本参数
  - icon: 💾
    title: 专为 NAS 设计
    details: 可被 Emby、Jellyfin 等媒体服务器一键识别，完整的元数据支持
  - icon: 🐳
    title: 部署简单
    details: 提供简单易用的 docker 镜像，支持多架构部署
---

<style>
:root {
  --vp-home-hero-name-color: transparent;
  --vp-home-hero-name-background: -webkit-linear-gradient(120deg, #bd34fe 30%, #41d1ff);

  --vp-home-hero-image-background-image: linear-gradient(-45deg, #bd34fe 50%, #47caff 50%);
  --vp-home-hero-image-filter: blur(44px);
}

@media (min-width: 640px) {
  :root {
    --vp-home-hero-image-filter: blur(56px);
  }
}

@media (min-width: 960px) {
  :root {
    --vp-home-hero-image-filter: blur(68px);
  }
}
</style>