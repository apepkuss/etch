-- 修复同一设备时间重叠的会话问题
-- 问题: 测试数据中同一设备在同一时间段有多个会话
-- 解决方案: 对于每个设备,按开始时间排序,调整会话的结束时间,确保不重叠

-- 步骤 1: 为每个设备的会话按开始时间排序并编号
WITH ranked_sessions AS (
  SELECT
    id,
    device_id,
    start_time,
    end_time,
    status,
    ROW_NUMBER() OVER (PARTITION BY device_id ORDER BY start_time) as rn
  FROM sessions
),
-- 步骤 2: 获取每个会话及其下一个会话的开始时间
sessions_with_next AS (
  SELECT
    s1.id,
    s1.device_id,
    s1.start_time,
    s1.end_time,
    s1.status,
    s2.start_time as next_start_time
  FROM ranked_sessions s1
  LEFT JOIN ranked_sessions s2
    ON s1.device_id = s2.device_id
    AND s2.rn = s1.rn + 1
),
-- 步骤 3: 计算需要更新的会话
sessions_to_update AS (
  SELECT
    id,
    device_id,
    start_time,
    end_time,
    status,
    next_start_time,
    CASE
      -- 如果有结束时间且与下一个会话重叠,则调整为下一个会话开始前1秒
      WHEN end_time IS NOT NULL
           AND next_start_time IS NOT NULL
           AND end_time > next_start_time
      THEN next_start_time - INTERVAL '1 second'
      -- 如果是 active 状态但有下一个会话,则设置结束时间
      WHEN status = 'active'
           AND next_start_time IS NOT NULL
      THEN next_start_time - INTERVAL '1 second'
      -- 否则保持原样
      ELSE end_time
    END as new_end_time,
    CASE
      -- 如果是 active 但需要设置结束时间,改为 completed
      WHEN status = 'active' AND next_start_time IS NOT NULL
      THEN 'completed'
      ELSE status
    END as new_status
  FROM sessions_with_next
  WHERE
    -- 只更新需要调整的记录
    (end_time IS NOT NULL AND next_start_time IS NOT NULL AND end_time > next_start_time)
    OR (status = 'active' AND next_start_time IS NOT NULL)
)
-- 步骤 4: 执行更新
UPDATE sessions
SET
  end_time = s.new_end_time,
  status = s.new_status,
  duration = EXTRACT(EPOCH FROM (s.new_end_time - sessions.start_time))::INTEGER
FROM sessions_to_update s
WHERE sessions.id = s.id
  AND (sessions.end_time IS DISTINCT FROM s.new_end_time
       OR sessions.status IS DISTINCT FROM s.new_status);

-- 验证: 检查是否还有重叠
DO $$
DECLARE
  overlap_count INTEGER;
BEGIN
  SELECT COUNT(*) INTO overlap_count
  FROM sessions s1
  JOIN sessions s2 ON s1.device_id = s2.device_id AND s1.id < s2.id
  WHERE s2.start_time < COALESCE(s1.end_time, NOW())
    AND COALESCE(s2.end_time, NOW()) > s1.start_time;

  IF overlap_count > 0 THEN
    RAISE NOTICE 'Warning: Still found % overlapping sessions after fix', overlap_count;
  ELSE
    RAISE NOTICE 'Success: No overlapping sessions found';
  END IF;
END $$;

-- 显示修复统计
SELECT
  'Total sessions' as metric,
  COUNT(*) as count
FROM sessions
UNION ALL
SELECT
  'Sessions per device' as metric,
  COUNT(*)::NUMERIC / COUNT(DISTINCT device_id) as count
FROM sessions
UNION ALL
SELECT
  'Devices' as metric,
  COUNT(DISTINCT device_id) as count
FROM sessions;
