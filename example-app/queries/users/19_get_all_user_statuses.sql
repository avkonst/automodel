-- @automodel
--    description: Get all possible user statuses currently in use
--    expect: multiple
-- @end

SELECT DISTINCT status 
FROM public.users 
ORDER BY status
