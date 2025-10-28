# API Gateway Dockerfile
FROM rust:1.75.0-slim as builder

# 设置工作目录
WORKDIR /app

# 安装系统依赖
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# 复制共享库
COPY ./shared /shared
RUN cd /shared && cargo build --release

# 复制API Gateway Cargo文件
COPY ./api-gateway/Cargo.toml ./api-gateway/Cargo.lock ./

# 创建源代码目录并复制
RUN mkdir src
COPY ./api-gateway/src ./src

# 构建API Gateway
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
RUN useradd -m -u 1000 gateway

# 设置工作目录
WORKDIR /app

# 从构建阶段复制编译后的二进制文件
COPY --from=builder /app/target/release/mvp_demo /usr/local/bin/
COPY --from=builder /shared/target/release/libecho_shared.so /usr/local/lib/

# 更改文件所有权
RUN chown -R gateway:gateway /app

# 切换到非root用户
USER gateway

# 暴露端口
EXPOSE 8080

# 健康检查
HEALTHCHECK --interval=30s --timeout=10s --start-period=60s --retries=3 \
    CMD curl -f http://localhost:8080/health || exit 1

# 启动API Gateway
CMD ["mvp_demo"]