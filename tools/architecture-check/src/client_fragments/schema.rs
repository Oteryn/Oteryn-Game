//! Small, fail-closed JSON-schema subset used by the checked-in fragment schema.
use serde_json::Value;

pub(super) fn validate(value: &Value, schema: &Value) -> Result<(), String> {
    if let Some(options) = schema["enum"].as_array()
        && !options.contains(value)
    {
        return Err(format!("unexpected value {value}"));
    }
    if let Some(kind) = schema["type"].as_str() {
        let valid = match kind {
            "object" => value.is_object(),
            "array" => value.is_array(),
            "string" => value.as_str().is_some_and(|s| !s.trim().is_empty()),
            "integer" => value.as_u64().is_some(),
            "boolean" => value.is_boolean(),
            _ => return Err(format!("unsupported schema type {kind}")),
        };
        if !valid {
            return Err(format!("expected {kind}, got {value}"));
        }
    }
    if let Some(required) = schema["required"].as_array() {
        for key in required {
            let key = key.as_str().ok_or("schema required key")?;
            if value.get(key).is_none() {
                return Err(format!("missing {key}"));
            }
        }
    }
    if let Some(object) = value.as_object() {
        for (key, value) in object {
            if let Some(child) = schema["properties"].get(key) {
                validate(value, child).map_err(|e| format!("{key}: {e}"))?;
            } else if schema["additionalProperties"] == false {
                return Err(format!("unknown fragment field {key}"));
            } else if schema["additionalProperties"].is_object() {
                validate(value, &schema["additionalProperties"])?;
            }
        }
    }
    if let Some(array) = value.as_array()
        && let Some(child) = schema.get("items")
    {
        for value in array {
            validate(value, child)?;
        }
    }
    Ok(())
}
