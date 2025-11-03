#!/bin/bash
# 测试 EchoKit Server 容器中可用的健康检查命令

set -e

echo "=== 测试 EchoKit Server 健康检查命令 ==="

# 确保配置文件存在
if [ ! -f "tests/echokit-server/config.toml" ]; then
    echo "生成配置文件..."
    export ELEVENLABS_API_KEY="sk_81c0ffbb4f09514f0225a2ef816be4954ffd6f70e6d7857d"
    export PARAFORMER_API_KEY="sk-94dc8f978a794975aefba43334731fe4"
    export OPENROUTER_API_KEY="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"
    cd tests/echokit-server
    ./generate-config.sh
    cd ../..
fi

# 启动容器
echo "启动 EchoKit Server..."
docker compose up -d echokit-server

echo "等待容器启动 (10秒)..."
sleep 10

# 测试不同的健康检查命令
echo ""
echo "=== 测试可用命令 ==="

commands=(
    "ls /"
    "ps aux"
    "netstat -an"
    "ss -tuln"
    "cat /etc/os-release"
    "which curl"
    "which wget"
    "which nc"
)

for cmd in "${commands[@]}"; do
    echo -n "测试: $cmd ... "
    if docker compose exec -T echokit-server sh -c "$cmd" >/dev/null 2>&1; then
        echo "✓ 可用"
    else
        echo "✗ 不可用"
    fi
done

# 测试端口检查方法
echo ""
echo "=== 测试端口检查方法 ==="

port_checks=(
    "netstat -an | grep 8080 | grep LISTEN"
    "ss -tuln | grep 8080"
    "nc -z localhost 8080"
    "timeout 1 sh -c 'cat < /dev/null > /dev/tcp/localhost/8080'"
)

for check in "${port_checks[@]}"; do
    echo -n "测试: $check ... "
    if docker compose exec -T echokit-server sh -c "$check" >/dev/null 2>&1; then
        echo "✓ 可用"
    else
        echo "✗ 不可用/未就绪"
    fi
done

# 查看容器内进程
echo ""
echo "=== 容器内进程 ==="
docker compose exec -T echokit-server ps aux | head -10

# 查看端口监听
echo ""
echo "=== 端口监听情况 ==="
docker compose exec -T echokit-server netstat -tuln 2>/dev/null || \
docker compose exec -T echokit-server ss -tuln 2>/dev/null || \
echo "无法获取端口信息"

# 查看日志
echo ""
echo "=== 容器日志 (最近 20 行) ==="
docker compose logs echokit-server --tail 20

echo ""
echo "完成测试！"
