-- @automodel
--    description: Get all public.users with all their fields
--    expect: multiple
--    types:
--      public.users.profile: "crate::models::UserProfile"
--    ensure_indexes: true
-- @end

SELECT id, name, email, age, profile, created_at, updated_at 
FROM public.users 
ORDER BY created_at DESC
