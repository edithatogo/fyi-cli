//! Validation of the bounded JSON Schema vocabulary used by embedded MCP tools.
//!
//! This is deliberately not a general JSON Schema implementation. Unknown
//! assertion keywords fail closed, including in unused branches and optional
//! properties. `format` is an annotation; handlers still validate dates and URLs.

use serde_json::{Number, Value};
use std::cmp::Ordering;

const INVALID_ARGUMENTS: &str = "Tool arguments do not match the advertised schema";
const MAX_DEPTH: usize = 64;
const MAX_SCHEMA_NODES: usize = 4_096;
const MAX_VALUE_NODES: usize = 100_000;
const MAX_VALIDATION_STEPS: usize = 100_000;
const I64_LOWER: f64 = -9_223_372_036_854_775_808.0;
const I64_UPPER_EXCLUSIVE: f64 = 9_223_372_036_854_775_808.0;
const U64_UPPER_EXCLUSIVE: f64 = 18_446_744_073_709_551_616.0;

/// Check the complete embedded schema, including currently unselected branches.
///
/// Only the assertion vocabulary below is supported. References, patterns,
/// conditional schemas and unknown keywords are rejected rather than ignored.
pub(crate) fn validate_schema(schema: &Value) -> Result<(), &'static str> {
    let mut remaining = MAX_SCHEMA_NODES;
    check_value_bounds(schema, 0, &mut remaining)?;
    check_schema(schema)
}

/// Validate without echoing caller data, then commit lossless integer conversions.
///
/// Failed validation leaves the caller's arguments unchanged. Defaults are not
/// inserted and strings/booleans are never coerced to other JSON types.
pub(crate) fn validate_arguments(
    schema: &Value,
    arguments: &mut Value,
) -> Result<(), &'static str> {
    validate_schema(schema)?;
    let mut remaining = MAX_VALUE_NODES;
    check_value_bounds(arguments, 0, &mut remaining)?;
    let mut normalized = arguments.clone();
    let mut remaining = MAX_VALIDATION_STEPS;
    validate_value(schema, &mut normalized, &mut remaining)?;
    if remaining == 0 {
        return Err(INVALID_ARGUMENTS);
    }
    *arguments = normalized;
    Ok(())
}

fn spend(remaining: &mut usize) -> Result<(), &'static str> {
    *remaining = remaining.checked_sub(1).ok_or(INVALID_ARGUMENTS)?;
    Ok(())
}

fn check_value_bounds(
    value: &Value,
    depth: usize,
    remaining: &mut usize,
) -> Result<(), &'static str> {
    spend(remaining)?;
    if depth > MAX_DEPTH {
        return Err(INVALID_ARGUMENTS);
    }
    match value {
        Value::Array(values) => {
            for value in values {
                check_value_bounds(value, depth + 1, remaining)?;
            }
        }
        Value::Object(values) => {
            for value in values.values() {
                check_value_bounds(value, depth + 1, remaining)?;
            }
        }
        _ => {}
    }
    Ok(())
}

fn check_schema(schema: &Value) -> Result<(), &'static str> {
    if schema.is_boolean() {
        return Ok(());
    }
    let object = schema.as_object().ok_or(INVALID_ARGUMENTS)?;
    for (keyword, value) in object {
        match keyword.as_str() {
            "type" => match value {
                Value::String(kind) if known_type(kind) => {}
                Value::Array(kinds)
                    if !kinds.is_empty()
                        && kinds
                            .iter()
                            .all(|kind| kind.as_str().is_some_and(known_type))
                        && !duplicates(kinds) => {}
                _ => return Err(INVALID_ARGUMENTS),
            },
            "properties" => {
                for subschema in value.as_object().ok_or(INVALID_ARGUMENTS)?.values() {
                    check_schema(subschema)?;
                }
            }
            "required" => {
                let fields = value.as_array().ok_or(INVALID_ARGUMENTS)?;
                if fields.iter().any(|field| !field.is_string()) || duplicates(fields) {
                    return Err(INVALID_ARGUMENTS);
                }
            }
            "additionalProperties" | "items" => check_schema(value)?,
            "enum" => {
                let options = value.as_array().ok_or(INVALID_ARGUMENTS)?;
                if options.is_empty() || duplicates(options) {
                    return Err(INVALID_ARGUMENTS);
                }
            }
            "minimum" | "maximum" if value.is_number() => {}
            "minLength" | "maxLength" | "minItems" | "maxItems"
                if nonnegative_integer(value).is_some() => {}
            "anyOf" | "oneOf" => {
                let alternatives = value.as_array().ok_or(INVALID_ARGUMENTS)?;
                if alternatives.is_empty() {
                    return Err(INVALID_ARGUMENTS);
                }
                for alternative in alternatives {
                    check_schema(alternative)?;
                }
            }
            "title" | "description" | "format" if value.is_string() => {}
            "default" | "const" => {}
            _ => return Err(INVALID_ARGUMENTS),
        }
    }
    Ok(())
}

fn known_type(kind: &str) -> bool {
    matches!(
        kind,
        "object" | "array" | "string" | "integer" | "number" | "boolean" | "null"
    )
}

fn duplicates(values: &[Value]) -> bool {
    values
        .iter()
        .enumerate()
        .any(|(index, value)| values[..index].iter().any(|other| equivalent(value, other)))
}

fn integer_number(value: &Value) -> Option<Number> {
    let number = value.as_number()?;
    if number.is_i64() || number.is_u64() {
        return Some(number.clone());
    }
    let number = number.as_f64()?;
    if !number.is_finite() || number.fract() != 0.0 {
        return None;
    }
    // Exclusive upper bounds matter: i64::MAX/u64::MAX round upwards as f64.
    let normalized = if (I64_LOWER..I64_UPPER_EXCLUSIVE).contains(&number) {
        Number::from(number as i64)
    } else if (0.0..U64_UPPER_EXCLUSIVE).contains(&number) {
        Number::from(number as u64)
    } else {
        return None;
    };
    (normalized.as_f64() == Some(number)).then_some(normalized)
}

fn nonnegative_integer(value: &Value) -> Option<u64> {
    integer_number(value)?.as_u64()
}

fn exact_integer(number: &Number) -> Option<i128> {
    number
        .as_i64()
        .map(i128::from)
        .or_else(|| number.as_u64().map(i128::from))
}

fn compare_integer_float(integer: i128, float: f64) -> Ordering {
    if float < I64_LOWER {
        return Ordering::Greater;
    }
    if float >= U64_UPPER_EXCLUSIVE {
        return Ordering::Less;
    }
    // Conversion to i128 is exact for this bounded, truncated float. Comparing
    // integer values as f64 instead would round adjacent large IDs together.
    match integer.cmp(&(float as i128)) {
        Ordering::Equal if float.fract() > 0.0 => Ordering::Less,
        Ordering::Equal if float.fract() < 0.0 => Ordering::Greater,
        ordering => ordering,
    }
}

fn compare_numbers(left: &Number, right: &Number) -> Option<Ordering> {
    match (exact_integer(left), exact_integer(right)) {
        (Some(left), Some(right)) => Some(left.cmp(&right)),
        (Some(left), None) => Some(compare_integer_float(left, right.as_f64()?)),
        (None, Some(right)) => Some(compare_integer_float(right, left.as_f64()?).reverse()),
        (None, None) => left.as_f64()?.partial_cmp(&right.as_f64()?),
    }
}

fn equivalent(left: &Value, right: &Value) -> bool {
    match (left, right) {
        (Value::Number(left), Value::Number(right)) => {
            compare_numbers(left, right) == Some(Ordering::Equal)
        }
        (Value::Array(left), Value::Array(right)) => {
            left.len() == right.len()
                && left
                    .iter()
                    .zip(right)
                    .all(|(left, right)| equivalent(left, right))
        }
        (Value::Object(left), Value::Object(right)) => {
            left.len() == right.len()
                && left
                    .iter()
                    .all(|(key, left)| right.get(key).is_some_and(|right| equivalent(left, right)))
        }
        _ => left == right,
    }
}

fn validate_type(declaration: &Value, value: &mut Value) -> Result<(), &'static str> {
    let allows = |kind: &str| {
        declaration.as_str() == Some(kind)
            || declaration
                .as_array()
                .is_some_and(|kinds| kinds.iter().any(|value| value.as_str() == Some(kind)))
    };
    if allows("integer") {
        if let Some(integer) = integer_number(value) {
            *value = Value::Number(integer);
            return Ok(());
        }
    }
    let kind = match value {
        Value::Object(_) => "object",
        Value::Array(_) => "array",
        Value::String(_) => "string",
        Value::Number(_) => "number",
        Value::Bool(_) => "boolean",
        Value::Null => "null",
    };
    allows(kind).then_some(()).ok_or(INVALID_ARGUMENTS)
}

fn validate_count(
    schema: &Value,
    count: usize,
    minimum: &str,
    maximum: &str,
) -> Result<(), &'static str> {
    let count = u64::try_from(count).map_err(|_| INVALID_ARGUMENTS)?;
    if schema
        .get(minimum)
        .and_then(nonnegative_integer)
        .is_some_and(|minimum| count < minimum)
        || schema
            .get(maximum)
            .and_then(nonnegative_integer)
            .is_some_and(|maximum| count > maximum)
    {
        return Err(INVALID_ARGUMENTS);
    }
    Ok(())
}

fn validate_value(
    schema: &Value,
    value: &mut Value,
    remaining: &mut usize,
) -> Result<(), &'static str> {
    spend(remaining)?;
    if let Some(accepts) = schema.as_bool() {
        return accepts.then_some(()).ok_or(INVALID_ARGUMENTS);
    }
    if let Some(declaration) = schema.get("type") {
        validate_type(declaration, value)?;
    }
    if schema
        .get("const")
        .is_some_and(|expected| !equivalent(value, expected))
        || schema
            .get("enum")
            .and_then(Value::as_array)
            .is_some_and(|options| !options.iter().any(|expected| equivalent(value, expected)))
    {
        return Err(INVALID_ARGUMENTS);
    }
    match value {
        Value::Object(fields) => {
            if let Some(required) = schema.get("required").and_then(Value::as_array) {
                for field in required {
                    if !fields.contains_key(field.as_str().ok_or(INVALID_ARGUMENTS)?) {
                        return Err(INVALID_ARGUMENTS);
                    }
                }
            }
            let properties = schema.get("properties").and_then(Value::as_object);
            for (name, value) in fields {
                if let Some(subschema) = properties.and_then(|properties| properties.get(name)) {
                    validate_value(subschema, value, remaining)?;
                } else if let Some(additional) = schema.get("additionalProperties") {
                    validate_value(additional, value, remaining)?;
                }
            }
        }
        Value::Array(values) => {
            validate_count(schema, values.len(), "minItems", "maxItems")?;
            if let Some(items) = schema.get("items") {
                for value in values {
                    validate_value(items, value, remaining)?;
                }
            }
        }
        Value::String(value) => {
            validate_count(schema, value.chars().count(), "minLength", "maxLength")?;
        }
        Value::Number(number) => {
            for (keyword, excluded) in [("minimum", Ordering::Less), ("maximum", Ordering::Greater)]
            {
                if let Some(bound) = schema.get(keyword).and_then(Value::as_number) {
                    if compare_numbers(number, bound).ok_or(INVALID_ARGUMENTS)? == excluded {
                        return Err(INVALID_ARGUMENTS);
                    }
                }
            }
        }
        _ => {}
    }
    for keyword in ["anyOf", "oneOf"] {
        if let Some(alternatives) = schema.get(keyword).and_then(Value::as_array) {
            validate_alternatives(alternatives, value, keyword == "oneOf", remaining)?;
        }
    }
    Ok(())
}

fn validate_alternatives(
    alternatives: &[Value],
    value: &mut Value,
    exclusive: bool,
    remaining: &mut usize,
) -> Result<(), &'static str> {
    let mut matched = None;
    for schema in alternatives {
        spend(remaining)?;
        let mut candidate = value.clone();
        if validate_value(schema, &mut candidate, remaining).is_ok() {
            if matched.is_some() {
                return Err(INVALID_ARGUMENTS);
            }
            matched = Some(candidate);
            if !exclusive {
                break;
            }
        }
    }
    *value = matched.ok_or(INVALID_ARGUMENTS)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn authority_schema() -> Value {
        json!({
            "type": "object",
            "properties": {
                "authorities": {
                    "type": "array", "minItems": 1, "maxItems": 2,
                    "items": {
                        "type": "object",
                        "properties": {
                            "slug": {"type": "string", "minLength": 1, "maxLength": 8},
                            "name": {"type": "string", "minLength": 1},
                            "url": {"type": ["string", "null"], "format": "uri"}
                        },
                        "required": ["slug", "name"],
                        "additionalProperties": false
                    }
                }
            },
            "required": ["authorities"],
            "additionalProperties": false
        })
    }

    #[test]
    fn validates_nested_authorities_without_default_or_format_coercion() {
        let schema = authority_schema();
        let mut arguments =
            json!({"authorities": [{"slug": "council", "name": "Synthetic", "url": null}]});
        let original = arguments.clone();
        assert_eq!(validate_arguments(&schema, &mut arguments), Ok(()));
        assert_eq!(arguments, original);
        let mut invalid_uri = json!("the handler checks the URI");
        assert_eq!(
            validate_arguments(
                &json!({"type": "string", "format": "uri"}),
                &mut invalid_uri
            ),
            Ok(())
        );
        let mut empty = json!({});
        assert_eq!(
            validate_arguments(
                &json!({"type": "object", "default": {"limit": 1}}),
                &mut empty
            ),
            Ok(())
        );
        assert_eq!(empty, json!({}));
    }

    #[test]
    fn rejects_missing_unknown_or_mistyped_nested_fields() {
        let schema = authority_schema();
        for mut invalid in [
            json!(null),
            json!([]),
            json!({}),
            json!({"authorities": []}),
            json!({"authorities": [{"slug": "x"}]}),
            json!({"authorities": [{"slug": "", "name": "Synthetic"}]}),
            json!({"authorities": [{"slug": 1, "name": "Synthetic"}]}),
            json!({"authorities": [{"slug": "x", "name": "Synthetic", "url": false}]}),
            json!({"authorities": [{"slug": "x", "name": "Synthetic", "unexpected": "secret"}]}),
            json!({"authorities": [{"slug": "x", "name": "Synthetic"}], "secret-key": true}),
        ] {
            assert_eq!(
                validate_arguments(&schema, &mut invalid),
                Err(INVALID_ARGUMENTS)
            );
        }
    }

    #[test]
    fn enforces_numeric_bounds_without_string_or_boolean_coercion() {
        let schema = json!({"type": "integer", "minimum": 1, "maximum": 500});
        for mut invalid in [
            json!(0),
            json!(-1),
            json!(501),
            json!(1.5),
            json!(true),
            json!("1"),
            json!(null),
        ] {
            assert_eq!(
                validate_arguments(&schema, &mut invalid),
                Err(INVALID_ARGUMENTS)
            );
        }
        for mut valid in [json!(1), json!(500)] {
            assert_eq!(validate_arguments(&schema, &mut valid), Ok(()));
        }
    }

    #[test]
    fn normalizes_mathematical_integers_for_existing_handlers() {
        let schema = json!({"type": "object", "properties": {
            "limit": {"type": "integer"},
            "values": {"type": "array", "items": {"type": ["integer", "null"]}}
        }});
        let mut arguments =
            json!({"limit": 1.0, "values": [-2.0, null, 0.0, 9_223_372_036_854_775_808.0]});
        assert_eq!(validate_arguments(&schema, &mut arguments), Ok(()));
        assert_eq!(arguments["limit"].as_i64(), Some(1));
        assert_eq!(arguments["values"][0].as_i64(), Some(-2));
        assert_eq!(arguments["values"][2].as_i64(), Some(0));
        assert_eq!(arguments["values"][3].as_u64(), Some(1_u64 << 63));
    }

    #[test]
    fn rejects_unrepresentable_integral_floats_and_preserves_integer_extremes() {
        let schema = json!({"type": "integer"});
        for mut invalid in [
            json!(18_446_744_073_709_551_616.0),
            json!(-9_223_372_036_854_777_856.0),
            json!(1e100),
        ] {
            assert_eq!(
                validate_arguments(&schema, &mut invalid),
                Err(INVALID_ARGUMENTS)
            );
        }
        for mut valid in [json!(i64::MIN), json!(i64::MAX), json!(u64::MAX)] {
            let original = valid.clone();
            assert_eq!(validate_arguments(&schema, &mut valid), Ok(()));
            assert_eq!(valid, original);
        }
        let mut lower = json!(-9_223_372_036_854_775_808.0);
        assert_eq!(validate_arguments(&schema, &mut lower), Ok(()));
        assert_eq!(lower.as_i64(), Some(i64::MIN));
    }

    #[test]
    fn compares_integer_and_float_bounds_without_rounding_large_integers() {
        for maximum in [
            json!(9_007_199_254_740_992_u64),
            json!(9_007_199_254_740_992.0),
        ] {
            let schema = json!({"type": "integer", "maximum": maximum});
            assert_eq!(
                validate_arguments(&schema, &mut json!(9_007_199_254_740_993_u64)),
                Err(INVALID_ARGUMENTS)
            );
            assert_eq!(
                validate_arguments(&schema, &mut json!(9_007_199_254_740_992_u64)),
                Ok(())
            );
        }
        assert_eq!(
            validate_arguments(
                &json!({"type": "integer", "maximum": u64::MAX}),
                &mut json!(u64::MAX)
            ),
            Ok(())
        );
        assert_eq!(
            validate_arguments(&json!({"type": "integer", "minimum": 1.5}), &mut json!(1)),
            Err(INVALID_ARGUMENTS)
        );
        assert_eq!(
            validate_arguments(&json!({"type": "integer", "maximum": -1.5}), &mut json!(-1)),
            Err(INVALID_ARGUMENTS)
        );
    }

    #[test]
    fn counts_unicode_characters_and_array_items() {
        let schema = json!({"type": "string", "minLength": 2, "maxLength": 2});
        assert_eq!(validate_arguments(&schema, &mut json!("é界")), Ok(()));
        assert_eq!(
            validate_arguments(&schema, &mut json!("é")),
            Err(INVALID_ARGUMENTS)
        );
        assert_eq!(
            validate_arguments(&schema, &mut json!("é界x")),
            Err(INVALID_ARGUMENTS)
        );
        let schema =
            json!({"type": "array", "minItems": 1, "maxItems": 2, "items": {"type": "boolean"}});
        assert_eq!(
            validate_arguments(&schema, &mut json!([false, true])),
            Ok(())
        );
        for mut invalid in [json!([]), json!([false, true, false]), json!(["false"])] {
            assert_eq!(
                validate_arguments(&schema, &mut invalid),
                Err(INVALID_ARGUMENTS)
            );
        }
    }

    #[test]
    fn enums_and_constants_use_json_numeric_equality() {
        assert_eq!(
            validate_arguments(&json!({"enum": [1]}), &mut json!(1.0)),
            Ok(())
        );
        assert_eq!(
            validate_arguments(&json!({"enum": [1]}), &mut json!(true)),
            Err(INVALID_ARGUMENTS)
        );
        assert_eq!(
            validate_arguments(
                &json!({"const": {"values": [1, null]}}),
                &mut json!({"values": [1.0, null]})
            ),
            Ok(())
        );
        assert_eq!(
            validate_arguments(
                &json!({"type": "string", "enum": ["draft", "submitted"]}),
                &mut json!("DRAFT")
            ),
            Err(INVALID_ARGUMENTS)
        );
    }

    #[test]
    fn handles_any_of_and_exactly_one_of_without_partial_normalization() {
        let schema = json!({"anyOf": [{"type": "integer"}, {"type": "string", "enum": ["all"]}]});
        let mut integral = json!(1.0);
        assert_eq!(validate_arguments(&schema, &mut integral), Ok(()));
        assert_eq!(integral.as_i64(), Some(1));
        assert_eq!(validate_arguments(&schema, &mut json!("all")), Ok(()));
        assert_eq!(
            validate_arguments(&schema, &mut json!(false)),
            Err(INVALID_ARGUMENTS)
        );
        let overlap = json!({"oneOf": [{"type": "integer"}, {"type": "number"}]});
        assert_eq!(
            validate_arguments(&overlap, &mut json!(1.0)),
            Err(INVALID_ARGUMENTS)
        );
        assert_eq!(validate_arguments(&overlap, &mut json!(1.5)), Ok(()));

        let schema = json!({"type": "object", "properties": {
            "a": {"type": "integer"}, "z": {"type": "string", "enum": ["allowed"]}
        }, "required": ["a", "z"]});
        let mut arguments = json!({"a": 1.0, "z": "secret"});
        let original = arguments.clone();
        assert_eq!(
            validate_arguments(&schema, &mut arguments),
            Err(INVALID_ARGUMENTS)
        );
        assert_eq!(arguments, original);
    }

    #[test]
    fn unknown_assertions_fail_closed_in_all_schema_branches() {
        for schema in [
            json!({"type": "string", "pattern": "allowed"}),
            json!({"type": "object", "properties": {"absent": {"unknownRule": true}}}),
            json!({"anyOf": [{}, {"$ref": "https://must-not-fetch.invalid/schema"}]}),
            json!({"oneOf": [{"type": "integer"}, {"type": "string", "maxLenght": 3}]}),
        ] {
            assert_eq!(validate_schema(&schema), Err(INVALID_ARGUMENTS));
            assert_eq!(
                validate_arguments(&schema, &mut json!({})),
                Err(INVALID_ARGUMENTS)
            );
        }
    }

    #[test]
    fn malformed_embedded_schemas_cannot_disable_assertions() {
        for schema in [
            json!({"type": "unknown"}),
            json!({"type": []}),
            json!({"type": ["string", "string"]}),
            json!({"type": ["string", 1]}),
            json!({"properties": []}),
            json!({"required": "secret-key"}),
            json!({"required": ["a", "a"]}),
            json!({"additionalProperties": 1}),
            json!({"enum": []}),
            json!({"enum": [1, 1.0]}),
            json!({"minimum": "0"}),
            json!({"maximum": true}),
            json!({"minLength": -1}),
            json!({"maxLength": 1.5}),
            json!({"items": [{"type": "string"}]}),
            json!({"minItems": false}),
            json!({"maxItems": null}),
            json!({"anyOf": []}),
            json!({"oneOf": {"type": "string"}}),
        ] {
            assert_eq!(validate_schema(&schema), Err(INVALID_ARGUMENTS));
        }
    }

    #[test]
    fn bounded_schema_vocabulary_includes_current_annotations_and_constraints() {
        let schema = json!({
            "title": "Synthetic contract", "description": "Current embedded vocabulary",
            "type": "object", "required": ["count"], "additionalProperties": false,
            "properties": {
                "count": {"type": "integer", "minimum": 0, "maximum": 10, "default": 1},
                "text": {"type": ["string", "null"], "minLength": 1, "maxLength": 50, "format": "date"},
                "flags": {"type": "array", "minItems": 0, "maxItems": 2, "items": {"type": "boolean"}},
                "state": {"enum": ["draft", "submitted"]},
                "phase": {"oneOf": [{"const": "prepare"}, {"const": "commit"}]},
                "extra": {"anyOf": [{"type": "null"}, {"type": "object"}]}
            }
        });
        assert_eq!(validate_schema(&schema), Ok(()));
        assert_eq!(
            validate_arguments(&schema, &mut json!({"count": 1.0})),
            Ok(())
        );
        let named_keyword =
            json!({"type": "object", "properties": {"pattern": {"type": "string"}}});
        assert_eq!(
            validate_arguments(&named_keyword, &mut json!({"pattern": "synthetic"})),
            Ok(())
        );
    }

    #[test]
    fn enforces_typed_additional_properties_and_boolean_schemas() {
        let schema = json!({"type": "object", "additionalProperties": {"type": "integer"}});
        let mut arguments = json!({"dynamic": 1.0});
        assert_eq!(validate_arguments(&schema, &mut arguments), Ok(()));
        assert_eq!(arguments["dynamic"].as_i64(), Some(1));
        assert_eq!(
            validate_arguments(&schema, &mut json!({"dynamic": true})),
            Err(INVALID_ARGUMENTS)
        );
        assert_eq!(validate_arguments(&json!(true), &mut json!([1])), Ok(()));
        assert_eq!(
            validate_arguments(&json!(false), &mut json!([1])),
            Err(INVALID_ARGUMENTS)
        );
    }

    #[test]
    fn excessive_schema_or_data_depth_fails_before_cloning() {
        let mut argument = json!(null);
        let mut schema = json!({"type": "null"});
        for _ in 0..=MAX_DEPTH {
            argument = json!([argument]);
            schema = json!({"type": "array", "items": schema});
        }
        assert_eq!(validate_schema(&schema), Err(INVALID_ARGUMENTS));
        assert_eq!(
            validate_arguments(&json!({}), &mut argument),
            Err(INVALID_ARGUMENTS)
        );
    }

    #[test]
    fn exhausted_branch_budget_cannot_make_one_of_appear_unique() {
        let schema = json!({"oneOf": [
            {},
            {"type": "array", "items": {"anyOf": [{"type": "integer"}, {"type": "null"}]}}
        ]});
        let mut arguments = Value::Array(vec![json!(1.0); 40_000]);
        assert_eq!(
            validate_arguments(&schema, &mut arguments),
            Err(INVALID_ARGUMENTS)
        );
        assert!(
            arguments[0].is_f64(),
            "failed validation must not mutate input"
        );
        let oversized_schema = json!({"enum": (0..MAX_SCHEMA_NODES).collect::<Vec<_>>()});
        assert_eq!(validate_schema(&oversized_schema), Err(INVALID_ARGUMENTS));
        let mut oversized_value = Value::Array(vec![Value::Null; MAX_VALUE_NODES]);
        assert_eq!(
            validate_arguments(&json!({}), &mut oversized_value),
            Err(INVALID_ARGUMENTS)
        );
    }

    #[test]
    fn failures_do_not_disclose_caller_fields_or_values() {
        let mut arguments = json!({"synthetic-secret-key": "synthetic-secret-value"});
        let schema = json!({"type": "object", "additionalProperties": false});
        let error = validate_arguments(&schema, &mut arguments).unwrap_err();
        assert_eq!(error, INVALID_ARGUMENTS);
        assert!(!error.contains("synthetic-secret"));
    }
}
