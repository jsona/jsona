use std::{
    cmp::Ordering,
    collections::HashSet,
    fmt::{Display, Formatter},
};

use either::Either;
use fancy_regex::Regex;
use indexmap::IndexMap;
use jsona::{
    dom::{KeyOrIndex, Keys, Node},
    error::ErrorObject,
    util::mapper::Mapper,
};
use jsona_schema::{Schema, SchemaType, REF_REGEX};
use once_cell::sync::Lazy;
use std::ops::Index;

const ERROR_SOURCE: &str = "validator";

static TIME_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^([01][0-9]|2[0-3]):([0-5][0-9]):([0-5][0-9])(\.[0-9]{6})?(([Zz])|([+|\-]([01][0-9]|2[0-3]):[0-5][0-9]))\z").unwrap()
});
static CONTROL_GROUPS_RE: Lazy<regex::Regex> =
    Lazy::new(|| regex::Regex::new(r"\\c[A-Za-z]").unwrap());
static UUID_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"^[0-9a-fA-F]{8}\b-[0-9a-fA-F]{4}\b-[0-9a-fA-F]{4}\b-[0-9a-fA-F]{4}\b-[0-9a-fA-F]{12}$",
    )
    .unwrap()
});

pub fn validate(
    defs: &IndexMap<String, Schema>,
    schema: &Schema,
    keys: &Keys,
    node: &Node,
) -> Vec<Error> {
    let mut errors = vec![];
    if let Some(schema) = resolve(defs, schema) {
        validate_impl(&mut errors, defs, schema, keys, node);
    }
    errors
}

fn validate_impl(
    errors: &mut Vec<Error>,
    defs: &IndexMap<String, Schema>,
    local_schema: &Schema,
    keys: &Keys,
    node: &Node,
) {
    let local_schema = match resolve(defs, local_schema) {
        Some(v) => v,
        None => return,
    };
    validate_type(errors, defs, local_schema, keys, node);
    validate_enum(errors, defs, local_schema, keys, node);
    validate_const(errors, defs, local_schema, keys, node);

    if node.is_object() {
        validate_properties(errors, defs, local_schema, keys, node);
        validate_required(errors, defs, local_schema, keys, node);
        validate_maxmin_properties(errors, defs, local_schema, keys, node);
    }

    if node.is_array() {
        validate_items(errors, defs, local_schema, keys, node);
        validate_contains(errors, defs, local_schema, keys, node);
        validate_maxmin_items(errors, defs, local_schema, keys, node);
        validate_unique_items(errors, defs, local_schema, keys, node);
    }

    if node.is_string() {
        validate_pattern(errors, defs, local_schema, keys, node);
        validate_maxmin_length(errors, defs, local_schema, keys, node);
        validate_format(errors, defs, local_schema, keys, node);
    }

    if node.is_number() {
        validate_maxmin(errors, defs, local_schema, keys, node);
        validate_multiple_of(errors, defs, local_schema, keys, node);
    }

    validate_allof(errors, defs, local_schema, keys, node);
    validate_anyof(errors, defs, local_schema, keys, node);
    validate_oneof(errors, defs, local_schema, keys, node);
    validate_not(errors, defs, local_schema, keys, node);
    validate_condiational(errors, defs, local_schema, keys, node);
}

fn validate_type(
    errors: &mut Vec<Error>,
    _defs: &IndexMap<String, Schema>,
    local_schema: &Schema,
    keys: &Keys,
    node: &Node,
) {
    let types = local_schema.types();
    if types.is_empty() {
        return;
    }
    let mut is_type_match = false;
    for schema_type in types.iter() {
        if schema_type.match_node(node) {
            is_type_match = true;
            break;
        }
    }
    if !is_type_match {
        errors.push(Error::new(
            keys,
            ErrorKind::Type {
                types: types.into_iter().collect(),
            },
        ));
    }
}

fn validate_enum(
    errors: &mut Vec<Error>,
    _defs: &IndexMap<String, Schema>,
    local_schema: &Schema,
    keys: &Keys,
    node: &Node,
) {
    if let Some(enum_items) = local_schema.enum_value.as_ref() {
        let value = node.to_plain_json();
        let mut contains = false;
        for enum_value in enum_items.iter() {
            if is_matching(&value, enum_value) {
                contains = true;
                break;
            }
        }
        if !contains {
            errors.push(Error::new(keys, ErrorKind::Enum));
        }
    }
}

fn validate_const(
    errors: &mut Vec<Error>,
    _defs: &IndexMap<String, Schema>,
    local_schema: &Schema,
    keys: &Keys,
    node: &Node,
) {
    if let Some(const_value) = local_schema.const_value.as_ref() {
        let value = node.to_plain_json();
        if !is_matching(&value, const_value) {
            errors.push(Error::new(keys, ErrorKind::Const));
        }
    }
}

fn validate_properties(
    errors: &mut Vec<Error>,
    defs: &IndexMap<String, Schema>,
    local_schema: &Schema,
    keys: &Keys,
    node: &Node,
) {
    let object = match node.as_object() {
        Some(v) => v,
        None => return,
    };
    for (key, value) in object.value().read().iter() {
        let new_keys = keys.join(KeyOrIndex::property(key.value()));
        let is_property_passed = if let Some(schema) = local_schema
            .properties
            .as_ref()
            .and_then(|v| v.get(key.value()))
        {
            validate_impl(errors, defs, schema, &new_keys, value);
            true
        } else {
            false
        };
        let mut is_pattern_passed = false;
        if let Some(patterns) = local_schema.pattern_properties.as_ref() {
            for (pat, schema) in patterns.iter() {
                if let Ok(re) = Regex::new(pat) {
                    if let Ok(true) = re.is_match(key.value()) {
                        validate_impl(errors, defs, schema, &new_keys, value);
                        is_pattern_passed = true;
                    }
                }
            }
        }
        if is_property_passed || is_pattern_passed {
            continue;
        }

        if let Some(additional_properties) = local_schema.additional_properties.as_ref() {
            match additional_properties.value.as_ref() {
                Either::Left(allowed) => {
                    if !allowed {
                        errors.push(Error::new(
                            keys,
                            ErrorKind::AdditionalProperties {
                                key: key.value().to_string(),
                            },
                        ));
                    }
                }
                Either::Right(schema) => validate_impl(errors, defs, schema, &new_keys, value),
            }
        }
    }
}

fn validate_required(
    errors: &mut Vec<Error>,
    _defs: &IndexMap<String, Schema>,
    local_schema: &Schema,
    keys: &Keys,
    node: &Node,
) {
    if let Some(required) = local_schema.required.as_ref() {
        let object = match node.as_object() {
            Some(v) => v,
            None => return,
        };
        let mut miss: Vec<String> = vec![];
        let map = object.value().read();
        let object_keys: HashSet<&str> = map.iter().map(|(k, _)| k.value()).collect();
        for key in required.iter() {
            if object_keys.contains(key.as_str()) {
                continue;
            }
            miss.push(key.to_string());
        }
        if !miss.is_empty() {
            errors.push(Error::new(keys, ErrorKind::Required { keys: miss }));
        }
    }
}

fn validate_maxmin_properties(
    errors: &mut Vec<Error>,
    _defs: &IndexMap<String, Schema>,
    local_schema: &Schema,
    keys: &Keys,
    node: &Node,
) {
    let object = match node.as_object() {
        Some(v) => v,
        None => return,
    };
    let len = object.value().read().len();
    if let Some(max_properties) = local_schema.max_properties.as_ref() {
        if len > *max_properties as usize {
            errors.push(Error::new(keys, ErrorKind::MaxProperties));
        }
    }
    if let Some(min_properties) = local_schema.min_properties.as_ref() {
        if len < *min_properties as usize {
            errors.push(Error::new(keys, ErrorKind::MinProperties));
        }
    }
}

fn validate_items(
    errors: &mut Vec<Error>,
    defs: &IndexMap<String, Schema>,
    local_schema: &Schema,
    keys: &Keys,
    node: &Node,
) {
    if let Some(items) = local_schema.items.as_ref() {
        let array = match node.as_array() {
            Some(v) => v,
            None => return,
        };
        match items.value.as_ref() {
            Either::Left(schema) => {
                for (idx, value) in array.value().read().iter().enumerate() {
                    let new_keys = keys.join(KeyOrIndex::Index(idx));
                    validate_impl(errors, defs, schema, &new_keys, value);
                }
            }
            Either::Right(schemas) => {
                let items = array.value().read();
                for (idx, (value, schema)) in items.iter().zip(schemas.iter()).enumerate() {
                    let new_keys = keys.join(KeyOrIndex::Index(idx));
                    validate_impl(errors, defs, schema, &new_keys, value);
                }
                let schemas_len = schemas.len();
                if items.len() > schemas_len {
                    if let Some(additional_items) = local_schema.additional_items.as_ref() {
                        match additional_items.value.as_ref() {
                            Either::Left(allowed) => {
                                if !allowed {
                                    errors.push(Error::new(keys, ErrorKind::AdditionalItems));
                                }
                            }
                            Either::Right(schema) => {
                                for (idx, value) in items.iter().skip(schemas_len).enumerate() {
                                    let new_keys = keys.join(KeyOrIndex::Index(idx + schemas_len));
                                    validate_impl(errors, defs, schema, &new_keys, value);
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

fn validate_contains(
    errors: &mut Vec<Error>,
    defs: &IndexMap<String, Schema>,
    local_schema: &Schema,
    keys: &Keys,
    node: &Node,
) {
    if let Some(schema) = local_schema.contains.as_ref() {
        let array = match node.as_array() {
            Some(v) => v,
            None => return,
        };
        let mut any_matched = false;
        for (idx, value) in array.value().read().iter().enumerate() {
            let mut local_errors = vec![];
            let new_keys = keys.join(KeyOrIndex::Index(idx));
            validate_impl(&mut local_errors, defs, schema, &new_keys, value);
            if local_errors.is_empty() {
                any_matched = true;
                break;
            }
        }
        if !any_matched {
            errors.push(Error::new(keys, ErrorKind::Contains))
        }
    }
}

fn validate_maxmin_items(
    errors: &mut Vec<Error>,
    _defs: &IndexMap<String, Schema>,
    local_schema: &Schema,
    keys: &Keys,
    node: &Node,
) {
    let array = match node.as_array() {
        Some(v) => v,
        None => return,
    };
    if let Some(max_items) = local_schema.max_items {
        if array.value().read().len() > max_items as usize {
            errors.push(Error::new(keys, ErrorKind::MaxItems))
        }
    }
    if let Some(min_items) = local_schema.min_items {
        if array.value().read().len() < min_items as usize {
            errors.push(Error::new(keys, ErrorKind::MinItems))
        }
    }
}

fn validate_unique_items(
    errors: &mut Vec<Error>,
    _defs: &IndexMap<String, Schema>,
    local_schema: &Schema,
    keys: &Keys,
    node: &Node,
) {
    if let Some(unique_items) = local_schema.unique_items {
        if !unique_items {
            return;
        }
        let array = match node.as_array() {
            Some(v) => v,
            None => return,
        };
        let items: Vec<serde_json::Value> = array
            .value()
            .read()
            .iter()
            .map(|v| v.to_plain_json())
            .collect();
        if !equal::is_unique(&items) {
            errors.push(Error::new(keys, ErrorKind::UniqueItems))
        }
    }
}

fn validate_pattern(
    errors: &mut Vec<Error>,
    _defs: &IndexMap<String, Schema>,
    local_schema: &Schema,
    keys: &Keys,
    node: &Node,
) {
    if let Some(pattern) = local_schema.pattern.as_ref() {
        let value = match node.as_string() {
            Some(v) => v.value(),
            None => return,
        };
        if let Ok(re) = convert_regex(pattern) {
            if !matches!(re.is_match(value), Ok(true)) {
                errors.push(Error::new(keys, ErrorKind::Pattern))
            }
        }
    }
}

fn validate_maxmin_length(
    errors: &mut Vec<Error>,
    _defs: &IndexMap<String, Schema>,
    local_schema: &Schema,
    keys: &Keys,
    node: &Node,
) {
    let value = match node.as_string() {
        Some(v) => v.value(),
        None => return,
    };
    if let Some(max_length) = local_schema.max_length.as_ref() {
        if value.len() > *max_length as usize {
            errors.push(Error::new(keys, ErrorKind::MaxLength))
        }
    }
    if let Some(min_length) = local_schema.min_length.as_ref() {
        if value.len() < *min_length as usize {
            errors.push(Error::new(keys, ErrorKind::MinLength))
        }
    }
}

fn validate_format(
    errors: &mut Vec<Error>,
    _defs: &IndexMap<String, Schema>,
    local_schema: &Schema,
    keys: &Keys,
    node: &Node,
) {
    if let Some(format) = local_schema.format.as_ref() {
        let value = match node.as_string() {
            Some(v) => v.value(),
            None => return,
        };
        let valid = match format.as_str() {
            "date" => formats::date(value),
            "date-time" => formats::date_time(value),
            "email" => formats::email(value),
            "hostname" => formats::hostname(value),
            "ipv4" => formats::ipv4(value),
            "ipv6" => formats::ipv6(value),
            "uri" => formats::uri(value),
            "regex" => formats::regex(value),
            "time" => formats::time(value),
            "uuid" => formats::uuid(value),
            _ => true,
        };
        if !valid {
            errors.push(Error::new(keys, ErrorKind::Format))
        }
    }
}

fn validate_maxmin(
    errors: &mut Vec<Error>,
    _defs: &IndexMap<String, Schema>,
    local_schema: &Schema,
    keys: &Keys,
    node: &Node,
) {
    let value = match node.as_number() {
        Some(v) => v.value(),
        None => return,
    };
    if let Some(maximum) = local_schema.maximum.as_ref() {
        let mut valid = true;
        if local_schema
            .exclusive_maximum
            .as_ref()
            .copied()
            .unwrap_or_default()
        {
            if value.as_f64() >= maximum.as_f64() {
                valid = false;
            }
        } else if value.as_f64() > maximum.as_f64() {
            valid = false;
        }
        if !valid {
            errors.push(Error::new(keys, ErrorKind::Maximum))
        }
    }
    if let Some(minimum) = local_schema.minimum.as_ref() {
        let mut valid = true;
        if local_schema
            .exclusive_minimum
            .as_ref()
            .copied()
            .unwrap_or_default()
        {
            if value.as_f64() <= minimum.as_f64() {
                valid = false;
            }
        } else if value.as_f64() < minimum.as_f64() {
            valid = false;
        }
        if !valid {
            errors.push(Error::new(keys, ErrorKind::Minimum))
        }
    }
}

fn validate_multiple_of(
    errors: &mut Vec<Error>,
    _defs: &IndexMap<String, Schema>,
    local_schema: &Schema,
    keys: &Keys,
    node: &Node,
) {
    if let Some(multiple_of) = local_schema.multiple_of.as_ref() {
        let value = match node.as_number().and_then(|v| v.value().as_f64()) {
            Some(v) => v,
            None => return,
        };
        let valid = if (value.fract() == 0f64) && (multiple_of.fract() == 0f64) {
            (value % multiple_of) == 0f64
        } else {
            let remainder: f64 = (value / multiple_of) % 1f64;
            let remainder_less_than_epsilon = matches!(
                remainder.partial_cmp(&f64::EPSILON),
                None | Some(Ordering::Less)
            );
            let remainder_less_than_one = remainder < (1f64 - f64::EPSILON);
            remainder_less_than_epsilon && remainder_less_than_one
        };
        if !valid {
            errors.push(Error::new(keys, ErrorKind::MultipleOf))
        }
    }
}

fn validate_allof(
    errors: &mut Vec<Error>,
    defs: &IndexMap<String, Schema>,
    local_schema: &Schema,
    keys: &Keys,
    node: &Node,
) {
    if let Some(all_off) = local_schema.all_of.as_ref() {
        for schema in all_off.iter() {
            validate_impl(errors, defs, schema, keys, node)
        }
    }
}

fn validate_anyof(
    errors: &mut Vec<Error>,
    defs: &IndexMap<String, Schema>,
    local_schema: &Schema,
    keys: &Keys,
    node: &Node,
) {
    if let Some(any_of) = local_schema.any_of.as_ref() {
        let mut collect_errors = vec![];
        let mut valid = false;
        for schema in any_of.iter() {
            let mut local_errors = vec![];
            validate_impl(&mut local_errors, defs, schema, keys, node);
            if local_errors.is_empty() {
                valid = true;
            } else {
                collect_errors.extend(local_errors);
            }
        }
        if !valid {
            errors.push(Error::new(
                keys,
                ErrorKind::AnyOf {
                    errors: collect_errors,
                },
            ));
        }
    }
}

fn validate_oneof(
    errors: &mut Vec<Error>,
    defs: &IndexMap<String, Schema>,
    local_schema: &Schema,
    keys: &Keys,
    node: &Node,
) {
    if let Some(one_of) = local_schema.one_of.as_ref() {
        let mut collect_errors = vec![];
        let mut valid = 0;
        let mut indexes = vec![];
        for (index, schema) in one_of.iter().enumerate() {
            let mut local_errors = vec![];
            validate_impl(&mut local_errors, defs, schema, keys, node);
            if local_errors.is_empty() {
                valid += 1;
            }
            if local_errors
                .iter()
                .filter(|v| {
                    v.keys.len() > keys.len()
                        || !matches!(
                            v.kind,
                            ErrorKind::AdditionalProperties { .. } | ErrorKind::Type { .. }
                        )
                })
                .count()
                > 0
            {
                indexes.push(index);
            }
            collect_errors.push(local_errors);
        }
        if valid == 1 {
            return;
        }
        if valid == 0 && indexes.len() == 1 {
            errors.extend(collect_errors.remove(indexes[0]))
        } else {
            errors.push(Error::new(
                keys,
                ErrorKind::OneOf {
                    errors: collect_errors.into_iter().flatten().collect(),
                },
            ));
        }
    }
}

fn validate_not(
    errors: &mut Vec<Error>,
    defs: &IndexMap<String, Schema>,
    local_schema: &Schema,
    keys: &Keys,
    node: &Node,
) {
    if let Some(schema) = local_schema.not.as_ref() {
        let mut local_errors = vec![];
        validate_impl(&mut local_errors, defs, schema, keys, node);
        if local_errors.is_empty() {
            errors.push(Error::new(keys, ErrorKind::Not));
        }
    }
}

fn validate_condiational(
    errors: &mut Vec<Error>,
    defs: &IndexMap<String, Schema>,
    local_schema: &Schema,
    keys: &Keys,
    node: &Node,
) {
    if let Some(if_schema) = local_schema.if_value.as_ref() {
        let mut local_errors = vec![];
        validate_impl(&mut local_errors, defs, if_schema, keys, node);
        if local_errors.is_empty() {
            if let Some(then_schema) = local_schema.then_value.as_ref() {
                validate_impl(errors, defs, then_schema, keys, node);
            }
        } else if let Some(else_schema) = local_schema.else_value.as_ref() {
            validate_impl(errors, defs, else_schema, keys, node);
        }
    }
}

#[derive(Debug)]
pub struct Error {
    pub keys: Keys,
    pub kind: ErrorKind,
}

impl Error {
    pub fn new(keys: &Keys, kind: ErrorKind) -> Self {
        Self {
            keys: keys.clone(),
            kind,
        }
    }
    pub fn to_error_object(&self, node: &Node, mapper: &Mapper) -> ErrorObject {
        let message = self.to_string();
        ErrorObject::new(
            ERROR_SOURCE,
            self.kind.name(),
            message,
            self.keys.mapper_range(node, mapper),
        )
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.keys, self.kind)
    }
}

#[derive(Debug)]
pub enum ErrorKind {
    Type { types: Vec<SchemaType> },
    Enum,
    Const,
    AdditionalProperties { key: String },
    Required { keys: Vec<String> },
    MaxProperties,
    MinProperties,
    AdditionalItems,
    Contains,
    MaxItems,
    MinItems,
    UniqueItems,
    Pattern,
    MaxLength,
    MinLength,
    Format,
    Maximum,
    Minimum,
    MultipleOf,
    AnyOf { errors: Vec<Error> },
    OneOf { errors: Vec<Error> },
    Not,
}

impl ErrorKind {
    pub fn name(&self) -> &'static str {
        match self {
            ErrorKind::Type { .. } => "Type",
            ErrorKind::Enum => "Enum",
            ErrorKind::Const => "Const",
            ErrorKind::AdditionalProperties { .. } => "AdditionalProperties",
            ErrorKind::Required { .. } => "Required",
            ErrorKind::MaxProperties => "MaxProperties",
            ErrorKind::MinProperties => "MinProperties",
            ErrorKind::AdditionalItems => "AdditionalItems",
            ErrorKind::Contains => "Contains",
            ErrorKind::MaxItems => "MaxItems",
            ErrorKind::MinItems => "MinItems",
            ErrorKind::UniqueItems => "UniqueItems",
            ErrorKind::Pattern => "Pattern",
            ErrorKind::MaxLength => "MaxLength",
            ErrorKind::MinLength => "MinLength",
            ErrorKind::Format => "Format",
            ErrorKind::Maximum => "Maximum",
            ErrorKind::Minimum => "Minimum",
            ErrorKind::MultipleOf => "MultipleOf",
            ErrorKind::AnyOf { .. } => "AnyOf",
            ErrorKind::OneOf { .. } => "OneOf",
            ErrorKind::Not => "Not",
        }
    }
}

impl Display for ErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ErrorKind::Type { types } => write!(
                f,
                "The value must be any of: {}",
                types
                    .iter()
                    .map(|v| v.to_string())
                    .collect::<Vec<String>>()
                    .join(",")
            ),
            ErrorKind::Enum => write!(f, "Enum conditions are not met"),
            ErrorKind::Const => write!(f, "Const condition is not met"),
            ErrorKind::AdditionalProperties { key } => {
                write!(f, "Additional property '{}' is not allowed", key)
            }
            ErrorKind::Required { keys } => {
                write!(f, "This properties {} is required", keys.join(","))
            }
            ErrorKind::MaxProperties => write!(f, "MaxProperties condition is not met"),
            ErrorKind::MinProperties => write!(f, "MinProperties condition is not met"),
            ErrorKind::AdditionalItems => write!(f, "Additional items are not allowed"),
            ErrorKind::Contains => write!(f, "Contains condition is not met"),
            ErrorKind::MaxItems => write!(f, "MaxItems condition is not met"),
            ErrorKind::MinItems => write!(f, "MinItems condition is not met"),
            ErrorKind::UniqueItems => write!(f, "UniqueItems condition is not met"),
            ErrorKind::Pattern => write!(f, "Pattern condition is not met"),
            ErrorKind::MinLength => write!(f, "MinLength condition is not met"),
            ErrorKind::MaxLength => write!(f, "MaxLength condition is not met"),
            ErrorKind::Format => write!(f, "Format condition is not met"),
            ErrorKind::Maximum => write!(f, "Maximum condition is not met"),
            ErrorKind::Minimum => write!(f, "Minimum condition is not met"),
            ErrorKind::MultipleOf => write!(f, "MultipleOf condition is not met"),
            ErrorKind::AnyOf { errors } => {
                let mut extra = "".into();
                if !errors.is_empty() {
                    extra = format!(
                        "; {}",
                        errors
                            .iter()
                            .map(|v| v.to_string())
                            .collect::<Vec<String>>()
                            .join("; ")
                    );
                }
                write!(f, "AnyOf conditions are not met{}", extra)
            }
            ErrorKind::OneOf { errors } => {
                let mut extra = ", more than one valid".into();
                if !errors.is_empty() {
                    extra = format!(
                        "; {}",
                        errors
                            .iter()
                            .map(|v| v.to_string())
                            .collect::<Vec<String>>()
                            .join("; ")
                    );
                }
                write!(f, "OneOf conditions are not met{}", extra)
            }
            ErrorKind::Not => write!(f, "Not condition is not met"),
        }
    }
}

fn resolve<'a>(defs: &'a IndexMap<String, Schema>, local_schema: &'a Schema) -> Option<&'a Schema> {
    let schema = match local_schema.ref_value.as_ref() {
        Some(ref_value) => {
            match REF_REGEX
                .captures(ref_value)
                .ok()
                .flatten()
                .and_then(|v| v.get(1))
                .and_then(|v| defs.get(v.as_str()))
            {
                Some(v) => v,
                None => return None,
            }
        }
        None => local_schema,
    };
    Some(schema)
}

fn is_matching(va: &serde_json::Value, vb: &serde_json::Value) -> bool {
    match va {
        serde_json::Value::Number(a) => match vb {
            serde_json::Value::Number(b) => a.as_f64().unwrap() == b.as_f64().unwrap(),
            _ => false,
        },
        _ => *va == *vb,
    }
}

mod formats {
    use super::*;
    use std::net::IpAddr;
    use std::str::FromStr;

    pub(crate) fn date(value: &str) -> bool {
        time::Date::parse(
            value,
            &time::macros::format_description!("[year]-[month]-[day]"),
        )
        .is_ok()
    }
    pub(crate) fn date_time(value: &str) -> bool {
        time::OffsetDateTime::parse(value, &time::format_description::well_known::Rfc3339).is_ok()
    }
    pub(crate) fn email(value: &str) -> bool {
        if let Some('.') = value.chars().next() {
            // dot before local part is not valid
            return false;
        }
        // This loop exits early if it finds `@`.
        // Therefore, match arms examine only the local part
        for (a, b) in value.chars().zip(value.chars().skip(1)) {
            match (a, b) {
                // two subsequent dots inside local part are not valid
                // dot after local part is not valid
                ('.', '.') | ('.', '@') => return false,
                // The domain part is not validated for simplicity
                (_, '@') => return true,
                (_, _) => continue,
            }
        }
        false
    }
    pub(crate) fn hostname(value: &str) -> bool {
        !(value.ends_with('-')
            || value.starts_with('-')
            || value.is_empty()
            || bytecount::num_chars(value.as_bytes()) > 255
            || value
                .chars()
                .any(|c| !(c.is_alphanumeric() || c == '-' || c == '.'))
            || value
                .split('.')
                .any(|part| bytecount::num_chars(part.as_bytes()) > 63))
    }

    pub(crate) fn ipv4(value: &str) -> bool {
        if value.starts_with('0') {
            return false;
        }
        match IpAddr::from_str(value) {
            Ok(i) => i.is_ipv4(),
            Err(_) => false,
        }
    }

    pub(crate) fn ipv6(value: &str) -> bool {
        match IpAddr::from_str(value) {
            Ok(i) => i.is_ipv6(),
            Err(_) => false,
        }
    }

    pub(crate) fn uri(value: &str) -> bool {
        url::Url::from_str(value).is_ok()
    }

    pub(crate) fn regex(value: &str) -> bool {
        convert_regex(value).is_ok()
    }

    pub(crate) fn time(value: &str) -> bool {
        matches!(TIME_RE.is_match(value), Ok(true))
    }

    pub(crate) fn uuid(value: &str) -> bool {
        matches!(UUID_RE.is_match(value), Ok(true))
    }
}

mod equal {
    use ahash::{AHashSet, AHasher};
    use serde_json::{Map, Value};
    use std::hash::{Hash, Hasher};
    // Based on implementation proposed by Sven Marnach:
    // https://stackoverflow.com/questions/60882381/what-is-the-fastest-correct-way-to-detect-that-there-are-no-duplicates-in-a-json
    #[derive(PartialEq)]
    pub(crate) struct HashedValue<'a>(&'a Value);

    impl Eq for HashedValue<'_> {}

    impl Hash for HashedValue<'_> {
        fn hash<H: Hasher>(&self, state: &mut H) {
            match self.0 {
                Value::Null => state.write_u32(3_221_225_473), // chosen randomly
                Value::Bool(ref item) => item.hash(state),
                Value::Number(ref item) => {
                    if let Some(number) = item.as_u64() {
                        number.hash(state);
                    } else if let Some(number) = item.as_i64() {
                        number.hash(state);
                    } else if let Some(number) = item.as_f64() {
                        number.to_bits().hash(state)
                    }
                }
                Value::String(ref item) => item.hash(state),
                Value::Array(ref items) => {
                    for item in items {
                        HashedValue(item).hash(state);
                    }
                }
                Value::Object(ref items) => {
                    let mut hash = 0;
                    for (key, value) in items {
                        // We have no way of building a new hasher of type `H`, so we
                        // hardcode using the default hasher of a hash map.
                        let mut item_hasher = AHasher::default();
                        key.hash(&mut item_hasher);
                        HashedValue(value).hash(&mut item_hasher);
                        hash ^= item_hasher.finish();
                    }
                    state.write_u64(hash);
                }
            }
        }
    }

    // Empirically calculated threshold after which the validator resorts to hashing.
    // Calculated for an array of mixed types, large homogenous arrays of primitive values might be
    // processed faster with different thresholds, but this one gives a good baseline for the common
    // case.
    const ITEMS_SIZE_THRESHOLD: usize = 15;

    #[inline]
    pub(crate) fn is_unique(items: &[Value]) -> bool {
        let size = items.len();
        if size <= 1 {
            // Empty arrays and one-element arrays always contain unique elements
            true
        } else if let [first, second] = items {
            !equal(first, second)
        } else if let [first, second, third] = items {
            !equal(first, second) && !equal(first, third) && !equal(second, third)
        } else if size <= ITEMS_SIZE_THRESHOLD {
            // If the array size is small enough we can compare all elements pairwise, which will
            // be faster than calculating hashes for each element, even if the algorithm is O(N^2)
            let mut idx = 0_usize;
            while idx < items.len() {
                let mut inner_idx = idx + 1;
                while inner_idx < items.len() {
                    if equal(&items[idx], &items[inner_idx]) {
                        return false;
                    }
                    inner_idx += 1;
                }
                idx += 1;
            }
            true
        } else {
            let mut seen = AHashSet::with_capacity(size);
            items.iter().map(HashedValue).all(move |x| seen.insert(x))
        }
    }

    #[inline]
    pub(crate) fn equal(left: &Value, right: &Value) -> bool {
        match (left, right) {
            (Value::String(left), Value::String(right)) => left == right,
            (Value::Bool(left), Value::Bool(right)) => left == right,
            (Value::Null, Value::Null) => true,
            (Value::Number(left), Value::Number(right)) => left.as_f64() == right.as_f64(),
            (Value::Array(left), Value::Array(right)) => equal_arrays(left, right),
            (Value::Object(left), Value::Object(right)) => equal_objects(left, right),
            (_, _) => false,
        }
    }

    #[inline]
    pub(crate) fn equal_arrays(left: &[Value], right: &[Value]) -> bool {
        left.len() == right.len() && {
            let mut idx = 0_usize;
            while idx < left.len() {
                if !equal(&left[idx], &right[idx]) {
                    return false;
                }
                idx += 1;
            }
            true
        }
    }

    #[inline]
    pub(crate) fn equal_objects(left: &Map<String, Value>, right: &Map<String, Value>) -> bool {
        left.len() == right.len()
            && left
                .iter()
                .zip(right)
                .all(|((ka, va), (kb, vb))| ka == kb && equal(va, vb))
    }
}

// ECMA 262 has differences
fn convert_regex(pattern: &str) -> Result<fancy_regex::Regex, fancy_regex::Error> {
    // replace control chars
    let new_pattern = CONTROL_GROUPS_RE.replace_all(pattern, replace_control_group);
    let mut out = String::with_capacity(new_pattern.len());
    let mut chars = new_pattern.chars().peekable();
    // To convert character group we need to iterate over chars and in case of `\` take a look
    // at the next char to detect whether this group should be converted
    while let Some(current) = chars.next() {
        if current == '\\' {
            // Possible character group
            if let Some(next) = chars.next() {
                match next {
                    'd' => out.push_str("[0-9]"),
                    'D' => out.push_str("[^0-9]"),
                    'w' => out.push_str("[A-Za-z0-9_]"),
                    'W' => out.push_str("[^A-Za-z0-9_]"),
                    's' => {
                        out.push_str("[ \t\n\r\u{000b}\u{000c}\u{2003}\u{feff}\u{2029}\u{00a0}]")
                    }
                    'S' => {
                        out.push_str("[^ \t\n\r\u{000b}\u{000c}\u{2003}\u{feff}\u{2029}\u{00a0}]")
                    }
                    _ => {
                        // Nothing interesting, push as is
                        out.push(current);
                        out.push(next)
                    }
                }
            } else {
                // End of the string, push the last char.
                // Note that it is an incomplete escape sequence and will lead to an error on
                // the next step
                out.push(current);
            }
        } else {
            // Regular character
            out.push(current);
        }
    }
    fancy_regex::Regex::new(&out)
}

fn replace_control_group(captures: &regex::Captures) -> String {
    // There will be no overflow, because the minimum value is 65 (char 'A')
    ((captures
        .index(0)
        .trim_start_matches(r"\c")
        .chars()
        .next()
        .expect("This is always present because of the regex rule. It has [A-Za-z] next")
        .to_ascii_uppercase() as u8
        - 64) as char)
        .to_string()
}
