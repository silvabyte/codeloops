//! Edit one owned Codex MCP table while retaining unrelated TOML and comments.
use crate::AppResult;
use serde::{Deserialize, Serialize};
use std::{path::PathBuf, str::FromStr};
use toml_edit::{Array, DocumentMut, Item, Table, value};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Edit {
    pub file: PathBuf,
    pub server_name: String,
    pub command: String,
    pub args: Vec<String>,
}

fn expected(edit: &Edit) -> Item {
    let mut table = Table::new();
    table["command"] = value(&edit.command);
    let mut args = Array::new();
    for argument in &edit.args {
        args.push(argument);
    }
    table["args"] = value(args);
    Item::Table(table)
}

fn matches(existing: &Item, edit: &Edit) -> bool {
    let Some(table) = existing.as_table() else {
        return false;
    };
    if table.len() != 2 || table.get("command").and_then(Item::as_str) != Some(&edit.command) {
        return false;
    }
    let Some(args) = table.get("args").and_then(Item::as_array) else {
        return false;
    };
    args.len() == edit.args.len()
        && args
            .iter()
            .zip(&edit.args)
            .all(|(actual, expected)| actual.as_str() == Some(expected))
}

pub fn apply(text: &str, edit: &Edit, removing: bool, owned: bool) -> AppResult<String> {
    let mut document = DocumentMut::from_str(text)?;
    let expected = expected(edit);
    let Some(servers) = document
        .entry("mcp_servers")
        .or_insert_with(|| Item::Table(Table::new()))
        .as_table_mut()
    else {
        return Err("Codex mcp_servers configuration must be a table".into());
    };

    match servers.get(&edit.server_name) {
        Some(existing) if !matches(existing, edit) || !owned => {
            return Err(format!(
                "configuration conflict at mcp_servers.{} in {}",
                edit.server_name,
                edit.file.display()
            )
            .into());
        }
        Some(_) if removing => {
            servers.remove(&edit.server_name);
        }
        Some(_) => {}
        None if !removing => {
            servers.insert(&edit.server_name, expected);
        }
        None => {}
    }
    if servers.is_empty() {
        document.remove("mcp_servers");
    }
    Ok(document.to_string())
}
