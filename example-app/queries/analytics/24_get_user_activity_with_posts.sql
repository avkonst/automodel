-- @automodel
--    description: Complex JOIN query with temporal filtering across multiple tables
--    expect: multiple
-- @end

SELECT 
  u.id as user_id,
  u.name,
  u.email,
  u.created_at as user_created_at,
  u.updated_at as user_updated_at,
  p.id as post_id,
  p.title,
  p.content,
  p.created_at as post_created_at,
  p.published_at,
  c.comment_count,
  EXTRACT(EPOCH FROM (NOW() - p.created_at))::float8/3600 as hours_since_post,
  DATE_TRUNC('day', p.created_at) as post_date
FROM public.users u
INNER JOIN public.posts p ON u.id = p.author_id
LEFT JOIN (
  SELECT post_id, COUNT(*) as comment_count
  FROM public.comments 
  GROUP BY post_id
) c ON p.id = c.post_id
WHERE u.created_at > #{since}
  AND p.published_at IS NOT NULL
  AND p.created_at BETWEEN #{start_date} AND #{end_date}
ORDER BY p.created_at DESC, u.name
