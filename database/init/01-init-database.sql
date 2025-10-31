-- Echo System 数据库初始化脚本

-- 创建数据库扩展
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";
CREATE EXTENSION IF NOT EXISTS "pg_trgm";

-- 创建用户表
CREATE TABLE IF NOT EXISTS users (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    username VARCHAR(50) UNIQUE NOT NULL,
    email VARCHAR(100) UNIQUE NOT NULL,
    password_hash VARCHAR(255) NOT NULL,
    role VARCHAR(20) NOT NULL DEFAULT 'Viewer' CHECK (role IN ('Admin', 'Manager', 'Viewer')),
    is_active BOOLEAN DEFAULT true,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- 创建设备表
CREATE TABLE IF NOT EXISTS devices (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    name VARCHAR(100) NOT NULL,
    device_type VARCHAR(50) NOT NULL DEFAULT 'smart_speaker',
    mac_address VARCHAR(17) UNIQUE,
    ip_address INET,
    status VARCHAR(20) NOT NULL DEFAULT 'offline' CHECK (status IN ('online', 'offline', 'restarting', 'maintenance')),
    firmware_version VARCHAR(50),
    battery_level INTEGER CHECK (battery_level >= 0 AND battery_level <= 100),
    volume_level INTEGER CHECK (volume_level >= 0 AND volume_level <= 100),
    location VARCHAR(100),
    last_seen TIMESTAMP WITH TIME ZONE,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- 创建会话表
CREATE TABLE IF NOT EXISTS sessions (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    device_id UUID NOT NULL REFERENCES devices(id) ON DELETE CASCADE,
    user_id UUID REFERENCES users(id) ON DELETE SET NULL,
    session_type VARCHAR(20) NOT NULL DEFAULT 'voice' CHECK (session_type IN ('voice', 'text', 'command')),
    status VARCHAR(20) NOT NULL DEFAULT 'active' CHECK (status IN ('active', 'completed', 'failed', 'cancelled')),
    transcript TEXT,
    response TEXT,
    confidence_score DECIMAL(3,2) CHECK (confidence_score >= 0.0 AND confidence_score <= 1.0),
    processing_time_ms INTEGER,
    audio_file_path VARCHAR(255),
    metadata JSONB,
    started_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    completed_at TIMESTAMP WITH TIME ZONE
);

-- 创建用户设备关联表
CREATE TABLE IF NOT EXISTS user_devices (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    device_id UUID NOT NULL REFERENCES devices(id) ON DELETE CASCADE,
    permission_level VARCHAR(20) NOT NULL DEFAULT 'user' CHECK (permission_level IN ('owner', 'admin', 'user', 'viewer')),
    assigned_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    UNIQUE(user_id, device_id)
);

-- 创建系统配置表
CREATE TABLE IF NOT EXISTS system_config (
    key VARCHAR(100) PRIMARY KEY,
    value TEXT,
    description TEXT,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- 创建索引
CREATE INDEX IF NOT EXISTS idx_users_username ON users(username);
CREATE INDEX IF NOT EXISTS idx_users_email ON users(email);
CREATE INDEX IF NOT EXISTS idx_users_role ON users(role);

CREATE INDEX IF NOT EXISTS idx_devices_status ON devices(status);
CREATE INDEX IF NOT EXISTS idx_devices_type ON devices(device_type);
CREATE INDEX IF NOT EXISTS idx_devices_last_seen ON devices(last_seen);

CREATE INDEX IF NOT EXISTS idx_sessions_device_id ON sessions(device_id);
CREATE INDEX IF NOT EXISTS idx_sessions_user_id ON sessions(user_id);
CREATE INDEX IF NOT EXISTS idx_sessions_status ON sessions(status);
CREATE INDEX IF NOT EXISTS idx_sessions_started_at ON sessions(started_at);
CREATE INDEX IF NOT EXISTS idx_sessions_type ON sessions(session_type);

CREATE INDEX IF NOT EXISTS idx_user_devices_user_id ON user_devices(user_id);
CREATE INDEX IF NOT EXISTS idx_user_devices_device_id ON user_devices(device_id);

-- 创建更新时间触发器函数
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ language 'plpgsql';

-- 为需要的表添加更新时间触发器
CREATE TRIGGER update_users_updated_at BEFORE UPDATE ON users
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_devices_updated_at BEFORE UPDATE ON devices
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_system_config_updated_at BEFORE UPDATE ON system_config
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

-- 插入默认管理员用户（密码: admin123，已使用 bcrypt �哈希）
INSERT INTO users (username, email, password_hash, role) VALUES
('admin', 'admin@echo-system.com', '$2b$12$kkSiszw6xbg8ck/h0dPFY.Pm8PtHh.JOiPWACtMliYS3P5wKtLDcW', 'Admin')
ON CONFLICT (username) DO NOTHING;

-- 插入测试设备
INSERT INTO devices (name, device_type, mac_address, ip_address, status, firmware_version, battery_level, volume_level, location) VALUES
('客厅音箱', 'smart_speaker', '00:11:22:33:44:55', '192.168.1.100', 'online', '1.0.0', 85, 50, '客厅'),
('卧室音箱', 'smart_speaker', '00:11:22:33:44:56', '192.168.1.101', 'online', '1.0.0', 92, 30, '卧室')
ON CONFLICT (mac_address) DO NOTHING;

-- 插入默认系统配置
INSERT INTO system_config (key, value, description) VALUES
('system_name', 'Echo智能音箱系统', '系统名称'),
('max_concurrent_sessions', '100', '最大并发会话数'),
('default_session_timeout', '300', '默认会话超时时间（秒）'),
('audio_retention_days', '30', '音频文件保留天数'),
('enable_analytics', 'true', '是否启用数据分析')
ON CONFLICT (key) DO NOTHING;

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
    COUNT(CASE WHEN s.started_at > NOW() - INTERVAL '24 hours' THEN 1 END) as sessions_24h,
    COUNT(CASE WHEN s.started_at > NOW() - INTERVAL '7 days' THEN 1 END) as sessions_7d
FROM devices d
LEFT JOIN sessions s ON d.id = s.device_id
GROUP BY d.id, d.name, d.device_type, d.status, d.location, d.battery_level, d.volume_level, d.last_seen;

-- 创建视图：每日使用统计
CREATE OR REPLACE VIEW daily_usage_stats AS
SELECT
    DATE(s.started_at) as date,
    COUNT(s.id) as total_sessions,
    COUNT(CASE WHEN s.status = 'completed' THEN 1 END) as completed_sessions,
    COUNT(CASE WHEN s.status = 'failed' THEN 1 END) as failed_sessions,
    AVG(s.processing_time_ms) as avg_processing_time,
    COUNT(DISTINCT s.device_id) as active_devices
FROM sessions s
WHERE s.started_at >= CURRENT_DATE - INTERVAL '30 days'
GROUP BY DATE(s.started_at)
ORDER BY date DESC;