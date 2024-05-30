use crate::prelude::*;
use async_std::io::ReadExt;
use async_zip::tokio::read::seek::ZipFileReader;
use reqwest::Client;
use tokio::{
    fs::File,
    io::{AsyncWriteExt, BufReader, BufWriter},
};
use tokio_stream::StreamExt;

pub async fn zip_from_url(client: &Client, url: &str, zip_path: &str) -> Result<()> {
    let response = client.get(url).send().await?;
    let mut stream = response.bytes_stream();
    let file = File::create(zip_path).await?;
    let mut zip_file = BufWriter::new(file);

    while let Some(item) = stream.next().await {
        let item = item?;
        zip_file.write_all(&item).await?;
    }

    zip_file.flush().await?;

    Ok(())
}

pub async fn zip_to_txt(zip_path: &str) -> Result<()> {
    let mut file = BufReader::new(File::open(zip_path).await?);
    let mut zip = ZipFileReader::with_tokio(&mut file).await?;
    let mut reader = zip.reader_with_entry(0).await?;
    let mut buffer = Vec::new();
    reader.read_to_end(&mut buffer).await?;
    let mut file =
        BufWriter::new(File::create(format!("{}.txt", &zip_path[..zip_path.len() - 4])).await?);
    file.write_all(&buffer).await?;
    file.flush().await?;

    Ok(())
}
