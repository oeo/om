use crate::git;
use serde::{Deserialize, Serialize};
use std::fs;

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct Config {
    pub min_score: Option<i32>,
    pub depth: Option<usize>,
    pub flat: Option<bool>,
    pub no_color: Option<bool>,
    pub git_root: Option<bool>,
    pub level: Option<i32>,
    pub no_headers: Option<bool>,
}

impl Config {
    pub fn merge(&mut self, other: Config) {
        if other.min_score.is_some() {
            self.min_score = other.min_score;
        }
        if other.depth.is_some() {
            self.depth = other.depth;
        }
        if other.flat.is_some() {
            self.flat = other.flat;
        }
        if other.no_color.is_some() {
            self.no_color = other.no_color;
        }
        if other.git_root.is_some() {
            self.git_root = other.git_root;
        }
        if other.level.is_some() {
            self.level = other.level;
        }
        if other.no_headers.is_some() {
            self.no_headers = other.no_headers;
        }
    }
}

pub fn load_config() -> Config {
    let mut config = Config::default();

    // 1. Load global config: ~/.om/config.toml
    if let Some(home_dir) = dirs::home_dir() {
        let global_path = home_dir.join(".om").join("config.toml");
        if let Ok(content) = fs::read_to_string(global_path) {
            if let Ok(global_config) = toml::from_str::<Config>(&content) {
                config.merge(global_config);
            }
        }
    }

    // 2. Load repo config: .om.toml (repo root)
    if let Ok(cwd) = std::env::current_dir() {
        if let Ok(root) = git::repo_root(&cwd) {
            let repo_path = root.join(".om.toml");
            if let Ok(content) = fs::read_to_string(repo_path) {
                if let Ok(repo_config) = toml::from_str::<Config>(&content) {
                    config.merge(repo_config);
                }
            }
        } else {
            // Fallback to current dir if not a git repo
            let repo_path = cwd.join(".om.toml");
            if let Ok(content) = fs::read_to_string(repo_path) {
                if let Ok(repo_config) = toml::from_str::<Config>(&content) {
                    config.merge(repo_config);
                }
            }
        }
    }

    config
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_parsing() {
        let toml_str = r#"
            min_score = 8
            depth = 2
            flat = true
        "#;
        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.min_score, Some(8));
        assert_eq!(config.depth, Some(2));
        assert_eq!(config.flat, Some(true));
    }

    #[test]
    fn test_config_merge() {
        let mut c1 = Config {
            min_score: Some(5),
            depth: Some(1),
            ..Config::default()
        };
        let c2 = Config {
            min_score: Some(8),
            ..Config::default()
        };
        c1.merge(c2);
        assert_eq!(c1.min_score, Some(8));
        assert_eq!(c1.depth, Some(1));
    }
}
