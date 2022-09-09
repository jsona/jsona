snapshot_schema_parse!("schema.jsona", parse_spec);
snapshot_schema_point!(
    "schema.jsona",
    point_spec,
    "_.value.bool",
    "_.value.array",
    "_.value.object2.k1",
    "null.value",
    "object.value.k2"
);
