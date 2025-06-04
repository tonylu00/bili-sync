# 📚 bili-sync 文档中心

> [!TIP]
> 当前版本：v2.6.2 | 上一稳定版：v2.5.1 | [版本对比详情](/version-comparison)

欢迎来到 bili-sync 的文档站点！这里包含了完整的使用指南、配置说明和开发文档。

## 🎯 快速导航

### 🚀 新用户入门
- **[项目介绍](/introduction)** - 了解 bili-sync 的功能和特性
- **[快速开始](/quick-start)** - 快速部署和配置指南
- **[程序部署指南](/program-deploy-guide)** - 详细的获取和运行说明

### 🖼️ 功能体验
- **[🌟 完整功能展示](/all-features)** - 所有功能的详细介绍和截图 ⭐ **推荐**
- **[功能展示](/features)** - Web 界面功能完整截图展示
- **[新版界面预览](/introduction#🖼️-新版-web-界面展示)** - v2.6.2 界面亮点

### ⚙️ 配置管理
- **[配置说明](/configuration)** - 详细的配置参数文档
- **[收藏夹管理](/favorite)** - 收藏夹配置说明（传统方式）
- **[合集管理](/collection)** - 视频合集/列表配置（传统方式）
- **[UP主投稿](/submission)** - UP主投稿下载配置（传统方式）

### 🛠️ 开发相关
- **[构建系统](/build-system)** - make.bat 构建系统详解
- **[项目设计](/design)** - 技术架构和设计理念
- **[前端开发](/frontend)** - 前端开发说明
- **[文档部署](/document-deploy-guide)** - 文档站点部署指南

## 📋 版本信息与迁移

### 🔄 版本对比与升级
- **[🆚 版本详细对比](/version-comparison)** - v2.5.1 vs v2.6.2 完整对比
- **[📋 全面对比总结](/comprehensive-comparison-summary)** - 带界面截图的详细对比
- **[📋 配置迁移指南](../MIGRATION_GUIDE.md)** - 从旧版本升级步骤
- **[📝 更新日志 2024-06](/update-log-2024-06)** - 2024年6月更新记录
- **[📝 更新日志 2025-06](/update-log-2025-06)** - 2025年6月更新记录

### 🔧 功能更新文档
- **[番剧合并修复](/bangumi-merge-fix)** - 番剧下载问题修复说明
- **[前端构建修复](/frontend-build-fix-2025-06-04)** - 前端构建流程改进
- **[综合更新](/comprehensive-update-2025-06-03)** - 重大功能更新详解
- **[任务控制优化](/task-control-optimization)** - 任务调度系统改进
- **[重置功能修复](/reset-function-fix)** - 系统重置功能完善
- **[风险警告功能](/risk-warning-feature)** - 新增安全特性说明

### 🎯 工具和参考
- **[构建产物指南](/artifacts-guide)** - GitHub Actions 构建说明
- **[构建产物快速参考](/artifacts-quick-reference)** - 一页纸快速操作指南
- **[功能截图指南](/artifacts-screenshots-guide)** - 图文并茂的操作步骤
- **[GitHub Actions 构建](/github-actions-build)** - CI/CD 配置详解
- **[参数说明](/args)** - 命令行参数参考
- **[常见问题](/question)** - FAQ 和故障排除

## 🆕 v2.6.2 版本亮点

### 🌟 革命性改进
- **🎨 数据库驱动架构** - 视频源配置完全迁移到数据库，告别手动编辑配置文件
- **🖥️ 现代化 Web UI** - 全新的管理界面，直观友好的操作体验
- **🛠️ Windows 构建支持** - 新增 make.bat 构建系统，简化开发流程
- **🐳 Docker 部署优化** - 提供 Docker Compose 配置，一键部署

### 🎯 核心新功能
- **📁 收藏夹智能管理** - 自动获取并显示用户所有收藏夹，支持一键选择
- **🎬 UP主合集支持** - 输入UP主ID即可浏览和选择其所有合集/系列
- **📺 番剧下载增强** - 完整支持番剧下载，包括单季和全季模式
- **🔍 搜索体验升级** - 网格布局展示，支持分页、实时预览
- **🔐 API Token 认证** - 安全的管理页面访问控制
- **🔥 配置热更新** - 配置修改立即生效，无需重启程序

### 📈 技术优势
- **⚡ 多线程下载配置** - 可视化配置下载参数，提升效率
- **🗄️ NFO文件智能重新生成** - 配置更改后自动匹配现有文件格式
- **🔧 错误处理改进** - 修复多个关键错误，提升程序稳定性
- **✨ 代码质量提升** - 清理编译警告，实现干净的代码库

## 🔄 升级指南

### 从 v2.5.1 升级到 v2.6.2

1. **📖 查看版本对比**
   - 详细阅读 [版本对比文档](/version-comparison)
   - 了解主要变更和新功能

2. **💾 备份现有配置**
   ```bash
   cp config.toml config.toml.backup
   cp data.sqlite data.sqlite.backup  # 如果存在
   ```

3. **📋 阅读迁移指南**
   - 查看 [配置迁移文档](../MIGRATION_GUIDE.md)
   - 了解视频源配置变更

4. **🛠️ 使用新的构建系统**（可选）
   ```bash
   # Windows 用户
   .\make.bat setup
   .\make.bat dev
   
   # 或继续使用传统方式
   cargo run
   ```

5. **🖥️ 重新配置视频源**
   - 启动程序后访问 Web 管理界面
   - 通过 UI 重新添加视频源
   - 验证下载功能正常

## 💡 使用建议

### 🆕 新用户推荐路径
1. **📚 了解项目** - 从 [项目介绍](/introduction) 开始
2. **🚀 快速部署** - 查看 [快速开始](/quick-start) 指南
3. **🖼️ 功能体验** - 浏览 [功能展示](/features) 了解 Web 界面
4. **⚙️ 详细配置** - 参考 [配置说明](/configuration) 进行个性化设置

### 👨‍💻 开发者推荐路径
1. **🏗️ 构建环境** - 查看 [构建系统](/build-system) 文档
2. **🎨 架构设计** - 了解 [项目设计](/design) 理念
3. **🖥️ 前端开发** - 参考 [前端开发](/frontend) 指南
4. **📚 文档贡献** - 查看 [文档部署](/document-deploy-guide) 指南

### 🔄 升级用户推荐路径
1. **🔍 版本对比** - 详细阅读 [版本对比](/version-comparison) 文档
2. **📋 迁移步骤** - 按照 [配置迁移指南](../MIGRATION_GUIDE.md) 操作
3. **📝 更新记录** - 关注相关 [更新日志](/update-log-2025-06)
4. **🆕 新功能** - 体验 [新版界面](/features) 功能

## 🎯 常见使用场景

### 🔥 我想要现成的程序
- **解决方案**：使用 GitHub Actions 云端编译
- **文档**：[操作截图指南](/artifacts-screenshots-guide)
- **快速**：[快速参考卡片](/artifacts-quick-reference)

### 🔥 我想修改代码后编译
- **解决方案**：修改代码 → 推送 → 自动编译
- **文档**：[Artifacts 完整指南](/artifacts-guide)
- **技术**：[GitHub Actions 说明](/github-actions-build)

### 🔥 我想在本地开发
- **解决方案**：使用 make.bat 构建系统
- **文档**：[构建系统指南](/build-system)
- **快速**：`.\make.bat setup && .\make.bat dev`

### 🔥 我想部署到服务器
- **解决方案**：使用 Docker 容器
- **文档**：[程序部署指南](/program-deploy-guide)
- **快速**：`docker-compose up -d`

## 📱 移动端用户

**好消息！** GitHub Actions 支持手机操作：

1. **📱 下载 GitHub App**
2. **👤 登录账号**
3. **📂 进入仓库 → Actions**
4. **⚡ 触发编译和查看状态**
5. **📥 在浏览器中下载文件**

详细步骤见：[操作截图指南](/artifacts-screenshots-guide)

## 🚨 遇到问题？

### 📋 问题排查清单

**编译问题：**
- ✅ 检查编译状态是否为 Success
- ✅ 查看详细错误日志  
- ✅ 尝试重新运行编译

**下载问题：**
- ✅ 确认编译已完成
- ✅ 检查 Artifacts 是否存在
- ✅ 确认文件未过期（90天）

**运行问题：**
- ✅ 下载了正确的平台版本
- ✅ 完整解压了文件
- ✅ Linux/macOS 添加了执行权限

### 📞 获取帮助

1. **📚 查看文档** - 先查看相关文档
2. **🔍 搜索 Issues** - 在仓库中搜索类似问题
3. **📝 提交 Issue** - 描述详细的问题和错误信息
4. **📊 查看日志** - 提供编译或运行的错误日志

## 🔗 相关链接

- **🏠 项目仓库**：[GitHub](https://github.com/amtoaer/bili-sync)
- **📖 在线文档**：[https://bili-sync.allwens.work/](https://bili-sync.allwens.work/)
- **📦 发布页面**：[GitHub Releases](https://github.com/amtoaer/bili-sync/releases)
- **🐛 问题反馈**：[GitHub Issues](https://github.com/amtoaer/bili-sync/issues)
- **🔧 编译地址**：[GitHub Actions](https://github.com/qq1582185982/bili-sync-01/actions)

## 🎉 成功案例

### ✅ 编译成功的标志
- 所有平台显示 ✅ Success
- Artifacts 部分有 5 个文件
- 文件大小约 9-20 MB

### ✅ 正确的文件名
```
bili-sync-rs-Windows-x86_64.zip        (19.8 MB)
bili-sync-rs-Linux-x86_64-musl.tar.gz  (10 MB)
bili-sync-rs-Linux-aarch64-musl.tar.gz (9.35 MB)
bili-sync-rs-Darwin-x86_64.tar.gz      (9.67 MB)
bili-sync-rs-Darwin-aarch64.tar.gz     (9.3 MB)
```

---

📝 **文档持续更新中**，如有问题或建议，欢迎提交 Issue 或 Pull Request！

**项目地址：** https://github.com/qq1582185982/bili-sync-01  
**编译地址：** https://github.com/qq1582185982/bili-sync-01/actions 