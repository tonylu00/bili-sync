# 构建系统指南

bili-sync v2.6.2 引入了现代化的构建系统，特别为 Windows 用户提供了便捷的批处理脚本。

## 🛠️ 构建工具对比

| 版本 | 构建工具 | 平台支持 | 特性 |
|------|----------|----------|------|
| v2.5.1 | Justfile | Linux/macOS | 基础构建 |
| v2.6.2 | make.bat | Windows 优先 | 一键设置、自动化构建 |

## 📋 make.bat 命令参考

### 基础命令

```bash
# 查看所有可用命令
.\make.bat help

# 一键设置开发环境
.\make.bat setup

# 启动开发服务器（前后端）
.\make.bat dev
```

### 开发命令

```bash
# 运行测试
.\make.bat test

# 格式化代码
.\make.bat fmt

# 代码检查
.\make.bat lint
```

### 构建命令

```bash
# 开发构建
.\make.bat build

# 发布构建
.\make.bat release

# 清理构建文件
.\make.bat clean

# 打包发布
.\make.bat package
```

### 文档命令

```bash
# 启动文档开发服务器
.\make.bat docs

# 构建文档
.\make.bat docs-build
```

### Docker 命令

```bash
# 构建 Docker 镜像
.\make.bat docker

# 启动 Docker Compose
.\make.bat compose
```

## 🚀 一键开发环境设置

### .\make.bat setup 详细流程

`setup` 命令会自动完成以下步骤：

1. **检查 Rust 环境**
   ```bash
   cargo --version  # 检查 Rust 是否已安装
   ```

2. **检查 Node.js 环境**
   ```bash
   node --version   # 检查 Node.js 是否已安装
   ```

3. **安装前端依赖**
   ```bash
   cd web
   npm install      # 安装 Svelte 前端依赖
   ```

4. **安装 autoprefixer**
   ```bash
   npm install autoprefixer --save-dev  # 确保 CSS 兼容性
   ```

5. **同步 SvelteKit**
   ```bash
   npx svelte-kit sync  # 同步 SvelteKit 配置
   ```

6. **构建前端**
   ```bash
   npm run build    # 构建前端静态资源
   ```

7. **检查 Rust 后端**
   ```bash
   cargo check      # 检查 Rust 依赖和编译
   ```

8. **安装文档依赖**
   ```bash
   cd docs
   npm install      # 安装 VitePress 文档依赖
   ```

### .\make.bat dev 开发模式

`dev` 命令会同时启动：

1. **Rust 后端服务** (端口 12345)
   ```bash
   cargo run --bin bili-sync-rs
   ```

2. **Svelte 前端开发服务器** (端口 5173)
   ```bash
   cd web && npm run dev
   ```

您可以访问：
- API 服务：http://localhost:12345
- 开发前端：http://localhost:5173

## 🔧 构建流程详解

### 开发构建 (.\make.bat build)

1. **前端构建检查**
   - 检查 `web/node_modules` 是否存在
   - 如不存在，自动运行 `npm install`

2. **autoprefixer 安装**
   - 确保 CSS 前缀自动添加功能可用

3. **SvelteKit 同步**
   - 同步框架配置和类型定义

4. **前端构建**
   - 运行 `npm run build` 生成静态资源

5. **Rust 后端构建**
   - 运行 `cargo build` 构建二进制文件

### 发布构建 (.\make.bat release)

类似开发构建，但使用优化编译：
```bash
cargo build --release
```

## 🐳 Docker 支持

### Docker 镜像构建

```bash
.\make.bat docker
```

等效于：
```bash
docker build -t bili-sync-rs .
```

### Docker Compose 部署

```bash
.\make.bat compose
```

等效于：
```bash
docker-compose up -d
```

使用项目根目录的 `docker-compose.yml` 配置。

## 📚 文档系统

### 文档开发

```bash
.\make.bat docs
```

启动 VitePress 开发服务器，通常在 http://localhost:5173

### 文档构建

```bash
.\make.bat docs-build
```

构建静态文档到 `docs/.vitepress/dist/`

## 🔍 故障排除

### 常见问题

1. **权限错误**
   ```bash
   # 如果遇到 PowerShell 执行策略问题
   Set-ExecutionPolicy -ExecutionPolicy Bypass -Scope Process
   ```

2. **Node.js 未安装**
   - 访问 https://nodejs.org/ 下载安装

3. **Rust 未安装**
   - 访问 https://rustup.rs/ 安装 Rust

4. **端口占用**
   ```bash
   # 检查端口占用
   netstat -ano | findstr :12345
   netstat -ano | findstr :5173
   ```

### 清理和重置

```bash
# 清理所有构建文件
.\make.bat clean

# 如果需要完全重置
rmdir /s /q web\node_modules
rmdir /s /q docs\node_modules
rmdir /s /q target
.\make.bat setup
```

## 🆚 传统构建方式

如果您更喜欢传统的手动构建方式：

```bash
# 后端开发
cargo run --bin bili-sync-rs

# 前端开发
cd web
npm install
npm run dev

# 前端构建
npm run build

# 后端构建
cargo build --release
```

## 💡 最佳实践

1. **首次使用**：运行 `.\make.bat setup` 设置环境
2. **日常开发**：使用 `.\make.bat dev` 启动开发环境
3. **代码检查**：提交前运行 `.\make.bat lint` 和 `.\make.bat test`
4. **发布构建**：使用 `.\make.bat release` 进行优化构建

---

构建系统让 bili-sync 的开发和部署变得更加简单！如有问题，请查看日志输出或提交 Issue。 