#!/bin/bash

# 总集成测试运行脚本
# 依次运行所有集成测试

set -e

# 颜色定义
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

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

# 配置
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
TEST_TIMEOUT=600
EXIT_CODE=0

# 默认端口配置（支持命令行参数覆盖）
API_BASE_URL="http://localhost:18080"
WEB_BASE_URL="http://localhost:18084"
ECHOKIT_URL="https://eu.echokit.dev"

# 测试结果统计
TOTAL_SUITES=0
PASSED_SUITES=0
FAILED_SUITES=0

# 检查依赖
check_dependencies() {
    log_info "检查测试依赖..."

    local missing_deps=()

    if ! command -v docker &> /dev/null; then
        missing_deps+=("docker")
    fi

    if ! docker compose version &> /dev/null && ! docker-compose version &> /dev/null; then
        missing_deps+=("docker compose")
    fi

    if ! command -v curl &> /dev/null; then
        missing_deps+=("curl")
    fi

    if [ ${#missing_deps[@]} -gt 0 ]; then
        log_error "缺少以下依赖: ${missing_deps[*]}"
        exit 1
    fi

    log_success "所有依赖检查通过"
}

# 准备测试环境
prepare_test_environment() {
    log_info "准备测试环境..."

    # 复制环境变量文件
    if [ ! -f "$PROJECT_ROOT/.env" ]; then
        if [ -f "$PROJECT_ROOT/.env.example" ]; then
            cp "$PROJECT_ROOT/.env.example" "$PROJECT_ROOT/.env"
            log_info "已创建 .env 文件"
        else
            log_error "未找到 .env.example 文件"
            exit 1
        fi
    fi

    # 确保测试脚本可执行
    chmod +x "$SCRIPT_DIR"/*.sh

    log_success "测试环境准备完成"
}

# 部署服务
deploy_services() {
    log_info "部署 Echo System 服务..."

    cd "$PROJECT_ROOT"

    # 停止现有服务
    log_info "停止现有服务..."
    docker compose down -v 2>/dev/null || true

    # 启动服务
    log_info "启动所有服务..."
    cd "$PROJECT_ROOT"
    if docker compose up -d; then
        log_success "服务部署成功"
    else
        log_error "服务部署失败"
        exit 1
    fi

    # 等待服务就绪
    log_info "等待服务完全启动..."
    local elapsed=0
    local max_wait=120

    while [ $elapsed -lt $max_wait ]; do
        if curl -s "$API_BASE_URL/health" >/dev/null 2>&1 && \
           curl -s "$WEB_BASE_URL/health" >/dev/null 2>&1; then
            log_success "所有服务已就绪"
            return 0
        fi

        echo -n "."
        sleep 5
        elapsed=$((elapsed + 5))
    done

    echo
    log_error "服务启动超时"
    return 1
}

# 运行单个测试套件
run_test_suite() {
    local test_name="$1"
    local test_script="$2"
    local test_description="$3"

    log_info "运行测试套件: $test_description"
    log_info "脚本: $test_script"

    TOTAL_SUITES=$((TOTAL_SUITES + 1))

    echo "========================================"
    echo "测试套件 $TOTAL_SUITES: $test_name"
    echo "========================================"

    if "$test_script"; then
        log_success "✅ $test_description - 通过"
        PASSED_SUITES=$((PASSED_SUITES + 1))
        echo
        return 0
    else
        log_error "❌ $test_description - 失败"
        FAILED_SUITES=$((FAILED_SUITES + 1))
        EXIT_CODE=1
        echo
        return 1
    fi
}

# 生成测试报告
generate_test_report() {
    log_info "生成测试报告..."

    local report_file="$PROJECT_ROOT/integration-test-report-$(date +%Y%m%d-%H%M%S).txt"

    cat > "$report_file" << EOF
Echo System 集成测试报告
========================

测试时间: $(date)
测试环境: GitHub Actions

测试结果汇总:
- 总测试套件数: $TOTAL_SUITES
- 通过测试套件: $PASSED_SUITES
- 失败测试套件: $FAILED_SUITES
- 成功率: $(( PASSED_SUITES * 100 / TOTAL_SUITES ))%

测试套件详情:
1. 用户界面与 API Gateway 集成测试 - ${PASSED_SUITES:-0}/1
2. API Gateway 与存储层集成测试 - $((PASSED_SUITES - 1))/1

系统信息:
- Docker 版本: $(docker --version)
- Docker Compose 版本: $(docker compose --version 2>/dev/null || docker-compose --version)
- 操作系统: $(uname -a)

EOF

    log_success "测试报告已生成: $report_file"
    echo
    cat "$report_file"
}

# 清理测试环境
cleanup_test_environment() {
    log_info "清理测试环境..."

    cd "$PROJECT_ROOT"

    # 可选：保留服务用于调试
    if [ "${KEEP_SERVICES:-false}" = "true" ]; then
        log_info "保留服务运行用于调试"
        log_info "访问地址:"
        log_info "  Web管理界面: $WEB_BASE_URL"
        log_info "  API Gateway:  $API_BASE_URL"
        log_info "  EchoKit Server: $ECHOKIT_URL"
        log_info "要停止服务，请运行: docker compose down"
    else
        log_info "停止所有服务..."
        docker compose down -v
        log_success "测试环境清理完成"
    fi
}

# 显示帮助信息
show_help() {
    echo "Echo System 集成测试运行器"
    echo ""
    echo "用法: $0 [选项]"
    echo ""
    echo "选项:"
    echo "  -h, --help              显示帮助信息"
    echo "  -u, --api-url URL       API Gateway URL (默认: $API_BASE_URL)"
    echo "  -w, --web-url URL       Web 界面 URL (默认: $WEB_BASE_URL)"
    echo "  -e, --echokit-url URL   EchoKit Server URL (默认: $ECHOKIT_URL)"
    echo "  -k, --keep-services     测试完成后保留服务运行"
    echo "  -t, --timeout SECONDS   测试超时时间 (默认: 600)"
    echo "  --skip-deployment       跳过服务部署步骤"
    echo "  --skip-cleanup         跳过环境清理步骤"
    echo ""
    echo "示例:"
    echo "  $0"
    echo "  $0 --keep-services"
    echo "  $0 --skip-deployment"
    echo "  $0 --api-url http://localhost:8080 --web-url http://localhost:3000"
    echo ""
}

# 主函数
main() {
    local skip_deployment=false
    local skip_cleanup=false

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
            -e|--echokit-url)
                ECHOKIT_URL="$2"
                shift 2
                ;;
            -k|--keep-services)
                export KEEP_SERVICES=true
                shift
                ;;
            -t|--timeout)
                TEST_TIMEOUT="$2"
                shift 2
                ;;
            --skip-deployment)
                skip_deployment=true
                shift
                ;;
            --skip-cleanup)
                skip_cleanup=true
                shift
                ;;
            *)
                log_error "未知参数: $1"
                show_help
                exit 1
                ;;
        esac
    done

    log_info "Echo System 集成测试开始"
    log_info "测试超时: ${TEST_TIMEOUT}s"

    # 设置超时处理
    trap 'log_error "测试超时"; exit 1' TERM
    (sleep $TEST_TIMEOUT && kill $$ 2>/dev/null) &
    local timeout_pid=$!

    # 执行测试流程
    {
        check_dependencies
        prepare_test_environment

        if [ "$skip_deployment" = false ]; then
            if ! deploy_services; then
                log_error "服务部署失败，退出测试"
                exit 1
            fi
        else
            log_info "跳过服务部署步骤"
        fi

        echo
        log_info "开始执行集成测试套件..."
        echo

        # 运行测试套件
        run_test_suite "Web-API集成" \
            "$SCRIPT_DIR/test_web_api_integration.sh --api-url \"$API_BASE_URL\" --web-url \"$WEB_BASE_URL\" --timeout 300" \
            "用户界面与 API Gateway 集成测试"

        run_test_suite "API-存储集成" \
            "$SCRIPT_DIR/test_api_storage_integration.sh --api-url \"$API_BASE_URL\" --timeout 300" \
            "API Gateway 与存储层集成测试"

        # 生成测试报告
        generate_test_report

        # 停止超时计时器
        kill $timeout_pid 2>/dev/null || true

        # 最终结果
        echo
        echo "========================================"
        echo "集成测试完成"
        echo "========================================"

        if [ $EXIT_CODE -eq 0 ]; then
            log_success "🎉 所有集成测试通过！"
        else
            log_error "❌ 存在失败的集成测试"
        fi

        echo
        log_info "测试结果: $PASSED_SUITES/$TOTAL_SUITES 通过"
        echo

    } || {
        # 错误处理
        kill $timeout_pid 2>/dev/null || true
        log_error "测试过程中发生错误"
        EXIT_CODE=1
    }

    # 清理环境
    if [ "$skip_cleanup" = false ]; then
        cleanup_test_environment
    else
        log_info "跳过环境清理步骤"
    fi

    exit $EXIT_CODE
}

# 执行主函数
main "$@"