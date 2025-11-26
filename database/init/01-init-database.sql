-- ============================================================================
-- Echo System 数据库初始化脚本
-- ============================================================================
-- 版本: 2025.01
-- 描述: 完整的数据库结构定义，包含所有表、索引、触发器和默认数据
-- 用途: 用于全新部署时自动初始化数据库（通过 PostgreSQL docker-entrypoint-initdb.d）
-- ============================================================================

-- ============================================================================
-- 1. 创建数据库扩展
-- ============================================================================

CREATE EXTENSION IF NOT EXISTS "uuid-ossp";
CREATE EXTENSION IF NOT EXISTS "pg_trgm";

-- ============================================================================
-- 2. 创建触发器函数
-- ============================================================================

-- 更新 updated_at 列的触发器函数
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- ============================================================================
-- 3. 创建用户表
-- ============================================================================

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

-- 用户表索引
CREATE INDEX IF NOT EXISTS idx_users_username ON users(username);
CREATE INDEX IF NOT EXISTS idx_users_email ON users(email);
CREATE INDEX IF NOT EXISTS idx_users_role ON users(role);

-- 用户表触发器
CREATE TRIGGER update_users_updated_at BEFORE UPDATE ON users
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

-- ============================================================================
-- 4. 创建设备表
-- ============================================================================

CREATE TABLE IF NOT EXISTS devices (
    id VARCHAR(255) PRIMARY KEY DEFAULT uuid_generate_v4(),
    name VARCHAR(100) NOT NULL,
    device_type VARCHAR(50) NOT NULL DEFAULT 'smart_speaker',
    mac_address VARCHAR(17) UNIQUE,
    ip_address INET,
    status VARCHAR(20) NOT NULL DEFAULT 'offline'
        CHECK (status IN ('online', 'offline', 'restarting', 'maintenance', 'pending')),
    firmware_version VARCHAR(50),
    battery_level INTEGER CHECK (battery_level >= 0 AND battery_level <= 100),
    volume_level INTEGER CHECK (volume_level >= 0 AND volume_level <= 100),
    location VARCHAR(100),
    last_seen TIMESTAMP WITH TIME ZONE,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),

    -- 设备注册相关字段
    pairing_code VARCHAR(10),
    registered_at TIMESTAMP WITH TIME ZONE,
    registration_token VARCHAR(64),
    serial_number VARCHAR(50) UNIQUE,

    -- 设备所有者和在线状态
    owner VARCHAR(100),
    is_online BOOLEAN DEFAULT false,

    -- EchoKit Server URL（必填字段）
    echokit_server_url VARCHAR(500) NOT NULL
);

-- 设备表索引
CREATE INDEX IF NOT EXISTS idx_devices_status ON devices(status);
CREATE INDEX IF NOT EXISTS idx_devices_type ON devices(device_type);
CREATE INDEX IF NOT EXISTS idx_devices_last_seen ON devices(last_seen);
CREATE INDEX IF NOT EXISTS idx_devices_mac_address ON devices(mac_address);
CREATE INDEX IF NOT EXISTS idx_devices_serial_number ON devices(serial_number);
CREATE INDEX IF NOT EXISTS idx_devices_pairing_code ON devices(pairing_code) WHERE pairing_code IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_devices_registration_token ON devices(registration_token) WHERE registration_token IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_devices_registered_at ON devices(registered_at) WHERE registered_at IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_devices_echokit_server_url ON devices(echokit_server_url) WHERE echokit_server_url IS NOT NULL;

-- 设备表触发器
CREATE TRIGGER update_devices_updated_at BEFORE UPDATE ON devices
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

-- ============================================================================
-- 5. 创建会话表
-- ============================================================================

CREATE TABLE IF NOT EXISTS sessions (
    id VARCHAR(255) PRIMARY KEY DEFAULT uuid_generate_v4()::VARCHAR,
    device_id VARCHAR(255) NOT NULL REFERENCES devices(id) ON DELETE CASCADE,
    user_id VARCHAR(255),
    session_type VARCHAR(20) NOT NULL DEFAULT 'voice'
        CHECK (session_type IN ('voice', 'text', 'command')),
    status VARCHAR(20) NOT NULL DEFAULT 'active'
        CHECK (status IN ('active', 'completed', 'failed', 'timeout')),
    transcription TEXT,
    response TEXT,
    confidence_score DECIMAL(3,2) CHECK (confidence_score >= 0.0 AND confidence_score <= 1.0),
    processing_time_ms INTEGER,
    duration INTEGER,
    audio_file_path VARCHAR(255),
    metadata JSONB,
    start_time TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    end_time TIMESTAMP WITH TIME ZONE
);

-- 会话表索引
CREATE INDEX IF NOT EXISTS idx_sessions_device_id ON sessions(device_id);
CREATE INDEX IF NOT EXISTS idx_sessions_user_id ON sessions(user_id);
CREATE INDEX IF NOT EXISTS idx_sessions_status ON sessions(status);
CREATE INDEX IF NOT EXISTS idx_sessions_start_time ON sessions(start_time DESC);
CREATE INDEX IF NOT EXISTS idx_sessions_session_type ON sessions(session_type);
CREATE INDEX IF NOT EXISTS idx_sessions_device_status ON sessions(device_id, status);
CREATE INDEX IF NOT EXISTS idx_sessions_start_time_status ON sessions(start_time DESC, status);

-- ============================================================================
-- 6. 创建设备注册令牌表
-- ============================================================================

CREATE TABLE IF NOT EXISTS device_registration_tokens (
    id SERIAL PRIMARY KEY,
    device_id VARCHAR(255) NOT NULL REFERENCES devices(id) ON DELETE CASCADE,
    pairing_code VARCHAR(255) NOT NULL,
    qr_token VARCHAR(255) NOT NULL,
    expires_at TIMESTAMP WITH TIME ZONE NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- 设备注册令牌表索引
CREATE INDEX IF NOT EXISTS idx_registration_tokens_device_id ON device_registration_tokens(device_id);
CREATE INDEX IF NOT EXISTS idx_registration_tokens_pairing_code ON device_registration_tokens(pairing_code);

-- ============================================================================
-- 7. 创建 EchoKit 服务器表
-- ============================================================================

CREATE TABLE IF NOT EXISTS echokit_servers (
    id SERIAL PRIMARY KEY,
    user_id VARCHAR(255) NOT NULL,
    server_url VARCHAR(512) NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    CONSTRAINT unique_user_server_url UNIQUE (user_id, server_url)
);

-- EchoKit 服务器表索引
CREATE INDEX IF NOT EXISTS idx_echokit_servers_user_id ON echokit_servers(user_id);
CREATE INDEX IF NOT EXISTS idx_echokit_servers_created_at ON echokit_servers(created_at DESC);

-- ============================================================================
-- 8. 创建用户设备关联表（可选）
-- ============================================================================

CREATE TABLE IF NOT EXISTS user_devices (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    device_id VARCHAR(255) NOT NULL REFERENCES devices(id) ON DELETE CASCADE,
    permission_level VARCHAR(20) NOT NULL DEFAULT 'user'
        CHECK (permission_level IN ('owner', 'admin', 'user', 'viewer')),
    assigned_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    UNIQUE(user_id, device_id)
);

-- 用户设备关联表索引
CREATE INDEX IF NOT EXISTS idx_user_devices_user_id ON user_devices(user_id);
CREATE INDEX IF NOT EXISTS idx_user_devices_device_id ON user_devices(device_id);

-- ============================================================================
-- 9. 创建系统配置表
-- ============================================================================

CREATE TABLE IF NOT EXISTS system_config (
    key VARCHAR(100) PRIMARY KEY,
    value TEXT,
    description TEXT,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- 系统配置表触发器
CREATE TRIGGER update_system_config_updated_at BEFORE UPDATE ON system_config
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

-- ============================================================================
-- 10. 插入默认数据
-- ============================================================================

-- 插入默认管理员用户（密码: admin123，使用 bcrypt 哈希）
INSERT INTO users (username, email, password_hash, role) VALUES
('admin', 'admin@echo-system.com', '$2b$12$kkSiszw6xbg8ck/h0dPFY.Pm8PtHh.JOiPWACtMliYS3P5wKtLDcW', 'Admin')
ON CONFLICT (username) DO NOTHING;

-- 插入默认系统配置
INSERT INTO system_config (key, value, description) VALUES
('system_name', 'Echo智能音箱系统', '系统名称'),
('max_concurrent_sessions', '100', '最大并发会话数'),
('default_session_timeout', '300', '默认会话超时时间（秒）'),
('audio_retention_days', '30', '音频文件保留天数'),
('enable_analytics', 'true', '是否启用数据分析'),
('default_echokit_server', 'wss://indie.echokit.dev/ws/ci-test-visitor', '默认 EchoKit Server URL')
ON CONFLICT (key) DO NOTHING;

-- ============================================================================
-- 11. 创建视图
-- ============================================================================

-- 设备状态概览视图
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
    d.is_online,
    d.echokit_server_url,
    COUNT(s.id) as total_sessions,
    COUNT(CASE WHEN s.start_time > NOW() - INTERVAL '24 hours' THEN 1 END) as sessions_24h,
    COUNT(CASE WHEN s.start_time > NOW() - INTERVAL '7 days' THEN 1 END) as sessions_7d
FROM devices d
LEFT JOIN sessions s ON d.id = s.device_id
GROUP BY d.id, d.name, d.device_type, d.status, d.location, d.battery_level, d.volume_level, d.last_seen, d.is_online, d.echokit_server_url;

-- 每日使用统计视图
CREATE OR REPLACE VIEW daily_usage_stats AS
SELECT
    DATE(s.start_time) as date,
    COUNT(s.id) as total_sessions,
    COUNT(CASE WHEN s.status = 'completed' THEN 1 END) as completed_sessions,
    COUNT(CASE WHEN s.status = 'failed' THEN 1 END) as failed_sessions,
    COUNT(CASE WHEN s.status = 'timeout' THEN 1 END) as timeout_sessions,
    AVG(s.processing_time_ms) as avg_processing_time,
    AVG(s.duration) as avg_duration,
    COUNT(DISTINCT s.device_id) as active_devices
FROM sessions s
WHERE s.start_time >= CURRENT_DATE - INTERVAL '30 days'
GROUP BY DATE(s.start_time)
ORDER BY date DESC;

-- ============================================================================
-- 12. 创建 Schema 版本记录表
-- ============================================================================

CREATE TABLE IF NOT EXISTS schema_versions (
    version VARCHAR(50) PRIMARY KEY,
    description TEXT,
    applied_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- 记录当前 schema 版本
INSERT INTO schema_versions (version, description) VALUES
('2025.01', '初始数据库结构：包含 users, devices, sessions, device_registration_tokens, echokit_servers 表，支持设备注册和 EchoKit Server URL 管理')
ON CONFLICT (version) DO NOTHING;

-- ============================================================================
-- 13. 完成提示
-- ============================================================================

DO $$
BEGIN
    RAISE NOTICE '==============================================================';
    RAISE NOTICE 'Echo System 数据库初始化完成！';
    RAISE NOTICE '==============================================================';
    RAISE NOTICE '已创建表:';
    RAISE NOTICE '  - users (用户表)';
    RAISE NOTICE '  - devices (设备表，包含 echokit_server_url 字段)';
    RAISE NOTICE '  - sessions (会话表)';
    RAISE NOTICE '  - device_registration_tokens (设备注册令牌表)';
    RAISE NOTICE '  - echokit_servers (EchoKit 服务器表)';
    RAISE NOTICE '  - user_devices (用户设备关联表)';
    RAISE NOTICE '  - system_config (系统配置表)';
    RAISE NOTICE '  - schema_versions (Schema 版本记录表)';
    RAISE NOTICE '';
    RAISE NOTICE '已创建视图:';
    RAISE NOTICE '  - device_status_overview';
    RAISE NOTICE '  - daily_usage_stats';
    RAISE NOTICE '';
    RAISE NOTICE '默认管理员账户:';
    RAISE NOTICE '  - 用户名: admin';
    RAISE NOTICE '  - 密码: admin123';
    RAISE NOTICE '';
    RAISE NOTICE 'Schema 版本: 2025.01';
    RAISE NOTICE '==============================================================';
END $$;
