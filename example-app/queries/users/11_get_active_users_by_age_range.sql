-- @automodel
--    description: Get active public.users within an age range - must return at least one user or fails
--    expect: at_least_one
--    types:
--      public.users.profile: "crate::models::UserProfile"
-- @end

SELECT id, name, email, age, profile, created_at 
FROM public.users 
WHERE age BETWEEN ${min_age} AND ${max_age} 
AND updated_at > NOW() - INTERVAL '30 days'
