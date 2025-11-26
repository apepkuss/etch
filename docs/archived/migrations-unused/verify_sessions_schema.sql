-- ============================================================================
-- Echo System 数据库验证脚本
-- 描述: 验证 sessions 表结构与代码模型匹配
-- 创建日期: 2025-01-19
-- ============================================================================

\echo '==================== Sessions 表结构验证 ===================='
\echo ''

-- 1. 验证所有字段存在且类型正确
\echo '1. 验证字段列表:'
SELECT
    column_name,
    data_type,
    CASE
        WHEN is_nullable = 'YES' THEN 'NULL'
        ELSE 'NOT NULL'
    END as nullable,
    column_default
FROM information_schema.columns
WHERE table_name = 'sessions'
ORDER BY ordinal_position;

\echo ''
\echo '2. 验证关键字段（与代码模型 Session 结构匹配）:'
SELECT
    CASE
        WHEN EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'sessions' AND column_name = 'id') THEN '✓'
        ELSE '✗'
    END as id,
    CASE
        WHEN EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'sessions' AND column_name = 'device_id') THEN '✓'
        ELSE '✗'
    END as device_id,
    CASE
        WHEN EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'sessions' AND column_name = 'user_id') THEN '✓'
        ELSE '✗'
    END as user_id,
    CASE
        WHEN EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'sessions' AND column_name = 'start_time') THEN '✓'
        ELSE '✗'
    END as start_time,
    CASE
        WHEN EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'sessions' AND column_name = 'end_time') THEN '✓'
        ELSE '✗'
    END as end_time,
    CASE
        WHEN EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'sessions' AND column_name = 'duration') THEN '✓'
        ELSE '✗'
    END as duration,
    CASE
        WHEN EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'sessions' AND column_name = 'transcription') THEN '✓'
        ELSE '✗'
    END as transcription,
    CASE
        WHEN EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'sessions' AND column_name = 'response') THEN '✓'
        ELSE '✗'
    END as response,
    CASE
        WHEN EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'sessions' AND column_name = 'status') THEN '✓'
        ELSE '✗'
    END as status;

\echo ''
\echo '3. 验证索引:'
SELECT indexname, indexdef
FROM pg_indexes
WHERE tablename = 'sessions'
ORDER BY indexname;

\echo ''
\echo '4. 验证约束（status 枚举值）:'
SELECT constraint_name, check_clause
FROM information_schema.check_constraints
WHERE constraint_name = 'sessions_status_check';

\echo ''
\echo '5. 验证外键关联:'
SELECT
    tc.constraint_name,
    tc.table_name,
    kcu.column_name,
    ccu.table_name AS foreign_table_name,
    ccu.column_name AS foreign_column_name
FROM information_schema.table_constraints AS tc
JOIN information_schema.key_column_usage AS kcu
    ON tc.constraint_name = kcu.constraint_name
JOIN information_schema.constraint_column_usage AS ccu
    ON ccu.constraint_name = tc.constraint_name
WHERE tc.constraint_type = 'FOREIGN KEY'
  AND tc.table_name = 'sessions';

\echo ''
\echo '6. 验证视图:'
SELECT table_name
FROM information_schema.views
WHERE table_name IN ('device_status_overview', 'daily_usage_stats');

\echo ''
\echo '==================== 验证完成 ===================='
