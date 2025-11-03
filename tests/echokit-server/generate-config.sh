#!/bin/bash
# 从环境变量生成 config.toml

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
TEMPLATE_FILE="${SCRIPT_DIR}/config.toml.template"
OUTPUT_FILE="${SCRIPT_DIR}/config.toml"

# 颜色定义
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

log_info() { echo -e "${GREEN}[INFO]${NC} $1"; }
log_error() { echo -e "${RED}[ERROR]${NC} $1"; }
log_warning() { echo -e "${YELLOW}[WARNING]${NC} $1"; }

# 检查模板文件是否存在
if [ ! -f "$TEMPLATE_FILE" ]; then
    log_error "模板文件不存在: $TEMPLATE_FILE"
    exit 1
fi

log_info "从模板生成配置文件..."

# 检查必需的环境变量
REQUIRED_VARS=(
    "ELEVENLABS_API_KEY"
    "PARAFORMER_API_KEY"
    "OPENROUTER_API_KEY"
)

MISSING_VARS=()
for var in "${REQUIRED_VARS[@]}"; do
    if [ -z "${!var}" ]; then
        MISSING_VARS+=("$var")
    fi
done

if [ ${#MISSING_VARS[@]} -gt 0 ]; then
    log_error "缺少以下必需的环境变量:"
    for var in "${MISSING_VARS[@]}"; do
        echo "  - $var"
    done
    log_warning "请设置这些环境变量后重试"
    log_warning "GitHub Actions 中设置方法: Settings → Secrets and variables → Actions → New repository secret"
    exit 1
fi

# 使用 envsubst 或 sed 替换环境变量
if command -v envsubst &> /dev/null; then
    # 使用 envsubst（推荐）
    envsubst < "$TEMPLATE_FILE" > "$OUTPUT_FILE"
else
    # 使用 sed 作为备选方案
    cp "$TEMPLATE_FILE" "$OUTPUT_FILE"
    sed -i.bak "s/\${ELEVENLABS_API_KEY}/${ELEVENLABS_API_KEY}/g" "$OUTPUT_FILE"
    sed -i.bak "s/\${PARAFORMER_API_KEY}/${PARAFORMER_API_KEY}/g" "$OUTPUT_FILE"
    sed -i.bak "s/\${OPENROUTER_API_KEY}/${OPENROUTER_API_KEY}/g" "$OUTPUT_FILE"
    rm -f "${OUTPUT_FILE}.bak"
fi

log_info "配置文件已生成: $OUTPUT_FILE"

# 验证生成的文件
if grep -q '\${' "$OUTPUT_FILE"; then
    log_warning "警告: 配置文件中仍有未替换的变量"
    grep '\${' "$OUTPUT_FILE"
    exit 1
fi

log_info "✓ 所有环境变量已成功替换"
