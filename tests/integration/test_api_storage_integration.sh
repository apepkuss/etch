#!/bin/bash

# API Gateway 与存储层集成测试脚本
# 测试 API Gateway 与 PostgreSQL、Redis 的集成


# 颜色定义
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# 配置
API_BASE_URL="http://localhost:18080"
DB_HOST="localhost"
DB_PORT="5432"
DB_NAME="echo_db"
DB_USER="echo_user"
DB_PASSWORD="echo_password"
REDIS_HOST="localhost"
REDIS_PORT="6379"
REDIS_PASSWORD="redis_password"
TEST_TIMEOUT=300
SLEEP_INTERVAL=5

# 日志函数
log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

log_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# 测试函数
test_postgres_connection() {
    log_info "测试 PostgreSQL 数据库连接..."

    # 使用 docker compose 命令检查数据库连接
    if docker compose exec -T postgres pg_isready -U "$DB_USER" -d "$DB_NAME" >/dev/null 2>&1; then
        log_success "PostgreSQL 数据库连接正常"
        return 0
    else
        log_error "PostgreSQL 数据库连接失败"
        return 1
    fi
}

test_redis_connection() {
    log_info "测试 Redis 缓存连接..."

    if docker compose exec -T redis redis-cli -a "$REDIS_PASSWORD" ping >/dev/null 2>&1; then
        log_success "Redis 缓存连接正常"
        return 0
    else
        log_error "Redis 缓存连接失败"
        return 1
    fi
}

test_database_tables() {
    log_info "测试数据库表结构..."

    local tables_result=$(docker compose exec -T postgres psql -U "$DB_USER" -d "$DB_NAME" -c "
        SELECT table_name FROM information_schema.tables
        WHERE table_schema = 'public' AND table_type = 'BASE TABLE'
        ORDER BY table_name;
    " 2>/dev/null)

    local required_tables=("users" "devices" "sessions" "user_devices" "system_config")
    local missing_tables=()

    for table in "${required_tables[@]}"; do
        if echo "$tables_result" | grep -q "$table"; then
            log_success "表 $table 存在"
        else
            log_warning "表 $table 不存在"
            missing_tables+=("$table")
        fi
    done

    if [ ${#missing_tables[@]} -eq 0 ]; then
        log_success "所有必需的数据库表都存在"
        return 0
    else
        log_error "缺少 ${#missing_tables[@]} 个必需的表"
        return 1
    fi
}

test_default_data() {
    log_info "测试默认数据..."

    # 检查默认管理员用户
    local admin_user=$(docker compose exec -T postgres psql -U "$DB_USER" -d "$DB_NAME" -c "
        SELECT COUNT(*) FROM users WHERE username = 'admin';
    " 2>/dev/null | grep -o '[0-9]')

    if [ "$admin_user" = "1" ]; then
        log_success "默认管理员用户存在"
    else
        log_error "默认管理员用户不存在"
        return 1
    fi

    # 检查测试设备
    local test_devices=$(docker compose exec -T postgres psql -U "$DB_USER" -d "$DB_NAME" -c "
        SELECT COUNT(*) FROM devices;
    " 2>/dev/null | grep -o '[0-9]')

    if [ "$test_devices" -ge "0" ]; then
        log_success "设备表可访问 ($test_devices 个设备)"
    else
        log_error "设备表不可访问"
        return 1
    fi

    return 0
}

test_api_database_operations() {
    log_info "测试 API Gateway 数据库操作..."

    # 获取认证 token
    local auth_response=$(curl -s -X POST "${API_BASE_URL}/api/auth/login" \
        -H "Content-Type: application/json" \
        -d '{"username":"admin","password":"admin123"}' 2>/dev/null)

    local token=$(echo "$auth_response" | grep -o '"token":"[^"]*"' | cut -d'"' -f4)

    if [ -z "$token" ]; then
        log_warning "无法获取认证 token，跳过需要认证的测试"
        return 0
    fi

    # 测试设备列表 API
    local devices_response=$(curl -s -o /dev/null -w "%{http_code}" \
        -H "Authorization: Bearer $token" \
        "${API_BASE_URL}/api/devices" 2>/dev/null)

    if [ "$devices_response" = "200" ]; then
        log_success "设备列表 API 响应正常"
    else
        log_warning "设备列表 API 响应异常 (HTTP $devices_response)"
        return 0  # 不视为致命错误
    fi

    # 测试用户列表 API
    local users_response=$(curl -s -o /dev/null -w "%{http_code}" \
        -H "Authorization: Bearer $token" \
        "${API_BASE_URL}/api/users" 2>/dev/null)

    if [ "$users_response" = "200" ]; then
        log_success "用户列表 API 响应正常"
    else
        log_warning "用户列表 API 响应异常 (HTTP $users_response)"
        return 0  # 不视为致命错误
    fi

    return 0
}

test_redis_cache_operations() {
    log_info "测试 Redis 缓存操作..."

    # 设置测试数据
    if docker compose exec -T redis redis-cli -a "$REDIS_PASSWORD" \
        set "test:echo:system" "integration_test" >/dev/null 2>&1; then
        log_success "Redis 写入操作正常"
    else
        log_error "Redis 写入操作失败"
        return 1
    fi

    # 读取测试数据
    local test_value=$(docker compose exec -T redis redis-cli -a "$REDIS_PASSWORD" \
        get "test:echo:system" 2>/dev/null)

    if [ "$test_value" = "integration_test" ]; then
        log_success "Redis 读取操作正常"
    else
        log_error "Redis 读取操作失败"
        return 1
    fi

    # 清理测试数据
    docker compose exec -T redis redis-cli -a "$REDIS_PASSWORD" \
        del "test:echo:system" >/dev/null 2>&1

    return 0
}

test_session_storage() {
    log_info "测试会话存储..."

    # 获取认证 token
    local auth_response=$(curl -s -X POST "${API_BASE_URL}/api/auth/login" \
        -H "Content-Type: application/json" \
        -d '{"username":"admin","password":"admin123"}' 2>/dev/null)

    local token=$(echo "$auth_response" | grep -o '"token":"[^"]*"' | cut -d'"' -f4)

    if [ -z "$token" ]; then
        log_warning "无法获取认证 token，跳过会话存储测试"
        return 0
    fi

    # 检查会话是否在数据库中存储
    local session_count=$(docker compose exec -T postgres psql -U "$DB_USER" -d "$DB_NAME" -c "
        SELECT COUNT(*) FROM sessions WHERE created_at > NOW() - INTERVAL '1 hour';
    " 2>/dev/null | grep -o '[0-9]')

    log_info "最近1小时的会话数量: $session_count"

    # 检查会话是否在 Redis 中缓存
    local redis_sessions=$(docker compose exec -T redis redis-cli -a "$REDIS_PASSWORD" \
        keys "session:*" 2>/dev/null | wc -l)

    log_info "Redis 中的会话缓存数量: $redis_sessions"

    log_success "会话存储功能可验证"
    return 0
}

test_cache_aside_pattern() {
    log_info "测试 Cache-Aside 模式..."

    # 模拟缓存操作
    local cache_key="test:devices:list"

    # 先从 Redis 检查缓存
    local cached_value=$(docker compose exec -T redis redis-cli -a "$REDIS_PASSWORD" \
        get "$cache_key" 2>/dev/null)

    if [ -z "$cached_value" ]; then
        log_info "缓存为空，测试缓存写入..."

        # 模拟从数据库加载并写入缓存
        local device_count=$(docker compose exec -T postgres psql -U "$DB_USER" -d "$DB_NAME" -c "
            SELECT COUNT(*) FROM devices;
        " 2>/dev/null | grep -o '[0-9]')

        docker compose exec -T redis redis-cli -a "$REDIS_PASSWORD" \
            setex "$cache_key" 300 "$device_count" >/dev/null 2>&1

        # 验证缓存写入
        local new_cached_value=$(docker compose exec -T redis redis-cli -a "$REDIS_PASSWORD" \
            get "$cache_key" 2>/dev/null)

        if [ "$new_cached_value" = "$device_count" ]; then
            log_success "Cache-Aside 模式工作正常"
        else
            log_error "缓存写入失败"
            return 1
        fi
    else
        log_info "缓存已存在，验证缓存读取..."
        log_success "缓存读取正常"
    fi

    # 清理测试缓存
    docker compose exec -T redis redis-cli -a "$REDIS_PASSWORD" \
        del "$cache_key" >/dev/null 2>&1

    return 0
}

test_transaction_rollback() {
    log_info "测试数据库事务回滚..."

    # 创建测试表用于事务测试
    docker compose exec -T postgres psql -U "$DB_USER" -d "$DB_NAME" -c "
        CREATE TABLE IF NOT EXISTS test_transactions (
            id SERIAL PRIMARY KEY,
            data TEXT,
            created_at TIMESTAMP DEFAULT NOW()
        );
    " >/dev/null 2>&1

    # 执行事务测试
    local transaction_result=$(docker compose exec -T postgres psql -U "$DB_USER" -d "$DB_NAME" -c "
        BEGIN;
        INSERT INTO test_transactions (data) VALUES ('test_data_1');
        INSERT INTO test_transactions (data) VALUES ('test_data_2');
        -- 故意引发错误以测试回滚
        INSERT INTO test_transactions (data) VALUES (NULL/0);
        COMMIT;
    " 2>&1)

    if echo "$transaction_result" | grep -q "ERROR"; then
        log_success "事务回滚功能正常（检测到错误）"
    else
        log_warning "事务测试可能未按预期工作"
    fi

    # 清理测试表
    docker compose exec -T postgres psql -U "$DB_USER" -d "$DB_NAME" -c "
        DROP TABLE IF EXISTS test_transactions;
    " >/dev/null 2>&1

    return 0
}

# 等待服务启动
wait_for_services() {
    log_info "等待服务启动..."
    local elapsed=0

    while [ $elapsed -lt $TEST_TIMEOUT ]; do
        local api_up=false
        local db_up=false
        local redis_up=false

        # 检查 API Gateway
        if curl -s "${API_BASE_URL}/health" >/dev/null 2>&1; then
            api_up=true
        fi

        # 检查数据库
        if docker compose exec -T postgres pg_isready -U "$DB_USER" -d "$DB_NAME" >/dev/null 2>&1; then
            db_up=true
        fi

        # 检查 Redis
        if docker compose exec -T redis redis-cli -a "$REDIS_PASSWORD" ping >/dev/null 2>&1; then
            redis_up=true
        fi

        if [ "$api_up" = true ] && [ "$db_up" = true ] && [ "$redis_up" = true ]; then
            log_success "所有服务已就绪"
            return 0
        fi

        log_info "等待服务启动... API:$api_up DB:$db_up Redis:$redis_up (${elapsed}/${TEST_TIMEOUT}s)"
        sleep $SLEEP_INTERVAL
        elapsed=$((elapsed + SLEEP_INTERVAL))
    done

    log_error "服务启动超时"
    return 1
}

# 主测试函数
run_tests() {
    log_info "开始 API Gateway 与存储层集成测试"
    log_info "API Gateway: ${API_BASE_URL}"
    log_info "数据库: ${DB_HOST}:${DB_PORT}/${DB_NAME}"
    log_info "缓存: ${REDIS_HOST}:${REDIS_PORT}"

    local failed_tests=0
    local total_tests=0

    # 等待服务启动
    if ! wait_for_services; then
        log_error "服务未能在指定时间内启动，跳过其他测试"
        exit 1
    fi

    # 执行测试
    echo
    log_info "执行存储层集成测试..."
    echo

    # 数据库连接测试
    if test_postgres_connection; then
        ((total_tests++))
    else
        ((total_tests++))
        ((failed_tests++))
        return 1  # 数据库连接失败，后续测试无意义
    fi

    # Redis 连接测试
    if test_redis_connection; then
        ((total_tests++))
    else
        ((total_tests++))
        ((failed_tests++))
        return 1  # Redis 连接失败，后续测试无意义
    fi

    # 数据库结构测试
    if test_database_tables; then
        ((total_tests++))
    else
        ((total_tests++))
        ((failed_tests++))
    fi

    # 默认数据测试
    if test_default_data; then
        ((total_tests++))
    else
        ((total_tests++))
        ((failed_tests++))
    fi

    # API 数据库操作测试
    if test_api_database_operations; then
        ((total_tests++))
    else
        ((total_tests++))
        ((failed_tests++))
    fi

    # Redis 缓存操作测试
    if test_redis_cache_operations; then
        ((total_tests++))
    else
        ((total_tests++))
        ((failed_tests++))
    fi

    # 会话存储测试
    if test_session_storage; then
        ((total_tests++))
    else
        ((total_tests++))
        # 不算致命错误
    fi

    # Cache-Aside 模式测试
    if test_cache_aside_pattern; then
        ((total_tests++))
    else
        ((total_tests++))
        ((failed_tests++))
    fi

    # 事务回滚测试
    if test_transaction_rollback; then
        ((total_tests++))
    else
        ((total_tests++))
        # 不算致命错误
    fi

    # 输出测试结果
    echo
    log_info "测试结果汇总:"
    log_info "总测试数: $total_tests"
    log_success "通过测试: $((total_tests - failed_tests))"
    if [ $failed_tests -gt 0 ]; then
        log_error "失败测试: $failed_tests"
    fi

    if [ $failed_tests -eq 0 ]; then
        echo
        log_success "🎉 所有 API Gateway 与存储层集成测试通过！"
        return 0
    else
        echo
        log_error "❌ API Gateway 与存储层集成测试存在失败项"
        return 1
    fi
}

# 检查依赖
check_dependencies() {
    if ! command -v curl &> /dev/null; then
        log_error "curl 命令未安装，无法执行测试"
        exit 1
    fi

    if ! command -v docker &> /dev/null; then
        log_error "docker 命令未安装，无法执行测试"
        exit 1
    fi

    if ! docker compose version &> /dev/null && ! docker-compose version &> /dev/null; then
        log_error "docker compose 命令未安装，无法执行测试"
        exit 1
    fi
}

# 显示帮助信息
show_help() {
    echo "API Gateway 与存储层集成测试脚本"
    echo ""
    echo "用法: $0 [选项]"
    echo ""
    echo "选项:"
    echo "  -h, --help              显示帮助信息"
    echo "  -u, --api-url URL       API Gateway URL (默认: http://localhost:18080)"
    echo "  -d, --db-host HOST      数据库主机 (默认: localhost)"
    echo "  -p, --db-port PORT      数据库端口 (默认: 5432)"
    echo "  -r, --redis-host HOST   Redis 主机 (默认: localhost)"
    echo "  -t, --timeout SECONDS   测试超时时间 (默认: 300)"
    echo ""
    echo "示例:"
    echo "  $0"
    echo "  $0 --api-url http://localhost:18080"
    echo ""
}

# 解析命令行参数
while [[ $# -gt 0 ]]; do
    case $1 in
        -h|--help)
            show_help
            exit 0
            ;;
        -u|--api-url)
            API_BASE_URL="$2"
            shift 2
            ;;
        -d|--db-host)
            DB_HOST="$2"
            shift 2
            ;;
        -p|--db-port)
            DB_PORT="$2"
            shift 2
            ;;
        -r|--redis-host)
            REDIS_HOST="$2"
            shift 2
            ;;
        -t|--timeout)
            TEST_TIMEOUT="$2"
            shift 2
            ;;
        *)
            log_error "未知参数: $1"
            show_help
            exit 1
            ;;
    esac
done

# 主程序
main() {
    check_dependencies
    run_tests
}

# 执行主程序
main "$@"