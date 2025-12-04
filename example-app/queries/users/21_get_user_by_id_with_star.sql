-- @automodel
--    description: Get a single user by ID using SELECT * to fetch all columns
--    expect: possible_one
--    types:
--      users.profile: "crate::models::UserProfile"
-- @end

SELECT * 
FROM users 
WHERE id = ${user_id}
