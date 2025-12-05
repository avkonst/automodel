-- @automodel
--    description: Partial update using diff-based comparison - auto-generates params struct for old/new comparison
--    expect: exactly_one
--    conditions_type: UserModel
--    return_type: UserModel
--    error_type: UserContentConstraints
-- @end

UPDATE public.users 
SET updated_at = NOW() 
#[, name = #{name?}] 
#[, email = #{email?}] 
#[, age = #{age?}] 
WHERE id = #{id} 
RETURNING id, name, email, age
