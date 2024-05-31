# Anbamap Scraper

This repository contains the database utility software for Anbamap.

&nbsp;

## Functionality Overview

In order to use this software, you will need a Redis database.

Anbamap Scraper scrapes text from configured media sources, uses two layers of region identification to determine which regions a piece of media is related to, and stores all of this information to a database. It will also clear out any information older than one week.

#### Identification Layers

The two layers of identification are as follows:

1. [flashgeotext](https://github.com/iwpnd/flashgeotext): a Python library that determines related regions from text with OK accuracy. This is mainly for handling capital letter-dependent meanings.
2. Keyphrase checking: an in-memory data structure of keyphrases that are checked against scraped text content. Please help me maintain [it](https://github.com/lichenaut/anbamap-scraper/blob/main/src/scrape/region.rs) as time goes on! In addition to manual input, it is informed from the following: [Geonames](https://download.geonames.org/export/dump/), [Forbes400](https://forbes400.onrender.com/api/forbes400/getAllBillionaires), [Wikidata](https://www.wikidata.org/wiki/Wikidata:Main_Page), and [Wikipedia](https://en.wikipedia.org/w/api.php?action=query&prop=revisions&rvprop=content&rvslots=main&format=json&titles=List_of_largest_private_non-governmental_companies_by_revenue).

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

## Deployment

1. Create a Docker volume, pull the Docker image, and run a container to generate setup files.
```bash
docker volume create anbamap_vol
docker pull lichenaut/anbamap-scraper:latest
docker run -v anbamap_vol:/scraper/data -e DOCKER_VOLUME=/scraper/data -e REDIS_ENDPOINT= -e REDIS_PASSWORD= -e YOUTUBE_API_KEY= -e YOUTUBE_CHANNEL_IDS= image-id
```
2. Automate this run command at an interval of your choice.

&nbsp;

## Environment Variables

| Environment Variable  | Description                          | Necessity |
| --------------------- | ------------------------------------ | --------- |
| `DOCKER_VOLUME`       | Arbitrarily-valued path.             | Mandatory |
| `REDIS_ENDPOINT`      | Your Redis database endpoint.        | Mandatory |
| `REDIS_PASSWORD`      | Your Redis database password.        | Mandatory |
| `YOUTUBE_API_KEY`     | Your Youtube Data API key.           | Optional  |
| `YOUTUBE_CHANNEL_IDS` | Comma-separated Youtube channel IDs. | Optional  |
