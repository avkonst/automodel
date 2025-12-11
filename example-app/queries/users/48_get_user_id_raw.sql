-- @automodel
--    description: Test single column without return_type - should return raw i32
--    expect: exactly_one
-- @end

SELECT id
FROM public.users
WHERE email = #{email}
