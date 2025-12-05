-- @automodel
--    description: Update user metadata - reuses UpdateUserProfileDiffParams struct
--    expect: exactly_one
--    conditions_type: UpdateUserProfileDiffParams
--    types:
--      profile: "crate::models::UserProfile"
-- @end

UPDATE public.users 
SET profile = ${profile}, updated_at = NOW() 
$[, name = ${name?}] 
$[, email = ${email?}] 
WHERE id = ${user_id} 
RETURNING id, name, email, updated_at
