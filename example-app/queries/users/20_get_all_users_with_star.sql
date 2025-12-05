-- @automodel
--    description: Get all public.users using SELECT * to fetch all columns
--    expect: multiple
--    types:
--      public.users.profile: "crate::models::UserProfile"
-- @end

SELECT * 
FROM public.users 
ORDER BY created_at DESC
