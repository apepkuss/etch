-- 将 echokit_server_url 字段改为必填（NOT NULL）
-- 前提：已清理所有 echokit_server_url 为 NULL 的设备记录
--
-- 执行前请确认：
-- 1. 所有现有设备都有 echokit_server_url 值
-- 2. API 已修改为创建设备时必须提供 echokit_server_url
--
-- 影响：
-- - 所有新设备必须有 echokit_server_url
-- - 更新设备时不能将 echokit_server_url 设为 NULL

-- 添加 NOT NULL 约束
ALTER TABLE devices
ALTER COLUMN echokit_server_url SET NOT NULL;

-- 验证约束
DO $$
BEGIN
    -- 检查是否有 NULL 值（理论上应该没有）
    IF EXISTS (SELECT 1 FROM devices WHERE echokit_server_url IS NULL) THEN
        RAISE EXCEPTION '错误：仍存在 echokit_server_url 为 NULL 的设备记录';
    END IF;

    RAISE NOTICE '✅ 成功：echokit_server_url 字段已设置为 NOT NULL';
END $$;

-- 更新字段注释
COMMENT ON COLUMN devices.echokit_server_url IS 'EchoKit Server URL (必填)：设备连接的 EchoKit Server 地址';
