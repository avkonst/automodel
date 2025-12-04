-- @automodel
--    description: Simple user lookup by ID with detailed info
--    expect: possible_one
-- @end

SELECT id, name, email, created_at
FROM users
WHERE id = ${user_id}
