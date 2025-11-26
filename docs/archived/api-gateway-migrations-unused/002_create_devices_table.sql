-- 创建设备表
CREATE TABLE IF NOT EXISTS devices (
    id VARCHAR(36) PRIMARY KEY,
    name VARCHAR(100) NOT NULL,
    device_type VARCHAR(50) NOT NULL,
    status VARCHAR(20) NOT NULL DEFAULT 'offline',
    location VARCHAR(200),
    firmware_version VARCHAR(50),
    battery_level INTEGER DEFAULT 100,
    volume INTEGER DEFAULT 50,
    last_seen TIMESTAMP WITH TIME ZONE,
    is_online BOOLEAN DEFAULT false,
    owner_id VARCHAR(36),
    config JSONB DEFAULT '{}',
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    FOREIGN KEY (owner_id) REFERENCES users(id) ON DELETE SET NULL
);

-- 创建索引
CREATE INDEX IF NOT EXISTS idx_devices_name ON devices(name);
CREATE INDEX IF NOT EXISTS idx_devices_type ON devices(device_type);
CREATE INDEX IF NOT EXISTS idx_devices_status ON devices(status);
CREATE INDEX IF NOT EXISTS idx_devices_owner ON devices(owner_id);
CREATE INDEX IF NOT EXISTS idx_devices_is_online ON devices(is_online);
CREATE INDEX IF NOT EXISTS idx_devices_last_seen ON devices(last_seen);

-- 创建更新时间触发器
CREATE TRIGGER update_devices_updated_at
    BEFORE UPDATE ON devices
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

-- 插入一些示例设备
INSERT INTO devices (id, name, device_type, status, location, is_online, owner_id)
VALUES
    ('device-001', 'Living Room Echo', 'echo_show_8', 'online', 'Living Room', true, 'admin-001'),
    ('device-002', 'Bedroom Echo', 'echo_dot_4', 'offline', 'Master Bedroom', false, 'admin-001'),
    ('device-003', 'Kitchen Echo', 'echo_pop', 'online', 'Kitchen', true, 'user-001')
ON CONFLICT (id) DO NOTHING;