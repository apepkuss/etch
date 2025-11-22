-- 设备注册表迁移：将device_id从UUID改为VARCHAR
-- 迁移步骤：
-- 1. 备份现有数据
-- 2. 删除外键约束
-- 3. 修改device_id类型
-- 4. 重建约束

-- 创建临时表备份现有设备数据
CREATE TABLE devices_backup AS SELECT * FROM devices;

-- 创建临时表存储新的设备ID映射
CREATE TABLE device_id_mapping (
    old_uuid UUID PRIMARY KEY,
    new_device_id VARCHAR(255) UNIQUE NOT NULL
);

-- 删除device_registration_tokens表的外键约束（如果存在）
DROP TABLE IF EXISTS device_registration_tokens CASCADE;

-- 修改devices表的device_id列类型
ALTER TABLE devices DROP CONSTRAINT IF EXISTS devices_pkey;
ALTER TABLE devices ALTER COLUMN id TYPE VARCHAR(255) USING id::TEXT;

-- 重新设置主键
ALTER TABLE devices ADD CONSTRAINT devices_pkey PRIMARY KEY (id);

-- 更新device_registration_tokens表的结构
CREATE TABLE device_registration_tokens (
    id SERIAL PRIMARY KEY,
    device_id VARCHAR(255) NOT NULL,
    pairing_code VARCHAR(255) NOT NULL,
    qr_token VARCHAR(255) NOT NULL,
    expires_at TIMESTAMP WITH TIME ZONE NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    FOREIGN KEY (device_id) REFERENCES devices(id) ON DELETE CASCADE
);

-- 创建索引以提高查询性能
CREATE INDEX idx_devices_serial_number ON devices(serial_number);
CREATE INDEX idx_devices_mac_address ON devices(mac_address);
CREATE INDEX idx_devices_pairing_code ON devices(pairing_code);
CREATE INDEX idx_registration_tokens_device_id ON device_registration_tokens(device_id);
CREATE INDEX idx_registration_tokens_pairing_code ON device_registration_tokens(pairing_code);