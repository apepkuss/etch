#!/bin/bash
# 测试 EchoKit Server 健康状态

set -e

echo "=== 测试 EchoKit Server 健康检查 ==="

# 生成配置文件（如果还没有）
if [ ! -f "tests/echokit-server/config.toml" ]; then
    echo "生成 config.toml..."
    export ELEVENLABS_API_KEY="sk_81c0ffbb4f09514f0225a2ef816be4954ffd6f70e6d7857d"
    export PARAFORMER_API_KEY="sk-94dc8f978a794975aefba43334731fe4"
    export OPENROUTER_API_KEY="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"

    cd tests/echokit-server
    ./generate-config.sh
    cd ../..
fi

# 停止现有容器
echo "停止现有容器..."
docker compose down -v echokit-server 2>/dev/null || true

# 启动 EchoKit Server
echo "启动 EchoKit Server..."
docker compose up -d echokit-server

# 等待容器启动
echo "等待容器启动 (30秒)..."
sleep 30

# 检查容器状态
echo ""
echo "=== 容器状态 ==="
docker compose ps echokit-server

# 检查容器日志
echo ""
echo "=== 容器日志 (最近 50 行) ==="
docker compose logs echokit-server --tail 50

# 测试不同的端点
echo ""
echo "=== 测试端点 ==="

endpoints=(
    "http://localhost:9988/"
    "http://localhost:9988/health"
    "http://localhost:9988/api/health"
    "http://localhost:9988/v1/health"
)

for endpoint in "${endpoints[@]}"; do
    echo -n "测试 $endpoint ... "
    response=$(curl -s -o /dev/null -w "%{http_code}" --max-time 5 "$endpoint" 2>/dev/null || echo "000")

    if [ "$response" = "000" ]; then
        echo "✗ 连接失败"
    elif [ "$response" = "200" ]; then
        echo "✓ 200 OK"
        # 显示响应内容
        echo "  响应内容:"
        curl -s --max-time 5 "$endpoint" 2>/dev/null | head -5 | sed 's/^/    /'
    else
        echo "⚠ HTTP $response"
    fi
done

# 检查健康检查状态
echo ""
echo "=== Docker 健康检查状态 ==="
docker inspect echo-echokit-server --format='{{.State.Health.Status}}' 2>/dev/null || echo "无健康检查状态"

# 最终诊断
echo ""
echo "=== 诊断建议 ==="
health_status=$(docker inspect echo-echokit-server --format='{{.State.Health.Status}}' 2>/dev/null || echo "unknown")

if [ "$health_status" = "healthy" ]; then
    echo "✅ EchoKit Server 健康状态正常"
elif [ "$health_status" = "unhealthy" ]; then
    echo "❌ EchoKit Server 不健康"
    echo "   请检查:"
    echo "   1. 配置文件是否正确 (tests/echokit-server/config.toml)"
    echo "   2. API Keys 是否有效"
    echo "   3. 容器日志中的错误信息"
elif [ "$health_status" = "starting" ]; then
    echo "⏳ EchoKit Server 仍在启动中，请等待更长时间"
else
    echo "⚠ 无法确定健康状态，可能未配置健康检查"
fi

echo ""
echo "完成测试！"
