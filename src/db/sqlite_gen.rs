use sqlx::sqlite::SqliteQueryResult;
use sqlx::Executor;
use std::collections::HashSet;
use std::fs;
use std::path::Path;
use std::{fs::File, io, io::BufRead};
use super::db_service::get_region_db_pool;

struct Region {
    region_code: String,
    keyphrases: String,
}

#[allow(unused_assignments)]
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
    if !txt_path.exists() { () }
    let all_regions_reader = io::BufReader::new(File::open(txt_path)?);
    for line in all_regions_reader.lines() {
        let line = line?;
        let mut fields = line.split("\t");
        let mut ascii_name = None;
        let mut feature_class = None;
        let mut feature_code = None;
        let mut region_code = None;
        let mut population = None;
        if let Some(name) = fields.nth(2) { ascii_name = Some(name); } else { continue; }
        if let Some(class) = fields.nth(3) { feature_class = Some(class) } else { continue; }
        if let Some(f_code) = fields.nth(0) { feature_code = Some(f_code) } else { continue; }
        if let Some(c_code) = fields.nth(0) { region_code = Some(c_code) } else { continue; }
        if let Some(pop) = fields.nth(5) {
            if let Ok(pop) = pop.parse::<u32>() { population = Some(pop) } else { continue; }
        } else { continue; }
        let ascii_name = ascii_name.unwrap();
        let feature_class = feature_class.unwrap();
        let feature_code = feature_code.unwrap();
        let region_code = region_code.unwrap();
        let population = population.unwrap();

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
    if !txt_path.exists() { () }
    let all_regions_reader = io::BufReader::new(File::open(txt_path)?);
    for line in all_regions_reader.lines() {
        let line = line?;
        let mut fields = line.split("\t");
        let mut region_code = None;
        let mut capital = None;
        if let Some(c_code) = fields.nth(0) { region_code = Some(c_code) } else { continue; }
        if let Some(cap) = fields.nth(4) { capital = Some(cap); } else { continue; }
        let region_code = region_code.unwrap();
        let capital = capital.unwrap();

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
            let keyphrases = keyphrases.into_iter().collect::<Vec<&str>>().join(",").to_lowercase();
            sqlx::query("UPDATE regions SET keyphrases = $1 WHERE region_code = $2")
                .bind(keyphrases)
                .bind(region.region_code)
                .execute(pool)
                .await
        },
        None => {
            let keyphrases = region.keyphrases.to_lowercase();
            sqlx::query("INSERT INTO regions (region_code, keyphrases) VALUES ($1, $2)")
                .bind(region.region_code)
                .bind(keyphrases)
                .execute(pool)
                .await
        },
    }
}