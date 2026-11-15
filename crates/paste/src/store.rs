use std::{
    fs,
    io::{self, Read, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

pub const ID_LENGTH: usize = 10;
const ID_ATTEMPTS: usize = 8;
const ALPHABET: &[u8; 62] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";

/// One file per paste, named by its id, within a total size budget.
// ponytail: a flat directory; shard by id prefix if it ever holds millions of pastes.
pub struct Store {
    dir: PathBuf,
    quota: u64,
    used: AtomicU64,
}

pub fn is_valid_id(id: &str) -> bool {
    id.len() == ID_LENGTH && id.bytes().all(|byte| byte.is_ascii_alphanumeric())
}

impl Store {
    /// Opens the directory, refusing new pastes once they total `quota` bytes.
    pub fn open(dir: impl Into<PathBuf>, quota: u64) -> io::Result<Self> {
        let dir = dir.into();
        fs::create_dir_all(&dir)?;
        let mut used = 0;
        for entry in fs::read_dir(&dir)? {
            used += entry?.metadata()?.len();
        }
        Ok(Self {
            dir,
            quota,
            used: AtomicU64::new(used),
        })
    }

    pub fn create(&self, code: &str) -> io::Result<String> {
        let size = code.len() as u64;
        if self.used.load(Ordering::Relaxed).saturating_add(size) > self.quota {
            return Err(io::ErrorKind::StorageFull.into());
        }
        for _ in 0..ID_ATTEMPTS {
            let id = random_id()?;
            let staged = self.dir.join(format!(".{id}.tmp"));
            // A hard link never replaces an existing paste and never exposes a half-written one.
            let linked =
                stage(&staged, code).and_then(|()| fs::hard_link(&staged, self.dir.join(&id)));
            let _ = fs::remove_file(&staged);
            match linked {
                Ok(()) => {
                    self.used.fetch_add(size, Ordering::Relaxed);
                    return Ok(id);
                }
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
                Err(error) => return Err(error),
            }
        }
        Err(io::Error::other("no free paste id"))
    }

    pub fn get(&self, id: &str) -> io::Result<Option<String>> {
        if !is_valid_id(id) {
            return Ok(None);
        }
        match fs::read_to_string(self.dir.join(id)) {
            Ok(code) => Ok(Some(code)),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(error),
        }
    }
}

fn stage(path: &Path, code: &str) -> io::Result<()> {
    let mut file = fs::File::create(path)?;
    file.write_all(code.as_bytes())?;
    file.sync_all()
}

fn random_id() -> io::Result<String> {
    // ponytail: /dev/urandom covers macOS and Linux (the Pi); use getrandom for other targets.
    let mut bytes = [0u8; ID_LENGTH];
    fs::File::open("/dev/urandom")?.read_exact(&mut bytes)?;
    Ok(bytes
        .iter()
        .map(|byte| ALPHABET[usize::from(*byte) % ALPHABET.len()] as char)
        .collect())
}

#[cfg(test)]
#[path = "store_tests.rs"]
mod tests;
