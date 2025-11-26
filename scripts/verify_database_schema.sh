#!/bin/bash

# ============================================================================
# 数据库 Schema 验证脚本
# ============================================================================
# 用途: 验证数据库结构是否与最新的 01-init-database.sql 对齐
# 使用: ./scripts/verify_database_schema.sh
# ============================================================================

set -e

echo "============================================================"
echo "Echo System 数据库 Schema 验证"
echo "============================================================"
echo ""

# 检查 Docker 容器是否运行
if ! docker ps | grep -q "echo-postgres"; then
    echo "❌ PostgreSQL 容器未运行"
    echo "请先启动容器: docker compose up -d postgres"
    exit 1
fi

echo "✅ PostgreSQL 容器正在运行"
echo ""

# ============================================================================
# 1. 检查所有表是否存在
# ============================================================================

echo "============================================================"
echo "1. 检查表是否存在"
echo "============================================================"

TABLES=(
    "users"
    "devices"
    "sessions"
    "device_registration_tokens"
    "echokit_servers"
    "user_devices"
    "system_config"
    "schema_versions"
)

for table in "${TABLES[@]}"; do
    if docker exec echo-postgres psql -U echo_user -d echo_db -tAc "SELECT EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name='$table');" | grep -q "t"; then
        echo "✅ 表 '$table' 存在"
    else
        echo "❌ 表 '$table' 不存在"
    fi
done

echo ""

# ============================================================================
# 2. 验证关键字段
# ============================================================================

echo "============================================================"
echo "2. 验证关键字段"
echo "============================================================"

# 检查 devices.echokit_server_url
echo -n "检查 devices.echokit_server_url... "
if docker exec echo-postgres psql -U echo_user -d echo_db -tAc "
SELECT column_name
FROM information_schema.columns
WHERE table_name='devices'
AND column_name='echokit_server_url';" | grep -q "echokit_server_url"; then
    echo "✅ 存在"

    # 检查是否为 NOT NULL
    is_nullable=$(docker exec echo-postgres psql -U echo_user -d echo_db -tAc "
    SELECT is_nullable
    FROM information_schema.columns
    WHERE table_name='devices'
    AND column_name='echokit_server_url';")

    if [ "$is_nullable" = "NO" ]; then
        echo "  ✅ NOT NULL 约束正确"
    else
        echo "  ⚠️  应该是 NOT NULL，但当前是 $is_nullable"
    fi
else
    echo "❌ 不存在"
fi

# 检查 devices.pairing_code
echo -n "检查 devices.pairing_code... "
if docker exec echo-postgres psql -U echo_user -d echo_db -tAc "
SELECT column_name
FROM information_schema.columns
WHERE table_name='devices'
AND column_name='pairing_code';" | grep -q "pairing_code"; then
    echo "✅ 存在"
else
    echo "❌ 不存在"
fi

# 检查 devices.owner
echo -n "检查 devices.owner... "
if docker exec echo-postgres psql -U echo_user -d echo_db -tAc "
SELECT column_name
FROM information_schema.columns
WHERE table_name='devices'
AND column_name='owner';" | grep -q "owner"; then
    echo "✅ 存在"
else
    echo "❌ 不存在"
fi

# 检查 sessions.duration
echo -n "检查 sessions.duration... "
if docker exec echo-postgres psql -U echo_user -d echo_db -tAc "
SELECT column_name
FROM information_schema.columns
WHERE table_name='sessions'
AND column_name='duration';" | grep -q "duration"; then
    echo "✅ 存在"
else
    echo "❌ 不存在"
fi

echo ""

# ============================================================================
# 3. 检查数据类型
# ============================================================================

echo "============================================================"
echo "3. 检查数据类型"
echo "============================================================"

# devices.id 应该是 VARCHAR(255)
device_id_type=$(docker exec echo-postgres psql -U echo_user -d echo_db -tAc "
SELECT data_type || COALESCE('(' || character_maximum_length || ')', '')
FROM information_schema.columns
WHERE table_name='devices' AND column_name='id';")

echo -n "devices.id 类型: $device_id_type ... "
if [ "$device_id_type" = "character varying(255)" ]; then
    echo "✅ 正确"
else
    echo "⚠️  应该是 VARCHAR(255)"
fi

# sessions.id 应该是 VARCHAR(255)
session_id_type=$(docker exec echo-postgres psql -U echo_user -d echo_db -tAc "
SELECT data_type || COALESCE('(' || character_maximum_length || ')', '')
FROM information_schema.columns
WHERE table_name='sessions' AND column_name='id';")

echo -n "sessions.id 类型: $session_id_type ... "
if [ "$session_id_type" = "character varying(255)" ]; then
    echo "✅ 正确"
else
    echo "⚠️  应该是 VARCHAR(255)"
fi

echo ""

# ============================================================================
# 4. 检查索引
# ============================================================================

echo "============================================================"
echo "4. 检查关键索引"
echo "============================================================"

INDEXES=(
    "idx_devices_echokit_server_url"
    "idx_devices_pairing_code"
    "idx_registration_tokens_pairing_code"
    "idx_echokit_servers_user_id"
    "idx_sessions_device_status"
)

for index in "${INDEXES[@]}"; do
    if docker exec echo-postgres psql -U echo_user -d echo_db -tAc "
    SELECT indexname
    FROM pg_indexes
    WHERE indexname='$index';" | grep -q "$index"; then
        echo "✅ 索引 '$index' 存在"
    else
        echo "❌ 索引 '$index' 不存在"
    fi
done

echo ""

# ============================================================================
# 5. 检查视图
# ============================================================================

echo "============================================================"
echo "5. 检查视图"
echo "============================================================"

VIEWS=(
    "device_status_overview"
    "daily_usage_stats"
)

for view in "${VIEWS[@]}"; do
    if docker exec echo-postgres psql -U echo_user -d echo_db -tAc "
    SELECT viewname
    FROM pg_views
    WHERE viewname='$view';" | grep -q "$view"; then
        echo "✅ 视图 '$view' 存在"
    else
        echo "❌ 视图 '$view' 不存在"
    fi
done

echo ""

# ============================================================================
# 6. 检查 Schema 版本
# ============================================================================

echo "============================================================"
echo "6. 检查 Schema 版本"
echo "============================================================"

if docker exec echo-postgres psql -U echo_user -d echo_db -c "
SELECT version, description, applied_at
FROM schema_versions
ORDER BY applied_at DESC;" 2>/dev/null; then
    echo ""
    echo "✅ Schema 版本记录正常"
else
    echo "⚠️  无法查询 schema_versions 表"
fi

echo ""

# ============================================================================
# 7. 检查默认数据
# ============================================================================

echo "============================================================"
echo "7. 检查默认数据"
echo "============================================================"

# 检查默认管理员用户
admin_count=$(docker exec echo-postgres psql -U echo_user -d echo_db -tAc "
SELECT COUNT(*) FROM users WHERE username='admin';")

if [ "$admin_count" -gt 0 ]; then
    echo "✅ 默认管理员账户存在"
    docker exec echo-postgres psql -U echo_user -d echo_db -c "
    SELECT username, email, role, is_active
    FROM users
    WHERE username='admin';"
else
    echo "⚠️  默认管理员账户不存在"
fi

echo ""

# 检查默认系统配置
config_count=$(docker exec echo-postgres psql -U echo_user -d echo_db -tAc "
SELECT COUNT(*) FROM system_config;")

echo "系统配置项数量: $config_count"

if [ "$config_count" -gt 0 ]; then
    echo "✅ 默认系统配置存在"
    docker exec echo-postgres psql -U echo_user -d echo_db -c "
    SELECT key, value
    FROM system_config
    ORDER BY key;"
else
    echo "⚠️  默认系统配置不存在"
fi

echo ""

# ============================================================================
# 8. 总结
# ============================================================================

echo "============================================================"
echo "验证完成"
echo "============================================================"
echo ""
echo "如果所有检查都显示 ✅，说明数据库 schema 与最新的"
echo "01-init-database.sql 完全对齐。"
echo ""
echo "如果有 ❌ 或 ⚠️，建议重建数据库:"
echo "  docker compose down -v"
echo "  docker compose up -d postgres"
echo ""
echo "============================================================"
