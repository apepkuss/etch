-- 移除 status 和 last_checked_at 字段,因为状态应该是动态检测的,不需要持久化
ALTER TABLE echokit_servers DROP COLUMN IF EXISTS status;
ALTER TABLE echokit_servers DROP COLUMN IF EXISTS last_checked_at;

-- 移除相关索引
DROP INDEX IF EXISTS idx_echokit_servers_status;
