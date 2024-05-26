use std::{error::Error, path::Path, process::Command};

pub async fn create_venv(exe_parent: &str) -> Result<(), Box<dyn Error>> {
    let venv_path = format!("{}/p3venv", exe_parent);
    let venv_path = Path::new(&venv_path);
    if venv_path.exists() {
        update_flashgeotext(exe_parent).await?;
        return Ok(());
    }

    let venv_cmd = Command::new("python3")
        .arg("-m")
        .arg("venv")
        .arg(venv_path)
        .output()?;

    if venv_cmd.status.success() {
        update_flashgeotext(exe_parent).await?;
    } else {
        let err = format!("Failed to create venv: {:?}", venv_cmd.stderr);
        tracing::error!(err);
        return Err(err.into());
    }

    Ok(())
}

async fn update_flashgeotext(exe_parent: &str) -> Result<(), Box<dyn Error>> {
    let flashgeotext_update = Command::new(format!("{}/p3venv/bin/python", exe_parent))
        .arg("-m")
        .arg("pip")
        .arg("install")
        .arg("--upgrade")
        .arg("flashgeotext")
        .output()?;

    if flashgeotext_update.status.success() {
        Ok(())
    } else {
        let err = format!(
            "Failed to update flashgeotext: {:?}",
            flashgeotext_update.stderr
        );
        tracing::error!(err);
        Err(err.into())
    }
}
