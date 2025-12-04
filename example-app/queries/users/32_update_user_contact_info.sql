-- @automodel
--    description: Update user contact info - reuses GetUserByIdAndEmailItem return struct as params
--    expect: exactly_one
--    parameters_type: GetUserByIdAndEmailItem
-- @end

UPDATE users 
SET name = ${name}, email = ${email} 
WHERE id = ${id} 
RETURNING id, name, email
