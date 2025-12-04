-- @automodel
--    description: Advanced user search with multiple optional filters using conditional syntax
--    expect: multiple
-- @end

SELECT id, name, email, age, created_at 
FROM users 
WHERE 1=1 
$[AND name ILIKE ${name_pattern?}] 
$[AND age >= ${min_age?}] 
$[AND created_at >= ${since?}] 
ORDER BY created_at DESC
