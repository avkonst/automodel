-- @automodel
--    description: Select user by email - returns UserModel
--    expect: possible_one
--    return_type: UserModel
-- @end

SELECT id, name, email, age 
FROM public.users 
WHERE email = #{email}
