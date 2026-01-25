use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Serialize, Deserialize)]
pub struct Session {
    pub name: String,
    pub files: HashMap<String, String>,
    #[serde(skip)]
    path: PathBuf,
}

impl Session {
    pub fn generate_id() -> String {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        format!("sess-{}", timestamp)
    }

    pub fn load(name: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let sessions_dir = Self::sessions_dir()?;
        let path = sessions_dir.join(format!("{}.json", name));

        if path.exists() {
            let content = fs::read_to_string(&path)?;
            let mut session: Session = serde_json::from_str(&content)?;
            session.path = path;
            Ok(session)
        } else {
            Ok(Session {
                name: name.to_string(),
                files: HashMap::new(),
                path,
            })
        }
    }

    pub fn save(&self) -> Result<(), Box<dyn std::error::Error>> {
        let sessions_dir = Self::sessions_dir()?;
        fs::create_dir_all(&sessions_dir)?;

        let content = serde_json::to_string_pretty(&self)?;
        fs::write(&self.path, content)?;

        Ok(())
    }

    pub fn was_read(&self, path: &str, hash: &str) -> bool {
        self.files.get(path).map(|h| h == hash).unwrap_or(false)
    }

    pub fn mark_read(&mut self, path: &str, hash: &str) {
        self.files.insert(path.to_string(), hash.to_string());
    }

    pub fn compute_hash(content: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(content);
        format!("{:x}", hasher.finalize())
    }

    pub fn clear(name: &str) -> Result<(), Box<dyn std::error::Error>> {
        let sessions_dir = Self::sessions_dir()?;
        let path = sessions_dir.join(format!("{}.json", name));

        if path.exists() {
            fs::remove_file(&path)?;
        }

        Ok(())
    }

    fn sessions_dir() -> Result<PathBuf, Box<dyn std::error::Error>> {
        let home = dirs::home_dir().ok_or("Could not determine home directory")?;
        Ok(home.join(".om").join("sessions"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_hash() {
        let content = b"hello world";
        let hash = Session::compute_hash(content);
        assert_eq!(hash.len(), 64);
    }

    #[test]
    fn test_was_read() {
        let mut session = Session {
            name: "test".to_string(),
            files: HashMap::new(),
            path: PathBuf::from("/tmp/test.json"),
        };

        let hash = "abc123";
        session.mark_read("file.rs", hash);
        assert!(session.was_read("file.rs", hash));
        assert!(!session.was_read("file.rs", "different"));
        assert!(!session.was_read("other.rs", hash));
    }

    #[test]
    fn test_generate_id() {
        let id = Session::generate_id();
        assert!(id.starts_with("sess-"));
    }
}
