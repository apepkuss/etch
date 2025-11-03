#!/bin/bash

# 集成测试验证脚本
# 快速验证所有集成测试脚本的完整性

# 颜色定义
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

log_info() { echo -e "${BLUE}[INFO]${NC} $1"; }
log_success() { echo -e "${GREEN}[SUCCESS]${NC} $1"; }
log_error() { echo -e "${RED}[ERROR]${NC} $1"; }

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

echo "======================================"
echo "集成测试验证"
echo "======================================"
echo

# 测试脚本列表
declare -a test_scripts=(
    "test_api_storage_integration.sh"
    "test_web_api_integration.sh"
    "test_bridge_echokit_integration.sh"
    "run_all_tests.sh"
)

# 验证测试脚本
log_info "验证测试脚本存在性和可执行性..."
echo

failed_checks=0

for script in "${test_scripts[@]}"; do
    script_path="$SCRIPT_DIR/$script"

    if [ ! -f "$script_path" ]; then
        log_error "✗ $script - 文件不存在"
        ((failed_checks++))
    elif [ ! -x "$script_path" ]; then
        log_error "✗ $script - 不可执行"
        ((failed_checks++))
    else
        log_success "✓ $script - 验证通过"

        # 验证脚本是否有帮助信息
        if "$script_path" --help >/dev/null 2>&1; then
            log_info "  └─ 帮助信息可用"
        fi
    fi
done

echo

# 验证 GitHub Actions 工作流
log_info "验证 GitHub Actions 工作流..."
echo

workflow_file="$SCRIPT_DIR/../../.github/workflows/test.yml"

if [ ! -f "$workflow_file" ]; then
    log_error "✗ GitHub Actions 工作流文件不存在"
    ((failed_checks++))
else
    log_success "✓ GitHub Actions 工作流文件存在"

    # 检查工作流中是否包含所有测试
    for script in "${test_scripts[@]}"; do
        if grep -q "$script" "$workflow_file" 2>/dev/null; then
            log_success "  └─ $script 已配置在 CI"
        else
            if [ "$script" != "run_all_tests.sh" ]; then
                log_error "  └─ $script 未配置在 CI"
                ((failed_checks++))
            fi
        fi
    done
fi

echo

# 验证 README 文档
log_info "验证 README 文档..."
echo

readme_file="$SCRIPT_DIR/README.md"

if [ ! -f "$readme_file" ]; then
    log_error "✗ README.md 文件不存在"
    ((failed_checks++))
else
    log_success "✓ README.md 文件存在"

    # 检查 README 中是否包含所有测试的文档
    for script in "${test_scripts[@]}"; do
        if grep -q "$script" "$readme_file" 2>/dev/null; then
            log_success "  └─ $script 已记录在文档"
        else
            log_error "  └─ $script 未记录在文档"
            ((failed_checks++))
        fi
    done
fi

echo

# 最终结果
echo "======================================"
if [ $failed_checks -eq 0 ]; then
    log_success "🎉 所有验证检查通过！"
    exit 0
else
    log_error "❌ $failed_checks 个验证检查失败"
    exit 1
fi
