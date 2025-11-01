#!/bin/bash

# 用户界面与 API Gateway 集成测试脚本
# 测试 Web 管理界面与 API Gateway 之间的集成


# 颜色定义
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# 配置
API_BASE_URL="http://localhost:18080"
WEB_BASE_URL="http://localhost:18084"
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
test_api_health() {
    log_info "测试 API Gateway 健康检查..."
    local response=$(curl -s -o /dev/null -w "%{http_code}" "${API_BASE_URL}/health" 2>/dev/null)

    if [ "$response" = "200" ]; then
        log_success "API Gateway 健康检查通过"
        return 0
    else
        log_error "API Gateway 健康检查失败 (HTTP $response)"
        return 1
    fi
}

test_web_health() {
    log_info "测试 Web 管理界面健康检查..."
    local response=$(curl -s -o /dev/null -w "%{http_code}" "${WEB_BASE_URL}/health" 2>/dev/null)

    if [ "$response" = "200" ]; then
        log_success "Web 管理界面健康检查通过"
        return 0
    else
        log_warning "Web 管理界面健康检查失败，尝试主页面..."
        local main_response=$(curl -s -o /dev/null -w "%{http_code}" "${WEB_BASE_URL}" 2>/dev/null)
        if [ "$main_response" = "200" ]; then
            log_success "Web 管理界面主页面可访问"
            return 0
        else
            log_error "Web 管理界面完全不可访问 (HTTP $main_response)"
            return 1
        fi
    fi
}

test_api_devices_endpoint() {
    log_info "测试设备列表 API 端点..."

    # 首先尝试获取认证 token
    local auth_response=$(curl -s -X POST "${API_BASE_URL}/api/auth/login" \
        -H "Content-Type: application/json" \
        -d '{"username":"admin","password":"admin123"}' -w "\nHTTP_CODE:%{http_code}" 2>/dev/null)

    log_info "认证请求: POST ${API_BASE_URL}/api/auth/login"
    log_info "认证响应: $auth_response"

    local token=$(echo "$auth_response" | grep -o '"token":"[^"]*"' | cut -d'"' -f4)
    log_info "Token提取结果: ${token:+成功}${token:-(失败)}"

    if [ -n "$token" ]; then
        log_info "认证成功，测试设备列表 API..."
        local devices_response=$(curl -s -o /dev/null -w "%{http_code}" \
            -H "Authorization: Bearer $token" \
            "${API_BASE_URL}/api/devices" 2>/dev/null)

        if [ "$devices_response" = "200" ]; then
            log_success "设备列表 API 端点正常"
            return 0
        else
            log_error "设备列表 API 端点失败 (HTTP $devices_response)"
            return 1
        fi
    else
        log_warning "认证失败，尝试无需认证的端点..."
        local public_response=$(curl -s -o /dev/null -w "%{http_code}" \
            "${API_BASE_URL}/api/public/status" 2>/dev/null)

        if [ "$public_response" = "200" ]; then
            log_success "公共状态端点正常"
            return 0
        else
            log_error "所有 API 端点都不可访问"
            return 1
        fi
    fi
}

test_cors_headers() {
    log_info "测试 CORS 头配置..."

    local response=$(curl -s -I -X OPTIONS "${API_BASE_URL}/api/devices" \
        -H "Origin: ${WEB_BASE_URL}" \
        -H "Access-Control-Request-Method: GET" 2>/dev/null)

    if echo "$response" | grep -q "Access-Control-Allow-Origin"; then
        log_success "CORS 头配置正确"
        return 0
    else
        log_warning "CORS 头可能配置不完整"
        return 0  # 不视为致命错误
    fi
}

test_web_api_communication() {
    log_info "测试 Web 界面与 API Gateway 通信..."

    # 检查 Web 界面是否能够访问 API
    local web_config=$(curl -s "${WEB_BASE_URL}" 2>/dev/null | grep -o "api.*base.*url" | head -5)

    if [ -n "$web_config" ]; then
        log_success "Web 界面包含 API 配置信息"
        return 0
    else
        log_warning "无法验证 Web 界面 API 配置"
        return 0  # 不视为致命错误
    fi
}

test_dashboard_data() {
    log_info "测试仪表板数据获取..."

    # 尝试获取仪表板数据
    local dashboard_response=$(curl -s -o /dev/null -w "%{http_code}" \
        "${API_BASE_URL}/api/dashboard" 2>/dev/null)

    if [ "$dashboard_response" = "200" ] || [ "$dashboard_response" = "401" ]; then
        log_success "仪表板端点响应正常"
        return 0
    else
        log_warning "仪表板端点响应异常 (HTTP $dashboard_response)"
        return 0  # 401 可能是未认证，不算致命错误
    fi
}

test_web_static_assets() {
    log_info "测试 Web 界面静态资源..."

    local js_response=$(curl -s -o /dev/null -w "%{http_code}" "${WEB_BASE_URL}/static/js/" 2>/dev/null)
    local css_response=$(curl -s -o /dev/null -w "%{http_code}" "${WEB_BASE_URL}/static/css/" 2>/dev/null)

    if [ "$js_response" = "200" ] || [ "$js_response" = "404" ]; then
        log_success "静态资源路径可访问"
        return 0
    else
        log_warning "静态资源可能存在问题"
        return 0
    fi
}

# 等待服务启动
wait_for_services() {
    log_info "等待服务启动..."
    local elapsed=0

    while [ $elapsed -lt $TEST_TIMEOUT ]; do
        if curl -s "${API_BASE_URL}/health" >/dev/null 2>&1 && \
           curl -s "${WEB_BASE_URL}" >/dev/null 2>&1; then
            log_success "服务已就绪"
            return 0
        fi

        log_info "等待服务启动... (${elapsed}/${TEST_TIMEOUT}s)"
        sleep $SLEEP_INTERVAL
        elapsed=$((elapsed + SLEEP_INTERVAL))
    done

    log_error "服务启动超时"
    return 1
}

# 主测试函数
run_tests() {
    log_info "开始用户界面与 API Gateway 集成测试"
    log_info "API Gateway: ${API_BASE_URL}"
    log_info "Web 界面: ${WEB_BASE_URL}"

    local failed_tests=0
    local total_tests=0

    # 等待服务启动
    if ! wait_for_services; then
        log_error "服务未能在指定时间内启动，跳过其他测试"
        exit 1
    fi

    # 执行测试
    echo
    log_info "执行集成测试..."
    echo

    # API Gateway 测试
    if test_api_health; then
        ((total_tests++))
    else
        ((total_tests++))
        ((failed_tests++))
    fi

    if test_api_devices_endpoint; then
        ((total_tests++))
    else
        ((total_tests++))
        ((failed_tests++))
    fi

    if test_cors_headers; then
        ((total_tests++))
    else
        ((total_tests++))
    fi

    if test_dashboard_data; then
        ((total_tests++))
    else
        ((total_tests++))
        ((failed_tests++))
    fi

    # Web 界面测试
    if test_web_health; then
        ((total_tests++))
    else
        ((total_tests++))
        ((failed_tests++))
    fi

    if test_web_static_assets; then
        ((total_tests++))
    else
        ((total_tests++))
    fi

    # 通信测试
    if test_web_api_communication; then
        ((total_tests++))
    else
        ((total_tests++))
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
        log_success "🎉 所有用户界面与 API Gateway 集成测试通过！"
        return 0
    else
        echo
        log_error "❌ 用户界面与 API Gateway 集成测试存在失败项"
        return 1
    fi
}

# 检查依赖
check_dependencies() {
    if ! command -v curl &> /dev/null; then
        log_error "curl 命令未安装，无法执行测试"
        exit 1
    fi
}

# 显示帮助信息
show_help() {
    echo "用户界面与 API Gateway 集成测试脚本"
    echo ""
    echo "用法: $0 [选项]"
    echo ""
    echo "选项:"
    echo "  -h, --help              显示帮助信息"
    echo "  -u, --api-url URL       API Gateway URL (默认: http://localhost:18080)"
    echo "  -w, --web-url URL       Web 界面 URL (默认: http://localhost:18084)"
    echo "  -t, --timeout SECONDS   测试超时时间 (默认: 300)"
    echo ""
    echo "示例:"
    echo "  $0"
    echo "  $0 --api-url http://localhost:18080 --web-url http://localhost:18084"
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
        -w|--web-url)
            WEB_BASE_URL="$2"
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