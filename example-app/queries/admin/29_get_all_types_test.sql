-- @automodel
--    description: Get a row with all PostgreSQL types by ID
--    expect: exactly_one
-- @end

SELECT
  id, bool_col, char_col, int2_col, int4_col, int8_col, float4_col, float8_col, numeric_col,
  name_col, text_col, varchar_col, bpchar_col, bytea_col, bit_col, varbit_col,
  date_col, time_col, timestamp_col, timestamptz_col, interval_col, timetz_col,
  int4_range_col, int8_range_col, num_range_col, ts_range_col, tstz_range_col, date_range_col,
  inet_col, cidr_col, macaddr_col, json_col, jsonb_col, uuid_col,
  bool_array_col, int4_array_col, int8_array_col, text_array_col, float8_array_col,
  int4_range_array_col, date_range_array_col,
  created_at
FROM all_types_test
WHERE id = ${id}
