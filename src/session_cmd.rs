use crate::cli::{SessionArgs, SessionCommand};
use crate::session::Session;

pub fn run(args: SessionArgs) -> Result<(), Box<dyn std::error::Error>> {
    match args.command {
        None => smart_init(),
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
