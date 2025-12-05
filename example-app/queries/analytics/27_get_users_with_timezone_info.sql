-- @automodel
--    description: Users with comprehensive timezone and temporal information
--    expect: multiple
-- @end

SELECT 
  id,
  name,
  email,
  created_at,
  created_at AT TIME ZONE 'UTC' AT TIME ZONE ${user_timezone} as created_at_user_tz,
  updated_at,
  updated_at AT TIME ZONE 'UTC' AT TIME ZONE ${user_timezone} as updated_at_user_tz,
  AGE(NOW(), created_at) as account_age,
  EXTRACT(EPOCH FROM AGE(NOW(), created_at))/86400 as account_age_days,
  DATE_PART('dow', created_at) as created_day_of_week,
  DATE_PART('hour', created_at) as created_hour,
  TO_CHAR(created_at, 'Day, Month DD, YYYY at HH24:MI:SS TZ') as formatted_created_at
FROM public.users 
WHERE created_at BETWEEN ${start_date} AND ${end_date}
  AND EXTRACT(EPOCH FROM AGE(NOW(), created_at))/86400 BETWEEN ${min_age_days} AND ${max_age_days}
ORDER BY created_at DESC
