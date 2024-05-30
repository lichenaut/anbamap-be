use crate::prelude::*;
use anyhow::anyhow;
use std::env::current_exe;

pub async fn get_parent_dir() -> Result<String> {
    let exe_path = current_exe()?;
    let exe_parent = match exe_path.parent() {
        Some(parent_dir) => parent_dir.display().to_string(),
        None => {
            let err = "Parent directory of executable is None";
            tracing::error!(err);
            return Err(anyhow!(err));
        }
    };

    Ok(exe_parent)
}
