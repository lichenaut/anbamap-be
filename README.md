# Anbamap Backend
This repository contains the backend server software for Anbamap.

[![Github All Releases](https://img.shields.io/github/downloads/lichenaut/anbamap-be/total.svg)]()

&nbsp;
## Functionality Overview

In order to host this software, you will need: a [Python 3 virtual environment](https://docs.python.org/3/library/venv.html), [Youtube Data API key](https://developers.google.com/youtube/registering_an_application), and [Upstash](https://upstash.com/) Redis database. Optionally, you can also use [Sentry](https://sentry.io/welcome/).

Every 2 hours, Anbamap-BE will scrape text from its configured media sources, use 2 layers of region identification to determine which regions a media is related to, and store all of this information to a database.

#### Identification Layers
The 2 layers of identification are as follows:
1. [flashgeotext](https://github.com/iwpnd/flashgeotext): a Python library that uses AI to determine related regions from text with OK accuracy. This is mainly for contextual meanings the keyphrase checker does not do a good job with.
2. Keyphrase checking: an in-memory data structure of keyphrases that are checked against scraped text content. Please help me maintain [this](https://github.com/lichenaut/anbamap-be/blob/dcfcc41ef99947fb45179c89a85d0fd462234121/src/region/regions.rs#L152) as time goes on!

&nbsp;
#### Rust Code: Database Storing
```rust
for (url, title, description, regions) in media {
    connection.hset(&url, "timestamp", now)?;
    connection.hset(&url, "title", title)?;
    connection.hset(&url, "description", description)?;
    connection.hset(url, "regions", regions.join(","))?;
}
```

&nbsp;
## Installation

### Linux

1. Download the latest non-executable (.exe) build file in this repository's [Releases](https://github.com/lichenaut/anbamap-be/releases) section and place it in its own directory.
2. Change your terminal's working directory to the aforementioned directory and paste these commands in:
   ```bash
   chmod +x ./anbamap-be-1_0_0
   ./anbamap-be-1_0_0
3. This will run the program, which will create a `variables.env` file within the aforementioned directory and terminate. This file is where you fill in your personal information. A Sentry DSN value is optional.
4. Re-run the program with
   ```bash
   ./anbamap-be-1_0_0
   ```
   and wait a couple minutes for it to download and generate its files.
5. Done! Check your Upstash Redis database keys with
   ```
   keys *
   ```
   in your Upstash CLI.

### Windows

1. Download the latest executable (.exe) build file in this repository's [Releases](https://github.com/lichenaut/anbamap-be/releases) section and place it in its own folder.
2. Launch the executable.
3. This will run the program, which will create a `variables.env` file within the aforementioned folder. This file is where you fill in your personal information. A Sentry DSN value is optional.
4. Re-launch the executable and wait a couple minutes for it to download and generate its files.
5. Done! Check your Upstash Redis database keys with
   ```
   keys *
   ```
   in your Upstash CLI.
