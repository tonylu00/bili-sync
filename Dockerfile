FROM alpine AS base

ARG TARGETPLATFORM

WORKDIR /app

RUN apk update && apk add --no-cache \
    ca-certificates \
    tzdata \
    ffmpeg \
    aria2

# 安装Rust构建环境
RUN apk add --no-cache \
    rust \
    cargo \
    musl-dev \
    nodejs \
    npm

# 复制源代码
COPY . .

# 构建前端
WORKDIR /app/web
RUN npm install && npm run build

# 构建后端
WORKDIR /app
RUN cargo build --release

FROM alpine AS runtime

WORKDIR /app

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

# 从构建阶段复制二进制文件
COPY --from=base /app/target/release/bili-sync-rs /app/bili-sync-rs

# 复制前端构建结果
COPY --from=base /app/web/build /app/web/build

RUN chmod +x /app/bili-sync-rs

ENTRYPOINT [ "/app/bili-sync-rs" ]

VOLUME [ "/app/.config/bili-sync" ]
