# 快速开始

> [!IMPORTANT]
> **项目来源说明**
> 
> 本项目 (`qq1582185982/bili-sync-01`) 是从原作者 [amtoaer/bili-sync](https://github.com/amtoaer/bili-sync) 的项目 `fork` 并进行大量更新和优化的版本。我们对原作者的贡献表示感谢，并在此基础上持续迭代。

程序使用 Rust 编写，不需要 Runtime 且并为各个平台提供了预编译文件，绝大多数情况下是没有使用障碍的。

> [!TIP]
> **v2.7.3 重大更新**：
> - 配置系统全面升级，支持热重载
> - 文件名处理增强，自动处理所有特殊字符
> - 初始设置向导，简化配置流程
> - 任务队列优化，彻底解决数据库锁定问题

> [!IMPORTANT]
> 如果您从旧版本升级，请查看 [配置迁移指南](./MIGRATION_GUIDE)。

## 🚀 开发环境快速设置

如果您想参与开发或从源码构建，推荐使用我们的一键设置：

### Windows 用户（推荐）
```bash
# 一键设置开发环境
.\make.bat setup

# 启动开发服务器
.\make.bat dev

# 查看所有可用命令
.\make.bat help
```

### 其他平台或传统方式
```bash
# 确保安装了 Rust 和 Node.js
cargo --version
node --version

# 安装前端依赖
cd web && npm install && cd ..

# 构建前端
cd web && npm run build && cd ..

# 运行程序
cargo run --bin bili-sync-rs
```

## 程序获取

程序为各个平台提供了预构建的二进制文件，并且打包了 `Linux/amd64` 与 `Linux/arm64` 两个平台的 Docker 镜像。用户可以自行选择使用哪种方式运行。

### 其一：下载平台二进制文件运行

> [!CAUTION]
> 如果你使用这种方式运行，请确保 FFmpeg 已被正确安装且位于 PATH 中，可通过执行 `ffmpeg` 命令访问。

在[程序发布页](https://github.com/qq1582185982/bili-sync-01/releases)选择最新版本中对应机器架构的压缩包，解压后会获取一个名为 `bili-sync-rs` 的可执行文件，直接双击执行。

### 其二： 使用 Docker Compose 运行

Linux/amd64 与 Linux/arm64 两个平台可直接使用 Docker 或 Docker Compose 运行，此处以 Compose 为例：

> [!TIP]
> **Docker 镜像选择**：我们提供了两个镜像地址供您选择：
> - **GitHub 镜像**：`qq1582185982/bili-sync:latest` 
> - **国内镜像**：`docker.cnb.cool/sviplk.com/docker/bili-sync:latest`
> 
> 建议国内用户使用国内镜像以获得更好的下载速度。

> 请注意其中的注释，有不清楚的地方可以先继续往下看。

```yaml
services:
  bili-sync-rs:
    # v2.7.3 最新版镜像
    # GitHub 镜像（国外用户推荐）
    image: qq1582185982/bili-sync:v2.7.3
    # 国内镜像（国内用户推荐）
    # image: docker.cnb.cool/sviplk.com/docker/bili-sync:v2.7.3
    restart: unless-stopped
    network_mode: bridge
    # 该选项请仅在日志终端支持彩色输出时启用，否则日志中可能会出现乱码
    tty: true
    # 非必需设置项，推荐设置为宿主机用户的 uid 及 gid (`$uid:$gid`)
    # 可以执行 `id ${user}` 获取 `user` 用户的 uid 及 gid
    # 程序下载的所有文件权限将与此处的用户保持一致，不设置默认为 Root
    user: 1000:1000
    hostname: bili-sync-rs
    container_name: bili-sync-rs
    # 程序默认绑定 0.0.0.0:12345 运行 http 服务
    # 可同时修改 compose 文件与 config.toml 变更服务运行的端口
    ports:
      - 12345:12345
    volumes:
      - ${你希望存储程序配置的目录}:/app/.config/bili-sync
      # 还需要有一些其它必要的挂载，包括 up 主信息位置、视频下载位置
      # 这些目录不是固定的，只需要确保此处的挂载与 bili-sync-rs 的配置文件相匹配
      # ...
    # 如果你使用的是群晖系统，请移除最后的 logging 配置，否则会导致日志不显示
    logging:
      driver: "local"
```

使用该 compose 文件，执行 `docker compose up -d` 即可运行。

## 程序配置

程序首次运行时会显示初始设置向导，引导您完成基本配置。配置现在完全存储在数据库中，支持热重载。

> [!IMPORTANT]
> **v2.7.3 配置系统变更！** 
> - ✅ 首次启动显示初始设置向导
> - ✅ 配置完全存储在数据库中，支持热重载
> - ✅ 所有视频源通过 Web 管理界面进行管理
> - ❌ config.toml 仅作为初始配置参考

### 配置文件位置

程序默认会将配置文件存储于 `${config_dir}/bili-sync/config.toml`，数据库文件存储于 `${config_dir}/bili-sync/data.sqlite`。

`config_dir` 的实际位置与操作系统和用户名有关。例如对于名为 Alice 的用户：

- **Linux**: `/home/Alice/.config`
- **Windows**: `C:\Users\Alice\AppData\Roaming`
- **macOS**: `/Users/Alice/Library/Application Support`
- **Docker**: `/app/.config` (容器内路径)

### 基础配置

以下是 `config.toml` 的一个示例，您只需要关注其中少数几个关键选项。

```toml
auth_token = "xxxxxxxx"
bind_address = "0.0.0.0:12345"
video_name = "{{title}}"
page_name = "{{bvid}}"
interval = 1200
upper_path = "/path/to/your/metadata/people" # 例如 Emby/Jellyfin 的元数据路径
nfo_time_type = "favtime"
time_format = "%Y-%m-%d"
cdn_sorting = false

[credential]
sessdata = ""
bili_jct = ""
buvid3 = ""
dedeuserid = ""
ac_time_value = ""

# ... 其他配置项保持默认即可
```

#### 关键配置项说明

- **`auth_token`**: 访问 Web UI 的密码，请务必修改为一个安全的字符串。
- **`bind_address`**: Web Server 监听的地址和端口。
- **`interval`**: 程序自动扫描同步的间隔时间（秒）。
- **`credential`**: 哔哩哔哩账号的身份凭据，是访问B站API所必需的。

> [!TIP]
> v2.7.3 新特性：
> - 配置修改后立即生效，无需重启
> - 通过 Web 界面的设置页面管理所有配置
> - 支持配置历史记录查看

## Web UI 使用指南

`bili-sync` 提供了一个现代化的 Web界面，用于管理和监控下载任务。

### 访问 Web UI

启动程序后，在浏览器中打开 `http://<你的IP地址>:12345` 即可访问。如果是本机运行，则为 `http://127.0.0.1:12345`。

首次访问时：
- 如果尚未设置凭据，会显示初始设置向导
- 如果已设置凭据，需要输入 `auth_token` 进行身份验证

### 管理视频源

所有视频源（如下载列表）的管理都已迁移至 Web UI。

#### 添加视频源

1.  在侧边栏点击 "添加视频源"。
2.  选择您想添加的视频源类型。
3.  根据提示输入必要的信息，或使用内置的搜索功能查找内容。
4.  设置一个名称和本地保存路径。
5.  点击"添加"即可。

支持的视频源类型及ID获取方式如下：

| 类型 | 说明 | 如何获取ID |
| :--- | :--- | :--- |
| **UP主投稿** | 下载指定UP主的所有视频 | **UP主ID (`uid`)**: 进入UP主空间，URL中的数字 (例如 `space.bilibili.com/123456`)。 |
| **收藏夹** | 下载指定收藏夹的所有视频 | **收藏夹ID (`fid`)**: 进入收藏夹页面，URL中的 `fid` 参数值。 |
| **合集/系列**| 下载B站的视频合集或系列 | **合集/系列ID**: 在合集或系列播放页面URL中寻找 `sid` 或 `series_id`。 |
| **番剧** | 下载指定的番剧 | **番剧ID (`season_id`)**: 在番剧播放页面URL中寻找 `season_id` 或 `ss` 后面的数字。 |
| **稍后观看** | 同步您的"稍后观看"列表 | 无需额外ID，自动同步。 |

> [!TIP]
> 推荐使用页面内的 **搜索功能**，通过关键词搜索视频、UP主或番剧，然后点击选择，系统会自动为您填充大部分信息，非常方便。

#### 查看和管理

您添加的所有视频源都会在 "订阅管理" 页面以卡片形式展示，您可以方便地：
- 查看同步状态和最近的视频
- 手动触发一次同步
- 编辑名称和保存路径
- 删除视频源

## 下一步

现在您已经基本了解了 `bili-sync` 的使用方法，可以深入探索更多功能了：
- 阅读 [配置文件](./configuration) 的详细说明。
- 阅读具体的视频源管理指南，例如 [收藏夹管理](./favorite) 和 [UP主投稿管理](./submission)。

## 运行

在配置文件填写完毕后，我们可以直接运行程序。如果配置文件无误，程序会自动开始下载收藏夹中的视频。并每隔 `interval` 秒重新扫描一次。

> [!SUCCESS]
> **现代化工作流程**：
> 1. 配置好 `credential` 和基本设置
> 2. 启动程序
> 3. 访问 `http://127.0.0.1:12345` Web 界面
> 4. 通过界面添加和管理视频源
> 5. 享受自动下载服务！

如果你希望了解更详细的配置项说明，可以查询[这里](/configuration)。
