-- 创建 echokit_servers 表
-- 用于存储用户配置的 EchoKit Server 列表

CREATE TABLE IF NOT EXISTS echokit_servers (
    id SERIAL PRIMARY KEY,
    user_id VARCHAR(255) NOT NULL,              -- 用户ID（关联 users 表）
    server_url VARCHAR(512) NOT NULL,           -- WebSocket URL
    status VARCHAR(20) DEFAULT 'unavailable',   -- 服务器状态：available, unavailable
    last_checked_at TIMESTAMPTZ,                -- 最近检查时间
    created_at TIMESTAMPTZ DEFAULT NOW(),       -- 创建时间
    updated_at TIMESTAMPTZ DEFAULT NOW(),       -- 更新时间

    -- 约束：同一用户不能添加重复的服务器 URL
    CONSTRAINT unique_user_server_url UNIQUE (user_id, server_url)
);

-- 创建索引以提高查询性能
CREATE INDEX IF NOT EXISTS idx_echokit_servers_user_id ON echokit_servers(user_id);
CREATE INDEX IF NOT EXISTS idx_echokit_servers_status ON echokit_servers(status);
CREATE INDEX IF NOT EXISTS idx_echokit_servers_created_at ON echokit_servers(created_at DESC);

-- 添加注释
COMMENT ON TABLE echokit_servers IS 'EchoKit Server 配置表，存储用户自定义的服务器列表';
COMMENT ON COLUMN echokit_servers.id IS '主键ID';
COMMENT ON COLUMN echokit_servers.user_id IS '用户ID，关联用户表';
COMMENT ON COLUMN echokit_servers.server_url IS 'WebSocket 服务器URL';
COMMENT ON COLUMN echokit_servers.status IS '服务器状态：available（可用）, unavailable（不可用）';
COMMENT ON COLUMN echokit_servers.last_checked_at IS '最近一次状态检查时间';
COMMENT ON COLUMN echokit_servers.created_at IS '记录创建时间';
COMMENT ON COLUMN echokit_servers.updated_at IS '记录更新时间';

-- 插入默认服务器（可选，用于演示）
-- 注意：这里使用的 user_id = 'system' 作为系统默认服务器
-- 实际使用时可以删除或替换为真实用户ID
INSERT INTO echokit_servers (user_id, server_url, status, last_checked_at)
VALUES
    ('system', 'wss://indie.echokit.dev', 'available', NOW())
ON CONFLICT (user_id, server_url) DO NOTHING;
