use crate::cli::{SessionArgs, SessionCommand};
use crate::session::Session;

pub fn run(args: SessionArgs) -> Result<(), Box<dyn std::error::Error>> {
    match args.command {
        None => smart_init(),
        Some(SessionCommand::List) => list(),
        Some(SessionCommand::Show { name }) => show(&name),
        Some(SessionCommand::Clear { name }) => clear(&name),
    }
}

fn smart_init() -> Result<(), Box<dyn std::error::Error>> {
    if let Ok(existing) = std::env::var("OM_SESSION") {
        println!("echo 'Session already active: {}'", existing);
    } else {
        let session_id = Session::generate_id();
        let session = Session::load(&session_id)?;
        session.save()?;
        println!(
            "export OM_SESSION={}; echo 'Session created: {}'",
            session_id, session_id
        );
    }

    Ok(())
}

fn list() -> Result<(), Box<dyn std::error::Error>> {
    let sessions = Session::list_all()?;

    if sessions.is_empty() {
        println!("No sessions found");
    } else {
        println!("Sessions:");
        for name in sessions {
            let active = std::env::var("OM_SESSION")
                .map(|s| s == name)
                .unwrap_or(false);
            let marker = if active { " (active)" } else { "" };
            println!("  {}{}", name, marker);
        }
    }

    Ok(())
}

fn show(name: &str) -> Result<(), Box<dyn std::error::Error>> {
    let session = Session::load(name)?;
    let entries = session.show();

    if entries.is_empty() {
        println!("Session '{}' has no tracked files", name);
    } else {
        println!("Session '{}' tracked files:", name);
        for (path, hash) in entries {
            println!("  {} {}", hash, path);
        }
    }

    Ok(())
}

fn clear(name: &str) -> Result<(), Box<dyn std::error::Error>> {
    Session::clear(name)?;
    println!("Cleared session '{}'", name);

    if let Ok(active) = std::env::var("OM_SESSION") {
        if active == name {
            println!("Note: This was your active session. Run 'unset OM_SESSION' to clear the environment variable.");
        }
    }

    Ok(())
}
