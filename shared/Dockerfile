# Echo Shared Library Dockerfile
FROM rust:1.90.0-slim AS builder

# 设置工作目录
WORKDIR /app

# 安装系统依赖
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# 复制共享库源代码
COPY . .

# 构建共享库
RUN cargo build --release

# 运行阶段镜像（将被其他服务继承）
FROM debian:bookworm-slim

# 安装运行时依赖
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# 创建非root用户
RUN useradd -m -u 1000 echo

# 设置工作目录
WORKDIR /app

# 从构建阶段复制编译后的共享库
COPY --from=0 /app/target/release/libecho_shared.so /usr/local/lib/
RUN ldconfig

# 切换到非root用户
USER echo

# 暴露端口（如果有需要的话）
# EXPOSE 9999

# 默认命令（这个镜像主要作为基础镜像使用）
CMD ["echo", "Echo Shared Library Ready"]