//! Data Transformation Engine
//!
//! Provides JSON transformation capabilities for flow steps including:
//! - JSONPath queries
//! - Field mapping and renaming
//! - Value transformations
//! - Array operations (map, filter, reduce)
//! - Conditional transformations
//! - Type conversions

use serde_json::{json, Value};
use common::{Error, Result};
use std::collections::HashMap;

/// Transform engine for JSON data manipulation
pub struct TransformEngine;

impl TransformEngine {
    /// Execute a transformation spec on input data
    pub fn transform(input: &Value, spec: &Value) -> Result<Value> {
        match spec {
            Value::Object(map) if map.contains_key("type") => {
                let transform_type = map["type"].as_str()
                    .ok_or_else(|| Error::Transform("Invalid transform type".into()))?;

                match transform_type {
                    "select" => Self::select_transform(input, spec),
                    "map" => Self::map_transform(input, spec),
                    "filter" => Self::filter_transform(input, spec),
                    "flatten" => Self::flatten_transform(input, spec),
                    "group" => Self::group_transform(input, spec),
                    "rename" => Self::rename_transform(input, spec),
                    "merge" => Self::merge_transform(input, spec),
                    "split" => Self::split_transform(input, spec),
                    "convert" => Self::convert_transform(input, spec),
                    "conditional" => Self::conditional_transform(input, spec),
                    "template" => Self::template_transform(input, spec),
                    _ => Err(Error::Transform(format!("Unknown transform type: {}", transform_type)))
                }
            }
            _ => Err(Error::Transform("Transform spec must be an object with 'type' field".into()))
        }
    }

    /// Select specific fields from input
    /// Example: {"type": "select", "fields": ["name", "email", "age"]}
    fn select_transform(input: &Value, spec: &Value) -> Result<Value> {
        let fields = spec["fields"].as_array()
            .ok_or_else(|| Error::Transform("'fields' must be an array".into()))?;

        // Handle array input - apply select to each element
        if let Value::Array(items) = input {
            let mut results = Vec::new();
            for item in items {
                if let Value::Object(input_map) = item {
                    let mut result = serde_json::Map::new();
                    for field in fields {
                        if let Some(field_name) = field.as_str() {
                            if let Some(value) = input_map.get(field_name) {
                                result.insert(field_name.to_string(), value.clone());
                            }
                        }
                    }
                    results.push(Value::Object(result));
                }
            }
            return Ok(Value::Array(results));
        }

        // Handle object input
        let mut result = serde_json::Map::new();

        if let Value::Object(input_map) = input {
            for field in fields {
                if let Some(field_name) = field.as_str() {
                    if let Some(value) = input_map.get(field_name) {
                        result.insert(field_name.to_string(), value.clone());
                    }
                }
            }
        }

        Ok(Value::Object(result))
    }

    /// Map array elements using a template
    /// Example: {"type": "map", "template": {"id": "{{id}}", "fullName": "{{firstName}} {{lastName}}"}}
    fn map_transform(input: &Value, spec: &Value) -> Result<Value> {
        if let Value::Array(items) = input {
            let template = &spec["template"];
            let mut results = Vec::new();

            for item in items {
                let transformed = Self::apply_template(item, template)?;
                results.push(transformed);
            }

            Ok(Value::Array(results))
        } else {
            Err(Error::Transform("Input must be an array for map transform".into()))
        }
    }

    /// Filter array elements based on condition
    /// Example: {"type": "filter", "condition": {"field": "age", "op": "gte", "value": 18}}
    fn filter_transform(input: &Value, spec: &Value) -> Result<Value> {
        if let Value::Array(items) = input {
            let condition = &spec["condition"];
            let mut results = Vec::new();

            for item in items {
                if Self::evaluate_condition(item, condition)? {
                    results.push(item.clone());
                }
            }

            Ok(Value::Array(results))
        } else {
            Err(Error::Transform("Input must be an array for filter transform".into()))
        }
    }

    /// Flatten nested objects
    /// Example: {"type": "flatten", "separator": "_"}
    fn flatten_transform(input: &Value, spec: &Value) -> Result<Value> {
        let separator = spec.get("separator")
            .and_then(|v| v.as_str())
            .unwrap_or(".");

        let mut result = serde_json::Map::new();
        Self::flatten_recursive(input, "", separator, &mut result);

        Ok(Value::Object(result))
    }

    fn flatten_recursive(
        value: &Value,
        prefix: &str,
        separator: &str,
        result: &mut serde_json::Map<String, Value>,
    ) {
        match value {
            Value::Object(map) => {
                for (key, val) in map {
                    let new_key = if prefix.is_empty() {
                        key.clone()
                    } else {
                        format!("{}{}{}", prefix, separator, key)
                    };
                    Self::flatten_recursive(val, &new_key, separator, result);
                }
            }
            _ => {
                result.insert(prefix.to_string(), value.clone());
            }
        }
    }

    /// Group array by field
    /// Example: {"type": "group", "by": "category"}
    fn group_transform(input: &Value, spec: &Value) -> Result<Value> {
        if let Value::Array(items) = input {
            let group_by = spec["by"].as_str()
                .ok_or_else(|| Error::Transform("'by' field required for group".into()))?;

            let mut groups: HashMap<String, Vec<Value>> = HashMap::new();

            for item in items {
                if let Value::Object(obj) = item {
                    if let Some(group_val) = obj.get(group_by) {
                        let key = group_val.to_string();
                        groups.entry(key).or_insert_with(Vec::new).push(item.clone());
                    }
                }
            }

            let mut result = serde_json::Map::new();
            for (key, values) in groups {
                result.insert(key, Value::Array(values));
            }

            Ok(Value::Object(result))
        } else {
            Err(Error::Transform("Input must be an array for group transform".into()))
        }
    }

    /// Rename fields
    /// Example: {"type": "rename", "mapping": {"oldName": "newName", "age": "userAge"}}
    fn rename_transform(input: &Value, spec: &Value) -> Result<Value> {
        if let Value::Object(input_map) = input {
            let mapping = spec["mapping"].as_object()
                .ok_or_else(|| Error::Transform("'mapping' must be an object".into()))?;

            let mut result = serde_json::Map::new();

            for (old_key, value) in input_map {
                let new_key = mapping.get(old_key)
                    .and_then(|v| v.as_str())
                    .unwrap_or(old_key);
                result.insert(new_key.to_string(), value.clone());
            }

            Ok(Value::Object(result))
        } else {
            Err(Error::Transform("Input must be an object for rename transform".into()))
        }
    }

    /// Merge multiple objects
    /// Example: {"type": "merge", "sources": ["{{data1}}", "{{data2}}"]}
    fn merge_transform(input: &Value, spec: &Value) -> Result<Value> {
        let mut result = serde_json::Map::new();

        // Merge input first
        if let Value::Object(map) = input {
            result.extend(map.clone());
        }

        // Merge additional sources
        if let Some(sources) = spec.get("sources").and_then(|v| v.as_array()) {
            for source in sources {
                if let Value::Object(map) = source {
                    result.extend(map.clone());
                }
            }
        }

        Ok(Value::Object(result))
    }

    /// Split string field into array
    /// Example: {"type": "split", "field": "tags", "delimiter": ","}
    fn split_transform(input: &Value, spec: &Value) -> Result<Value> {
        let field = spec["field"].as_str()
            .ok_or_else(|| Error::Transform("'field' required for split".into()))?;
        let delimiter = spec.get("delimiter")
            .and_then(|v| v.as_str())
            .unwrap_or(",");

        if let Value::Object(mut obj) = input.clone() {
            if let Some(Value::String(text)) = obj.get(field) {
                let parts: Vec<Value> = text.split(delimiter)
                    .map(|s| Value::String(s.trim().to_string()))
                    .collect();
                obj.insert(field.to_string(), Value::Array(parts));
            }
            Ok(Value::Object(obj))
        } else {
            Err(Error::Transform("Input must be an object for split transform".into()))
        }
    }

    /// Convert field types
    /// Example: {"type": "convert", "fields": {"age": "number", "active": "boolean"}}
    fn convert_transform(input: &Value, spec: &Value) -> Result<Value> {
        let conversions = spec["fields"].as_object()
            .ok_or_else(|| Error::Transform("'fields' must be an object".into()))?;

        if let Value::Object(mut obj) = input.clone() {
            for (field, target_type) in conversions {
                if let Some(value) = obj.get(field).cloned() {
                    let converted = Self::convert_value(&value, target_type.as_str().unwrap_or("string"))?;
                    obj.insert(field.clone(), converted);
                }
            }
            Ok(Value::Object(obj))
        } else {
            Err(Error::Transform("Input must be an object for convert transform".into()))
        }
    }

    fn convert_value(value: &Value, target_type: &str) -> Result<Value> {
        match target_type {
            "string" => Ok(Value::String(value.to_string())),
            "number" => {
                if let Some(n) = value.as_f64() {
                    Ok(json!(n))
                } else if let Some(s) = value.as_str() {
                    s.parse::<f64>()
                        .map(|n| json!(n))
                        .map_err(|_| Error::Transform(format!("Cannot convert '{}' to number", s)))
                } else {
                    Err(Error::Transform("Cannot convert to number".into()))
                }
            }
            "boolean" => {
                match value {
                    Value::Bool(b) => Ok(Value::Bool(*b)),
                    Value::String(s) => {
                        match s.to_lowercase().as_str() {
                            "true" | "1" | "yes" => Ok(Value::Bool(true)),
                            "false" | "0" | "no" => Ok(Value::Bool(false)),
                            _ => Err(Error::Transform(format!("Cannot convert '{}' to boolean", s)))
                        }
                    }
                    Value::Number(n) => Ok(Value::Bool(n.as_f64().unwrap_or(0.0) != 0.0)),
                    _ => Err(Error::Transform("Cannot convert to boolean".into()))
                }
            }
            "array" => {
                if value.is_array() {
                    Ok(value.clone())
                } else {
                    Ok(Value::Array(vec![value.clone()]))
                }
            }
            _ => Err(Error::Transform(format!("Unknown target type: {}", target_type)))
        }
    }

    /// Conditional transformation
    /// Example: {"type": "conditional", "if": {"field": "age", "op": "gte", "value": 18}, 
    ///           "then": {"status": "adult"}, "else": {"status": "minor"}}
    fn conditional_transform(input: &Value, spec: &Value) -> Result<Value> {
        let condition = &spec["if"];
        let then_value = &spec["then"];
        let else_value = spec.get("else");

        if Self::evaluate_condition(input, condition)? {
            Self::apply_template(input, then_value)
        } else if let Some(else_val) = else_value {
            Self::apply_template(input, else_val)
        } else {
            Ok(input.clone())
        }
    }

    /// Template-based transformation
    /// Example: {"type": "template", "template": {"fullName": "{{firstName}} {{lastName}}", "age": "{{age}}"}}
    fn template_transform(input: &Value, spec: &Value) -> Result<Value> {
        let template = &spec["template"];
        Self::apply_template(input, template)
    }

    /// Apply template with variable substitution
    fn apply_template(input: &Value, template: &Value) -> Result<Value> {
        match template {
            Value::String(s) => {
                let result = Self::substitute_variables(s, input);
                Ok(Value::String(result))
            }
            Value::Object(map) => {
                let mut result = serde_json::Map::new();
                for (key, val) in map {
                    result.insert(key.clone(), Self::apply_template(input, val)?);
                }
                Ok(Value::Object(result))
            }
            Value::Array(arr) => {
                let mut result = Vec::new();
                for item in arr {
                    result.push(Self::apply_template(input, item)?);
                }
                Ok(Value::Array(result))
            }
            _ => Ok(template.clone())
        }
    }

    /// Substitute variables like {{fieldName}} in string
    fn substitute_variables(template: &str, data: &Value) -> String {
        let mut result = template.to_string();
        
        // Find all {{variable}} patterns
        let re = regex::Regex::new(r"\{\{([^}]+)\}\}").unwrap();
        
        for cap in re.captures_iter(template) {
            let full_match = &cap[0];
            let var_name = cap[1].trim();
            
            if let Value::Object(map) = data {
                if let Some(value) = map.get(var_name) {
                    let replacement = match value {
                        Value::String(s) => s.clone(),
                        Value::Number(n) => n.to_string(),
                        Value::Bool(b) => b.to_string(),
                        _ => value.to_string()
                    };
                    result = result.replace(full_match, &replacement);
                }
            }
        }
        
        result
    }

    /// Evaluate a condition
    fn evaluate_condition(data: &Value, condition: &Value) -> Result<bool> {
        let field = condition["field"].as_str()
            .ok_or_else(|| Error::Transform("Condition must have 'field'".into()))?;
        let op = condition["op"].as_str()
            .ok_or_else(|| Error::Transform("Condition must have 'op'".into()))?;
        let expected = &condition["value"];

        let actual = if let Value::Object(map) = data {
            map.get(field)
        } else {
            None
        };

        if actual.is_none() {
            return Ok(false);
        }

        let actual = actual.unwrap();

        match op {
            "eq" => Ok(actual == expected),
            "ne" => Ok(actual != expected),
            "gt" => Self::compare_values(actual, expected, |a, b| a > b),
            "gte" => Self::compare_values(actual, expected, |a, b| a >= b),
            "lt" => Self::compare_values(actual, expected, |a, b| a < b),
            "lte" => Self::compare_values(actual, expected, |a, b| a <= b),
            "contains" => {
                if let (Value::String(s), Value::String(needle)) = (actual, expected) {
                    Ok(s.contains(needle.as_str()))
                } else {
                    Ok(false)
                }
            }
            "in" => {
                if let Value::Array(arr) = expected {
                    Ok(arr.contains(actual))
                } else {
                    Ok(false)
                }
            }
            _ => Err(Error::Transform(format!("Unknown operator: {}", op)))
        }
    }

    fn compare_values<F>(a: &Value, b: &Value, op: F) -> Result<bool>
    where
        F: Fn(f64, f64) -> bool,
    {
        let a_num = a.as_f64().ok_or_else(|| Error::Transform("Cannot compare non-numeric values".into()))?;
        let b_num = b.as_f64().ok_or_else(|| Error::Transform("Cannot compare non-numeric values".into()))?;
        Ok(op(a_num, b_num))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_select_transform() {
        let input = json!({
            "name": "John",
            "email": "john@example.com",
            "age": 30,
            "password": "secret"
        });

        let spec = json!({
            "type": "select",
            "fields": ["name", "email", "age"]
        });

        let result = TransformEngine::transform(&input, &spec).unwrap();
        assert_eq!(result["name"], "John");
        assert_eq!(result["email"], "john@example.com");
        assert!(result.get("password").is_none());
    }

    #[test]
    fn test_rename_transform() {
        let input = json!({
            "firstName": "John",
            "lastName": "Doe",
            "age": 30
        });

        let spec = json!({
            "type": "rename",
            "mapping": {
                "firstName": "first_name",
                "lastName": "last_name"
            }
        });

        let result = TransformEngine::transform(&input, &spec).unwrap();
        assert_eq!(result["first_name"], "John");
        assert_eq!(result["last_name"], "Doe");
        assert_eq!(result["age"], 30);
    }

    #[test]
    fn test_filter_transform() {
        let input = json!([
            {"name": "John", "age": 25},
            {"name": "Jane", "age": 17},
            {"name": "Bob", "age": 30}
        ]);

        let spec = json!({
            "type": "filter",
            "condition": {
                "field": "age",
                "op": "gte",
                "value": 18
            }
        });

        let result = TransformEngine::transform(&input, &spec).unwrap();
        let arr = result.as_array().unwrap();
        assert_eq!(arr.len(), 2);
        assert_eq!(arr[0]["name"], "John");
        assert_eq!(arr[1]["name"], "Bob");
    }
}
