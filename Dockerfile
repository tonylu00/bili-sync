FROM alpine AS base

ARG TARGETPLATFORM

WORKDIR /app

RUN apk update && apk add --no-cache \
    ca-certificates \
    tzdata \
    ffmpeg \
    aria2

COPY ./bili-sync-rs-Linux-*.tar.gz  ./targets/

RUN if [ "$TARGETPLATFORM" = "linux/amd64" ]; then \
    tar xzvf ./targets/bili-sync-rs-Linux-x86_64-musl.tar.gz -C ./; \
    else \
    tar xzvf ./targets/bili-sync-rs-Linux-aarch64-musl.tar.gz -C ./; \
    fi

RUN rm -rf ./targets && chmod +x ./bili-sync-rs

FROM alpine

WORKDIR /app

# 安装运行时需要的依赖
RUN apk update && apk add --no-cache \
    ca-certificates \
    tzdata \
    ffmpeg \
    aria2

ENV LANG=zh_CN.UTF-8 \
    TZ=Asia/Shanghai \
    HOME=/app \
    RUST_BACKTRACE=1 \
    RUST_LOG=None,bili_sync=info

# 只复制必要的文件
COPY --from=base /app/bili-sync-rs /app/bili-sync-rs

ENTRYPOINT [ "/app/bili-sync-rs" ]

VOLUME [ "/app/.config/bili-sync" ]
