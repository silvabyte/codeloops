//! Download an immutable export through public queries and publish its completion
//! descriptor last. Verification needs neither the service nor the source checkout.
use crate::{
    AppResult,
    http::Client,
    installation::{atomic_write, hash},
};
use base64::{Engine, engine::general_purpose::STANDARD};
use serde_json::{Value, json};
use session_history::model::{MAX_CAPTURE_BYTES, Query};
use std::{
    fs::{self, File},
    os::unix::fs::PermissionsExt,
    path::Path,
};

fn hash_field<'a>(value: &'a Value, field: &str) -> AppResult<&'a str> {
    let hash = value[field].as_str().ok_or("missing export hash")?;
    validate_hash(hash)?;
    Ok(hash)
}

fn validate_hash(hash: &str) -> AppResult<()> {
    if hash.len() != 64
        || !hash
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err("invalid export hash".into());
    }
    Ok(())
}

async fn download(client: &Client, expected: &str) -> AppResult<Vec<u8>> {
    validate_hash(expected)?;
    let mut bytes = Vec::new();
    loop {
        let offset = bytes.len();
        let chunk = client
            .post(
                "/v1/history/query",
                &Query::Artifact {
                    hash: expected.into(),
                    offset,
                    limit: 65536,
                },
            )
            .await?;
        let total = chunk["total_bytes"]
            .as_u64()
            .ok_or("missing artifact size")?;
        if total > (MAX_CAPTURE_BYTES * 2) as u64 || chunk["offset"] != offset {
            return Err("invalid artifact size or offset".into());
        }
        bytes.extend(STANDARD.decode(chunk["data"].as_str().ok_or("missing artifact data")?)?);
        if bytes.len() as u64 > total {
            return Err("artifact exceeds declared size".into());
        }
        if chunk["next_offset"].is_null() {
            if bytes.len() as u64 != total || hash(&bytes) != expected {
                return Err("export artifact failed hash or length verification".into());
            }
            return Ok(bytes);
        }
        if bytes.len() <= offset || chunk["next_offset"] != bytes.len() {
            return Err("invalid artifact continuation".into());
        }
    }
}

pub async fn download_bundle(
    client: &Client,
    descriptor: Value,
    output: &Path,
) -> AppResult<Value> {
    let manifest_hash = hash_field(&descriptor, "manifest_hash")?;
    let bytes = download(client, manifest_hash).await?;
    let manifest: Value = serde_json::from_slice(&bytes)?;
    let inventory = manifest["artifacts"]
        .as_object()
        .ok_or("missing artifact inventory")?;
    // Refuse existing output, including a previous interrupted export. A completion
    // descriptor appears only once every file has been durably verified.
    fs::create_dir(output)?;
    fs::set_permissions(output, fs::Permissions::from_mode(0o700))?;
    let artifacts = output.join("artifacts");
    fs::create_dir(&artifacts)?;
    for (hash, size) in inventory {
        let content = download(client, hash).await?;
        if size.as_u64() != Some(content.len() as u64) {
            return Err("export inventory length mismatch".into());
        }
        atomic_write(&artifacts.join(hash), &content, false)?;
    }
    File::open(&artifacts)?.sync_all()?;
    atomic_write(&output.join("manifest.json"), &bytes, false)?;
    atomic_write(
        &output.join("export.json"),
        &serde_json::to_vec_pretty(&descriptor)?,
        false,
    )?;
    Ok(json!({
        "exported": true,
        "output": output,
        "descriptor": descriptor,
    }))
}

pub fn verify(output: &Path) -> AppResult<Value> {
    let descriptor: Value = serde_json::from_slice(&fs::read(output.join("export.json"))?)?;
    let bytes = fs::read(output.join("manifest.json"))?;
    if hash(&bytes) != hash_field(&descriptor, "manifest_hash")? {
        return Err("export manifest hash mismatch".into());
    }
    let manifest: Value = serde_json::from_slice(&bytes)?;
    if manifest["schema_version"] != 1 || manifest["format"] != "codeloops-session-export" {
        return Err("unsupported export format".into());
    }
    let inventory = manifest["artifacts"]
        .as_object()
        .ok_or("missing artifact inventory")?;
    for (expected, size) in inventory {
        validate_hash(expected)?;
        let path = output.join("artifacts").join(expected);
        let metadata = fs::symlink_metadata(&path)?;
        if !metadata.is_file()
            || size.as_u64() != Some(metadata.len())
            || metadata.len() > (MAX_CAPTURE_BYTES * 2) as u64
        {
            return Err(format!("invalid export artifact length: {expected}").into());
        }
        if hash(&fs::read(path)?) != *expected {
            return Err(format!("export artifact hash mismatch: {expected}").into());
        }
    }
    for page in manifest["record_pages"]
        .as_array()
        .ok_or("missing record pages")?
    {
        if !inventory.contains_key(hash_field(page, "hash")?) {
            return Err("record page absent from inventory".into());
        }
    }
    Ok(json!({
        "verified": true,
        "descriptor": descriptor,
        "artifact_count": inventory.len(),
    }))
}
