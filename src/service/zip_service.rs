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
    let mut zip_file = BufWriter::new(File::create(zip_path).await?);
    let mut stream = client.get(url).send().await?.bytes_stream();

    while let Some(item) = stream.next().await {
        zip_file.write_all(&item?).await?;
    }
    zip_file.flush().await?;

    Ok(())
}

pub async fn zip_to_txt(zip_path: &str) -> Result<()> {
    let mut zip_reader = BufReader::new(File::open(zip_path).await?);
    let mut zip_reader = ZipFileReader::with_tokio(&mut zip_reader).await?;
    let mut zip_reader = zip_reader.reader_with_entry(0).await?;

    let mut buffer = Vec::new();
    zip_reader.read_to_end(&mut buffer).await?;
    let mut file =
        BufWriter::new(File::create(format!("{}.txt", &zip_path[..zip_path.len() - 4])).await?);
    file.write_all(&buffer).await?;
    file.flush().await?;

    Ok(())
}
