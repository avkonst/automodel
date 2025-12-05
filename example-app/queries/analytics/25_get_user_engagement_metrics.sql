-- @automodel
--    description: Complex multi-CTE query calculating user engagement metrics with temporal analysis
--    expect: multiple
-- @end

WITH user_activity AS (
  SELECT 
    u.id,
    u.name,
    u.email,
    u.created_at,
    COUNT(DISTINCT p.id) as post_count,
    COUNT(DISTINCT c.id) as comment_count,
    MAX(p.created_at) as last_post_date,
    MAX(c.created_at) as last_comment_date,
    AVG(EXTRACT(EPOCH FROM (p.published_at - p.created_at))::float8/3600) as avg_publish_delay_hours
  FROM public.users u
  LEFT JOIN public.posts p ON u.id = p.author_id 
    AND p.created_at >= DATE_TRUNC('month', NOW()) - INTERVAL '3 months'
  LEFT JOIN public.comments c ON u.id = c.author_id 
    AND c.created_at >= DATE_TRUNC('month', NOW()) - INTERVAL '3 months'
  GROUP BY u.id, u.name, u.email, u.created_at
),
engagement_scores AS (
  SELECT 
    *,
    (post_count * 3 + comment_count) as engagement_score,
    CASE 
      WHEN last_post_date > NOW() - INTERVAL '7 days' OR 
           last_comment_date > NOW() - INTERVAL '7 days' THEN 'active'
      WHEN last_post_date > NOW() - INTERVAL '30 days' OR 
           last_comment_date > NOW() - INTERVAL '30 days' THEN 'semi_active'
      ELSE 'inactive'
    END as activity_status,
    EXTRACT(EPOCH FROM (NOW() - GREATEST(
      COALESCE(last_post_date, '1970-01-01'::timestamp), 
      COALESCE(last_comment_date, '1970-01-01'::timestamp)
    )))::float8/86400 as days_since_last_activity
  FROM user_activity
)
SELECT 
  es.*,
  RANK() OVER (ORDER BY engagement_score DESC) as engagement_rank,
  PERCENT_RANK() OVER (ORDER BY engagement_score) as engagement_percentile
FROM engagement_scores es
WHERE engagement_score > #{min_engagement_score}
ORDER BY engagement_score DESC, name
LIMIT #{limit_results}
