-- 为 devices 表添加 echokit_server_url 字段
-- 这个字段用于存储设备关联的 EchoKit Server URL

ALTER TABLE devices
ADD COLUMN IF NOT EXISTS echokit_server_url VARCHAR(500);

-- 添加注释说明
COMMENT ON COLUMN devices.echokit_server_url IS 'EchoKit Server URL that the device connects to';

-- 添加索引以提高查询性能
CREATE INDEX IF NOT EXISTS idx_devices_echokit_server_url
ON devices(echokit_server_url)
WHERE echokit_server_url IS NOT NULL;

-- 可选：如果需要确保 echokit_server_url 必须来自 echokit_servers 表，可以添加外键约束
-- 但考虑到灵活性，这里先不添加外键约束，只做软关联
-- 如果将来需要，可以取消下面的注释
/*
ALTER TABLE devices
ADD CONSTRAINT fk_devices_echokit_server_url
FOREIGN KEY (echokit_server_url)
REFERENCES echokit_servers(server_url)
ON DELETE SET NULL
ON UPDATE CASCADE;
*/
