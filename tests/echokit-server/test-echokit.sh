#!/bin/bash
# EchoKit Server 测试脚本

set -e

echo "=== EchoKit Server 测试开始 ==="

# 健康检查
echo "1. 健康检查..."
curl -f http://localhost:10030/health

# API 测试
echo "2. API 测试..."
curl -s http://localhost:10030/v1/models | jq .

# VAD 测试
echo "3. VAD 测试..."
curl -f http://localhost:10030/vad/health || echo "VAD 服务未启用"

echo "=== 测试完成 ==="