extern crate redis;
use crate::prelude::*;
use crate::service::var_service::get_redis_client;
use itertools::Itertools;
use redis::Commands;
use std::{
    collections::{HashMap, HashSet},
    time::{SystemTime, UNIX_EPOCH},
};

pub async fn update_db(media: Vec<(String, String, String, HashSet<String>)>) -> Result<()> {
    let client = get_redis_client().await?;
    let mut connection = client.get_connection()?;
    let mut pipe = redis::pipe();
    let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
    let region_codes: Vec<String> = connection.keys("*")?;
    let mut keys_to_delete = Vec::new();
    for code in region_codes {
        let urls: Vec<String> = connection.hkeys(&code)?;
        for url in urls {
            let key = format!("{}:{}", code, url);
            let timestamp: u64 = connection.hget(&key, "timestamp")?;
            if now - timestamp > 604800 {
                keys_to_delete.push(key);
            }
        }
    }

    if !keys_to_delete.is_empty() {
        connection.del(keys_to_delete)?;
    }

    let now = now.to_string();
    let region_codes = vec![
        "ad", "ae", "af", "ag", "ai", "al", "am", "ao", "aq", "ar", "as", "at", "au", "aw", "ax",
        "az", "ba", "bb", "bd", "be", "bf", "bg", "bh", "bi", "bj", "bl", "bm", "bn", "bo", "bq",
        "br", "bs", "bt", "bv", "bw", "by", "bz", "ca", "cc", "cd", "cf", "cg", "ch", "ci", "ck",
        "cl", "cm", "cn", "co", "cr", "cu", "cv", "cw", "cx", "cy", "cz", "de", "dj", "dk", "dm",
        "do", "dz", "ec", "ee", "eg", "eh", "er", "es", "et", "fi", "fj", "fk", "fm", "fo", "fr",
        "ga", "gb", "gd", "ge", "gf", "gg", "gh", "gi", "gl", "gm", "gn", "gp", "gq", "gr", "gs",
        "gt", "gu", "gw", "gy", "hk", "hm", "hn", "hr", "ht", "hu", "id", "ie", "il", "im", "in",
        "io", "iq", "ir", "is", "it", "je", "jm", "jo", "jp", "ke", "kg", "kh", "ki", "km", "kn",
        "kp", "kr", "kw", "ky", "kz", "la", "lb", "lc", "li", "lk", "lr", "ls", "lt", "lu", "lv",
        "ly", "ma", "mc", "md", "me", "mf", "mg", "mh", "mk", "ml", "mm", "mn", "mo", "mp", "mq",
        "mr", "ms", "mt", "mu", "mv", "mw", "mx", "my", "mz", "na", "nc", "ne", "nf", "ng", "ni",
        "nl", "no", "np", "nr", "nu", "nz", "om", "pa", "pe", "pf", "pg", "ph", "pk", "pl", "pm",
        "pn", "pr", "ps", "pt", "pw", "py", "qa", "re", "ro", "rs", "ru", "rw", "sa", "sb", "sc",
        "sd", "se", "sg", "sh", "si", "sj", "sk", "sl", "sm", "sn", "so", "sr", "ss", "st", "sv",
        "sx", "sy", "sz", "tc", "td", "tf", "tg", "th", "tj", "tk", "tl", "tm", "tn", "to", "tr",
        "tt", "tv", "tw", "tz", "ua", "ug", "um", "us", "uy", "uz", "va", "vc", "ve", "vg", "vi",
        "vn", "vu", "wf", "ws", "ye", "yt", "za", "zm", "zw",
    ];

    for code in region_codes {
        let mut url_data: HashMap<&str, HashMap<&str, String>> = HashMap::new();

        for (url, title, body, regions) in &media {
            if !regions.contains(&code.to_string()) {
                continue;
            }

            let mut data: HashMap<&str, String> = HashMap::new();
            data.insert("timestamp", now.to_string());
            data.insert("title", title.to_string());
            data.insert("body", body.to_string());
            data.insert("regions", regions.iter().join(","));

            url_data.insert(url, data);
        }

        for (url, data) in &url_data {
            pipe.cmd("HMSET")
                .arg(format!("{}:{}", code, url))
                .arg(data)
                .ignore();
        }
    }

    pipe.execute(&mut connection);

    Ok(())
}
