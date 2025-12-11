-- @automodel
--    description: Test custom derive traits
--    expect: exactly_one
--    parameters_type: true
--    return_type: UserWithCustomDerives
--    parameters_type_derives:
--      - serde::Serialize
--      - serde::Deserialize
--    return_type_derives:
--      - serde::Serialize
--      - serde::Deserialize
--      - PartialEq
-- @end

SELECT id, name, email, age
FROM public.users
WHERE id = #{user_id}
