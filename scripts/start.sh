#!/bin/bash

# Echo System Docker Compose 启动脚本
# 用于快速启动和部署完整的 Echo System

set -e

# 颜色定义
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# 打印带颜色的消息
print_message() {
    local color=$1
    local message=$2
    echo -e "${color}${message}${NC}"
}

# 打印标题
print_title() {
    echo
    print_message $BLUE "=================================================="
    print_message $BLUE "  Echo System Docker Compose 部署脚本"
    print_message $BLUE "=================================================="
    echo
}

# 检查 Docker 和 Docker Compose
check_dependencies() {
    print_message $YELLOW "检查依赖..."

    if ! command -v docker &> /dev/null; then
        print_message $RED "错误: Docker 未安装"
        exit 1
    fi

    if ! command -v docker-compose &> /dev/null && ! docker compose version &> /dev/null; then
        print_message $RED "错误: Docker Compose 未安装"
        exit 1
    fi

    print_message $GREEN "✓ 依赖检查通过"
}

# 检查环境变量文件
check_env_file() {
    if [ ! -f .env ]; then
        print_message $YELLOW "创建 .env 文件..."
        cp .env.example .env
        print_message $GREEN "✓ 已创建 .env 文件（使用默认配置）"
        print_message $YELLOW "请根据需要修改 .env 文件中的配置"
    else
        print_message $GREEN "✓ .env 文件已存在"
    fi
}

# 创建必要的目录
create_directories() {
    print_message $YELLOW "创建必要的目录..."

    mkdir -p mosquitto/data mosquitto/log
    mkdir -p database/init
    mkdir -p logs
    mkdir -p data/postgres
    mkdir -p data/redis
    mkdir -p data/pgadmin

    print_message $GREEN "✓ 目录创建完成"
}

# 检查 EchoKit Server 配置
check_echokit_config() {
    print_message $YELLOW "检查 EchoKit Server 配置..."

    # 检查环境变量
    if [ -f .env ]; then
        if grep -q "ECHOKIT_WEBSOCKET_URL" .env 2>/dev/null; then
            print_message $GREEN "✓ EchoKit Server 配置已设置（使用外部服务）"
        else
            print_message $YELLOW "⚠ 未找到 ECHOKIT_WEBSOCKET_URL 配置"
            print_message $YELLOW "  将使用默认配置: wss://indie.echokit.dev/ws/ci-test-visitor"
        fi
    fi
}

# 拉取最新镜像
pull_images() {
    print_message $YELLOW "拉取 Docker 镜像..."

    docker-compose pull

    print_message $GREEN "✓ 镜像拉取完成"
}

# 构建自定义镜像
build_images() {
    print_message $YELLOW "构建自定义镜像..."

    docker-compose build --no-cache

    print_message $GREEN "✓ 镜像构建完成"
}

# 启动服务
start_services() {
    print_message $YELLOW "启动服务..."

    # 先启动数据库和缓存服务
    print_message $BLUE "启动数据库和缓存服务..."
    docker-compose up -d postgres redis

    # 等待数据库就绪
    print_message $BLUE "等待数据库就绪..."
    sleep 10

    # 启动其他服务
    print_message $BLUE "启动应用服务..."
    docker-compose up -d

    print_message $GREEN "✓ 所有服务启动完成"
}

# 等待服务健康检查
wait_for_health() {
    print_message $YELLOW "等待服务健康检查..."

    local services=("postgres" "redis" "bridge" "api-gateway" "web-management")
    local max_attempts=30
    local attempt=1

    while [ $attempt -le $max_attempts ]; do
        print_message $BLUE "健康检查轮次 $attempt/$max_attempts..."

        local all_healthy=true

        for service in "${services[@]}"; do
            local health=$(docker-compose ps -q $service | xargs docker inspect --format='{{.State.Health.Status}}' 2>/dev/null || echo "none")

            if [ "$health" != "healthy" ] && [ "$health" != "none" ]; then
                print_message $YELLOW "  $service: $health"
                all_healthy=false
            elif [ "$health" == "healthy" ]; then
                print_message $GREEN "  ✓ $service"
            elif [ "$health" == "none" ]; then
                print_message $YELLOW "  $service: 无健康检查"
            fi
        done

        if [ "$all_healthy" = true ]; then
            print_message $GREEN "✓ 所有服务健康检查通过"
            break
        fi

        sleep 5
        ((attempt++))
    done

    if [ $attempt -gt $max_attempts ]; then
        print_message $RED "警告: 部分服务可能未完全启动，请检查日志"
    fi
}

# 显示服务状态
show_status() {
    print_message $YELLOW "服务状态:"
    docker-compose ps

    echo
    print_message $BLUE "访问地址:"
    echo "  📱 Web管理界面:    http://localhost:10034 (admin/admin123)"
    echo "  🔌 API Gateway:    http://localhost:10033"
    echo "  🌐 Bridge服务:     ws://localhost:10031 (WebSocket)"
    echo "                     udp://localhost:10032 (UDP音频)"
    echo "  🧠 EchoKit Server: https://indie.echokit.dev (外部服务)"
    echo "  🗄️  数据库管理:     http://localhost:10037 (admin@echo-system.com/admin123)"
    echo "  💾 Redis管理:      http://localhost:10038 (admin/admin123)"
    echo "  📡 MQTT Broker:    localhost:10039"
    echo
    print_message $BLUE "健康检查:"
    echo "  🟢 API健康检查:    http://localhost:10033/health"
    echo "  🟢 Web健康检查:    http://localhost:10034/health"
    echo "  🟢 Bridge健康:     http://localhost:10031/health"
    echo
}

# 显示常用命令
show_commands() {
    print_message $BLUE "常用命令:"
    echo "  📊 查看日志:        docker-compose logs -f [service-name]"
    echo "  🔄 重启服务:        docker-compose restart [service-name]"
    echo "  🛑 停止所有服务:    docker-compose down"
    echo "  🧹 清理数据:        docker-compose down -v"
    echo "  🔧 进入容器:        docker-compose exec [service-name] sh"
    echo
}

# 主函数
main() {
    print_title

    # 检查是否在项目根目录
    if [ ! -f "docker-compose.yml" ]; then
        print_message $RED "错误: 请在项目根目录运行此脚本"
        exit 1
    fi

    # 解析命令行参数
    case "${1:-start}" in
        "start")
            check_dependencies
            check_env_file
            create_directories
            check_echokit_config
            pull_images
            build_images
            start_services
            wait_for_health
            show_status
            show_commands
            print_message $GREEN "🎉 Echo System 部署完成!"
            ;;
        "stop")
            print_message $YELLOW "停止所有服务..."
            docker-compose down
            print_message $GREEN "✓ 服务已停止"
            ;;
        "restart")
            print_message $YELLOW "重启所有服务..."
            docker-compose restart
            print_message $GREEN "✓ 服务已重启"
            ;;
        "status")
            show_status
            ;;
        "logs")
            docker-compose logs -f
            ;;
        "clean")
            print_message $RED "警告: 这将删除所有数据!"
            read -p "确认继续? (y/N): " -n 1 -r
            echo
            if [[ $REPLY =~ ^[Yy]$ ]]; then
                docker-compose down -v --rmi all
                docker system prune -f
                print_message $GREEN "✓ 清理完成"
            fi
            ;;
        "help"|"-h"|"--help")
            echo "用法: $0 [command]"
            echo
            echo "命令:"
            echo "  start    - 启动所有服务（默认）"
            echo "  stop     - 停止所有服务"
            echo "  restart  - 重启所有服务"
            echo "  status   - 显示服务状态"
            echo "  logs     - 查看日志"
            echo "  clean    - 清理所有数据（危险）"
            echo "  help     - 显示帮助信息"
            echo
            ;;
        *)
            print_message $RED "未知命令: $1"
            print_message $YELLOW "使用 '$0 help' 查看可用命令"
            exit 1
            ;;
    esac
}

# 运行主函数
main "$@"