# Bridge Service Dockerfile
FROM rust:1.90.0-slim AS builder

# 设置工作目录
WORKDIR /app

# 安装系统依赖
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    cmake \
    build-essential \
    && rm -rf /var/lib/apt/lists/*

# 复制 shared库
COPY shared /shared

# 复制Bridge服务Cargo文件
COPY bridge/Cargo.toml bridge/Cargo.lock ./

# 创建源代码目录并复制
RUN mkdir src
COPY bridge/src ./src

# 构建Bridge服务
# 设置环境变量禁用 SQLx 编译时检查
ENV SQLX_OFFLINE=true
RUN cargo build --release

# 运行阶段
FROM debian:bookworm-slim

# 安装运行时依赖
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    curl \
    && rm -rf /var/lib/apt/lists/*

# 创建非root用户
RUN useradd -m -u 1000 bridge

# 设置工作目录
WORKDIR /app

# 从构建阶段复制编译后的二进制文件
COPY --from=builder /app/target/release/echo-bridge /usr/local/bin/

# 创建配置目录
RUN mkdir -p /app/config

# 更改文件所有权
RUN chown -R bridge:bridge /app

# 切换到非root用户
USER bridge

# 暴露端口
EXPOSE 8081 8082

# 健康检查
HEALTHCHECK --interval=30s --timeout=10s --start-period=60s --retries=3 \
    CMD curl -f http://localhost:8082/health || exit 1

# 启动Bridge服务
CMD ["echo-bridge"]