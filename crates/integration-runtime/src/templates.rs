use serde_json::Value;

/// Recursively resolve `{{path.to.value}}` templates in a JSON value using
/// `context` as the lookup source.
pub fn resolve_templates(value: &Value, context: &Value) -> Value {
    match value {
        Value::String(s) => resolve_template_str(s, context),
        Value::Array(arr) => Value::Array(arr.iter().map(|v| resolve_templates(v, context)).collect()),
        Value::Object(obj) => {
            let mut map = serde_json::Map::new();
            for (k, v) in obj {
                map.insert(k.clone(), resolve_templates(v, context));
            }
            Value::Object(map)
        }
        other => other.clone(),
    }
}

/// Resolve templates within a single string.
/// If the *entire* string is one `{{expr}}`, the typed JSON value is returned
/// (preserving numbers, booleans, etc.). Otherwise string interpolation is
/// performed and a `Value::String` is returned.
pub fn resolve_template_str(s: &str, context: &Value) -> Value {
    let trimmed = s.trim();
    if trimmed.starts_with("{{") && trimmed.ends_with("}}") && trimmed.len() > 4 {
        let inner = trimmed[2..trimmed.len() - 2].trim();
        if !inner.contains("{{") {
            if let Some(v) = resolve_expr(inner, context) {
                return v;
            } else {
                return Value::Null;
            }
        }
    }

    let mut result = String::with_capacity(s.len());
    let mut remaining = s;
    while let Some(start) = remaining.find("{{") {
        result.push_str(&remaining[..start]);
        remaining = &remaining[start + 2..];
        if let Some(end) = remaining.find("}}") {
            let expr = remaining[..end].trim();
            let replacement = resolve_expr(expr, context)
                .map(|v| match v {
                    Value::String(s) => s,
                    Value::Null => String::new(),
                    other => other.to_string(),
                })
                .unwrap_or_default();
            result.push_str(&replacement);
            remaining = &remaining[end + 2..];
        } else {
            result.push_str("{{");
        }
    }
    result.push_str(remaining);
    Value::String(result)
}

/// Resolve a template expression that may include a `|| default` fallback.
pub fn resolve_expr(expr: &str, context: &Value) -> Option<Value> {
    if let Some(pipe) = expr.find("||") {
        let path = expr[..pipe].trim();
        let default_raw = expr[pipe + 2..].trim().trim_matches('\'').trim_matches('"');
        lookup_path(path, context)
            .map(|v| v.clone())
            .or_else(|| Some(Value::String(default_raw.to_string())))
    } else {
        lookup_path(expr, context).map(|v| v.clone())
    }
}

/// Walk a dotted path (e.g. `"trigger.query_params.name"`) through a JSON value.
pub fn lookup_path<'a>(path: &str, context: &'a Value) -> Option<&'a Value> {
    let mut current = context;
    for part in path.split('.') {
        let part = part.trim();
        if part.is_empty() {
            return None;
        }
        current = current.get(part)?;
    }
    Some(current)
}

/// Evaluate a simple condition expression against the current payload context.
///
/// Supported forms after template resolution:
///   - `"true"` / `"false"`
///   - `"null"` / `"EOF"` → false
///   - `"<a> != <b>"`, `"<a> == <b>"`
pub fn evaluate_condition(condition: &str, context: &Value) -> bool {
    let resolved = match resolve_template_str(condition, context) {
        Value::String(s) => s,
        Value::Bool(b) => return b,
        Value::Null => return false,
        other => other.to_string(),
    };

    match resolved.trim() {
        "true" => true,
        "false" | "null" | "EOF" => false,
        expr if expr.contains("!=") => {
            let mut parts = expr.splitn(2, "!=").map(|s| s.trim());
            match (parts.next(), parts.next()) {
                (Some(a), Some(b)) => a != b,
                _ => false,
            }
        }
        expr if expr.contains("==") => {
            let mut parts = expr.splitn(2, "==").map(|s| s.trim());
            match (parts.next(), parts.next()) {
                (Some(a), Some(b)) => a == b,
                _ => false,
            }
        }
        other => {
            tracing::warn!("evaluate_condition: unsupported expression '{}'", other);
            false
        }
    }
}
