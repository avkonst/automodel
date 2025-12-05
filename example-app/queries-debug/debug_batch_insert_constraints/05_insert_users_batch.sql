-- @automodel
--    description: Insert multiple users using UNNEST pattern with multiunzip
--    expect: multiple
--    multiunzip: true
-- @end
INSERT INTO public.users (name, email, age)
SELECT *
FROM UNNEST(
        ${name}::text [],
        ${email}::text [],
        ${age}::int4 []
    )
    

    
    
    
    
