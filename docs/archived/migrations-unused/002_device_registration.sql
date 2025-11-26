-- 设备注册功能扩展
-- 为设备表添加注册相关字段，创建设备注册令牌表

-- 添加新的设备状态枚举值
DO $$ BEGIN
    ALTER TYPE device_status ADD VALUE 'pending';
EXCEPTION
    WHEN duplicate_object THEN null;
END $$;

DO $$ BEGIN
    ALTER TYPE device_status ADD VALUE 'registration_expired';
EXCEPTION
    WHEN duplicate_object THEN null;
END $$;

-- 为设备表添加注册相关字段
ALTER TABLE devices
ADD COLUMN IF NOT EXISTS pairing_code VARCHAR(10),
ADD COLUMN IF NOT EXISTS registered_at TIMESTAMP WITH TIME ZONE,
ADD COLUMN IF NOT EXISTS registration_token VARCHAR(64);

-- 添加配对码唯一性约束
ALTER TABLE devices
ADD CONSTRAINT devices_pairing_code_unique UNIQUE (pairing_code)
WHERE pairing_code IS NOT NULL;

-- 添加注册令牌唯一性约束
ALTER TABLE devices
ADD CONSTRAINT devices_registration_token_unique UNIQUE (registration_token)
WHERE registration_token IS NOT NULL;

-- 创建设备注册令牌表
CREATE TABLE IF NOT EXISTS device_registration_tokens (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    device_id UUID NOT NULL REFERENCES devices(id) ON DELETE CASCADE,
    pairing_code VARCHAR(10) UNIQUE NOT NULL,
    qr_token VARCHAR(64) UNIQUE NOT NULL,
    expires_at TIMESTAMP WITH TIME ZONE NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    used BOOLEAN DEFAULT FALSE,
    max_attempts INTEGER DEFAULT 3,
    attempts_count INTEGER DEFAULT 0,

    -- 约束
    CONSTRAINT registration_tokens_pairing_code_format CHECK (pairing_code ~* '^[A-Z0-9]{6,10}$'),
    CONSTRAINT registration_tokens_positive_expires_at CHECK (expires_at > created_at),
    CONSTRAINT registration_tokens_positive_max_attempts CHECK (max_attempts > 0),
    CONSTRAINT registration_tokens_attempts_not_exceed_max CHECK (attempts_count <= max_attempts)
);

-- 创建设备注册事件日志表
CREATE TABLE IF NOT EXISTS device_registration_events (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    device_id UUID REFERENCES devices(id) ON DELETE SET NULL,
    registration_token_id UUID REFERENCES device_registration_tokens(id) ON DELETE SET NULL,
    event_type VARCHAR(20) NOT NULL, -- 'created', 'verified', 'expired', 'failed', 'cancelled'
    event_description TEXT,
    client_info JSONB DEFAULT '{}', -- 客户端信息（IP地址、User-Agent等）
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),

    -- 约束
    CONSTRAINT registration_events_valid_type CHECK (event_type IN ('created', 'verified', 'expired', 'failed', 'cancelled'))
);

-- 创建索引
-- 设备注册令牌表索引
CREATE INDEX IF NOT EXISTS idx_device_registration_tokens_device_id ON device_registration_tokens(device_id);
CREATE INDEX IF NOT EXISTS idx_device_registration_tokens_pairing_code ON device_registration_tokens(pairing_code);
CREATE INDEX IF NOT EXISTS idx_device_registration_tokens_qr_token ON device_registration_tokens(qr_token);
CREATE INDEX IF NOT EXISTS idx_device_registration_tokens_expires_at ON device_registration_tokens(expires_at);
CREATE INDEX IF NOT EXISTS idx_device_registration_tokens_used ON device_registration_tokens(used);
CREATE INDEX IF NOT EXISTS idx_device_registration_tokens_created_at ON device_registration_tokens(created_at);

-- 设备注册事件日志表索引
CREATE INDEX IF NOT EXISTS idx_device_registration_events_device_id ON device_registration_events(device_id);
CREATE INDEX IF NOT EXISTS idx_device_registration_events_token_id ON device_registration_events(registration_token_id);
CREATE INDEX IF NOT EXISTS idx_device_registration_events_type ON device_registration_events(event_type);
CREATE INDEX IF NOT EXISTS idx_device_registration_events_created_at ON device_registration_events(created_at);

-- 设备表新增字段索引
CREATE INDEX IF NOT EXISTS idx_devices_pairing_code ON devices(pairing_code) WHERE pairing_code IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_devices_registration_token ON devices(registration_token) WHERE registration_token IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_devices_registered_at ON devices(registered_at) WHERE registered_at IS NOT NULL;

-- 创建函数：生成配对码
CREATE OR REPLACE FUNCTION generate_pairing_code()
RETURNS VARCHAR(10) AS $$
DECLARE
    new_code VARCHAR(10);
    code_exists BOOLEAN;
    max_attempts INTEGER := 10;
    attempts INTEGER := 0;
BEGIN
    LOOP
        -- 生成6位大写字母数字组合
        new_code := upper(substring(encode(gen_random_bytes(4), 'base64'), 1, 6));
        -- 替换特殊字符为字母数字
        new_code := regexp_replace(new_code, '[^A-Z0-9]', '', 'g');

        -- 确保长度在6-8位之间
        IF length(new_code) < 6 THEN
            new_code := new_code || substring('ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789', 1, 6 - length(new_code));
        ELSIF length(new_code) > 8 THEN
            new_code := substring(new_code, 1, 8);
        END IF;

        -- 检查是否已存在
        SELECT EXISTS(SELECT 1 FROM device_registration_tokens WHERE pairing_code = new_code) OR
               EXISTS(SELECT 1 FROM devices WHERE pairing_code = new_code)
        INTO code_exists;

        IF NOT code_exists THEN
            EXIT;
        END IF;

        attempts := attempts + 1;
        IF attempts >= max_attempts THEN
            RAISE EXCEPTION 'Failed to generate unique pairing code after % attempts', max_attempts;
        END IF;
    END LOOP;

    RETURN new_code;
END;
$$ LANGUAGE plpgsql;

-- 创建函数：生成QR令牌
CREATE OR REPLACE FUNCTION generate_qr_token()
RETURNS VARCHAR(64) AS $$
BEGIN
    RETURN encode(sha256(random()::text::bytea), 'base64');
END;
$$ LANGUAGE plpgsql;

-- 创建函数：清理过期的注册令牌
CREATE OR REPLACE FUNCTION cleanup_expired_registration_tokens()
RETURNS INTEGER AS $$
DECLARE
    deleted_count INTEGER;
BEGIN
    -- 删除过期的注册令牌
    DELETE FROM device_registration_tokens
    WHERE expires_at < NOW() AND used = FALSE;

    GET DIAGNOSTICS deleted_count = ROW_COUNT;

    -- 将对应的设备状态更新为 registration_expired
    UPDATE devices
    SET status = 'registration_expired'
    WHERE status = 'pending'
    AND registration_token IN (
        SELECT registration_token FROM device_registration_tokens
        WHERE expires_at < NOW() AND used = FALSE
    );

    RETURN deleted_count;
END;
$$ LANGUAGE plpgsql;

-- 创建触发器：记录设备注册事件
CREATE OR REPLACE FUNCTION log_device_registration_event()
RETURNS TRIGGER AS $$
BEGIN
    IF TG_OP = 'INSERT' THEN
        -- 设备创建事件
        INSERT INTO device_registration_events (device_id, event_type, event_description)
        VALUES (NEW.id, 'created', 'Device created with registration pending status');
        RETURN NEW;
    ELSIF TG_OP = 'UPDATE' THEN
        -- 状态变更事件
        IF OLD.status != NEW.status THEN
            INSERT INTO device_registration_events (device_id, event_type, event_description)
            VALUES (NEW.id,
                CASE NEW.status
                    WHEN 'online' THEN 'verified'
                    WHEN 'registration_expired' THEN 'expired'
                    ELSE 'failed'
                END,
                format('Device status changed from %s to %s', OLD.status, NEW.status)
            );
        END IF;
        RETURN NEW;
    END IF;
    RETURN NULL;
END;
$$ LANGUAGE plpgsql;

-- 添加触发器
DROP TRIGGER IF EXISTS device_registration_events_trigger ON devices;
CREATE TRIGGER device_registration_events_trigger
    AFTER INSERT OR UPDATE ON devices
    FOR EACH ROW
    EXECUTE FUNCTION log_device_registration_event();

-- 创建视图：待注册设备统计
CREATE OR REPLACE VIEW pending_registrations AS
SELECT
    d.id as device_id,
    d.name as device_name,
    d.device_type,
    d.location,
    d.pairing_code,
    d.created_at as registration_initiated,
    t.expires_at,
    t.attempts_count,
    t.max_attempts,
    CASE
        WHEN t.expires_at < NOW() THEN 'expired'
        WHEN t.attempts_count >= t.max_attempts THEN 'attempts_exceeded'
        ELSE 'pending'
    END as registration_status
FROM devices d
JOIN device_registration_tokens t ON d.registration_token = t.qr_token
WHERE d.status = 'pending'
ORDER BY d.created_at DESC;

-- 添加表注释
COMMENT ON TABLE device_registration_tokens IS '设备注册令牌表 - 管理设备注册过程中的临时令牌';
COMMENT ON TABLE device_registration_events IS '设备注册事件日志表 - 记录设备注册过程中的所有事件';
COMMENT ON VIEW pending_registrations IS '待注册设备统计视图 - 显示所有等待注册的设备及其状态';

-- 添加列注释
COMMENT ON COLUMN devices.pairing_code IS '设备配对码 - 用于手动输入注册';
COMMENT ON COLUMN devices.registered_at IS '设备注册完成时间';
COMMENT ON COLUMN devices.registration_token IS '设备注册令牌 - 用于二维码注册';
COMMENT ON COLUMN device_registration_tokens.pairing_code IS '配对码 - 6-10位大写字母数字组合';
COMMENT ON COLUMN device_registration_tokens.qr_token IS '二维码令牌 - Base64编码的64位字符串';
COMMENT ON COLUMN device_registration_tokens.expires_at IS '过期时间 - 令牌失效时间';
COMMENT ON COLUMN device_registration_tokens.max_attempts IS '最大尝试次数';
COMMENT ON COLUMN device_registration_tokens.attempts_count IS '当前尝试次数';

-- 创建定时清理任务（需要pg_cron扩展）
-- 注意：需要先安装pg_cron扩展：CREATE EXTENSION IF NOT EXISTS pg_cron;
-- SELECT cron.schedule('cleanup-expired-tokens', '*/5 * * * *', 'SELECT cleanup_expired_registration_tokens();');