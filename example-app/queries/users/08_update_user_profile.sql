-- @automodel
--    description: Update a user's profile by their ID
--    expect: exactly_one
--    types:
--      public.users.profile: "crate::models::UserProfile"
--      profile: "crate::models::UserProfile"
--    telemetry:
--      include_params: []
-- @end

UPDATE public.users 
SET profile = ${profile}, updated_at = NOW() 
WHERE id = ${user_id} 
RETURNING id, name, email, age, profile, updated_at
