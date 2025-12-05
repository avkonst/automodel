-- @automodel
--    description: Delete user by ID and email - reuses GetUserByIdAndEmailParams struct
--    expect: exactly_one
--    parameters_type: GetUserByIdAndEmailParams
-- @end

DELETE FROM public.users 
WHERE id = ${id} AND email = ${email} 
RETURNING id, email
