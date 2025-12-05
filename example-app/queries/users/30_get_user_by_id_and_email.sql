-- @automodel
--    description: Get a user by ID and email - generates GetUserByIdAndEmailParams struct and GetUserByIdAndEmailItem return struct
--    expect: possible_one
--    parameters_type: true
-- @end

SELECT id, name, email 
FROM public.users 
WHERE id = #{id} AND email = #{email}
