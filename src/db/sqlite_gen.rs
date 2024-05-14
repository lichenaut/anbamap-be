use std::collections::HashSet;
use std::fs;
use std::path::Path;
use sqlx::sqlite::SqliteConnectOptions;
use sqlx::sqlite::SqliteQueryResult;
use sqlx::Executor;
use sqlx::SqlitePool;
use std::{fs::File, io, io::BufRead};

struct Country {
    country_code: String,
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
                if filename.starts_with("country_db") { fs::remove_file(path)?; }
            }
        }
    }

    let pool = SqlitePool::connect_with(SqliteConnectOptions::new()
        .filename("country_db.sqlite")
        .create_if_missing(true)
        .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
        .shared_cache(true)
        .synchronous(sqlx::sqlite::SqliteSynchronous::Normal)
    ).await?;

    pool.execute(
        "CREATE TABLE IF NOT EXISTS countries (
            country_code TEXT PRIMARY KEY,
            keyphrases TEXT
        )"
    ).await?;

    let txt_path = format!("{}/src/db/allCountries.txt", current_dir.display()); // https://download.geonames.org/export/dump/allCountries.zip
    let txt_path = Path::new(&txt_path);
    if !txt_path.exists() { () }
    let all_countries_reader = io::BufReader::new(File::open(txt_path)?);
    for line in all_countries_reader.lines() {
        let line = line?;
        let mut fields = line.split("\t");
        let mut ascii_name = None;
        let mut feature_class = None;
        let mut feature_code = None;
        let mut country_code = None;
        let mut population = None;
        if let Some(name) = fields.nth(2) { ascii_name = Some(name); } else { continue; }
        if let Some(class) = fields.nth(3) { feature_class = Some(class) } else { continue; }
        if let Some(f_code) = fields.nth(0) { feature_code = Some(f_code) } else { continue; }
        if let Some(c_code) = fields.nth(0) { country_code = Some(c_code) } else { continue; }
        if let Some(pop) = fields.nth(5) {
            if let Ok(pop) = pop.parse::<u32>() { population = Some(pop) } else { continue; }
        } else { continue; }
        let ascii_name = ascii_name.unwrap();
        let feature_class = feature_class.unwrap();
        let feature_code = feature_code.unwrap();
        let country_code = country_code.unwrap();
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

        update_country(&pool, Country {
            country_code: country_code.to_string(),
            keyphrases: format!("{}", ascii_name)
        }).await?;
    }

    let txt_path = format!("{}/src/db/countryInfo.txt", current_dir.display()); // https://download.geonames.org/export/dump/countryInfo.txt
    let txt_path = Path::new(&txt_path);
    if !txt_path.exists() { () }
    let all_countries_reader = io::BufReader::new(File::open(txt_path)?);
    for line in all_countries_reader.lines() {
        let line = line?;
        let mut fields = line.split("\t");
        let mut country_code = None;
        let mut capital = None;
        if let Some(c_code) = fields.nth(0) { country_code = Some(c_code) } else { continue; }
        if let Some(cap) = fields.nth(3) { capital = Some(cap); } else { continue; }
        let country_code = country_code.unwrap();
        let capital = capital.unwrap();

        update_country(&pool, Country {
            country_code: country_code.to_string(),
            keyphrases: format!("{}", capital)
        }).await?;
    }

    Ok(())
}

async fn update_country(pool: &sqlx::Pool<sqlx::Sqlite>, country: Country) -> Result<SqliteQueryResult, sqlx::Error> {
    let row: Option<(String,)> = sqlx::query_as("SELECT keyphrases FROM countries WHERE country_code = $1")
        .bind(&country.country_code)
        .fetch_optional(pool)
        .await?;

    match row {
        Some((existing_keyphrases,)) => {
            let mut keyphrases = existing_keyphrases.split(",").collect::<Vec<&str>>();
            keyphrases.extend(country.keyphrases.split(","));
            let keyphrases: HashSet<&str> = keyphrases.into_iter().collect();
            let keyphrases = keyphrases.into_iter().collect::<Vec<&str>>().join(",").to_lowercase();
            sqlx::query("UPDATE countries SET keyphrases = $1 WHERE country_code = $2")
                .bind(keyphrases)
                .bind(country.country_code)
                .execute(pool)
                .await
        },
        None => {
            let keyphrases = country.keyphrases.to_lowercase();
            sqlx::query("INSERT INTO countries (country_code, keyphrases) VALUES ($1, $2)")
                .bind(country.country_code)
                .bind(keyphrases)
                .execute(pool)
                .await
        },
    }
}