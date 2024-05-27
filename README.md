# Anbamap Backend

This repository contains the backend server software for Anbamap.

[![Github All Releases](https://img.shields.io/github/downloads/lichenaut/anbamap-be/total.svg)]()

&nbsp;

## Functionality Overview

In order to use this software, you will need a Redis database. Additionally, you can use [Sentry](https://sentry.io/welcome/).

Anbamap-BE scrapes text from configured media sources, uses two layers of region identification to determine which regions a piece of media is related to, and stores all of this information to a database. It will also clear out any information older than one week.

#### Identification Layers

The two layers of identification are as follows:

1. [flashgeotext](https://github.com/iwpnd/flashgeotext): a Python library that determines related regions from text with OK accuracy. This is mainly for handling capital letter-dependent meanings.
2. Keyphrase checking: an in-memory data structure of keyphrases that are checked against scraped text content. Please help me maintain [it](https://github.com/lichenaut/anbamap-be/blob/main/src/region/regions.rs) as time goes on! In addition to manual input, it is informed from the following: [Geonames](https://download.geonames.org/export/dump/), [Forbes400](https://forbes400.onrender.com/api/forbes400/getAllBillionaires), [Wikidata](https://www.wikidata.org/wiki/Wikidata:Main_Page), and [Wikipedia](https://en.wikipedia.org/w/api.php?action=query&prop=revisions&rvprop=content&rvslots=main&format=json&titles=List_of_largest_private_non-governmental_companies_by_revenue).

&nbsp;

#### Rust Code: Database Structure

```rust
for (url, title, description, regions) in media {
    connection.hset(&url, "timestamp", now)?;
    connection.hset(&url, "title", title)?;
    connection.hset(&url, "body", description)?;
    connection.hset(url, "regions", regions.join(","))?;
}
```

&nbsp;

## Local Deployment

1. Install Python 3.
2. Download the latest build file in this repository's [Releases](https://github.com/lichenaut/anbamap-be/releases) section and place it in its own directory.
3. Set up your [Environment Variables](#environment-variables).
4. Change your terminal's working directory to the aforementioned directory, paste these commands in:
   ```bash
   chmod +x ./anbamap-be
   ./anbamap-be
   ```
   and wait a couple minutes while it sets up. If you get an archive-related error, it may be a network error. Delete "allCountries.zip" and redo this step.

&nbsp;

## Remote Deployment

TODO

&nbsp;

## Environment Variables

| Environment Variable  | Description                          | Necessity |
| --------------------- | ------------------------------------ | --------- |
| `REDIS_ENDPOINT`      | Your Redis database endpoint.        | Mandatory |
| `REDIS_PASSWORD`      | Your Redis database password.        | Mandatory |
| `SENTRY_DSN`          | Your Data Source Name for Sentry.    | Optional  |
| `YOUTUBE_API_KEY`     | Your Youtube Data API key.           | Optional  |
| `YOUTUBE_CHANNEL_IDS` | Comma-separated Youtube channel IDs. | Optional  |
