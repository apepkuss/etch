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
exec ./echokit_server "$ECHOKET_CONFIG"
