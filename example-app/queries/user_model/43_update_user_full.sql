-- @automodel
--    description: Full update of user - reuses UserModel for both parameters and return type
--    expect: exactly_one
--    parameters_type: UserModel
--    return_type: UserModel
--    error_type: UserContentConstraints
-- @end

UPDATE public.users 
SET name = #{name}, email = #{email}, age = #{age?} 
WHERE id = #{id} 
RETURNING id, name, email, age
