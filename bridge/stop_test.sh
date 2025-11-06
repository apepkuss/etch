#!/bin/bash
# 停止 Bridge 测试服务脚本

echo "🛑 停止 Bridge 测试服务..."
echo ""

# 从 PID 文件读取并停止
if [ -f "logs/bridge.pid" ]; then
    BRIDGE_PID=$(cat logs/bridge.pid)
    if ps -p $BRIDGE_PID > /dev/null 2>&1; then
        echo "🔴 停止 Bridge 服务 (PID: $BRIDGE_PID)..."
        kill $BRIDGE_PID 2>/dev/null || true
        sleep 1
        # 强制停止（如果还在运行）
        if ps -p $BRIDGE_PID > /dev/null 2>&1; then
            kill -9 $BRIDGE_PID 2>/dev/null || true
        fi
        echo "✅ Bridge 服务已停止"
    else
        echo "⚠️  Bridge 服务未运行"
    fi
    rm -f logs/bridge.pid
else
    # 通过端口查找并停止
    if lsof -Pi :10031 -sTCP:LISTEN -t >/dev/null 2>&1 ; then
        echo "🔴 停止占用端口 10031 的进程..."
        lsof -Pi :10031 -sTCP:LISTEN -t | xargs kill -9 2>/dev/null || true
        echo "✅ 已停止"
    else
        echo "⚠️  端口 10031 未被占用"
    fi
fi

echo ""
echo "✅ 测试服务已停止"
echo "📋 日志文件保留在 logs/ 目录中"
