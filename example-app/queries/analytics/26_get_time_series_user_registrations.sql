-- @automodel
--    description: Time series analysis of user registrations with age demographics
--    expect: multiple
-- @end

WITH time_series AS (
  SELECT 
    DATE_TRUNC('day', created_at) as period_start,
    COUNT(*) as registrations_count,
    COUNT(*) FILTER (WHERE age BETWEEN 18 AND 30) as young_adult_count,
    COUNT(*) FILTER (WHERE age BETWEEN 31 AND 50) as middle_aged_count, 
    COUNT(*) FILTER (WHERE age > 50) as senior_count,
    AVG(age) as avg_age,
    MIN(created_at) as first_registration,
    MAX(created_at) as last_registration
  FROM users
  WHERE created_at BETWEEN ${start_date} AND ${end_date}
  GROUP BY DATE_TRUNC('day', created_at)
  HAVING COUNT(*) >= ${min_registrations}
)
SELECT 
  *,
  EXTRACT(EPOCH FROM (last_registration - first_registration))::float8/3600 as period_span_hours
FROM time_series
ORDER BY period_start DESC
