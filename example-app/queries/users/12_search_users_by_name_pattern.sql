-- @automodel
--    description: Search users by name pattern - expects at least one match
--    expect: at_least_one
-- @end

SELECT id, name, email 
FROM users 
WHERE name ILIKE ${pattern} 
ORDER BY name
