-- @automodel
--    description: Find user by criteria - uses GetUserByIdAndEmailParams for params and UserSummary for return
--    expect: possible_one
--    parameters_type: GetUserByIdAndEmailParams
--    return_type: UserSummary
-- @end

SELECT id, name, email 
FROM public.users 
WHERE id = #{id} AND email = #{email}
