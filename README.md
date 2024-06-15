# Anbamap Scraper

This repository contains the database utility software for Anbamap.

&nbsp;

## Overview

Anbamap Scraper scrapes text from [configured media sources](#environment-variables), uses two layers of region identification to determine which regions a piece of media is related to, and stores this information. [By default](#environment-variables), it will also clear out any data aged older than one week.

#### Identification Layers

The two layers of identification are as follows:

1. [flashgeotext](https://github.com/iwpnd/flashgeotext): a Python library that determines related regions from text with OK accuracy. This is mainly for handling capital letter-dependent meanings.
2. Keyphrase checking: an in-memory data structure of keyphrases that are checked against scraped text content. Please help me maintain [the keyphrases](https://github.com/lichenaut/anbamap-scraper/blob/main/src/scrape/region.rs) as time goes on! In addition to manual input, it is informed from the following: [Geonames](https://download.geonames.org/export/dump/), [Forbes400](https://forbes400.onrender.com/api/forbes400/getAllBillionaires), [Wikidata](https://www.wikidata.org/wiki/Wikidata:Main_Page), and [Wikipedia](https://en.wikipedia.org/w/api.php?action=query&prop=revisions&rvprop=content&rvslots=main&format=json&titles=List_of_largest_private_non-governmental_companies_by_revenue).

&nbsp;

#### Database Structure

The database will be located in a Docker volume as 'media_db.sqlite'. See [Deployment](#deployment).

##### Rust Code

```rust
pool.execute(
    "CREATE TABLE IF NOT EXISTS urls (
        url TEXT PRIMARY KEY,
        timestamp INTEGER,
        title TEXT,
        body TEXT
    )",
)
.await?;
pool.execute(
    "CREATE TABLE IF NOT EXISTS url_regions (
        url TEXT,
        region_code TEXT,
        PRIMARY KEY (url, region_code),
        FOREIGN KEY (url) REFERENCES urls (url)
    )",
)
.await?;
```

##### Column Descriptions

| Column      | Description                           |
| ----------- | ------------------------------------- |
| `timestamp` | UNIX seconds time of scrape.          |
| `title`     | Primary text of scraped media.        |
| `body`      | Secondary text of scraped media.      |
| `regions`   | Comma-separated related region codes. |

&nbsp;

## Deployment

1. Create a Docker volume, pull the Docker image, and run a container.

```bash
docker volume create anbamap_vol
docker pull lichenaut/anbamap-scraper:latest
docker run -v anbamap_vol:/scraper/data -e DOCKER_VOLUME=/scraper/data image-id
```

The first run will take a few minutes to set up files.

2. Automate this run command at an interval of your choice.

&nbsp;

## Environment Variables

| Environment Variable  | Description                                                                                                                                                                                                             |
| --------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `DOCKER_VOLUME`       | Arbitrarily-valued path. Only mandatory variable.                                                                                                                                                                       |
| `ACCURACY_B`          | `boolean` value for [https://accuracy.org/news-releases/](https://accuracy.org/news-releases/)                                                                                                                          |
| `AMNESTY_B`           | `boolean` value for [https://www.amnestyusa.org/news/](https://www.amnestyusa.org/news/)                                                                                                                                |
| `ANTIWAR_B`           | `boolean` value for [https://www.antiwar.com/latest.php](https://www.antiwar.com/latest.php)                                                                                                                            |
| `CJ_B`                | `boolean` value for [https://caitlinjohnstone.com.au/category/article/](https://caitlinjohnstone.com.au/category/article/)                                                                                              |
| `CONSORTIUM_B`        | `boolean` value for [https://consortiumnews.com/yyyy/mm/dd/](https://consortiumnews.com/yyyy/mm/dd/)                                                                                                                    |
| `DN_B`                | `boolean` value for [https://www.democracynow.org/yyyy/m/d/headlines](https://www.democracynow.org/yyyy/m/d/headlines)                                                                                                  |
| `EI_B`                | `boolean` value for [https://electronicintifada.net/news](https://electronicintifada.net/news) and [https://electronicintifada.net/blog](https://electronicintifada.net/blog)                                           |
| `GE_B`                | `boolean` value for [https://geopoliticaleconomy.com/yyyy/mm/dd/](https://geopoliticaleconomy.com/yyyy/mm/dd/)                                                                                                          |
| `GRAYZONE_B`          | `boolean` value for [https://thegrayzone.com/yyyy/mm/dd/](https://thegrayzone.com/yyyy/mm/dd/)                                                                                                                          |
| `HRW_B`               | `boolean` value for [https://www.hrw.org/news](https://www.hrw.org/news)                                                                                                                                                |
| `INTERCEPT_B`         | `boolean` value for [https://theintercept.com/yyyy/mm/dd/](https://theintercept.com/yyyy/mm/dd/)                                                                                                                        |
| `JC_B`                | `boolean` value for [https://www.jonathan-cook.net/blog/yyyy-dd-mm/](https://www.jonathan-cook.net/blog/yyyy-dd-mm/)                                                                                                    |
| `OS_B`                | `boolean` value for [https://www.opensecrets.org/news/yyyy/mm/](https://www.opensecrets.org/news/yyyy/mm/) and [https://www.opensecrets.org/news/reports?year=yyyy](https://www.opensecrets.org/news/reports?year=yyyy) |
| `PROPUBLICA_B`        | `boolean` value for [https://www.propublica.org/archive/yyyy/mm/](https://www.propublica.org/archive/yyyy/mm/)                                                                                                          |
| `SUBSTACK_URLS`       | Comma-separated Substack archive URLs.                                                                                                                                                                                  |
| `TRUTHOUT_B`          | `boolean` value for [https://truthout.org/latest/](https://truthout.org/latest/)                                                                                                                                        |
| `TI_B`                | `boolean` value for [https://www.typeinvestigations.org/all/?post_date=mmddyyyy+mmddyyyy/](https://www.typeinvestigations.org/all/?post_date=mmddyyyy+mmddyyyy/)                                                        |
| `UR_B`                | `boolean` value for [https://unicornriot.ninja/category/global/](https://unicornriot.ninja/category/global/)                                                                                                            |
| `YOUTUBE_API_KEY`     | Your Youtube Data API key.                                                                                                                                                                                              |
| `YOUTUBE_CHANNEL_IDS` | Comma-separated Youtube channel IDs.                                                                                                                                                                                    |
