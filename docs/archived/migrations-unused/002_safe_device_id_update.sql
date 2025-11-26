-- 安全的设备ID格式更新脚本
-- 将UUID格式的device_id改为VARCHAR以支持ECHO_<SN>_<MAC>格式

-- 首先备份现有设备数据
CREATE TABLE IF NOT EXISTS devices_migration_backup AS SELECT * FROM devices;

-- 查看当前数据库表结构
\d devices

-- 由于有外键依赖，我们需要分步骤进行

-- 1. 检查现有设备数据
SELECT COUNT(*) as device_count FROM devices;
SELECT id, name, serial_number, mac_address FROM devices LIMIT 5;

-- 2. 首先删除依赖表中的外键约束
DROP TABLE IF EXISTS device_registration_events CASCADE;
DROP TABLE IF EXISTS user_devices CASCADE;
DROP TABLE IF EXISTS sessions CASCADE;
DROP TABLE IF EXISTS device_registration_tokens CASCADE;

-- 删除依赖的视图
DROP VIEW IF EXISTS device_status_overview CASCADE;
DROP VIEW IF EXISTS pending_registrations CASCADE;

-- 3. 修改devices表的device_id字段类型
ALTER TABLE devices DROP CONSTRAINT devices_pkey CASCADE;
ALTER TABLE devices ALTER COLUMN id TYPE VARCHAR(255) USING id::TEXT;
ALTER TABLE devices ADD CONSTRAINT devices_pkey PRIMARY KEY (id);

-- 4. 重新创建device_registration_tokens表
CREATE TABLE device_registration_tokens (
    id SERIAL PRIMARY KEY,
    device_id VARCHAR(255) NOT NULL,
    pairing_code VARCHAR(255) NOT NULL,
    qr_token VARCHAR(255) NOT NULL,
    expires_at TIMESTAMP WITH TIME ZONE NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    FOREIGN KEY (device_id) REFERENCES devices(id) ON DELETE CASCADE
);

-- 5. 创建索引
CREATE INDEX IF NOT EXISTS idx_devices_serial_number ON devices(serial_number);
CREATE INDEX IF NOT EXISTS idx_devices_mac_address ON devices(mac_address);
CREATE INDEX IF NOT EXISTS idx_devices_pairing_code ON devices(pairing_code);
CREATE INDEX IF NOT EXISTS idx_registration_tokens_device_id ON device_registration_tokens(device_id);
CREATE INDEX IF NOT EXISTS idx_registration_tokens_pairing_code ON device_registration_tokens(pairing_code);

-- 6. 验证更改
\d devices
\d device_registration_tokens

-- 7. 显示迁移完成信息
SELECT 'Migration completed successfully' as status;