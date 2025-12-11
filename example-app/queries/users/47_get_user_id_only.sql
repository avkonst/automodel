-- @automodel
--    description: Test single column with explicit return_type - should generate UserId struct
--    expect: exactly_one
--    return_type: UserId
--    return_type_derives:
--      - serde::Serialize
--      - serde::Deserialize
--      - PartialEq
--      - Eq
-- @end

SELECT id
FROM public.users
WHERE email = #{email}
