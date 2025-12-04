-- @automodel
--    description: Get all users with all their fields
--    expect: multiple
--    types:
--      users.profile: "crate::models::UserProfile"
--    ensure_indexes: true
-- @end

SELECT id, name, email, age, profile, created_at, updated_at 
FROM users 
ORDER BY created_at DESC
