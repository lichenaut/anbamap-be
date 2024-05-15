use sqlx::sqlite::SqliteQueryResult;
use sqlx::Executor;
use std::collections::HashSet;
use std::fs;
use std::path::Path;
use std::{fs::File, io, io::BufRead};
use unidecode::unidecode;
use super::db_service::get_region_db_pool;

struct Region {
    region_code: String,
    keyphrases: String,
}

pub async fn gen_sqlite_db() -> Result<(), Box<dyn std::error::Error>> {
    let current_dir = std::env::current_dir()?;
    for entry in fs::read_dir(&current_dir)? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_file() { continue; }

        if let Some(filename) = path.file_name() {
            if let Some(filename) = filename.to_str() {
                if filename.starts_with("region_db") { fs::remove_file(path)?; }
            }
        }
    }

    let pool = get_region_db_pool().await?;
    pool.execute(
        "CREATE TABLE IF NOT EXISTS regions (
            region_code TEXT PRIMARY KEY,
            keyphrases TEXT
        )"
    ).await?;
    
    let txt_path = format!("{}/src/db/allCountries.txt", current_dir.display()); // https://download.geonames.org/export/dump/allCountries.zip
    let txt_path = Path::new(&txt_path);
    if !txt_path.exists() { return Ok(()); }
    let all_regions_reader = io::BufReader::new(File::open(txt_path)?);
    for line in all_regions_reader.lines() {
        let line = line?;
        let mut fields = line.split("\t");
        let ascii_name = match fields.nth(2) {
            Some(name) => name,
            None => continue,
        };
        let feature_class = match fields.nth(3) {
            Some(class) => class,
            None => continue,
        };
        let feature_code = match fields.nth(0) {
            Some(f_code) => f_code,
            None => continue,
        };
        let region_code = match fields.nth(0) {
            Some(c_code) => c_code,
            None => continue,
        };
        let population = match fields.nth(5) {
            Some(pop) => {
                match pop.parse::<u32>() {
                    Ok(pop) => pop,
                    Err(_) => continue,
                }
            },
            None => continue,
        };

        if feature_code.contains('H') { continue; }

        match feature_class {
            "A" =>  {
                if population < 490000 { continue; }
            },
            "P" => {
                match feature_code {
                    "PPLC" => {},
                    _ => {
                        if population < 290000 { continue; }
                    },
                }
            },
            _ => continue,
        }

        update_region(&pool, Region {
            region_code: region_code.to_string(),
            keyphrases: format!("{}", ascii_name),
        }).await?;
    }

    let txt_path = format!("{}/src/db/countryInfo.txt", current_dir.display()); // https://download.geonames.org/export/dump/countryInfo.txt
    let txt_path = Path::new(&txt_path);
    if !txt_path.exists() { return Ok(()); }
    let all_regions_reader = io::BufReader::new(File::open(txt_path)?);
    for line in all_regions_reader.lines() {
        let line = line?;
        let mut fields = line.split("\t");
        let region_code = match fields.nth(0) {
            Some(c_code) => c_code,
            None => continue,
        };
        let capital = match fields.nth(4) {
            Some(cap) => cap,
            None => continue,
        };

        update_region(&pool, Region {
            region_code: region_code.to_string(),
            keyphrases: format!("{}", capital),
        }).await?;
    }

    Ok(())
}

async fn update_region(pool: &sqlx::Pool<sqlx::Sqlite>, region: Region) -> Result<SqliteQueryResult, sqlx::Error> {
    let row: Option<(String,)> = sqlx::query_as("SELECT keyphrases FROM regions WHERE region_code = $1")
        .bind(&region.region_code)
        .fetch_optional(pool)
        .await?;

    match row {
        Some((existing_keyphrases,)) => {
            let mut keyphrases = existing_keyphrases.split(",").collect::<Vec<&str>>();
            keyphrases.extend(region.keyphrases.split(","));
            let keyphrases: HashSet<&str> = keyphrases.into_iter().collect();
            let keyphrases = unidecode(&keyphrases.into_iter().collect::<Vec<&str>>().join(",").to_lowercase());
            sqlx::query("UPDATE regions SET keyphrases = $1 WHERE region_code = $2")
                .bind(keyphrases)
                .bind(region.region_code)
                .execute(pool)
                .await
        },
        None => {
            let keyphrases = unidecode(&region.keyphrases.to_lowercase());
            sqlx::query("INSERT INTO regions (region_code, keyphrases) VALUES ($1, $2)")
                .bind(region.region_code)
                .bind(keyphrases)
                .execute(pool)
                .await
        },
    }
}