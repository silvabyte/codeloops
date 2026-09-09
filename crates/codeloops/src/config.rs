use crate::AppResult;
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, File},
    io::Write,
    net::SocketAddr,
    path::{Path, PathBuf},
};
use uuid::Uuid;

#[derive(Clone, Serialize, Deserialize)]
pub struct Config {
    pub root: PathBuf,
    pub address: SocketAddr,
    pub token: String,
}

pub fn default_root() -> PathBuf {
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| home.join(".local/share"))
        .join("codeloops-history/preview")
}

impl Config {
    pub fn open(root: PathBuf, address: SocketAddr) -> AppResult<Self> {
        if !address.ip().is_loopback() {
            return Err("service address must be loopback".into());
        }
        fs::create_dir_all(&root)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&root, fs::Permissions::from_mode(0o700))?;
        }
        let path = root.join("credential");
        let token = match fs::read_to_string(&path) {
            Ok(token) => token,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                let token = format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple());
                let mut tmp = tempfile::NamedTempFile::new_in(&root)?;
                tmp.write_all(token.as_bytes())?;
                tmp.as_file().sync_all()?;
                match tmp.persist_noclobber(&path) {
                    Ok(_) => {
                        File::open(&root)?.sync_all()?;
                        token
                    }
                    Err(e) if e.error.kind() == std::io::ErrorKind::AlreadyExists => {
                        fs::read_to_string(&path)?
                    }
                    Err(e) => return Err(e.error.into()),
                }
            }
            Err(e) => return Err(e.into()),
        };
        if token.len() != 64 || !token.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err("invalid credential file".into());
        }
        Ok(Self {
            root,
            address,
            token,
        })
    }
    pub fn archive(&self) -> impl AsRef<Path> {
        self.root.join("archive")
    }
    pub fn url(&self, path: &str) -> String {
        format!("http://{}{path}", self.address)
    }
}
