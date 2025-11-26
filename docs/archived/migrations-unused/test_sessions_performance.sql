-- ============================================================================
-- Echo System 性能测试脚本
-- 描述: 测试 sessions 表的索引性能
-- 创建日期: 2025-01-19
-- ============================================================================

\echo '==================== Sessions 表性能测试 ===================='
\echo ''

-- 1. 生成测试数据（1000 条会话记录）
\echo '1. 生成测试数据（1000 条会话）...'
DO $$
DECLARE
    device_ids VARCHAR[] := ARRAY(SELECT id FROM devices LIMIT 5);
    device_count INTEGER := array_length(device_ids, 1);
    session_statuses VARCHAR[] := ARRAY['active', 'completed', 'failed', 'timeout'];
    i INTEGER;
BEGIN
    IF device_count IS NULL OR device_count = 0 THEN
        RAISE EXCEPTION '没有可用的设备，请先创建测试设备';
    END IF;

    FOR i IN 1..1000 LOOP
        INSERT INTO sessions (
            id,
            device_id,
            user_id,
            session_type,
            status,
            transcription,
            response,
            duration,
            start_time,
            end_time
        ) VALUES (
            'test_session_' || i,
            device_ids[1 + (i % device_count)],
            'test_user_' || (i % 10),
            'voice',
            session_statuses[1 + (i % 4)],
            '测试转录内容 ' || i || '：今天天气怎么样？',
            '测试响应内容 ' || i || '：今天天气晴朗，温度适宜。',
            floor(random() * 300)::INTEGER,
            NOW() - (random() * INTERVAL '30 days'),
            CASE
                WHEN (i % 4) != 0 THEN NOW() - (random() * INTERVAL '29 days')
                ELSE NULL
            END
        );
    END LOOP;

    RAISE NOTICE '成功生成 1000 条测试会话记录';
END $$;

\echo ''
\echo '2. 验证数据量:'
SELECT
    COUNT(*) as total_sessions,
    COUNT(*) FILTER (WHERE status = 'active') as active_sessions,
    COUNT(*) FILTER (WHERE status = 'completed') as completed_sessions,
    COUNT(*) FILTER (WHERE status = 'failed') as failed_sessions,
    COUNT(*) FILTER (WHERE status = 'timeout') as timeout_sessions
FROM sessions;

\echo ''
\echo '3. 测试查询性能（带 EXPLAIN ANALYZE）:'

\echo ''
\echo '3.1 测试单设备查询（使用索引 idx_sessions_device_id）:'
EXPLAIN ANALYZE
SELECT id, device_id, start_time, status, transcription, response
FROM sessions
WHERE device_id = (SELECT id FROM devices LIMIT 1)
ORDER BY start_time DESC
LIMIT 20;

\echo ''
\echo '3.2 测试按状态过滤（使用索引 idx_sessions_status）:'
EXPLAIN ANALYZE
SELECT id, device_id, start_time, status, transcription
FROM sessions
WHERE status = 'completed'
ORDER BY start_time DESC
LIMIT 20;

\echo ''
\echo '3.3 测试设备+状态组合查询（使用复合索引 idx_sessions_device_status）:'
EXPLAIN ANALYZE
SELECT id, device_id, start_time, status, transcription, response
FROM sessions
WHERE device_id = (SELECT id FROM devices LIMIT 1)
  AND status = 'completed'
ORDER BY start_time DESC
LIMIT 20;

\echo ''
\echo '3.4 测试时间范围查询（使用索引 idx_sessions_start_time）:'
EXPLAIN ANALYZE
SELECT id, device_id, start_time, status, transcription
FROM sessions
WHERE start_time >= NOW() - INTERVAL '7 days'
ORDER BY start_time DESC
LIMIT 20;

\echo ''
\echo '3.5 测试时间+状态组合查询（使用复合索引 idx_sessions_start_time_status）:'
EXPLAIN ANALYZE
SELECT id, device_id, start_time, status, transcription, response
FROM sessions
WHERE start_time >= NOW() - INTERVAL '7 days'
  AND status = 'completed'
ORDER BY start_time DESC
LIMIT 20;

\echo ''
\echo '3.6 测试统计聚合查询（FILTER 优化）:'
EXPLAIN ANALYZE
SELECT
    COUNT(*) as total,
    COUNT(*) FILTER (WHERE status = 'active') as active,
    COUNT(*) FILTER (WHERE status = 'completed') as completed,
    COUNT(*) FILTER (WHERE status = 'failed') as failed,
    COUNT(*) FILTER (WHERE status = 'timeout') as timeout,
    AVG(duration) FILTER (WHERE status = 'completed') as avg_duration,
    COUNT(*) FILTER (WHERE DATE(start_time) = CURRENT_DATE) as today_sessions
FROM sessions;

\echo ''
\echo '4. 索引使用情况统计:'
SELECT
    schemaname,
    tablename,
    indexname,
    idx_scan as index_scans,
    idx_tup_read as tuples_read,
    idx_tup_fetch as tuples_fetched
FROM pg_stat_user_indexes
WHERE tablename = 'sessions'
ORDER BY idx_scan DESC;

\echo ''
\echo '5. 表大小统计:'
SELECT
    pg_size_pretty(pg_total_relation_size('sessions')) as total_size,
    pg_size_pretty(pg_relation_size('sessions')) as table_size,
    pg_size_pretty(pg_total_relation_size('sessions') - pg_relation_size('sessions')) as indexes_size;

\echo ''
\echo '==================== 性能测试完成 ===================='
\echo ''
\echo '提示: 如需清理测试数据，执行以下命令:'
\echo "DELETE FROM sessions WHERE id LIKE 'test_session_%';"
