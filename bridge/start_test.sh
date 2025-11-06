#!/bin/bash
# Bridge WebUI 集成测试快速启动脚本

set -e

echo "🚀 Bridge WebUI 集成测试启动器"
echo "================================"
echo ""

# 服务依赖清单
echo "📦 测试服务依赖清单："
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "必需服务："
echo "  🟢 Bridge WebSocket    端口: 10031 (HTTP + WebSocket)"
echo "     └─ 提供 WebSocket 通信和静态文件服务"
echo ""
echo "可选服务（用于设备通信测试）："
echo "  🟡 Bridge UDP Server   端口: 8083 (默认，可配置)"
echo "     └─ 接收设备音频数据"
echo "  🟡 MQTT Broker         端口: 1883 (mqtt:1883)"
echo "     └─ 设备控制和状态同步"
echo ""
echo "外部依赖（对话模式测试）："
echo "  🔵 EchoKit Server      URL: wss://indie.echokit.dev/ws/{visitor-id}"
echo "     └─ 提供语音识别和对话服务"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""
echo "💡 提示："
echo "   - WebSocket 测试只需要 Bridge WebSocket (10031) 服务"
echo "   - UDP/MQTT 错误不影响 WebSocket 测试"
echo "   - EchoKit Server 仅对话模式需要，连接测试可忽略"
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

# 检查 WebSocket 端口 (10031)
if lsof -Pi :10031 -sTCP:LISTEN -t >/dev/null 2>&1 ; then
    echo "⚠️  警告：WebSocket 端口 10031 已被占用"
    read -p "   是否停止占用进程？(y/N): " kill_process
    if [ "$kill_process" = "y" ] || [ "$kill_process" = "Y" ]; then
        lsof -Pi :10031 -sTCP:LISTEN -t | xargs kill -9
        echo "✅ 已停止端口 10031 的进程"
    fi
    echo ""
fi

# 检查默认 UDP 端口 (8083)
UDP_PORT=8083
if lsof -Pi :8083 -sTCP:LISTEN -t >/dev/null 2>&1 || lsof -Pi :8083 -sUDP:LISTEN -t >/dev/null 2>&1 ; then
    echo "⚠️  警告：默认 UDP 端口 8083 已被占用"
    echo "   （UDP 端口用于设备通信，WebSocket 测试可使用其他端口）"

    while true; do
        read -p "   请输入替代 UDP 端口 [18083]: " custom_udp_port
        UDP_PORT=${custom_udp_port:-18083}

        # 验证端口号是否有效
        if [[ "$UDP_PORT" =~ ^[0-9]+$ ]] && [ "$UDP_PORT" -ge 1024 ] && [ "$UDP_PORT" -le 65535 ]; then
            # 检查新端口是否可用
            if lsof -Pi :$UDP_PORT -sTCP:LISTEN -t >/dev/null 2>&1 || lsof -Pi :$UDP_PORT -sUDP:LISTEN -t >/dev/null 2>&1 ; then
                echo "   ❌ 端口 $UDP_PORT 也被占用，请选择其他端口"
            else
                echo "   ✅ 将使用 UDP 端口: $UDP_PORT"
                break
            fi
        else
            echo "   ❌ 无效端口号，请输入 1024-65535 之间的数字"
        fi
    done
    echo ""
fi

# 设置 EchoKit URL（可选）
echo "🔧 配置 EchoKit Server URL（对话模式测试需要）"
echo "   格式: wss://indie.echokit.dev/ws/{your-visitor-id}"
read -p "   EchoKit URL [wss://indie.echokit.dev/ws/ci-test-visitor]: " echokit_url
echokit_url=${echokit_url:-wss://indie.echokit.dev/ws/ci-test-visitor}
export ECHOKIT_WEBSOCKET_URL="$echokit_url"
echo "   已设置: $ECHOKIT_WEBSOCKET_URL"
echo ""

# 配置 UDP 端口
export BRIDGE_UDP_BIND_ADDRESS="0.0.0.0:${UDP_PORT}"
echo "ℹ️  UDP 服务绑定地址: $BRIDGE_UDP_BIND_ADDRESS"
echo "   （WebSocket 测试不依赖 UDP 服务）"
echo ""

# 启动服务
echo "🎬 启动测试环境..."
echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "📍 测试 URL: http://localhost:10031/index_zh_test.html"
echo "📍 Bridge WebSocket: ws://localhost:10031/ws/"
echo "📍 EchoKit Server: $ECHOKIT_WEBSOCKET_URL"
echo "📍 静态文件服务: Bridge 内置"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

# 创建日志目录
mkdir -p logs

# 启动 Bridge 服务（后台）
echo "🚀 启动 Bridge 服务..."
if [ -f "../target/release/echo-bridge" ]; then
    nohup ../target/release/echo-bridge > logs/bridge.log 2>&1 &
else
    nohup cargo run --release --bin echo-bridge > logs/bridge.log 2>&1 &
fi
BRIDGE_PID=$!
echo "   PID: $BRIDGE_PID"
echo "   日志: logs/bridge.log"

# 等待 Bridge 启动
echo "⏳ 等待 Bridge 启动（WebSocket 服务）..."
for i in {1..10}; do
    if lsof -Pi :10031 -sTCP:LISTEN -t >/dev/null 2>&1 ; then
        echo "✅ Bridge WebSocket 服务启动成功"
        break
    fi
    sleep 1
    echo "   等待中... ($i/10)"
done

# 检查 Bridge 是否成功启动
if ! lsof -Pi :10031 -sTCP:LISTEN -t >/dev/null 2>&1 ; then
    echo "❌ 错误：Bridge WebSocket 服务启动失败"
    echo "   查看日志: tail -f logs/bridge.log"
    echo ""
    echo "💡 提示："
    echo "   - UDP/MQTT 错误可以忽略（仅影响设备通信）"
    echo "   - 如果看到 'WebSocket server listening' 则服务正常"
    exit 1
fi
echo ""

# 根据选择的模式启动 HTTP 服务器（仅模式 2 需要）
# 保存 Bridge PID 到文件
echo $BRIDGE_PID > logs/bridge.pid

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "✨ 测试环境已就绪！"
echo ""
echo "📖 测试步骤："
echo "   1. 打开浏览器访问: http://localhost:10031/index_zh_test.html"
echo "   2. 点击\"连接\"按钮"
echo "   3. 开始测试（参考 TESTING_GUIDE.md）"
echo ""
echo "📊 实时监控："
echo "   Bridge 日志: tail -f logs/bridge.log"
echo ""
echo "🛑 停止服务："
echo "   ./stop_test.sh"
echo "   或手动: kill $BRIDGE_PID"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

# 询问是否自动打开浏览器
read -p "🌐 是否自动打开浏览器？(Y/n): " open_browser
open_browser=${open_browser:-y}

if [ "$open_browser" = "y" ] || [ "$open_browser" = "Y" ]; then
    echo "🚀 正在打开浏览器..."

    TEST_URL="http://localhost:10031/index_zh_test.html"

    if command -v open &> /dev/null; then
        open "$TEST_URL"
    elif command -v xdg-open &> /dev/null; then
        xdg-open "$TEST_URL"
    else
        echo "⚠️  无法自动打开浏览器，请手动访问："
        echo "   $TEST_URL"
    fi
fi

echo ""
echo "✅ 启动完成！按 Ctrl+C 或运行 ./stop_test.sh 停止服务"
echo ""

# 显示实时日志
echo "📜 实时 Bridge 日志 (Ctrl+C 退出):"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
tail -f logs/bridge.log
