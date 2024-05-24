use async_std::path::PathBuf;
use reqwest::Client;
use std::{error::Error, io::copy};
use tokio::io::AsyncWriteExt;
use tokio_stream::StreamExt;
use zip::ZipArchive;

pub async fn zip_from_url(client: &Client, url: &str, zip_path: &str) -> Result<(), Box<dyn Error>> {
    tracing::info!("Downloading zip file from {} to {}", url, zip_path);

    let response = client.get(url).send().await?;
    let mut stream = response.bytes_stream();
    let mut zip_file = tokio::fs::File::create(zip_path).await?;
    while let Some(item) = stream.next().await {
        let item = item?;
        zip_file.write_all(&item).await?;
    }

    Ok(())
}

pub async fn unzip_files_to(zip_path: &str, parent_dir: &str) -> Result<(), Box<dyn Error>> {
    let file = std::fs::File::open(zip_path)?;
    let mut archive = ZipArchive::new(file)?;
    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        let outpath = match file.enclosed_name() {
            Some(path) => path.to_owned(),
            None => continue,
        };
        
        let mut path = PathBuf::from(parent_dir);
        path.push(outpath);
        let mut outfile = std::fs::File::create(&path)?;
        copy(&mut file, &mut outfile)?;
    }

    Ok(())
}