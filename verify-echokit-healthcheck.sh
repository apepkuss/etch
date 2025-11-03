#!/bin/bash
# 验证 EchoKit Server 健康检查修复

set -e

echo "=== 验证 EchoKit Server 健康检查 ==="

# 生成配置文件
if [ ! -f "tests/echokit-server/config.toml" ]; then
    echo "生成配置文件..."
    export ELEVENLABS_API_KEY="sk_81c0ffbb4f09514f0225a2ef816be4954ffd6f70e6d7857d"
    export PARAFORMER_API_KEY="sk-94dc8f978a794975aefba43334731fe4"
    export OPENROUTER_API_KEY="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"
    cd tests/echokit-server
    ./generate-config.sh
    cd ../..
fi

# 停止并清理
echo "清理旧容器..."
docker compose down -v echokit-server 2>/dev/null || true

# 启动
echo "启动 EchoKit Server..."
docker compose up -d echokit-server

# 监控健康状态
echo ""
echo "=== 监控健康状态（90秒） ==="
for i in {1..18}; do
    sleep 5

    status=$(docker inspect echo-echokit-server --format='{{.State.Health.Status}}' 2>/dev/null || echo "no_healthcheck")
    echo "[$((i*5))秒] 健康状态: $status"

    if [ "$status" = "healthy" ]; then
        echo "✅ 健康检查通过！"
        break
    elif [ "$status" = "unhealthy" ]; then
        echo "❌ 健康检查失败"
        break
    fi
done

# 显示结果
echo ""
echo "=== 最终状态 ==="
docker compose ps echokit-server

echo ""
echo "=== 容器日志 ==="
docker compose logs echokit-server --tail 30

echo ""
echo "=== 健康检查详情 ==="
docker inspect echo-echokit-server --format='{{json .State.Health}}' 2>/dev/null | jq '.' || echo "无健康检查配置"

echo ""
echo "=== 测试健康检查命令 ==="
echo "测试: cat /proc/*/cmdline | tr '\\0' '\\n' | grep -q echokit"
docker compose exec -T echokit-server sh -c "cat /proc/*/cmdline 2>/dev/null | tr '\\0' '\\n' | grep -q echokit" \
    && echo "✓ EchoKit 进程存在" \
    || echo "✗ EchoKit 进程不存在"

echo ""
echo "完成！"
