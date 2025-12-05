-- @automodel
--    description: Get user details with age and created_at - generates UserDetails return struct
--    expect: exactly_one
--    return_type: UserDetails
-- @end

SELECT id, name, email, age, created_at 
FROM public.users 
WHERE id = ${user_id}
