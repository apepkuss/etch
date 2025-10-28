-- 初始数据库架构
-- 创建Echo智能音箱系统的所有基础表

-- 启用必要的扩展
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";
CREATE EXTENSION IF NOT EXISTS "pgcrypto";

-- 创建用户角色枚举
DO $$ BEGIN
    CREATE TYPE user_role AS ENUM ('admin', 'user', 'viewer');
EXCEPTION
    WHEN duplicate_object THEN null;
END $$;

-- 创建设备类型枚举
DO $$ BEGIN
    CREATE TYPE device_type AS ENUM ('speaker', 'display', 'hub');
EXCEPTION
    WHEN duplicate_object THEN null;
END $$;

-- 创建设备状态枚举
DO $$ BEGIN
    CREATE TYPE device_status AS ENUM ('online', 'offline', 'maintenance', 'error');
EXCEPTION
    WHEN duplicate_object THEN null;
END $$;

-- 创建会话状态枚举
DO $$ BEGIN
    CREATE TYPE session_status AS ENUM ('active', 'completed', 'failed', 'timeout');
EXCEPTION
    WHEN duplicate_object THEN null;
END $$;

-- 创建设备权限枚举
DO $$ BEGIN
    CREATE TYPE device_permission AS ENUM ('owner', 'admin', 'user', 'viewer');
EXCEPTION
    WHEN duplicate_object THEN null;
END $$;

-- 创建用户表
CREATE TABLE IF NOT EXISTS users (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    username VARCHAR(50) UNIQUE NOT NULL,
    email VARCHAR(255) UNIQUE NOT NULL,
    password_hash VARCHAR(255) NOT NULL,
    role user_role NOT NULL DEFAULT 'user',
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    is_active BOOLEAN DEFAULT TRUE,

    -- 约束
    CONSTRAINT users_username_length CHECK (LENGTH(username) >= 3),
    CONSTRAINT users_email_format CHECK (email ~* '^[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}$')
);

-- 创建设备表
CREATE TABLE IF NOT EXISTS devices (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    name VARCHAR(100) NOT NULL,
    device_type device_type NOT NULL,
    status device_status NOT NULL DEFAULT 'offline',
    location VARCHAR(100),
    firmware_version VARCHAR(50),
    battery_level INTEGER DEFAULT 0 CHECK (battery_level >= 0 AND battery_level <= 100),
    volume INTEGER DEFAULT 50 CHECK (volume >= 0 AND volume <= 100),
    last_seen TIMESTAMP WITH TIME ZONE,
    is_online BOOLEAN DEFAULT FALSE,
    owner_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    config JSONB DEFAULT '{}',

    -- 约束
    CONSTRAINT devices_name_length CHECK (LENGTH(name) >= 1),
    CONSTRAINT devices_firmware_format CHECK (firmware_version ~* '^\d+\.\d+\.\d+$' OR firmware_version IS NULL)
);

-- 创建会话表
CREATE TABLE IF NOT EXISTS sessions (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    device_id UUID NOT NULL REFERENCES devices(id) ON DELETE CASCADE,
    user_id UUID REFERENCES users(id) ON DELETE SET NULL,
    status session_status NOT NULL DEFAULT 'active',
    started_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    ended_at TIMESTAMP WITH TIME ZONE,
    wake_reason VARCHAR(50),
    transcript TEXT,
    response TEXT,
    audio_url VARCHAR(500),
    metadata JSONB DEFAULT '{}',

    -- 约束
    CONSTRAINT sessions_ended_after_started CHECK (ended_at IS NULL OR ended_at >= started_at),
    CONSTRAINT sessions_transcript_when_completed CHECK (
        (status = 'completed' AND transcript IS NOT NULL) OR
        status != 'completed'
    )
);

-- 创建用户设备关联表
CREATE TABLE IF NOT EXISTS user_devices (
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    device_id UUID NOT NULL REFERENCES devices(id) ON DELETE CASCADE,
    permission device_permission NOT NULL DEFAULT 'owner',
    granted_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    granted_by UUID NOT NULL REFERENCES users(id),

    PRIMARY KEY (user_id, device_id),

    -- 确保每个设备至少有一个所有者
    CONSTRAINT user_devices_at_least_one_owner CHECK (
        NOT EXISTS (
            SELECT 1 FROM devices d
            WHERE NOT EXISTS (
                SELECT 1 FROM user_devices ud
                WHERE ud.device_id = d.id AND ud.permission = 'owner'
            )
        )
    )
);

-- 创建系统事件日志表
CREATE TABLE IF NOT EXISTS system_events (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    event_type VARCHAR(50) NOT NULL,
    source_service VARCHAR(50) NOT NULL,
    entity_type VARCHAR(50) NOT NULL, -- 'user', 'device', 'session'
    entity_id UUID NOT NULL,
    description TEXT,
    metadata JSONB DEFAULT '{}',
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- 创建设备配置变更历史表
CREATE TABLE IF NOT EXISTS device_config_history (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    device_id UUID NOT NULL REFERENCES devices(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(id),
    old_config JSONB,
    new_config JSONB,
    changed_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- 创建索引
-- 用户表索引
CREATE INDEX IF NOT EXISTS idx_users_username ON users(username);
CREATE INDEX IF NOT EXISTS idx_users_email ON users(email);
CREATE INDEX IF NOT EXISTS idx_users_role ON users(role);
CREATE INDEX IF NOT EXISTS idx_users_is_active ON users(is_active);

-- 设备表索引
CREATE INDEX IF NOT EXISTS idx_devices_owner_id ON devices(owner_id);
CREATE INDEX IF NOT EXISTS idx_devices_status ON devices(status);
CREATE INDEX IF NOT EXISTS idx_devices_type ON devices(device_type);
CREATE INDEX IF NOT EXISTS idx_devices_last_seen ON devices(last_seen);
CREATE INDEX IF NOT EXISTS idx_devices_is_online ON devices(is_online);
CREATE INDEX IF NOT EXISTS idx_devices_location ON devices(location);

-- 会话表索引
CREATE INDEX IF NOT EXISTS idx_sessions_device_id ON sessions(device_id);
CREATE INDEX IF NOT EXISTS idx_sessions_user_id ON sessions(user_id);
CREATE INDEX IF NOT EXISTS idx_sessions_status ON sessions(status);
CREATE INDEX IF NOT EXISTS idx_sessions_started_at ON sessions(started_at);
CREATE INDEX IF NOT EXISTS idx_sessions_ended_at ON sessions(ended_at);

-- 用户设备关联表索引
CREATE INDEX IF NOT EXISTS idx_user_devices_user_id ON user_devices(user_id);
CREATE INDEX IF NOT EXISTS idx_user_devices_device_id ON user_devices(device_id);
CREATE INDEX IF NOT EXISTS idx_user_devices_permission ON user_devices(permission);

-- 系统事件表索引
CREATE INDEX IF NOT EXISTS idx_system_events_type ON system_events(event_type);
CREATE INDEX IF NOT EXISTS idx_system_events_source ON system_events(source_service);
CREATE INDEX IF NOT EXISTS idx_system_events_entity ON system_events(entity_type, entity_id);
CREATE INDEX IF NOT EXISTS idx_system_events_created_at ON system_events(created_at);

-- 设备配置历史表索引
CREATE INDEX IF NOT EXISTS idx_device_config_history_device_id ON device_config_history(device_id);
CREATE INDEX IF NOT EXISTS idx_device_config_history_user_id ON device_config_history(user_id);
CREATE INDEX IF NOT EXISTS idx_device_config_history_changed_at ON device_config_history(changed_at);

-- 创建更新时间戳的触发器函数
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ language 'plpgsql';

-- 为需要的表添加更新时间戳触发器
CREATE TRIGGER update_users_updated_at BEFORE UPDATE ON users
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_devices_updated_at BEFORE UPDATE ON devices
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

-- 创建会话结束的触发器函数
CREATE OR REPLACE FUNCTION end_session_when_device_offline()
RETURNS TRIGGER AS $$
BEGIN
    -- 当设备离线时，结束所有活跃的会话
    IF NEW.is_online = FALSE AND OLD.is_online = TRUE THEN
        UPDATE sessions
        SET status = 'timeout', ended_at = NOW()
        WHERE device_id = NEW.id AND status = 'active';
    END IF;
    RETURN NEW;
END;
$$ language 'plpgsql';

-- 添加设备状态变化触发器
CREATE TRIGGER end_sessions_on_device_offline AFTER UPDATE ON devices
    FOR EACH ROW EXECUTE FUNCTION end_session_when_device_offline();

-- 插入初始数据
-- 创建默认管理员用户（密码: admin123）
INSERT INTO users (username, email, password_hash, role)
VALUES (
    'admin',
    'admin@echo.local',
    '$2b$12$LQv3c1yqBWVHxkd0LHAkCOYz6TtxMQJqhN8/LewdBPj5LJCu/TLh.', -- admin123的bcrypt hash
    'admin'
) ON CONFLICT (username) DO NOTHING;

-- 插入示例设备（仅用于测试）
INSERT INTO devices (name, device_type, status, location, firmware_version, owner_id)
SELECT
    'Living Room Speaker',
    'speaker'::device_type,
    'offline'::device_status,
    'Living Room',
    '1.0.0',
    id
FROM users
WHERE username = 'admin'
LIMIT 1
ON CONFLICT DO NOTHING;

-- 为示例设备创建用户设备关联
INSERT INTO user_devices (user_id, device_id, permission, granted_by)
SELECT
    u.id,
    d.id,
    'owner'::device_permission,
    u.id
FROM users u, devices d
WHERE u.username = 'admin' AND d.name = 'Living Room Speaker'
LIMIT 1
ON CONFLICT DO NOTHING;

-- 创建视图
-- 设备详情视图（包含用户信息）
CREATE OR REPLACE VIEW device_details AS
SELECT
    d.id,
    d.name,
    d.device_type,
    d.status,
    d.location,
    d.firmware_version,
    d.battery_level,
    d.volume,
    d.last_seen,
    d.is_online,
    d.config,
    d.created_at,
    d.updated_at,
    u.id as owner_id,
    u.username as owner_username,
    u.email as owner_email
FROM devices d
JOIN users u ON d.owner_id = u.id;

-- 用户设备统计视图
CREATE OR REPLACE VIEW user_device_stats AS
SELECT
    u.id as user_id,
    u.username,
    COUNT(d.id) as total_devices,
    COUNT(CASE WHEN d.is_online = TRUE THEN 1 END) as online_devices,
    COUNT(CASE WHEN d.device_type = 'speaker' THEN 1 END) as speaker_count,
    COUNT(CASE WHEN d.device_type = 'display' THEN 1 END) as display_count,
    COUNT(CASE WHEN d.device_type = 'hub' THEN 1 END) as hub_count,
    MAX(d.last_seen) as last_device_activity
FROM users u
LEFT JOIN devices d ON u.id = d.owner_id
GROUP BY u.id, u.username;

COMMENT ON TABLE users IS '用户表 - 存储系统用户信息';
COMMENT ON TABLE devices IS '设备表 - 存储智能音箱设备信息';
COMMENT ON TABLE sessions IS '会话表 - 存储语音交互会话记录';
COMMENT ON TABLE user_devices IS '用户设备关联表 - 管理用户对设备的权限';
COMMENT ON TABLE system_events IS '系统事件日志表 - 记录重要的系统事件';
COMMENT ON TABLE device_config_history IS '设备配置变更历史表 - 追踪设备配置变更';