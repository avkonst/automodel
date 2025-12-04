-- @automodel
--    description: Get PostgreSQL version
--    expect: exactly_one
-- @end

SELECT version() as pg_version
