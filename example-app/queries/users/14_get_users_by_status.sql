-- @automodel
--    description: Get public.users by their status (enum parameter and enum output)
--    expect: multiple
-- @end

SELECT id, name, email, status 
FROM public.users 
WHERE status = #{user_status} 
ORDER BY name
