-- @automodel
--    description: Update user fields conditionally - only updates fields that are provided (not None)
--    expect: exactly_one
-- @end

UPDATE users 
SET updated_at = NOW() 
$[, name = ${name?}] 
$[, email = ${email?}] 
$[, age = ${age?}] 
WHERE id = ${user_id} 
RETURNING id, name, email, age, updated_at
