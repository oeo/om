use glob::Pattern;
use std::fs;
use std::path::Path;

pub struct IgnorePatterns {
    patterns: Vec<Pattern>,
}

impl IgnorePatterns {
    pub fn load(root: &Path) -> Self {
        let mut patterns = Vec::new();

        if let Some(home) = dirs::home_dir() {
            let global_ignore = home.join(".omignore");
            if let Ok(ps) = Self::parse_file(&global_ignore) {
                patterns.extend(ps);
            }
        }

        let local_ignore = root.join(".omignore");
        if let Ok(ps) = Self::parse_file(&local_ignore) {
            patterns.extend(ps);
        }

        IgnorePatterns { patterns }
    }

    fn parse_file(path: &Path) -> Result<Vec<Pattern>, Box<dyn std::error::Error>> {
        let content = fs::read_to_string(path)?;
        let mut patterns = Vec::new();

        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            let pattern = if line.starts_with("**/") {
                line.to_string()
            } else if line.ends_with('/') {
                format!("{}**", line)
            } else if line.contains('*') || line.contains('?') {
                format!("**/{}", line)
            } else {
                line.to_string()
            };

            if let Ok(p) = Pattern::new(&pattern) {
                patterns.push(p);
            }
        }

        Ok(patterns)
    }

    pub fn is_ignored(&self, path: &str) -> bool {
        self.patterns.iter().any(|p| p.matches(path))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    proptest! {
        #[test]
        fn test_is_ignored_never_panics(s in "\\PC*") {
            let patterns = vec![Pattern::new("**/node_modules/**").unwrap()];
            let ignore = IgnorePatterns { patterns };
            ignore.is_ignored(&s);
        }
    }

    #[test]
    fn test_pattern_matching() {
        let patterns = vec![
            Pattern::new("**/*.lock").unwrap(),
            Pattern::new("**/*-lock.*").unwrap(),
            Pattern::new("**/node_modules/**").unwrap(),
            Pattern::new("dist/**").unwrap(),
        ];

        let ignore = IgnorePatterns { patterns };

        assert!(ignore.is_ignored("package-lock.json"));
        assert!(ignore.is_ignored("Cargo.lock"));
        assert!(ignore.is_ignored("src/node_modules/foo/bar.js"));
        assert!(ignore.is_ignored("dist/bundle.js"));
        assert!(!ignore.is_ignored("src/main.rs"));
    }

    #[test]
    fn test_parse_file_logic() {
        let mut tmp = NamedTempFile::new().unwrap();
        writeln!(tmp, "# comment").unwrap();
        writeln!(tmp, "  ").unwrap();
        writeln!(tmp, "target/").unwrap();
        writeln!(tmp, "*.log").unwrap();
        writeln!(tmp, "foo.txt").unwrap();
        writeln!(tmp, "**/bar/*").unwrap();

        let patterns = IgnorePatterns::parse_file(tmp.path()).unwrap();
        let ignore = IgnorePatterns { patterns };

        assert!(ignore.is_ignored("target/debug/exe"));
        assert!(ignore.is_ignored("some/path/test.log"));
        assert!(ignore.is_ignored("foo.txt"));
        assert!(ignore.is_ignored("a/bar/b"));
        assert!(!ignore.is_ignored("src/main.rs"));

        assert_eq!(ignore.patterns.len(), 4);
    }

    #[test]
    fn test_directory_pattern_expansion() {
        let mut tmp = NamedTempFile::new().unwrap();
        writeln!(tmp, "dir/").unwrap();

        let patterns = IgnorePatterns::parse_file(tmp.path()).unwrap();
        let ignore = IgnorePatterns { patterns };

        assert!(ignore.is_ignored("dir/file.txt"));
        assert!(ignore.is_ignored("dir/subdir/file.txt"));
    }
}
