-- @automodel
--    description: Update user profile with conditional name/email - generates UpdateUserProfileDiffParams
--    expect: exactly_one
--    conditions_type: true
--    types:
--      public.users.profile: "crate::models::UserProfile"
--      profile: "crate::models::UserProfile"
-- @end

UPDATE public.users 
SET profile = #{profile}, updated_at = NOW() 
#[, name = #{name?}] 
#[, email = #{email?}] 
WHERE id = #{user_id} 
RETURNING id, name, email, profile, updated_at
