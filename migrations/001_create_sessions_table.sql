-- ============================================================================
-- Echo System 数据库迁移脚本
-- 版本: 001
-- 描述: 创建 sessions 表（兼容现有 devices 表结构）
-- 创建日期: 2025-01-19
-- ============================================================================

BEGIN;

-- 创建会话表（字段名已匹配代码模型）
CREATE TABLE IF NOT EXISTS sessions (
    id VARCHAR(255) PRIMARY KEY DEFAULT uuid_generate_v4()::VARCHAR,
    device_id VARCHAR(255) NOT NULL REFERENCES devices(id) ON DELETE CASCADE,
    user_id VARCHAR(255),  -- 可为空，因为有些会话可能没有关联用户
    session_type VARCHAR(20) NOT NULL DEFAULT 'voice' CHECK (session_type IN ('voice', 'text', 'command')),
    status VARCHAR(20) NOT NULL DEFAULT 'active' CHECK (status IN ('active', 'completed', 'failed', 'timeout')),
    transcription TEXT,  -- 语音转录文本
    response TEXT,  -- AI 响应内容
    confidence_score DECIMAL(3,2) CHECK (confidence_score >= 0.0 AND confidence_score <= 1.0),
    processing_time_ms INTEGER,
    duration INTEGER,  -- 会话时长（秒）
    audio_file_path VARCHAR(255),
    metadata JSONB,
    start_time TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    end_time TIMESTAMP WITH TIME ZONE
);

-- 创建索引
CREATE INDEX IF NOT EXISTS idx_sessions_device_id ON sessions(device_id);
CREATE INDEX IF NOT EXISTS idx_sessions_user_id ON sessions(user_id);
CREATE INDEX IF NOT EXISTS idx_sessions_status ON sessions(status);
CREATE INDEX IF NOT EXISTS idx_sessions_start_time ON sessions(start_time DESC);
CREATE INDEX IF NOT EXISTS idx_sessions_session_type ON sessions(session_type);

-- 创建复合索引（优化常用查询）
CREATE INDEX IF NOT EXISTS idx_sessions_device_status ON sessions(device_id, status);
CREATE INDEX IF NOT EXISTS idx_sessions_start_time_status ON sessions(start_time DESC, status);

-- 添加表注释
COMMENT ON TABLE sessions IS '语音交互会话记录表';
COMMENT ON COLUMN sessions.id IS '会话唯一标识';
COMMENT ON COLUMN sessions.device_id IS '设备ID（外键关联 devices 表）';
COMMENT ON COLUMN sessions.user_id IS '用户ID（可为空）';
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

-- 创建视图：设备状态概览
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

-- 创建视图：每日使用统计
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

-- 输出成功信息
DO $$
DECLARE
    column_count INTEGER;
    index_count INTEGER;
BEGIN
    SELECT COUNT(*) INTO column_count
    FROM information_schema.columns
    WHERE table_name = 'sessions';

    SELECT COUNT(*) INTO index_count
    FROM pg_indexes
    WHERE tablename = 'sessions';

    RAISE NOTICE '=== Sessions 表创建成功 ===';
    RAISE NOTICE '列数: %', column_count;
    RAISE NOTICE '索引数: %', index_count;
END $$;

COMMIT;
