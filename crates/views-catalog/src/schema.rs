//! JSON Schema for the tree form of a view, generated from the catalog.
//!
//! The tree form is the node map written as nested JSON: `{"type": "Slider", "props": {...},
//! "children": [...]}`. A literal prop is a JSON string; an expression is `{"$expr": "..."}`;
//! text children are strings. This is the shape an A2UI adapter maps to and from.

use serde_json::{Map, Value, json};

use crate::{COMPONENTS, Children, Type, UNIVERSAL, VERSION, tokens};

pub fn json_schema() -> Value {
    let mut defs = Map::new();
    defs.insert("expr".into(), json!({
        "type": "object", "required": ["$expr"], "additionalProperties": false,
        "properties": {"$expr": {"type": "string", "description": "Expression source, as written between braces."}}
    }));
    defs.insert("text".into(), json!({"type": "string"}));
    let mut any_node = Vec::new();
    for c in COMPONENTS {
        let mut props = Map::new();
        let mut required = Vec::new();
        for p in c.props.iter().chain(UNIVERSAL.iter()) {
            props.insert(p.name.into(), prop_schema(p.ty, p.literal_only, p.doc));
            if p.required {
                required.push(p.name);
            }
        }
        let children = match c.children {
            Children::None => json!({"type": "array", "maxItems": 0}),
            Children::Text => {
                json!({"type": "array", "items": {"anyOf": [{"$ref": "#/$defs/text"}, {"$ref": "#/$defs/expr"}]}})
            }
            Children::Any => json!({"type": "array", "items": {"$ref": "#/$defs/child"}}),
            Children::Only(tags) => {
                json!({"type": "array", "items": {"anyOf": tags.iter().map(|t| json!({"$ref": format!("#/$defs/{t}")})).collect::<Vec<_>>()}})
            }
        };
        defs.insert(c.tag.into(), json!({
            "type": "object",
            "description": c.doc,
            "x-category": c.category.name(),
            "x-role": c.role,
            "required": ["type"],
            "additionalProperties": false,
            "properties": {
                "type": {"const": c.tag},
                "props": {"type": "object", "properties": props, "required": required, "additionalProperties": false},
                "children": children,
            }
        }));
        if c.tag != "View" && c.parent.is_none() {
            any_node.push(json!({"$ref": format!("#/$defs/{}", c.tag)}));
        }
    }
    any_node.push(json!({"$ref": "#/$defs/text"}));
    any_node.push(json!({"$ref": "#/$defs/expr"}));
    defs.insert("child".into(), json!({"anyOf": any_node}));
    json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "$id": format!("isonomic:views/catalog/{VERSION}"),
        "title": format!("Isonomic View (catalog {VERSION})"),
        "description": "A view in tree form. Generated from the catalog; do not edit by hand.",
        "$ref": "#/$defs/View",
        "$defs": defs,
    })
}

fn prop_schema(ty: Type, literal_only: bool, doc: &str) -> Value {
    let lit = match ty {
        Type::Text | Type::Asset | Type::Class => json!({"type": "string"}),
        Type::Number => json!({"type": "number"}),
        Type::Bool => json!({"type": "boolean"}),
        Type::Enum(v) => json!({"enum": v}),
        Type::Tone => json!({"enum": tokens::TONES}),
        Type::Color => json!({"enum": tokens::PALETTE}),
        Type::Icon => json!({"enum": tokens::ICONS}),
        Type::Date => json!({"type": "string", "pattern": "^\\d{4}-\\d{2}-\\d{2}$"}),
        Type::List | Type::Record | Type::Any | Type::Bind(_) | Type::Action => Value::Null,
    };
    let expr = json!({"$ref": "#/$defs/expr"});
    let mut s = match (lit.is_null(), literal_only) {
        (true, _) => expr,
        (false, true) => lit,
        (false, false) => json!({"anyOf": [lit, expr]}),
    };
    if let Value::Object(m) = &mut s {
        m.insert("description".into(), json!(doc));
        m.insert("x-type".into(), json!(ty.describe()));
    }
    s
}
