-- ============================================================================
-- Echo System 数据库迁移脚本
-- 版本: 002
-- 描述: 更新 sessions 表结构以匹配代码模型
-- 创建日期: 2025-01-19
-- 作者: Echo System Team
-- ============================================================================

-- 开始事务
BEGIN;

-- ============================================================================
-- 1. 字段重命名：匹配代码中的命名
-- ============================================================================

-- 重命名 transcript -> transcription
ALTER TABLE sessions
    RENAME COLUMN transcript TO transcription;

-- 重命名 started_at -> start_time
ALTER TABLE sessions
    RENAME COLUMN started_at TO start_time;

-- 重命名 completed_at -> end_time
ALTER TABLE sessions
    RENAME COLUMN completed_at TO end_time;

-- ============================================================================
-- 2. 添加新字段
-- ============================================================================

-- 添加 duration 字段（会话时长，单位：秒）
ALTER TABLE sessions
    ADD COLUMN IF NOT EXISTS duration INTEGER;

-- ============================================================================
-- 3. 更新约束：匹配代码中的枚举值
-- ============================================================================

-- 删除旧的状态约束
ALTER TABLE sessions
    DROP CONSTRAINT IF EXISTS sessions_status_check;

-- 添加新的状态约束（匹配 SessionStatus 枚举）
-- 代码中的枚举值: Active, Completed, Failed, Timeout
-- 数据库存储使用小写: active, completed, failed, timeout
ALTER TABLE sessions
    ADD CONSTRAINT sessions_status_check
    CHECK (status IN ('active', 'completed', 'failed', 'timeout'));

-- 更新 session_type 约束（保持现有逻辑）
ALTER TABLE sessions
    DROP CONSTRAINT IF EXISTS sessions_session_type_check;

ALTER TABLE sessions
    ADD CONSTRAINT sessions_session_type_check
    CHECK (session_type IN ('voice', 'text', 'command'));

-- ============================================================================
-- 4. 更新索引：优化查询性能
-- ============================================================================

-- 删除旧索引（基于旧字段名）
DROP INDEX IF EXISTS idx_sessions_started_at;

-- 创建新索引（基于新字段名）
CREATE INDEX IF NOT EXISTS idx_sessions_start_time ON sessions(start_time DESC);

-- 添加复合索引：device_id + status（常用组合查询）
CREATE INDEX IF NOT EXISTS idx_sessions_device_status
    ON sessions(device_id, status);

-- 添加复合索引：start_time + status（时间范围 + 状态过滤）
CREATE INDEX IF NOT EXISTS idx_sessions_start_time_status
    ON sessions(start_time DESC, status);

-- ============================================================================
-- 5. 数据迁移：更新现有数据
-- ============================================================================

-- 将现有 status 值转换为小写（如果有大写数据）
UPDATE sessions
SET status = LOWER(status)
WHERE status != LOWER(status);

-- 计算已有会话的 duration（如果 end_time 已存在但 duration 为空）
UPDATE sessions
SET duration = EXTRACT(EPOCH FROM (end_time - start_time))::INTEGER
WHERE end_time IS NOT NULL
  AND duration IS NULL;

-- ============================================================================
-- 6. 更新视图：修正字段引用
-- ============================================================================

-- 删除旧视图
DROP VIEW IF EXISTS device_status_overview;

-- 重新创建视图（使用新字段名）
CREATE OR REPLACE VIEW device_status_overview AS
SELECT
    d.id,
    d.name,
    d.device_type,
    d.status,
    d.location,
    d.battery_level,
    d.volume_level,
    d.last_seen,
    COUNT(s.id) as total_sessions,
    COUNT(CASE WHEN s.start_time > NOW() - INTERVAL '24 hours' THEN 1 END) as sessions_24h,
    COUNT(CASE WHEN s.start_time > NOW() - INTERVAL '7 days' THEN 1 END) as sessions_7d
FROM devices d
LEFT JOIN sessions s ON d.id = s.device_id
GROUP BY d.id, d.name, d.device_type, d.status, d.location, d.battery_level, d.volume_level, d.last_seen;

-- 删除旧的每日统计视图
DROP VIEW IF EXISTS daily_usage_stats;

-- 重新创建每日统计视图（使用新字段名）
CREATE OR REPLACE VIEW daily_usage_stats AS
SELECT
    DATE(s.start_time) as date,
    COUNT(s.id) as total_sessions,
    COUNT(CASE WHEN s.status = 'completed' THEN 1 END) as completed_sessions,
    COUNT(CASE WHEN s.status = 'failed' THEN 1 END) as failed_sessions,
    COUNT(CASE WHEN s.status = 'timeout' THEN 1 END) as timeout_sessions,
    AVG(s.processing_time_ms) as avg_processing_time,
    AVG(s.duration) as avg_duration_seconds,
    COUNT(DISTINCT s.device_id) as active_devices
FROM sessions s
WHERE s.start_time >= CURRENT_DATE - INTERVAL '30 days'
GROUP BY DATE(s.start_time)
ORDER BY date DESC;

-- ============================================================================
-- 7. 添加注释：文档化表结构
-- ============================================================================

COMMENT ON TABLE sessions IS '语音交互会话记录表';
COMMENT ON COLUMN sessions.id IS '会话唯一标识（UUID）';
COMMENT ON COLUMN sessions.device_id IS '设备ID（外键关联 devices 表）';
COMMENT ON COLUMN sessions.user_id IS '用户ID（外键关联 users 表，可为空）';
COMMENT ON COLUMN sessions.session_type IS '会话类型：voice（语音）、text（文本）、command（命令）';
COMMENT ON COLUMN sessions.status IS '会话状态：active（进行中）、completed（已完成）、failed（失败）、timeout（超时）';
COMMENT ON COLUMN sessions.transcription IS '语音转录文本（ASR 结果）';
COMMENT ON COLUMN sessions.response IS 'AI 响应内容（LLM 输出）';
COMMENT ON COLUMN sessions.confidence_score IS '识别置信度（0.0-1.0）';
COMMENT ON COLUMN sessions.processing_time_ms IS '处理耗时（毫秒）';
COMMENT ON COLUMN sessions.duration IS '会话时长（秒），从 start_time 到 end_time';
COMMENT ON COLUMN sessions.audio_file_path IS '音频文件存储路径';
COMMENT ON COLUMN sessions.metadata IS '扩展元数据（JSON 格式）';
COMMENT ON COLUMN sessions.start_time IS '会话开始时间（UTC）';
COMMENT ON COLUMN sessions.end_time IS '会话结束时间（UTC，可为空）';

-- ============================================================================
-- 8. 验证迁移结果
-- ============================================================================

-- 输出表结构信息
DO $$
DECLARE
    column_count INTEGER;
    index_count INTEGER;
    constraint_count INTEGER;
BEGIN
    -- 统计列数
    SELECT COUNT(*) INTO column_count
    FROM information_schema.columns
    WHERE table_name = 'sessions';

    -- 统计索引数
    SELECT COUNT(*) INTO index_count
    FROM pg_indexes
    WHERE tablename = 'sessions';

    -- 统计约束数
    SELECT COUNT(*) INTO constraint_count
    FROM information_schema.table_constraints
    WHERE table_name = 'sessions';

    RAISE NOTICE '=== Sessions 表迁移完成 ===';
    RAISE NOTICE '列数: %', column_count;
    RAISE NOTICE '索引数: %', index_count;
    RAISE NOTICE '约束数: %', constraint_count;
END $$;

-- 提交事务
COMMIT;

-- ============================================================================
-- 迁移验证查询（手动执行以验证结果）
-- ============================================================================

-- 查看所有列
-- SELECT column_name, data_type, is_nullable
-- FROM information_schema.columns
-- WHERE table_name = 'sessions'
-- ORDER BY ordinal_position;

-- 查看所有索引
-- SELECT indexname, indexdef
-- FROM pg_indexes
-- WHERE tablename = 'sessions';

-- 查看所有约束
-- SELECT constraint_name, constraint_type
-- FROM information_schema.table_constraints
-- WHERE table_name = 'sessions';

-- 测试查询性能（EXPLAIN ANALYZE）
-- EXPLAIN ANALYZE
-- SELECT id, device_id, start_time, status, transcription, response
-- FROM sessions
-- WHERE device_id = 'some-device-id'
--   AND status = 'completed'
--   AND start_time >= NOW() - INTERVAL '7 days'
-- ORDER BY start_time DESC
-- LIMIT 20;
