#!/bin/bash

# Echo System 部署验证脚本
# 用于验证 Docker Compose 部署的完整性和功能

set -e

# 颜色定义
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# 计数器
TOTAL_CHECKS=0
PASSED_CHECKS=0

# 打印函数
print_header() {
    echo -e "${BLUE}==================================================${NC}"
    echo -e "${BLUE}  Echo System 部署验证${NC}"
    echo -e "${BLUE}==================================================${NC}"
    echo
}

print_success() {
    echo -e "${GREEN}✓ $1${NC}"
    ((PASSED_CHECKS++))
}

print_error() {
    echo -e "${RED}✗ $1${NC}"
}

print_warning() {
    echo -e "${YELLOW}⚠ $1${NC}"
}

print_info() {
    echo -e "${BLUE}ℹ $1${NC}"
}

# 检查函数
check_service() {
    local service=$1
    local url=$2
    local description=$3
    local expected_status=${4:-200}

    ((TOTAL_CHECKS++))
    print_info "检查 $description..."

    if response=$(curl -s -o /dev/null -w "%{http_code}" "$url" 2>/dev/null); then
        if [ "$response" = "$expected_status" ]; then
            print_success "$description ($response)"
            return 0
        else
            print_error "$description (HTTP $response，期望 $expected_status)"
            return 1
        fi
    else
        print_error "$description (连接失败)"
        return 1
    fi
}

check_docker_service() {
    local service=$1
    local description=$2

    ((TOTAL_CHECKS++))
    print_info "检查 Docker 服务: $description..."

    if docker-compose ps "$service" | grep -q "Up"; then
        print_success "$description 运行中"
        return 0
    else
        print_error "$description 未运行"
        docker-compose ps "$service"
        return 1
    fi
}

check_database_connection() {
    ((TOTAL_CHECKS++))
    print_info "检查数据库连接..."

    if docker-compose exec -T postgres pg_isready -U echo_user -d echo_db >/dev/null 2>&1; then
        print_success "PostgreSQL 连接正常"
        return 0
    else
        print_error "PostgreSQL 连接失败"
        return 1
    fi
}

check_redis_connection() {
    ((TOTAL_CHECKS++))
    print_info "检查 Redis 连接..."

    if docker-compose exec -T redis redis-cli -a redis_password ping | grep -q "PONG"; then
        print_success "Redis 连接正常"
        return 0
    else
        print_error "Redis 连接失败"
        return 1
    fi
}

check_web_interface() {
    local url=$1
    local description=$2
    local search_pattern=$3

    ((TOTAL_CHECKS++))
    print_info "检查 Web 界面: $description..."

    if content=$(curl -s "$url" 2>/dev/null); then
        if echo "$content" | grep -q "$search_pattern"; then
            print_success "$description 内容正常"
            return 0
        else
            print_error "$description 内容异常"
            return 1
        fi
    else
        print_error "$description 无法访问"
        return 1
    fi
}

# 主要验证函数
verify_docker_services() {
    print_info "验证 Docker 服务状态..."

    check_docker_service "postgres" "PostgreSQL 数据库"
    check_docker_service "redis" "Redis 缓存"
    check_docker_service "echokit-server" "EchoKit Server"
    check_docker_service "bridge" "Bridge 服务"
    check_docker_service "api-gateway" "API Gateway"
    check_docker_service "web-management" "Web 管理界面"
    check_docker_service "pgadmin" "pgAdmin 管理界面"
    check_docker_service "redis-commander" "Redis Commander"
    check_docker_service "mqtt" "MQTT Broker"
}

verify_database_data() {
    print_info "验证数据库数据..."

    ((TOTAL_CHECKS++))
    print_info "检查默认用户数据..."

    if docker-compose exec -T postgres psql -U echo_user -d echo_db -c "SELECT COUNT(*) FROM users WHERE username = 'admin';" | grep -q "1"; then
        print_success "默认管理员用户存在"
        return 0
    else
        print_error "默认管理员用户不存在"
        return 1
    fi
}

verify_api_endpoints() {
    print_info "验证 API 端点..."

    check_service "http://localhost:9031/health" "API Gateway 健康检查"
    check_service "http://localhost:9031/api/devices" "设备列表 API"
    check_service "http://localhost:9031/api/sessions" "会话记录 API"
    check_service "http://localhost:9031/api/dashboard" "仪表板数据 API"
}

verify_web_interfaces() {
    print_info "验证 Web 界面..."

    check_service "http://localhost:9030" "Web 管理界面"
    check_service "http://localhost:9030/health" "Web 管理界面健康检查"
    check_web_interface "http://localhost:9030" "Web 管理界面内容" "Echo智能音箱管理系统"

    check_service "http://localhost:9035" "pgAdmin 管理界面"
    check_service "http://localhost:9036" "Redis Commander 管理界面"
}

verify_ai_services() {
    print_info "验证 AI 服务..."

    check_service "http://localhost:9034" "EchoKit Server"
}

verify_connectivity() {
    print_info "验证服务间连通性..."

    # API Gateway 到数据库
    ((TOTAL_CHECKS++))
    print_info "检查 API Gateway 到数据库的连接..."

    if docker-compose exec -T api-gateway curl -s http://postgres:5432 >/dev/null 2>&1 || \
       docker-compose exec -T api-gateway timeout 5 sh -c "echo > /dev/tcp/postgres/5432" >/dev/null 2>&1; then
        print_success "API Gateway 可访问数据库"
    else
        print_warning "API Gateway 到数据库连接检查跳过（网络限制）"
    fi

    # Bridge 到 EchoKit Server
    ((TOTAL_CHECKS++))
    print_info "检查 Bridge 到 EchoKit Server 的连接..."

    if docker-compose exec -T bridge timeout 5 sh -c "echo > /dev/tcp/echokit-server/9988" >/dev/null 2>&1; then
        print_success "Bridge 可访问 EchoKit Server"
    else
        print_warning "Bridge 到 EchoKit Server 连接检查跳过（协议限制）"
    fi
}

show_system_info() {
    print_info "系统信息:"
    echo "  🐳 Docker 版本: $(docker --version)"
    echo "  🐙 Docker Compose 版本: $(docker-compose --version)"
    echo "  🖥️  系统信息: $(uname -a)"
    echo "  💾 内存使用: $(free -h 2>/dev/null || echo 'N/A (macOS)')"
    echo "  💿 磁盘使用: $(df -h . | tail -1)"
    echo

    print_info "容器资源使用:"
    docker-compose ps --format "table {{.Name}}\t{{.Status}}\t{{.Ports}}" | head -10
    echo
}

show_access_urls() {
    print_info "访问地址:"
    echo "  📱 Web管理界面:    http://localhost:9030"
    echo "     用户名: admin, 密码: admin123"
    echo "  🔌 API Gateway:    http://localhost:9031"
    echo "  🧠 EchoKit Server: http://localhost:9034"
    echo "  🗄️  数据库管理:     http://localhost:9035"
    echo "     邮箱: admin@echo-system.com, 密码: admin123"
    echo "  💾 Redis管理:      http://localhost:9036"
    echo "     用户名: admin, 密码: admin123"
    echo "  📡 MQTT Broker:    localhost:9037"
    echo
}

show_next_steps() {
    print_info "后续步骤:"
    echo "  1. 访问 Web 管理界面进行系统配置"
    echo "  2. 添加和配置智能音箱设备"
    echo "  3. 测试语音交互功能"
    echo "  4. 查看会话记录和系统统计"
    echo "  5. 根据需要调整系统配置"
    echo
    print_info "管理命令:"
    echo "  📊 查看日志:        docker-compose logs -f [service-name]"
    echo "  🔄 重启服务:        docker-compose restart [service-name]"
    echo "  🛑 停止系统:        docker-compose down"
    echo "  🧹 完全清理:        docker-compose down -v"
    echo
}

# 主函数
main() {
    print_header

    # 检查 Docker 和 Docker Compose
    if ! command -v docker &> /dev/null; then
        print_error "Docker 未安装"
        exit 1
    fi

    if ! command -v docker-compose &> /dev/null && ! docker compose version &> /dev/null; then
        print_error "Docker Compose 未安装"
        exit 1
    fi

    # 检查是否在项目根目录
    if [ ! -f "docker-compose.yml" ]; then
        print_error "请在项目根目录运行此脚本"
        exit 1
    fi

    # 检查服务是否运行
    if ! docker-compose ps | grep -q "Up"; then
        print_error "服务未运行，请先执行: ./start.sh"
        exit 1
    fi

    echo "开始验证部署..."
    echo

    # 执行验证
    verify_docker_services
    echo

    verify_database_data
    echo

    verify_api_endpoints
    echo

    verify_web_interfaces
    echo

    verify_ai_services
    echo

    verify_connectivity
    echo

    # 显示系统信息
    show_system_info
    show_access_urls

    # 显示结果
    echo -e "${BLUE}==================================================${NC}"
    echo -e "${BLUE}  验证结果${NC}"
    echo -e "${BLUE}==================================================${NC}"
    echo

    if [ $PASSED_CHECKS -eq $TOTAL_CHECKS ]; then
        print_success "所有检查通过! ($PASSED_CHECKS/$TOTAL_CHECKS)"
        echo -e "${GREEN}🎉 Echo System 部署验证成功!${NC}"
    else
        print_error "部分检查失败 ($PASSED_CHECKS/$TOTAL_CHECKS)"
        echo -e "${YELLOW}请检查服务状态和日志${NC}"
        echo -e "${YELLOW}运行 'docker-compose logs' 查看详细信息${NC}"
    fi

    echo

    show_next_steps

    # 返回适当的退出码
    if [ $PASSED_CHECKS -eq $TOTAL_CHECKS ]; then
        exit 0
    else
        exit 1
    fi
}

# 运行主函数
main "$@"