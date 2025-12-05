-- @automodel
--    description: Get public.users created after a specific timestamp - expects at least one user
--    expect: at_least_one
--    types:
--      public.users.profile: "crate::models::UserProfile"
-- @end

SELECT id, name, email, age, profile, created_at, updated_at 
FROM public.users 
WHERE created_at > #{since} 
ORDER BY created_at DESC
