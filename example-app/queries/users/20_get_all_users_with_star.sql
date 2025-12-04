-- @automodel
--    description: Get all users using SELECT * to fetch all columns
--    expect: multiple
--    types:
--      users.profile: "crate::models::UserProfile"
-- @end

SELECT * 
FROM users 
ORDER BY created_at DESC
