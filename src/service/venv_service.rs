use crate::prelude::*;
use anyhow::anyhow;
use std::{fs::write, path::Path, process::Command};

pub async fn create_venv(docker_volume: &str) -> Result<()> {
    let venv_path = format!("{}/p3venv", docker_volume);
    let venv_path = Path::new(&venv_path);
    if venv_path.exists() {
        update_flashgeotext(docker_volume).await?;
        return Ok(());
    }

    let venv_cmd = Command::new("python3")
        .arg("-m")
        .arg("venv")
        .arg(venv_path)
        .output()?;

    if venv_cmd.status.success() {
        update_flashgeotext(docker_volume).await?;
    } else {
        let err = format!("Failed to create venv: {:?}", venv_cmd.stderr);
        tracing::error!(err);
        return Err(anyhow!(err));
    }

    write(
        format!("{}/media_to_regions.py", docker_volume),
        "from flashgeotext.geotext import GeoText

geotext = GeoText()

def get_regions(text):
    result = geotext.extract(input_text=text)
    regions = list(result['countries'].keys())
    return regions",
    )?;

    write(
        format!("{}/url_to_body.py", docker_volume),
        "from newspaper import Article

def get_body(url):
    article = Article(url)
    article.download()
    article.parse()
    return article.text",
    )?;

    Ok(())
}

async fn update_flashgeotext(docker_volume: &str) -> Result<()> {
    let flashgeotext_update = Command::new(format!("{}/p3venv/bin/python", docker_volume))
        .arg("-m")
        .arg("pip")
        .arg("install")
        .arg("--upgrade")
        .arg("flashgeotext")
        .arg("newspaper3k")
        .arg("lxml_html_clean")
        .output()?;

    if flashgeotext_update.status.success() {
        Ok(())
    } else {
        let err = format!(
            "Failed to update flashgeotext and newspaper3k: {:?}",
            flashgeotext_update.stderr
        );
        tracing::error!(err);
        Err(anyhow!(err))
    }
}
