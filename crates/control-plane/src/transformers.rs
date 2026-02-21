//! Transformer Registry and Discovery
//!
//! Provides a registry of available transformers and their capabilities
//! so frontends can discover and use them dynamically

use axum::{Json, response::IntoResponse, http::StatusCode};
use serde::{Deserialize, Serialize};
use serde_json::json;

/// Metadata about a transformer type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransformerMetadata {
    pub id: String,
    pub name: String,
    pub description: String,
    pub version: String,
    pub category: String,
    pub schema: serde_json::Value,
    pub examples: Vec<TransformerExample>,
}

/// Example usage of a transformer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransformerExample {
    pub name: String,
    pub description: String,
    pub input: serde_json::Value,
    pub spec: serde_json::Value,
    pub output: serde_json::Value,
}

/// Registry of available transformers
pub struct TransformerRegistry {
    transformers: Vec<TransformerMetadata>,
}

impl TransformerRegistry {
    pub fn new() -> Self {
        Self {
            transformers: Self::default_transformers(),
        }
    }

    /// Register default JSON transformers
    fn default_transformers() -> Vec<TransformerMetadata> {
        vec![
            // JSON Transformer
            TransformerMetadata {
                id: "json".to_string(),
                name: "JSON Transformer".to_string(),
                description: "Transform JSON data with 11 built-in operations".to_string(),
                version: "1.0.0".to_string(),
                category: "data".to_string(),
                schema: json!({
                    "type": "object",
                    "required": ["type"],
                    "properties": {
                        "type": {
                            "type": "string",
                            "enum": [
                                "select", "map", "filter", "flatten", "group",
                                "rename", "merge", "split", "convert",
                                "conditional", "template"
                            ],
                            "description": "Type of transformation to apply"
                        }
                    },
                    "allOf": [
                        {
                            "if": {"properties": {"type": {"const": "select"}}},
                            "then": {
                                "required": ["fields"],
                                "properties": {
                                    "fields": {
                                        "type": "array",
                                        "items": {"type": "string"},
                                        "description": "Fields to select"
                                    }
                                }
                            }
                        },
                        {
                            "if": {"properties": {"type": {"const": "filter"}}},
                            "then": {
                                "required": ["condition"],
                                "properties": {
                                    "condition": {
                                        "type": "object",
                                        "required": ["field", "op", "value"],
                                        "properties": {
                                            "field": {"type": "string"},
                                            "op": {
                                                "type": "string",
                                                "enum": ["eq", "ne", "gt", "gte", "lt", "lte", "contains", "in"]
                                            },
                                            "value": {}
                                        }
                                    }
                                }
                            }
                        }
                    ]
                }),
                examples: vec![
                    TransformerExample {
                        name: "Select Fields".to_string(),
                        description: "Remove sensitive fields".to_string(),
                        input: json!({
                            "name": "John",
                            "email": "john@example.com",
                            "password": "secret"
                        }),
                        spec: json!({
                            "type": "select",
                            "fields": ["name", "email"]
                        }),
                        output: json!({
                            "name": "John",
                            "email": "john@example.com"
                        }),
                    },
                    TransformerExample {
                        name: "Filter Array".to_string(),
                        description: "Filter by condition".to_string(),
                        input: json!([
                            {"name": "John", "age": 25},
                            {"name": "Jane", "age": 17}
                        ]),
                        spec: json!({
                            "type": "filter",
                            "condition": {
                                "field": "age",
                                "op": "gte",
                                "value": 18
                            }
                        }),
                        output: json!([
                            {"name": "John", "age": 25}
                        ]),
                    },
                ],
            },
        ]
    }

    /// Get all registered transformers
    pub fn list(&self) -> &[TransformerMetadata] {
        &self.transformers
    }

    /// Get transformer by ID
    pub fn get(&self, id: &str) -> Option<&TransformerMetadata> {
        self.transformers.iter().find(|t| t.id == id)
    }

    /// Register a new transformer (for future extensibility)
    pub fn register(&mut self, transformer: TransformerMetadata) {
        // Remove existing transformer with same ID
        self.transformers.retain(|t| t.id != transformer.id);
        self.transformers.push(transformer);
    }
}

impl Default for TransformerRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// API handler to list all transformers
pub async fn list_transformers() -> impl IntoResponse {
    let registry = TransformerRegistry::new();
    Json(json!({
        "transformers": registry.list()
    }))
}

/// API handler to get transformer details
pub async fn get_transformer(
    axum::extract::Path(id): axum::extract::Path<String>,
) -> impl IntoResponse {
    let registry = TransformerRegistry::new();
    
    if let Some(transformer) = registry.get(&id) {
        (StatusCode::OK, Json(json!(transformer)))
    } else {
        (
            StatusCode::NOT_FOUND,
            Json(json!({
                "error": format!("Transformer not found: {}", id)
            }))
        )
    }
}

/// API handler to get transformer capabilities
pub async fn get_transformer_capabilities() -> impl IntoResponse {
    Json(json!({
        "json": {
            "operations": [
                {
                    "type": "select",
                    "description": "Select specific fields from object or array",
                    "input_types": ["object", "array"],
                    "output_type": "same_as_input",
                    "required_fields": ["fields"],
                    "example": {
                        "type": "select",
                        "fields": ["name", "email"]
                    }
                },
                {
                    "type": "map",
                    "description": "Transform array elements using template",
                    "input_types": ["array"],
                    "output_type": "array",
                    "required_fields": ["template"],
                    "example": {
                        "type": "map",
                        "template": {
                            "fullName": "{{firstName}} {{lastName}}"
                        }
                    }
                },
                {
                    "type": "filter",
                    "description": "Filter array by condition",
                    "input_types": ["array"],
                    "output_type": "array",
                    "required_fields": ["condition"],
                    "operators": ["eq", "ne", "gt", "gte", "lt", "lte", "contains", "in"],
                    "example": {
                        "type": "filter",
                        "condition": {
                            "field": "age",
                            "op": "gte",
                            "value": 18
                        }
                    }
                },
                {
                    "type": "rename",
                    "description": "Rename object fields",
                    "input_types": ["object", "array"],
                    "output_type": "same_as_input",
                    "required_fields": ["mapping"],
                    "example": {
                        "type": "rename",
                        "mapping": {
                            "firstName": "first_name"
                        }
                    }
                },
                {
                    "type": "convert",
                    "description": "Convert field types",
                    "input_types": ["object"],
                    "output_type": "object",
                    "required_fields": ["fields"],
                    "supported_types": ["string", "number", "boolean", "array"],
                    "example": {
                        "type": "convert",
                        "fields": {
                            "age": "number",
                            "active": "boolean"
                        }
                    }
                },
                {
                    "type": "flatten",
                    "description": "Flatten nested objects",
                    "input_types": ["object"],
                    "output_type": "object",
                    "optional_fields": ["separator"],
                    "example": {
                        "type": "flatten",
                        "separator": "_"
                    }
                },
                {
                    "type": "group",
                    "description": "Group array by field",
                    "input_types": ["array"],
                    "output_type": "object",
                    "required_fields": ["by"],
                    "example": {
                        "type": "group",
                        "by": "category"
                    }
                },
                {
                    "type": "merge",
                    "description": "Merge multiple objects",
                    "input_types": ["object"],
                    "output_type": "object",
                    "optional_fields": ["sources"],
                    "example": {
                        "type": "merge",
                        "sources": ["{{data1}}", "{{data2}}"]
                    }
                },
                {
                    "type": "split",
                    "description": "Split string field into array",
                    "input_types": ["object"],
                    "output_type": "object",
                    "required_fields": ["field"],
                    "optional_fields": ["delimiter"],
                    "example": {
                        "type": "split",
                        "field": "tags",
                        "delimiter": ","
                    }
                },
                {
                    "type": "conditional",
                    "description": "Conditional transformation (if/then/else)",
                    "input_types": ["object"],
                    "output_type": "object",
                    "required_fields": ["if", "then"],
                    "optional_fields": ["else"],
                    "example": {
                        "type": "conditional",
                        "if": {
                            "field": "age",
                            "op": "gte",
                            "value": 18
                        },
                        "then": {"status": "adult"},
                        "else": {"status": "minor"}
                    }
                },
                {
                    "type": "template",
                    "description": "Apply template with variable substitution",
                    "input_types": ["object"],
                    "output_type": "object",
                    "required_fields": ["template"],
                    "example": {
                        "type": "template",
                        "template": {
                            "fullName": "{{firstName}} {{lastName}}"
                        }
                    }
                }
            ]
        },
        "xml": {
            "status": "planned",
            "operations": []
        },
        "dataweave": {
            "status": "planned",
            "operations": []
        }
    }))
}
