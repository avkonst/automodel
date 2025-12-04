-- @automodel
--    description: Get users created after a specific timestamp - expects at least one user
--    expect: at_least_one
--    types:
--      users.profile: "crate::models::UserProfile"
-- @end

SELECT id, name, email, age, profile, created_at, updated_at 
FROM users 
WHERE created_at > ${since} 
ORDER BY created_at DESC
