use lazy_static::lazy_static;
use rayon::prelude::*;
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct ScoredFile {
    pub path: String,
    pub score: i32,
    #[allow(dead_code)]
    pub reason: String,
}

lazy_static! {
    static ref PROJECT_FILES: HashMap<&'static str, i32> = {
        let mut m = HashMap::new();
        m.insert("Cargo.toml", 8);
        m.insert("package.json", 8);
        m.insert("go.mod", 8);
        m.insert("pom.xml", 8);
        m.insert("build.gradle", 8);
        m.insert("Dockerfile", 8);
        m.insert("docker-compose.yml", 8);
        m.insert("Makefile", 8);
        m.insert("CMakeLists.txt", 8);
        m.insert("tsconfig.json", 8);
        m.insert("setup.py", 8);
        m.insert("pyproject.toml", 8);
        m
    };
    static ref IMPORTANT_DIRS: Vec<&'static str> =
        vec!["src", "core", "lib", "app", "pkg", "internal", "cmd",];
    static ref DOMAIN_DIRS: Vec<&'static str> = vec![
        "api",
        "server",
        "client",
        "models",
        "services",
        "handlers",
        "controllers",
        "routes",
        "middleware",
        "database",
        "db",
        "auth",
        "components",
        "views",
        "utils",
    ];
    static ref TEST_DIRS: Vec<&'static str> = vec!["test", "tests", "spec", "__tests__",];
    static ref LOW_DIRS: Vec<&'static str> = vec![
        "vendor",
        "third_party",
        "fixtures",
        "mocks",
        "docs",
        "examples",
        "scripts",
        "tools",
        "dist",
        "build",
        "out",
        "target",
        "node_modules",
        "archived",
        "legacy",
        "debug",
        "research",
        "tmp",
        "temp",
        "backup",
        "artifacts",
        ".artifacts",
        "drizzle",
        "migrations",
    ];
}

pub fn score_file(filepath: &str) -> ScoredFile {
    let path = Path::new(filepath);
    let filename = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
    let mut score = 7;
    let mut reasons = Vec::new();

    if filename == "main.rs" || filename == "lib.rs" || filename == "mod.rs" {
        return ScoredFile {
            path: filepath.to_string(),
            score: 10,
            reason: "entry point".to_string(),
        };
    }

    if filename.starts_with("main.")
        || filename.starts_with("index.")
        || filename.starts_with("app.")
        || filename.starts_with("server.")
        || filename.starts_with("cli.")
    {
        return ScoredFile {
            path: filepath.to_string(),
            score: 10,
            reason: "entry point".to_string(),
        };
    }

    if filename == "README.md" || filename == "README" || filename == "README.rst" {
        let mut score = 10;
        let components: Vec<&str> = filepath.split('/').collect();
        for component in components {
            if LOW_DIRS.contains(&component) || TEST_DIRS.contains(&component) {
                score = 5;
                break;
            }
        }
        return ScoredFile {
            path: filepath.to_string(),
            score,
            reason: "readme".to_string(),
        };
    }

    if filename.starts_with("config.") || filename.starts_with("settings.") {
        return ScoredFile {
            path: filepath.to_string(),
            score: 9,
            reason: "config".to_string(),
        };
    }

    if filename.ends_with(".lock")
        || filename.contains("-lock.")
        || filename.contains(".lock.")
        || filename.ends_with(".min.js")
        || filename.ends_with(".min.css")
        || filename.ends_with(".map")
        || filename.ends_with(".d.ts")
        || filename.ends_with(".pyc")
        || filename.contains(".generated.")
        || filename.ends_with(".backup")
        || filename.ends_with(".bak")
        || filename.ends_with(".tmp")
        || filename.ends_with(".sql")
    {
        return ScoredFile {
            path: filepath.to_string(),
            score: 2,
            reason: "generated or insignificant".to_string(),
        };
    }

    if let Some(&project_score) = PROJECT_FILES.get(filename) {
        return ScoredFile {
            path: filepath.to_string(),
            score: project_score,
            reason: "project file".to_string(),
        };
    }

    if filename.starts_with("test_")
        || filename.contains("_test.")
        || filename.contains(".test.")
        || filename.contains(".spec.")
    {
        return ScoredFile {
            path: filepath.to_string(),
            score: 5,
            reason: "test file".to_string(),
        };
    }

    if filename == "__init__.py" {
        return ScoredFile {
            path: filepath.to_string(),
            score: 3,
            reason: "init file".to_string(),
        };
    }

    let components: Vec<&str> = filepath.split('/').collect();
    let depth = components.len() - 1;

    for component in &components[..components.len().saturating_sub(1)] {
        if IMPORTANT_DIRS.contains(component) {
            score += 2;
            reasons.push("important dir");
            break;
        }
    }

    for component in &components[..components.len().saturating_sub(1)] {
        if DOMAIN_DIRS.contains(component) {
            score += 1;
            reasons.push("domain dir");
            break;
        }
    }

    for component in &components[..components.len().saturating_sub(1)] {
        if TEST_DIRS.contains(component) {
            score -= 2;
            reasons.push("test dir");
            break;
        }
    }

    for component in &components[..components.len().saturating_sub(1)] {
        if LOW_DIRS.contains(component) {
            score -= 3;
            reasons.push("low priority dir");
            break;
        }
    }

    if depth == 0 {
        score += 1;
        reasons.push("root level");
    } else if depth > 4 {
        score -= 2;
        reasons.push("deep nesting");
    } else if depth > 2 {
        score -= 1;
        reasons.push("nested");
    }

    let extension = path.extension().and_then(|s| s.to_str()).unwrap_or("");
    if extension == "proto" || extension == "graphql" || extension == "gql" || extension == "thrift"
    {
        score += 1;
        reasons.push("schema file");
    }

    if (extension == "md" || extension == "rst")
        && filename != "README.md"
        && filename != "README.rst"
    {
        score -= 1;
        reasons.push("doc file");
    }

    score = score.clamp(1, 10);

    let reason = if reasons.is_empty() {
        "base score".to_string()
    } else {
        reasons.join(", ")
    };

    ScoredFile {
        path: filepath.to_string(),
        score,
        reason,
    }
}

pub fn score_files(files: Vec<String>) -> Vec<ScoredFile> {
    files.into_par_iter().map(|f| score_file(&f)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn test_score_always_valid(s in "\\PC*") {
            let scored = score_file(&s);
            prop_assert!(scored.score >= 1 && scored.score <= 10);
        }

        #[test]
        fn test_depth_score_consistency(s in "[a-z0-9/]{1,100}") {
            let scored = score_file(&s);
            let depth = s.split('/').count() - 1;
            if depth > 4 {
                prop_assert!(scored.score <= 8);
            }
        }
    }

    #[test]
    fn test_entry_points() {
        assert_eq!(score_file("src/main.rs").score, 10);
        assert_eq!(score_file("src/lib.rs").score, 10);
        assert_eq!(score_file("index.js").score, 10);
        assert_eq!(score_file("app.py").score, 10);
    }

    #[test]
    fn test_readme() {
        assert_eq!(score_file("README.md").score, 10);
        assert_eq!(score_file("docs/README.md").score, 5); // Now 5 because docs is a low priority dir
    }

    #[test]
    fn test_config() {
        assert_eq!(score_file("config.toml").score, 9);
        assert_eq!(score_file("settings.json").score, 9);
    }

    #[test]
    fn test_project_files() {
        assert_eq!(score_file("Cargo.toml").score, 8);
        assert_eq!(score_file("package.json").score, 8);
        assert_eq!(score_file("Dockerfile").score, 8);
    }

    #[test]
    fn test_test_files() {
        assert_eq!(score_file("test_foo.py").score, 5);
        assert_eq!(score_file("foo_test.go").score, 5);
        assert_eq!(score_file("foo.test.ts").score, 5);
    }

    #[test]
    fn test_generated() {
        assert_eq!(score_file("package-lock.json").score, 2);
        assert_eq!(score_file("bundle.min.js").score, 2);
        assert_eq!(score_file("types.generated.ts").score, 2);
    }

    #[test]
    fn test_directory_modifiers() {
        let scored_src = score_file("src/handler.rs");
        let scored_vendor = score_file("vendor/utils.rs");
        assert!(scored_src.score > scored_vendor.score);
    }

    #[test]
    fn test_depth() {
        let score_root = score_file("file.rs").score;
        let score_deep = score_file("a/b/c/d/e/file.rs").score;
        assert!(score_root > score_deep);
    }

    #[test]
    fn test_directory_rules() {
        assert_eq!(score_file("src/foo.rs").score, 9);
        assert_eq!(score_file("core/foo.rs").score, 9);

        assert_eq!(score_file("api/foo.rs").score, 8);

        assert_eq!(score_file("tests/foo.rs").score, 5);

        assert_eq!(score_file("vendor/foo.rs").score, 4);
    }

    #[test]
    fn test_depth_scoring() {
        assert_eq!(score_file("foo.rs").score, 8);
        assert_eq!(score_file("a/foo.rs").score, 7);
        assert_eq!(score_file("a/b/foo.rs").score, 7);
        assert_eq!(score_file("a/b/c/foo.rs").score, 6);
        assert_eq!(score_file("a/b/c/d/foo.rs").score, 6);
        assert_eq!(score_file("a/b/c/d/e/foo.rs").score, 5);
    }

    #[test]
    fn test_schema_files() {
        assert_eq!(score_file("schema.proto").score, 9);
        assert_eq!(score_file("api/schema.graphql").score, 9);
    }

    #[test]
    fn test_doc_files() {
        assert_eq!(score_file("docs.md").score, 7);
        assert_eq!(score_file("README.md").score, 10);
    }
}
