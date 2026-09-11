//! Edit only owned JSONC nodes, retaining surrounding settings and comments.
use crate::AppResult;
use jsonc_parser::{
    ParseOptions,
    cst::{CstInputValue, CstRootNode},
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::PathBuf;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Edit {
    pub file: PathBuf,
    pub path: Vec<String>,
    pub value: Value,
    pub array: bool,
}

pub fn contains(text: &str, path: &[&str]) -> AppResult<bool> {
    let root = CstRootNode::parse(text, &ParseOptions::default())?;
    let mut object = root
        .object_value()
        .ok_or("client configuration must be an object")?;
    let (name, parents) = path.split_last().ok_or("empty configuration path")?;
    for parent in parents {
        let Some(property) = object.get(parent) else {
            return Ok(false);
        };
        object = property
            .object_value()
            .ok_or("configuration parent must be an object")?;
    }
    Ok(object.get(name).is_some())
}

fn input(value: &Value) -> CstInputValue {
    match value {
        Value::Null => CstInputValue::Null,
        Value::Bool(value) => CstInputValue::Bool(*value),
        Value::Number(value) => CstInputValue::Number(value.to_string()),
        Value::String(value) => CstInputValue::String(value.clone()),
        Value::Array(values) => CstInputValue::Array(values.iter().map(input).collect()),
        Value::Object(values) => CstInputValue::Object(
            values
                .iter()
                .map(|(key, value)| (key.clone(), input(value)))
                .collect(),
        ),
    }
}

pub fn defaults(text: &str, opencode: bool, hooks: bool) -> AppResult<String> {
    let root = CstRootNode::parse(text, &ParseOptions::default())?;
    let object = root
        .object_value()
        .ok_or("client configuration must be an object")?;
    if hooks {
        match object.get("version") {
            Some(version) if version.to_serde_value() != Some(Value::from(1)) => {
                return Err("unsupported Cursor hooks version; expected version 1".into());
            }
            None => {
                object.append("version", CstInputValue::Number("1".into()));
            }
            _ => {}
        }
    }
    if opencode && object.get("$schema").is_none() {
        object.append(
            "$schema",
            CstInputValue::String("https://opencode.ai/config.json".into()),
        );
    }
    Ok(root.to_string())
}

pub fn apply(text: &str, edit: &Edit, removing: bool, owned: bool) -> AppResult<String> {
    let root = CstRootNode::parse(text, &ParseOptions::default())?;
    let mut object = root
        .object_value()
        .ok_or("client configuration must be an object")?;
    let (name, parents) = edit.path.split_last().ok_or("empty configuration path")?;
    for parent in parents {
        object = match object.get(parent) {
            Some(property) => property
                .object_value()
                .ok_or("configuration parent must be an object")?,
            None if removing => return Ok(text.into()),
            None => object.object_value_or_set(parent),
        };
    }
    if edit.array {
        let property = match object.get(name) {
            Some(property) => property,
            None if removing => return Ok(text.into()),
            None => object.append(name, CstInputValue::Array(Vec::new())),
        };
        let array = property
            .value()
            .and_then(|node| node.as_array())
            .ok_or("configuration hook/plugin must be an array")?;
        if let Some(command) = edit.value.get("command") {
            for node in array.elements() {
                if let Some(value) = node.to_serde_value()
                    && value.get("command") == Some(command)
                    && value != edit.value
                {
                    return Err(
                        format!("owned hook options changed in {}", edit.file.display()).into(),
                    );
                }
            }
        }
        let matches: Vec<_> = array
            .elements()
            .into_iter()
            .filter(|node| node.to_serde_value().as_ref() == Some(&edit.value))
            .collect();
        if !matches.is_empty() && !owned {
            return Err(format!(
                "unowned integration already exists in {}",
                edit.file.display()
            )
            .into());
        }
        if removing {
            for node in matches {
                node.remove();
            }
        } else if matches.is_empty() {
            array.append(input(&edit.value));
        }
    } else if let Some(property) = object.get(name) {
        if property.to_serde_value().as_ref() != Some(&edit.value) || !owned {
            return Err(format!(
                "configuration conflict at {} in {}",
                edit.path.join("."),
                edit.file.display()
            )
            .into());
        }
        if removing {
            property.remove();
        }
    } else if !removing {
        object.append(name, input(&edit.value));
    }
    Ok(root.to_string())
}
