# 程序部署指南

本指南将帮助你部署和运行 bili-sync 程序。

## 程序获取

程序为各个平台提供了预构建的二进制文件，并且打包了 `Linux/amd64` 与 `Linux/arm64` 两个平台的 Docker 镜像。用户可以自行选择使用哪种方式运行。

### 其一：下载平台二进制文件运行

> [!CAUTION]
> 如果你使用这种方式运行，请确保 FFmpeg 已被正确安装且位于 PATH 中，可通过执行 `ffmpeg` 命令访问。

在[程序发布页](https://github.com/amtoaer/bili-sync/releases)选择最新版本中对应机器架构的压缩包，解压后会获取一个名为 `bili-sync-rs` 的可执行文件，直接双击执行。

### 其二： 使用 Docker Compose 运行

Linux/amd64 与 Linux/arm64 两个平台可直接使用 Docker 或 Docker Compose 运行，此处以 Compose 为例：
> 请注意其中的注释，有不清楚的地方可以先继续往下看。

```yaml
services:
  bili-sync-rs:
    # 不推荐使用 latest 这种模糊的 tag，最好直接指明版本号
    image: amtoaer/bili-sync-rs:latest
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