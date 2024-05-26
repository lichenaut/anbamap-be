use std::{env::current_exe, error::Error};

pub async fn get_parent_dir() -> Result<String, Box<dyn Error>> {
    let exe_path = current_exe()?;
    let exe_parent = match exe_path.parent() {
        Some(parent_dir) => parent_dir.display().to_string(),
        None => {
            let err = "Parent directory of executable is None";
            tracing::error!(err);
            return Err(err.into());
        }
    };

    Ok(exe_parent)
}
