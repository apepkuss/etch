#!/bin/bash

# EchoKit Server 下载和部署脚本
# 从 GitHub releases 下载最新的 EchoKit Server

set -e

# 颜色定义
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# 配置
ECHOKIT_VERSION="v0.1.2"
ECHOKIT_REPO="second-state/echokit_server"
DOWNLOAD_DIR="./downloads"
ECHOKIT_SERVER_DIR="./echokit-server-deployment"

# 打印函数
print_info() {
    echo -e "${BLUE}ℹ $1${NC}"
}

print_success() {
    echo -e "${GREEN}✓ $1${NC}"
}

print_warning() {
    echo -e "${YELLOW}⚠ $1${NC}"
}

print_error() {
    echo -e "${RED}✗ $1${NC}"
}

# 检查依赖
check_dependencies() {
    print_info "检查依赖..."

    if ! command -v curl &> /dev/null; then
        print_error "curl 未安装"
        exit 1
    fi

    if ! command -v tar &> /dev/null; then
        print_error "tar 未安装"
        exit 1
    fi

    print_success "依赖检查通过"
}

# 获取最新版本
get_latest_version() {
    print_info "获取 EchoKit Server 最新版本..."

    local api_url="https://api.github.com/repos/$ECHOKIT_REPO/releases/latest"

    if command -v jq &> /dev/null; then
        ECHOKIT_VERSION=$(curl -s "$api_url" | jq -r '.tag_name')
    else
        # 如果没有 jq，使用 grep 和 cut
        ECHOKIT_VERSION=$(curl -s "$api_url" | grep '"tag_name":' | cut -d '"' -f 4)
    fi

    if [ -z "$ECHOKIT_VERSION" ] || [ "$ECHOKIT_VERSION" = "null" ]; then
        print_warning "无法获取最新版本，使用默认版本: $ECHOKIT_VERSION"
    else
        print_success "找到最新版本: $ECHOKIT_VERSION"
    fi
}

# 创建目录
create_directories() {
    print_info "创建目录..."

    mkdir -p "$DOWNLOAD_DIR"
    mkdir -p "$ECHOKIT_SERVER_DIR"

    print_success "目录创建完成"
}

# 检测系统架构
detect_architecture() {
    local arch=$(uname -m)

    case $arch in
        x86_64)
            arch="x86_64"
            ;;
        aarch64|arm64)
            arch="aarch64"
            ;;
        *)
            print_error "不支持的架构: $arch"
            exit 1
            ;;
    esac

    echo "$arch"
}

# 下载 EchoKit Server
download_echokit_server() {
    print_info "下载 EchoKit Server $ECHOKIT_VERSION..."

    local arch=$(detect_architecture)
    local filename="echokit_server-${ECHOKIT_VERSION}-${arch}-unknown-linux-gnu.tar.gz"
    local download_url="https://github.com/$ECHOKIT_REPO/releases/download/$ECHOKIT_VERSION/$filename"
    local download_path="$DOWNLOAD_DIR/$filename"

    # 检查是否已经下载
    if [ -f "$download_path" ]; then
        print_warning "文件已存在，跳过下载: $filename"
    else
        print_info "下载地址: $download_url"

        if curl -L -o "$download_path" "$download_url"; then
            print_success "下载完成: $filename"
        else
            print_error "下载失败: $filename"
            exit 1
        fi
    fi

    # 解压文件
    print_info "解压文件..."
    cd "$DOWNLOAD_DIR"

    if tar -xzf "$filename" -C "../$ECHOKIT_SERVER_DIR" --strip-components=1; then
        print_success "解压完成"
    else
        print_error "解压失败"
        exit 1
    fi

    cd ..
}

# 创建配置文件
create_config() {
    print_info "创建 EchoKit Server 配置..."

    # 创建配置目录
    mkdir -p "$ECHOKIT_SERVER_DIR/config"

    # 创建基本的配置文件
    cat > "$ECHOKIT_SERVER_DIR/config/config.toml" << 'EOF'
[server]
host = "0.0.0.0"
port = 9988
max_connections = 1000

[logging]
level = "info"
file = "/app/logs/echokit-server.log"

[ai]
model = "default"
temperature = 0.7
max_tokens = 1000

[audio]
sample_rate = 16000
channels = 1
format = "pcm16"
EOF

    print_success "配置文件创建完成"
}

# 创建启动脚本
create_startup_script() {
    print_info "创建启动脚本..."

    cat > "$ECHOKIT_SERVER_DIR/start.sh" << 'EOF'
#!/bin/bash

# EchoKit Server 启动脚本

set -e

# 颜色定义
GREEN='\033[0;32m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

print_info() {
    echo -e "${BLUE}ℹ $1${NC}"
}

print_success() {
    echo -e "${GREEN}✓ $1${NC}"
}

# 设置环境变量
export RUST_LOG=${RUST_LOG:-info}
export ECHOKET_CONFIG=${ECHOKET_CONFIG:-/app/config/config.toml}

print_info "启动 EchoKit Server..."
print_info "日志级别: $RUST_LOG"
print_info "配置文件: $ECHOKET_CONFIG"

# 创建日志目录
mkdir -p /app/logs

# 启动服务器
exec ./echokit-server --config "$ECHOKET_CONFIG"
EOF

    chmod +x "$ECHOKIT_SERVER_DIR/start.sh"

    print_success "启动脚本创建完成"
}

# 创建 Dockerfile
create_dockerfile() {
    print_info "创建 EchoKit Server Dockerfile..."

    cat > "$ECHOKIT_SERVER_DIR/Dockerfile" << 'EOF'
FROM debian:bookworm-slim

# 设置工作目录
WORKDIR /app

# 安装必要的依赖
RUN apt-get update && apt-get install -y \
    curl \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# 创建非 root 用户
RUN groupadd -r echokit && useradd -r -g echokit echokit

# 复制 EchoKit Server 二进制文件和配置
COPY echokit-server /app/echokit-server
COPY config/ /app/config/
COPY start.sh /app/start.sh

# 创建日志目录
RUN mkdir -p /app/logs && \
    chown -R echokit:echokit /app

# 切换到非 root 用户
USER echokit

# 暴露端口
EXPOSE 9988

# 健康检查
HEALTHCHECK --interval=30s --timeout=10s --start-period=40s --retries=3 \
    CMD curl -f http://localhost:9988/health || exit 1

# 启动服务器
CMD ["/app/start.sh"]
EOF

    print_success "Dockerfile 创建完成"
}

# 更新 docker-compose.yml
update_docker_compose() {
    print_info "更新 docker-compose.yml..."

    # 备份原始文件
    if [ -f "docker-compose.yml" ]; then
        cp docker-compose.yml docker-compose.yml.backup
        print_success "已备份原始 docker-compose.yml"
    fi

    # 创建新的 docker-compose.yml
    cat > docker-compose.yml << EOF
version: '3.8'

services:
  # PostgreSQL 数据库
  postgres:
    image: postgres:15-alpine
    container_name: echo-postgres
    restart: unless-stopped
    environment:
      POSTGRES_DB: echo_db
      POSTGRES_USER: echo_user
      POSTGRES_PASSWORD: echo_password
      POSTGRES_INITDB_ARGS: "--encoding=UTF-8 --lc-collate=C --lc-ctype=C"
    volumes:
      - postgres_data:/var/lib/postgresql/data
      - ./database/init:/docker-entrypoint-initdb.d
    ports:
      - "5432:5432"
    networks:
      - echo-network
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U echo_user -d echo_db"]
      interval: 10s
      timeout: 5s
      retries: 5

  # Redis 缓存
  redis:
    image: redis:7-alpine
    container_name: echo-redis
    restart: unless-stopped
    command: redis-server --appendonly yes --requirepass redis_password
    volumes:
      - redis_data:/data
    ports:
      - "6379:6379"
    networks:
      - echo-network
    healthcheck:
      test: ["CMD", "redis-cli", "--raw", "incr", "ping"]
      interval: 10s
      timeout: 3s
      retries: 5

  # pgAdmin 数据库管理
  pgadmin:
    image: dpage/pgadmin4:latest
    container_name: echo-pgadmin
    restart: unless-stopped
    environment:
      PGADMIN_DEFAULT_EMAIL: admin@echo-system.com
      PGADMIN_DEFAULT_PASSWORD: admin123
      PGADMIN_CONFIG_SERVER_MODE: 'False'
    volumes:
      - pgadmin_data:/var/lib/pgadmin
    ports:
      - "5050:80"
    networks:
      - echo-network
    depends_on:
      - postgres

  # Redis Commander 管理界面
  redis-commander:
    image: rediscommander/redis-commander:latest
    container_name: echo-redis-commander
    restart: unless-stopped
    environment:
      REDIS_HOSTS: local:redis:6379:0:redis_password
      HTTP_USER: admin
      HTTP_PASSWORD: admin123
    ports:
      - "8081:8081"
    networks:
      - echo-network
    depends_on:
      - redis

  # EchoKit Server AI 推理服务 (从官方 releases 下载)
  echokit-server:
    build:
      context: ./echokit-server-deployment
      dockerfile: Dockerfile
    container_name: echo-echokit-server
    restart: unless-stopped
    environment:
      RUST_LOG: info
      ECHOKET_CONFIG: /app/config/config.toml
    volumes:
      - ./echokit-server-deployment/config:/app/config:ro
      - echokit_logs:/app/logs
    ports:
      - "9988:9988"
    networks:
      - echo-network
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:9988/health"]
      interval: 30s
      timeout: 10s
      retries: 3
      start_period: 40s

  # Bridge 服务
  bridge:
    build:
      context: ./bridge
      dockerfile: Dockerfile
    container_name: echo-bridge
    restart: unless-stopped
    environment:
      RUST_LOG: info
      DATABASE_URL: postgres://echo_user:echo_password@postgres:5432/echo_db
      REDIS_URL: redis://:redis_password@redis:6379
      # EchoKit Server 配置
      ECHOKIT_WEBSOCKET_URL: ws://echokit-server:9988/v1/realtime
      ECHOKET_API_BASE_URL: http://echokit-server:9988
      # MQTT 配置
      MQTT_BROKER_URL: tcp://mqtt:1883
      # 网络配置
      WEBSOCKET_PORT: 8082
      UDP_PORT: 8083
    ports:
      - "8082:8082"  # WebSocket
      - "8083:8083"  # UDP
    networks:
      - echo-network
    depends_on:
      postgres:
        condition: service_healthy
      redis:
        condition: service_healthy
      echokit-server:
        condition: service_healthy
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:8082/health"]
      interval: 30s
      timeout: 10s
      retries: 3
      start_period: 40s

  # API Gateway
  api-gateway:
    build:
      context: ./api-gateway
      dockerfile: Dockerfile
    container_name: echo-api-gateway
    restart: unless-stopped
    environment:
      RUST_LOG: info
      DATABASE_URL: postgres://echo_user:echo_password@postgres:5432/echo_db
      REDIS_URL: redis://:redis_password@redis:6379
      JWT_SECRET: your-super-secret-jwt-key-change-in-production
      JWT_EXPIRATION_HOURS: 24
      # 服务发现
      BRIDGE_WEBSOCKET_URL: ws://bridge:8082
      # CORS 配置
      CORS_ORIGINS: "http://localhost:5174,http://localhost:3000"
    ports:
      - "8080:8080"
    networks:
      - echo-network
    depends_on:
      postgres:
        condition: service_healthy
      redis:
        condition: service_healthy
      bridge:
        condition: service_healthy
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:8080/health"]
      interval: 30s
      timeout: 10s
      retries: 3
      start_period: 40s

  # Web 管理界面
  web-management:
    build:
      context: ./echo-web-management
      dockerfile: Dockerfile
    container_name: echo-web-management
    restart: unless-stopped
    environment:
      REACT_APP_API_BASE_URL: http://localhost:8080
      REACT_APP_WS_URL: ws://localhost:8080
    ports:
      - "5174:5174"
    networks:
      - echo-network
    depends_on:
      api-gateway:
        condition: service_healthy
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:5174/health"]
      interval: 30s
      timeout: 10s
      retries: 3
      start_period: 40s

  # MQTT Broker (可选)
  mqtt:
    image: eclipse-mosquitto:2.0
    container_name: echo-mqtt
    restart: unless-stopped
    volumes:
      - ./mosquitto/mosquitto.conf:/mosquitto/config/mosquitto.conf:ro
      - mqtt_data:/mosquitto/data
      - mqtt_logs:/mosquitto/log
    ports:
      - "1883:1883"
      - "9001:9001"
    networks:
      - echo-network

# 网络配置
networks:
  echo-network:
    driver: bridge
    ipam:
      config:
        - subnet: 172.20.0.0/16

# 数据卷
volumes:
  postgres_data:
    driver: local
  redis_data:
    driver: local
  pgadmin_data:
    driver: local
  echokit_logs:
    driver: local
  mqtt_data:
    driver: local
  mqtt_logs:
    driver: local
EOF

    print_success "docker-compose.yml 更新完成"
}

# 显示信息
show_info() {
    print_success "EchoKit Server 部署准备完成!"
    echo
    print_info "文件位置:"
    echo "  📦 EchoKit Server: $ECHOKIT_SERVER_DIR/"
    echo "  ⚙️  配置文件: $ECHOKIT_SERVER_DIR/config/"
    echo "  🐳 Dockerfile: $ECHOKIT_SERVER_DIR/Dockerfile"
    echo "  📋 docker-compose: docker-compose.yml"
    echo
    print_info "下一步:"
    echo "  1. 检查配置: $ECHOKIT_SERVER_DIR/config/config.toml"
    echo "  2. 启动系统: ./start.sh"
    echo "  3. 验证部署: ./verify-deployment.sh"
    echo
}

# 主函数
main() {
    echo "EchoKit Server 下载和部署脚本"
    echo "================================"
    echo

    # 检查是否在项目根目录
    if [ ! -f "docker-compose.yml" ] && [ ! -f "docker-compose.yml.backup" ]; then
        print_error "请在项目根目录运行此脚本"
        exit 1
    fi

    check_dependencies

    # 解析命令行参数
    case "${1:-latest}" in
        "latest")
            get_latest_version
            ;;
        "version")
            echo "使用指定版本: $ECHOKIT_VERSION"
            ;;
        "help"|"-h"|"--help")
            echo "用法: $0 [option]"
            echo
            echo "选项:"
            echo "  latest   - 下载最新版本（默认）"
            echo "  version  - 使用默认版本 $ECHOKIT_VERSION"
            echo "  help     - 显示帮助信息"
            echo
            exit 0
            ;;
        *)
            ECHOKIT_VERSION="$1"
            print_info "使用指定版本: $ECHOKIT_VERSION"
            ;;
    esac

    create_directories
    download_echokit_server
    create_config
    create_startup_script
    create_dockerfile
    update_docker_compose
    show_info
}

# 运行主函数
main "$@"