use crate::cli::InitArgs;
use std::fs;
use std::path::PathBuf;

const DEFAULT_OMIGNORE: &str = r#"# Lock files
*.lock
package-lock.json
Cargo.lock
yarn.lock
Gemfile.lock
poetry.lock

# Generated files
*.min.js
*.min.css
*.map
*.d.ts
*.pyc
*.generated.*

# Build output
dist/
build/
out/
target/
.next/
.nuxt/
.vuepress/dist/

# Changelogs and history
CHANGELOG.md
HISTORY.md
NEWS.md

# Editor and IDE
.vscode/
.idea/
*.swp
*.swo
*~

# Vendor and dependencies
vendor/
node_modules/
"#;

pub fn run(args: InitArgs) -> Result<(), Box<dyn std::error::Error>> {
    let path = if args.global {
        let home = dirs::home_dir().ok_or("Could not determine home directory")?;
        home.join(".omignore")
    } else {
        PathBuf::from(".omignore")
    };

    if path.exists() && !args.force {
        return Err(format!(
            "{} already exists. Use --force to overwrite.",
            path.display()
        )
        .into());
    }

    fs::write(&path, DEFAULT_OMIGNORE)?;

    let location = if args.global { "global" } else { "local" };
    println!("Created {} .omignore at {}", location, path.display());

    Ok(())
}
