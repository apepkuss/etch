-- 创建会话表
CREATE TABLE IF NOT EXISTS sessions (
    id VARCHAR(36) PRIMARY KEY,
    device_id VARCHAR(36) NOT NULL,
    user_id VARCHAR(36),
    status VARCHAR(20) NOT NULL DEFAULT 'active',
    started_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    ended_at TIMESTAMP WITH TIME ZONE,
    wake_reason VARCHAR(100),
    transcript TEXT,
    audio_file_path VARCHAR(500),
    duration_seconds INTEGER,
    config JSONB DEFAULT '{}',
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    FOREIGN KEY (device_id) REFERENCES devices(id) ON DELETE CASCADE,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE SET NULL
);

-- 创建索引
CREATE INDEX IF NOT EXISTS idx_sessions_device ON sessions(device_id);
CREATE INDEX IF NOT EXISTS idx_sessions_user ON sessions(user_id);
CREATE INDEX IF NOT EXISTS idx_sessions_status ON sessions(status);
CREATE INDEX IF NOT EXISTS idx_sessions_started_at ON sessions(started_at);
CREATE INDEX IF NOT EXISTS idx_sessions_ended_at ON sessions(ended_at);

-- 创建更新时间触发器
CREATE TRIGGER update_sessions_updated_at
    BEFORE UPDATE ON sessions
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

-- 插入一些示例会话
INSERT INTO sessions (id, device_id, user_id, status, wake_reason, duration_seconds)
VALUES
    ('session-001', 'device-001', 'admin-001', 'completed', 'voice_wake', 120),
    ('session-002', 'device-002', 'admin-001', 'active', 'manual', NULL),
    ('session-003', 'device-003', 'user-001', 'failed', 'voice_wake', 30)
ON CONFLICT (id) DO NOTHING;