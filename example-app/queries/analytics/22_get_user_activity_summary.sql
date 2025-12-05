-- @automodel
--    description: Complex CTE query combining recent public.users with aggregate statistics
--    expect: multiple
-- @end

WITH recent_users AS (
  SELECT id, name, email, created_at,
         ROW_NUMBER() OVER (ORDER BY created_at DESC) as rank
  FROM public.users 
  WHERE created_at > NOW() - INTERVAL '30 days'
),
user_stats AS (
  SELECT 
    COUNT(*) as total_users,
    COUNT(CASE WHEN created_at > NOW() - INTERVAL '7 days' THEN 1 END) as weekly_users,
    AVG(age)::float8 as avg_age
  FROM public.users
)
SELECT 
  ru.id,
  ru.name, 
  ru.email,
  ru.created_at,
  ru.rank,
  us.total_users,
  us.weekly_users,
  us.avg_age
FROM recent_users ru
CROSS JOIN user_stats us
WHERE ru.rank <= 10
ORDER BY ru.rank
