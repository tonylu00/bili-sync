# 文档部署指南

本指南将帮助你将 bili-sync 的文档部署到 GitHub Pages，让其他人可以在线访问。

## 🚀 自动部署（推荐）

我已经为你创建了 GitHub Actions 工作流文件（`.github/workflows/deploy-docs.yml`），它会自动构建和部署文档。

### 步骤 1：提交代码到 GitHub

```bash
git add .
git commit -m "feat: 添加文档自动部署"
git push origin main
```

### 步骤 2：启用 GitHub Pages

1. 访问你的 GitHub 仓库：https://github.com/qq1582185982/bili-sync-01
2. 点击 **Settings**（设置）标签
3. 在左侧菜单找到 **Pages**
4. 在 **Source** 部分，选择 **GitHub Actions**
5. 点击 **Save**

### 步骤 3：等待部署完成

1. 访问 **Actions** 标签页查看部署进度
2. 第一次部署可能需要几分钟
3. 部署成功后，你可以在以下地址访问文档：

   **https://qq1582185982.github.io/bili-sync-01/**

## 🛠️ 手动部署

如果你想在本地测试文档：

### 1. 安装依赖

```bash
cd docs
npm install
```

### 2. 本地预览

```bash
npm run docs:dev
```

访问 http://localhost:5173 查看文档

### 3. 构建文档

```bash
npm run docs:build
```

构建后的文件在 `docs/.vitepress/dist` 目录中

## 📝 常见问题

### Q: 为什么文档样式看起来不对？

A: 确保 `docs/.vitepress/config.mts` 中的 `base` 配置与你的仓库名一致：

```typescript
export default defineConfig({
  base: "/bili-sync-01/", // 必须与GitHub仓库名一致
  // ...
})
```

### Q: 如何更新文档？

A: 只需要修改文档文件并推送到 main 分支，GitHub Actions 会自动重新部署。

### Q: 部署失败怎么办？

A: 检查以下几点：
1. GitHub Pages 是否已启用
2. Actions 权限是否正确设置
3. 查看 Actions 日志排查错误

### Q: 看到 "Unable to resolve action" linter 错误？

A: 这些可能是编辑器插件的误报。我们使用的都是 GitHub 官方 actions 的正确版本：
- `actions/checkout@v4`
- `actions/setup-node@v4`
- `actions/configure-pages@v5`
- `actions/upload-pages-artifact@v3`
- `actions/deploy-pages@v4`

如果在实际运行时遇到问题，请检查 Actions 日志获取真实错误信息。

## 🔗 其他部署选项

除了 GitHub Pages，你还可以选择：

- **Vercel**: https://vercel.com/
- **Netlify**: https://www.netlify.com/
- **Cloudflare Pages**: https://pages.cloudflare.com/

这些平台通常提供更快的构建速度和更多功能。 