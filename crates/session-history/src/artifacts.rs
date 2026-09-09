use crate::{Error, Result, model::MAX_CAPTURE_BYTES};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File},
    io::{Read, Write},
    path::{Path, PathBuf},
};

pub(crate) fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

pub(crate) struct Artifacts(PathBuf);

impl Artifacts {
    pub fn open(root: &Path) -> Result<Self> {
        fs::create_dir_all(root)?;
        Ok(Self(root.to_owned()))
    }

    pub fn put(&self, bytes: &[u8]) -> Result<String> {
        let id = hash(bytes);
        let path = self.0.join(&id);
        if path.exists() {
            self.get(&id)?;
            return Ok(id);
        }
        let mut temporary = tempfile::NamedTempFile::new_in(&self.0)?;
        temporary.write_all(&zstd::stream::encode_all(bytes, 3)?)?;
        temporary.as_file().sync_all()?;
        match temporary.persist_noclobber(&path) {
            Ok(_) => {}
            Err(error) if error.error.kind() == std::io::ErrorKind::AlreadyExists => {
                self.get(&id)?;
            }
            Err(error) => return Err(Error::Io(error.error)),
        }
        File::open(&self.0)?.sync_all()?;
        Ok(id)
    }

    pub fn get(&self, id: &str) -> Result<Vec<u8>> {
        if id.len() != 64
            || !id
                .bytes()
                .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
        {
            return Err(Error::Invalid("invalid artifact hash".into()));
        }
        let read = || -> std::io::Result<Vec<u8>> {
            let decoder = zstd::stream::read::Decoder::new(File::open(self.0.join(id))?)?;
            let mut bytes = Vec::new();
            decoder
                .take((MAX_CAPTURE_BYTES * 2 + 1) as u64)
                .read_to_end(&mut bytes)?;
            Ok(bytes)
        };
        let bytes = read().map_err(|_| Error::UnavailableArtifact(id.into()))?;
        if bytes.len() > MAX_CAPTURE_BYTES * 2 || hash(&bytes) != id {
            return Err(Error::UnavailableArtifact(id.into()));
        }
        Ok(bytes)
    }
}
