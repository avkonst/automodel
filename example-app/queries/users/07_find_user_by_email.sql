-- @automodel
--    description: Find a user by their email address
--    expect: possible_one
--    types:
--      profile: "crate::models::UserProfile"
--    telemetry:
--      include_params: [email]
--      include_sql: false
--    ensure_indexes: false
-- @end

SELECT id, name, email, age, profile, created_at, updated_at 
FROM public.users 
WHERE email = #{email}
