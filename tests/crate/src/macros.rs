macro_rules! snapshot_jsona_syntax {
    ($file:expr, $fn_name:ident) => {
        #[test]
        fn $fn_name() {
            let content = crate::helper::include_fixtures($file);
            let parse = jsona::parser::parse(&content);
            let output = if parse.errors.is_empty() {
                jsona::syntax::stringify_syntax(0, parse.into_syntax().into()).unwrap()
            } else {
                serde_json::to_string_pretty(
                    &parse
                        .errors
                        .iter()
                        .map(|v| v.to_string())
                        .collect::<Vec<String>>(),
                )
                .unwrap()
            };
            insta::assert_snapshot!(output);
        }
    };
}

macro_rules! snapshot_jsona_parse {
    ($file:expr, $fn_name:ident) => {
        #[test]
        fn $fn_name() {
            use std::str::FromStr;
            let content = crate::helper::include_fixtures($file);
            let mapper = jsona::util::mapper::Mapper::new_utf16(&content, false);
            let output = match jsona::dom::Node::from_str(&content) {
                Ok(data) => serde_json::to_string_pretty(&data.to_plain_json()).unwrap(),
                Err(error) => {
                    serde_json::to_string_pretty(&error.to_error_objects(&mapper)).unwrap()
                }
            };
            insta::assert_snapshot!(output);
        }
    };
}

macro_rules! snapshot_jsona_format {
    ($file:expr, $fn_name:ident) => {
        #[test]
        fn $fn_name() {
            let content = crate::helper::include_fixtures($file);
            let output = jsona::formatter::format(&content, jsona::formatter::Options::default());
            insta::assert_snapshot!(output);
        }
    };
}

macro_rules! snapshot_schema_parse {
    ($file:expr, $fn_name:ident) => {
        #[test]
        fn $fn_name() {
            use std::str::FromStr;
            let content = crate::helper::include_fixtures($file);
            let mapper = jsona::util::mapper::Mapper::new_utf16(&content, false);
            let node = jsona::dom::Node::from_str(&content).unwrap();
            let output = match jsona_schema::Schema::try_from(&node) {
                Ok(schema) => serde_json::to_string_pretty(&schema).unwrap(),
                Err(errors) => {
                    let errors = errors
                        .into_iter()
                        .map(|v| v.to_error_object(&node, &mapper))
                        .collect::<Vec<_>>();
                    serde_json::to_string_pretty(&errors).unwrap()
                }
            };
            insta::assert_snapshot!(output);
        }
    };
}

macro_rules! snapshot_schema_point {
    ($file:expr, $fn_name:ident $(, $point_key:expr )+) => {
        #[test]
        fn $fn_name() {
            use std::str::FromStr;
            let content = crate::helper::include_fixtures($file);
            let node = jsona::dom::Node::from_str(&content).unwrap();
            let schema = jsona_schema::Schema::try_from(&node).unwrap();
			let mut output = indexmap::IndexMap::new();
			$(
				let keys = $point_key.parse().unwrap();
				output.insert($point_key, schema.pointer(&keys));
			)+
            insta::assert_snapshot!(serde_json::to_string_pretty(&output).unwrap());
        }
    };
}

macro_rules! snapshot_schema_validator_point {
    ($file:expr, $fn_name:ident $(, $point_key:expr )+) => {
        #[test]
        fn $fn_name() {
            use std::str::FromStr;
            let content = crate::helper::include_fixtures($file);
            let node = jsona::dom::Node::from_str(&content).unwrap();
            let validator = jsona_schema_validator::JSONASchemaValidator::try_from(&node).unwrap();
			let mut output = indexmap::IndexMap::new();
			$(
				let keys = $point_key.parse().unwrap();
				output.insert($point_key, validator.pointer(&keys));
			)+
            insta::assert_snapshot!(serde_json::to_string_pretty(&output).unwrap());
        }
    };
}
