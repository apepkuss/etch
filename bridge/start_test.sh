#!/bin/bash
# Bridge WebUI 集成测试快速启动脚本

set -e

echo "🚀 Bridge WebUI 集成测试启动器"
echo "================================"
echo ""

# 检查当前目录
if [ ! -f "Cargo.toml" ]; then
    echo "❌ 错误：请在 bridge 目录下运行此脚本"
    exit 1
fi

# 检查测试文件是否存在
if [ ! -f "resources/index_zh_test.html" ]; then
    echo "❌ 错误：找不到 resources/index_zh_test.html"
    echo "请先运行：cp ../echokit_server/resources/index_zh.html ./resources/index_zh_test.html"
    exit 1
fi

echo "📋 测试准备清单:"
echo "  ✅ 测试文件: resources/index_zh_test.html"
echo "  ✅ Bridge 代码: src/"
echo ""

# 询问是否需要编译
echo "❓ 是否需要重新编译 Bridge？(推荐首次运行选择 y)"
read -p "   编译 (y/N): " compile
echo ""

if [ "$compile" = "y" ] || [ "$compile" = "Y" ]; then
    echo "🔨 编译 Bridge..."
    cargo build --release
    echo "✅ 编译完成"
    echo ""
fi

# 检查端口占用
echo "🔍 检查端口占用..."
if lsof -Pi :10031 -sTCP:LISTEN -t >/dev/null 2>&1 ; then
    echo "⚠️  警告：端口 10031 已被占用"
    read -p "   是否停止占用进程？(y/N): " kill_process
    if [ "$kill_process" = "y" ] || [ "$kill_process" = "Y" ]; then
        lsof -Pi :10031 -sTCP:LISTEN -t | xargs kill -9
        echo "✅ 已停止进程"
    fi
    echo ""
fi

if lsof -Pi :8000 -sTCP:LISTEN -t >/dev/null 2>&1 ; then
    echo "⚠️  警告：端口 8000 已被占用"
    read -p "   是否停止占用进程？(y/N): " kill_http
    if [ "$kill_http" = "y" ] || [ "$kill_http" = "Y" ]; then
        lsof -Pi :8000 -sTCP:LISTEN -t | xargs kill -9
        echo "✅ 已停止进程"
    fi
    echo ""
fi

# 设置 EchoKit URL（可选）
echo "🔧 配置 EchoKit Server URL（对话模式测试需要）"
read -p "   EchoKit URL [ws://localhost:9988/v1/realtime]: " echokit_url
echokit_url=${echokit_url:-ws://localhost:9988/v1/realtime}
export ECHOKIT_WEBSOCKET_URL="$echokit_url"
echo "   已设置: $ECHOKIT_WEBSOCKET_URL"
echo ""

# 启动服务
echo "🎬 启动测试环境..."
echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "📍 测试 URL: http://localhost:8000/resources/index_zh_test.html"
echo "📍 Bridge WebSocket: ws://localhost:10031/ws/"
echo "📍 EchoKit Server: $ECHOKIT_WEBSOCKET_URL"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

# 创建日志目录
mkdir -p logs

# 启动 Bridge 服务（后台）
echo "🚀 启动 Bridge 服务..."
if [ -f "target/release/echo-bridge" ]; then
    nohup ./target/release/echo-bridge > logs/bridge.log 2>&1 &
else
    nohup cargo run --release > logs/bridge.log 2>&1 &
fi
BRIDGE_PID=$!
echo "   PID: $BRIDGE_PID"
echo "   日志: logs/bridge.log"

# 等待 Bridge 启动
echo "⏳ 等待 Bridge 启动..."
sleep 3

# 检查 Bridge 是否成功启动
if ! lsof -Pi :10031 -sTCP:LISTEN -t >/dev/null 2>&1 ; then
    echo "❌ 错误：Bridge 启动失败"
    echo "   查看日志: tail -f logs/bridge.log"
    exit 1
fi
echo "✅ Bridge 启动成功"
echo ""

# 启动 HTTP 服务器（后台）
echo "🌐 启动 HTTP 测试服务器..."
nohup python3 -m http.server 8000 > logs/http.log 2>&1 &
HTTP_PID=$!
echo "   PID: $HTTP_PID"
echo "   日志: logs/http.log"

# 等待 HTTP 服务器启动
sleep 2

if ! lsof -Pi :8000 -sTCP:LISTEN -t >/dev/null 2>&1 ; then
    echo "❌ 错误：HTTP 服务器启动失败"
    kill $BRIDGE_PID 2>/dev/null || true
    exit 1
fi
echo "✅ HTTP 服务器启动成功"
echo ""

# 保存 PID 到文件
echo $BRIDGE_PID > logs/bridge.pid
echo $HTTP_PID > logs/http.pid

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "✨ 测试环境已就绪！"
echo ""
echo "📖 测试步骤："
echo "   1. 打开浏览器访问: http://localhost:8000/resources/index_zh_test.html"
echo "   2. 点击\"连接\"按钮"
echo "   3. 开始测试（参考 TESTING_GUIDE.md）"
echo ""
echo "📊 实时监控："
echo "   Bridge 日志: tail -f logs/bridge.log"
echo "   HTTP 日志:   tail -f logs/http.log"
echo ""
echo "🛑 停止服务："
echo "   ./stop_test.sh"
echo "   或手动: kill $BRIDGE_PID $HTTP_PID"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

# 询问是否自动打开浏览器
read -p "🌐 是否自动打开浏览器？(Y/n): " open_browser
open_browser=${open_browser:-y}

if [ "$open_browser" = "y" ] || [ "$open_browser" = "Y" ]; then
    echo "🚀 正在打开浏览器..."
    if command -v open &> /dev/null; then
        open "http://localhost:8000/resources/index_zh_test.html"
    elif command -v xdg-open &> /dev/null; then
        xdg-open "http://localhost:8000/resources/index_zh_test.html"
    else
        echo "⚠️  无法自动打开浏览器，请手动访问："
        echo "   http://localhost:8000/resources/index_zh_test.html"
    fi
fi

echo ""
echo "✅ 启动完成！按 Ctrl+C 或运行 ./stop_test.sh 停止服务"
echo ""

# 显示实时日志
echo "📜 实时 Bridge 日志 (Ctrl+C 退出):"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
tail -f logs/bridge.log
