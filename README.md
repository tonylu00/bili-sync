![bili-sync](https://socialify.git.ci/amtoaer/bili-sync/image?description=1&font=KoHo&issues=1&language=1&logo=https%3A%2F%2Fs2.loli.net%2F2023%2F12%2F02%2F9EwT2yInOu1d3zm.png&name=1&owner=1&pattern=Signal&pulls=1&stargazers=1&theme=Light)

[![GitHub Release](https://img.shields.io/github/v/release/amtoaer/bili-sync)](https://github.com/amtoaer/bili-sync/releases/latest)
[![Test](https://github.com/amtoaer/bili-sync/actions/workflows/test.yml/badge.svg)](https://github.com/amtoaer/bili-sync/actions/workflows/test.yml)
[![Release](https://github.com/amtoaer/bili-sync/actions/workflows/release.yml/badge.svg)](https://github.com/amtoaer/bili-sync/actions/workflows/release.yml)
[![Downloads](https://img.shields.io/github/downloads/amtoaer/bili-sync/total)](https://github.com/amtoaer/bili-sync/releases)

## 简介

> [!NOTE]
> [点击此处](https://bili-sync.allwens.work/)查看文档

bili-sync 是一款专为 NAS 用户编写的哔哩哔哩同步工具，由 Rust & Tokio 驱动。

> [!TIP]
> **最新版本已包含重大改进！** 视频源配置已完全迁移到数据库管理，新增收藏夹智能管理、UP主合集支持等功能。详见下方"🔧 最新更新 (2024)"部分。

## 🌟 项目亮点

### 🆕 相比原版本的主要改进

- **🎨 Web 管理界面增强** - 可直接在管理页面添加视频源，告别手动编辑配置文件
- **🗄️ 数据库架构升级** - 视频源配置完全迁移到数据库管理，配置文件更加简洁
- **📁 收藏夹智能管理** - 自动展示用户所有收藏夹，支持快速选择，解决大ID精度问题
- **🎬 UP主合集支持** - 输入UP主ID即可浏览和选择其所有合集/系列
- **🔍 搜索体验升级** - 搜索结果在主区域展示，支持分页、网格布局、实时预览
- **📺 番剧下载支持** - 完整支持番剧下载，包括单季和全季模式
- **⚙️ 配置系统升级** - 新的 V2 配置格式，配置文件与视频源分离，更加清晰
- **🐳 Docker 部署优化** - 多阶段构建、健康检查、更好的缓存策略
- **🛠️ 开发体验提升** - 完整的开发工具链和详细文档
- **🔧 智能任务调度** - 自动避免数据库锁定冲突，提高稳定性

### 🔧 最新更新 (2024)

#### 🎉 2024-06-01 更新 - 文档系统全面升级 📚
- **📚 VitePress 文档系统** - 搭建了现代化的文档网站，支持中文搜索和暗黑模式
- **🚀 GitHub Actions 自动部署** - 推送代码即可自动更新文档，实现 CI/CD 流程
- **📄 功能展示页面** - 添加了详细的功能截图和使用说明
- **📖 部署指南** - 提供了完整的文档部署教程
- **🌐 在线文档** - 访问地址：https://qq1582185982.github.io/bili-sync-01/

#### 🎉 2024-06-01 更新 - 重大功能升级
- **🗄️ 视频源数据库迁移** - 视频源配置从配置文件完全迁移到数据库，配置文件不再包含视频源信息
- **📁 收藏夹功能完善** - 实现直接显示当前用户的所有收藏夹列表，无需搜索即可快速选择
- **🔢 修复收藏夹ID精度问题** - 解决了大ID收藏夹（如3594171330）被截断的问题，现在使用完整的64位ID
- **🎬 UP主合集搜索** - 新增UP主合集和系列搜索功能，输入UP主ID即可查看并选择其所有合集
- **🔍 搜索结果优化** - 改进搜索结果显示布局，支持分页浏览，优化了缩略图显示（4行×3列网格布局）
- **🖼️ 番剧缩略图修复** - 修复番剧搜索结果中缩略图无法显示的问题，正确处理B站图片防盗链
- **🗃️ 数据库兼容性修复** - 恢复误删的迁移文件，修复`latest_row_at`列缺失导致的兼容性问题
- **🎯 搜索体验提升** - 搜索结果实时显示在右侧主内容区，支持番剧/影视混合搜索

#### 🐛 关键问题修复
- **✅ 修复程序崩溃问题** - 解决了"index not found"panic错误，修复了配置文件与数据库不一致导致的崩溃
- **✅ 修复番剧重命名失效** - 番剧命名模板现在可以正确实时更新生效
- **✅ 修复NFO文件命名不一致** - 番剧NFO文件现在与视频文件使用相同的命名格式
- **✅ 清理编译警告** - 实现完全无警告的干净编译，提升代码质量

#### 🚀 功能增强
- **🔐 API Token认证升级** - 全新的登录界面，支持Token验证和安全退出
- **⚡ 多线程下载配置** - 管理页面新增多线程下载设置，支持线程数和最小文件大小配置
- **📏 用户体验优化** - 最小文件大小单位改为MB显示，更加直观易懂
- **⏱️ 智能配置更新** - 扫描间隔时间修改后立即生效，无需等待完整周期
- **✂️ 模板截取函数** - 新增truncate函数支持，可截取模板变量的指定长度（如{{truncate title 20}}）

#### 🔧 技术改进
- **🗄️ 重大架构升级** - 视频源配置从配置文件迁移到数据库，config.toml不再包含视频源信息
- **📝 动态配置加载** - 所有视频源类型立即应用更新的配置模板
- **🛡️ 安全时机控制** - 配置检测在安全时机执行，不中断活跃下载任务
- **🎯 智能文件匹配** - NFO文件重新生成时智能匹配现有视频文件格式
- **🔄 完整认证流程** - 通过API调用验证Token，完整的状态管理和错误处理
- **⚡ 视频源管理优化** - 支持动态增删改查，无需重启程序，实时生效
- **⏰ 智能等待机制** - 扫描间隔等待期间智能检测配置更新，支持配置热切换
- **⏸️ 任务暂停控制** - 重命名操作时自动暂停扫描任务，避免数据库锁定冲突
- **📊 日志系统优化** - 404错误降级为debug日志，减少无意义的错误信息干扰

#### 📋 管理界面改进
- **👆 一键操作** - 支持Enter键登录，操作更加便捷
- **💡 详细说明** - 配置页面增加完整的使用提示和变量说明
- **🎨 美观设计** - 专业的登录卡片设计，提升用户体验
- **🔍 实时反馈** - 友好的错误提示，区分Token错误和网络错误

### 💡 技术优势

- **高性能**: Rust + Tokio 异步架构，支持高并发下载
- **易部署**: 提供多平台二进制文件和 Docker 镜像
- **易使用**: 友好的 Web 管理界面，无需命令行操作
- **易维护**: 清晰的项目结构和完整的开发文档
- **高稳定**: 智能错误处理和自动重试机制

## 快速开始

### 开发环境设置

```bash
# 克隆项目
git clone <repository-url>
cd bili-sync

# 方法 1: 使用批处理文件 (推荐)
.\make.bat setup

# 方法 2: 使用 PowerShell 脚本
# 首先设置执行策略
Set-ExecutionPolicy -ExecutionPolicy Bypass -Scope Process
.\make.ps1 setup

# 启动开发服务器
.\make.bat dev
# 或
.\make.ps1 dev
```

### 常用命令

```bash
# 查看所有可用任务
.\make.bat help

# 运行测试
.\make.bat test

# 构建项目
.\make.bat build

# 构建发布版本
.\make.bat release

# 清理构建文件
.\make.bat clean
```

> **注意**: 如果您遇到 PowerShell 执行策略问题，建议使用 `.\make.bat` 命令，或者先运行 `Set-ExecutionPolicy -ExecutionPolicy Bypass -Scope Process` 来临时允许脚本执行。

## 项目结构

```
├── crates/                 # Rust 后端代码
│   ├── bili_sync/          # 主应用程序
│   ├── bili_sync_entity/   # 数据库实体定义
│   └── bili_sync_migration/# 数据库迁移脚本
├── web/                    # Svelte 前端应用
├── docs/                   # VitePress 文档站点
├── scripts/                # 辅助工具脚本
├── assets/                 # 项目资源文件
├── make.bat               # Windows 批处理任务脚本
└── make.ps1               # PowerShell 任务脚本
```

详细的项目结构说明请查看 [PROJECT_STRUCTURE.md](./PROJECT_STRUCTURE.md)

## 效果演示

### 概览
![概览](./assets/overview.webp)
### 详情
![详情](./assets/detail.webp)
### 播放（使用 infuse）
![播放](./assets/play.webp)
### 文件排布
![文件](./assets/dir.webp)

## 🖼️ 功能截图

想要查看最新版本的详细功能截图？请访问[功能展示文档](./docs/features.md)，包含：

- 📁 收藏夹智能管理界面
- 🎬 UP主合集选择功能
- 🔍 增强的搜索结果展示
- 📺 番剧和影视搜索界面

所有截图均基于最新版本，展示了Web管理界面的各项新功能。

## 功能与路线图

### 🎯 核心功能
- [x] 使用用户填写的凭据认证，并在必要时自动刷新
- [x] **可以在管理页面 `0.0.0.0:12345` 内添加视频源** 🆕
- [x] 支持收藏夹与视频列表/视频合集的下载
- [x] **支持直接显示和选择用户收藏夹，无需手动输入ID** 🆕
- [x] **支持UP主合集列表展示和快速选择** 🆕
- [x] **支持番剧的下载，包括单季模式和全季模式** 🆕
- [x] 自动选择用户设置范围内最优的视频和音频流，并在下载完成后使用 FFmpeg 合并          
- [x] 使用 Tokio 与 Reqwest，对视频、视频分页进行异步并发下载
- [x] 使用媒体服务器支持的文件命名，方便一键作为媒体库导入
- [x] 当前轮次下载失败会在下一轮下载时重试，失败次数过多自动丢弃
- [x] 使用数据库保存媒体信息，避免对同个视频的多次请求
- [x] 打印日志，并在请求出现风控时自动终止，等待下一轮执行
- [x] 提供多平台的二进制可执行文件，为 Linux 平台提供了立即可用的 Docker 镜像
- [x] 支持对"稍后再看"内视频的自动扫描与下载
- [x] 支持对 UP 主投稿视频的自动扫描与下载
- [x] 支持限制任务的并行度和接口请求频率

### 🚀 新增特性
- [x] **Web 管理界面增强** - 通过友好的 Web 界面直接管理视频源，无需手动编辑配置文件
- [x] **收藏夹管理优化** - 自动获取并显示用户所有收藏夹，支持快速选择，解决ID精度问题
- [x] **UP主合集支持** - 输入UP主ID即可查看其所有合集和系列，支持一键选择添加
- [x] **搜索结果改进** - 搜索结果在右侧主区域显示，支持分页、网格布局、缩略图预览
- [x] **番剧下载支持** - 完整支持番剧下载，包括单季和全季模式
- [x] **配置文件格式升级** - 新的 V2 配置格式，更简洁（视频源已迁移到数据库）
- [x] **Docker 部署优化** - 多阶段构建、健康检查、更好的缓存策略
- [x] **开发工具链改进** - 完整的开发环境设置和构建脚本
- [x] **智能任务调度** - 重命名操作时自动暂停扫描任务，避免数据库锁定冲突
- [x] **API Token认证机制** - 安全的管理页面访问控制，支持Token验证和状态管理
- [x] **多线程下载配置** - 可视化配置多线程下载参数，提升大文件下载效率
- [x] **智能配置热更新** - 配置修改后立即生效，无需重启程序
- [x] **NFO文件智能重新生成** - 配置更改后自动匹配现有文件格式重新生成NFO
- [x] **数据库存储架构** - 视频源完全迁移到数据库存储，配置文件不再包含视频源，管理更加灵活

### 🔄 持续改进
- [x] **项目结构优化** - 更清晰的代码组织和技术栈说明
- [x] **UI 组件扩展** - 新增对话框、标签、选择器等交互组件
- [x] **搜索界面优化** - 搜索结果展示从左侧移至右侧主区域，支持网格布局和分页
- [x] **数据库兼容性** - 修复旧版本数据库升级问题，自动添加缺失的字段
- [x] **ID精度处理** - 解决JavaScript大数字精度问题，确保64位ID正确处理
- [x] **配置管理增强** - 统一的 `config.toml` 配置文件，支持运行时修改（不再包含视频源）
- [x] **文档和开发体验** - 详细的快速开始指南和贡献指南
- [x] **错误处理改进** - 修复多个关键错误，提升程序稳定性
- [x] **性能优化** - 智能文件匹配和动态配置加载机制
- [x] **用户体验提升** - 更直观的界面设计和操作反馈
- [x] **代码质量** - 清理编译警告，实现干净的代码库

### 📋 开发路线图
- [ ] 下载单个文件时支持断点续传与并发下载
- [ ] 更多媒体服务器兼容性优化
- [ ] 高级过滤和搜索功能
- [ ] 批量操作和管理功能

## 开发指南

### 技术栈

- **后端**: Rust + Tokio + Axum + SeaORM
- **前端**: Svelte + TypeScript + Tailwind CSS
- **数据库**: SQLite
- **文档**: VitePress
- **部署**: Docker + Docker Compose

### 开发流程

1. **环境准备**: 确保安装了 Rust 和 Node.js
2. **依赖安装**: 运行 `.\make.bat setup`
3. **启动开发**: 运行 `.\make.bat dev`
4. **代码检查**: 运行 `.\make.bat lint`
5. **运行测试**: 运行 `.\make.bat test`

### 可用的任务脚本

项目提供了多种任务脚本供您选择：

- **`make.bat`** - Windows 批处理文件，无需特殊权限
- **`make.ps1`** - PowerShell 脚本，功能更丰富

### 贡献指南

1. Fork 本项目
2. 创建功能分支 (`git checkout -b feature/AmazingFeature`)
3. 提交更改 (`git commit -m 'Add some AmazingFeature'`)
4. 推送到分支 (`git push origin feature/AmazingFeature`)
5. 创建 Pull Request

## 参考与借鉴

该项目实现过程中主要参考借鉴了如下的项目，感谢他们的贡献：

+ [bilibili-API-collect](https://github.com/SocialSisterYi/bilibili-API-collect) B 站的第三方接口文档
+ [bilibili-api](https://github.com/Nemo2011/bilibili-api) 使用 Python 调用接口的参考实现
+ [danmu2ass](https://github.com/gwy15/danmu2ass) 本项目弹幕下载功能的缝合来源
