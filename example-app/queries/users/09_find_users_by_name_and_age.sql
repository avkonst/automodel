-- @automodel
--    description: Find users by name pattern with optional minimum age filter (using conditional syntax)
--    expect: multiple
-- @end

SELECT id, name, email, age 
FROM users 
WHERE name ILIKE ${name_pattern} 
$[AND age >= ${min_age?}] 
AND name = ${name_exact} 
$[AND age <= ${max_age?}] 
ORDER BY name
