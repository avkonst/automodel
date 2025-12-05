-- @automodel
--    description: Update user status and return the new status
--    expect: exactly_one
-- @end

UPDATE public.users 
SET status = ${new_status} 
WHERE id = ${user_id} 
RETURNING id, status
