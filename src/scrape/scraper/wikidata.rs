use crate::prelude::*;
use reqwest::Client;
use serde_json::Value;

pub async fn region_code_to_figures(client: &Client, iso_code: &str) -> Result<Vec<String>> {
    let mut figures = Vec::new();
    let property = match get_property_from_iso(iso_code) {
        Some(property) => format!("Q{}", property),
        None => {
            tracing::error!("No Wikidata property found for ISO code: {}", iso_code);
            return Ok(figures);
        }
    };

    let url = format!(
        "https://www.wikidata.org/w/api.php?action=wbgetentities&ids={}&props=claims&format=json",
        property
    );
    let response = client.get(&url).send().await?;
    if !response.status().is_success() {
        tracing::error!("Non-success response from Wikidata: {}", response.status());
        return Ok(figures);
    }

    let json: Value = response.json().await?;
    let claims = &json["entities"][&property]["claims"]["P6"];
    let last_claim = match claims.as_array() {
        Some(claims) => match claims.last() {
            Some(last_claim) => last_claim,
            None => return Ok(figures),
        },
        None => return Ok(figures),
    };

    let figure_id = match last_claim["mainsnak"]["datavalue"]["value"]["id"].as_str() {
        Some(figure_id) => figure_id,
        None => return Ok(figures),
    };

    let url = format!("https://www.wikidata.org/w/api.php?action=wbgetentities&ids={}&props=labels&languages=en&format=json", figure_id);
    let response = client.get(&url).send().await?;
    let json2: Value = response.json().await?;
    match json2["entities"][figure_id]["labels"]["en"]["value"].as_str() {
        Some(figure_name) => push_to_figures(figure_name, &mut figures)?,
        None => return Ok(figures),
    };

    let claims = &json["entities"][property]["claims"]["P35"];
    let last_claim = match claims.as_array() {
        Some(claims) => match claims.last() {
            Some(last_claim) => last_claim,
            None => return Ok(figures),
        },
        None => return Ok(figures),
    };

    let figure_id = match last_claim["mainsnak"]["datavalue"]["value"]["id"].as_str() {
        Some(figure_id) => figure_id,
        None => return Ok(figures),
    };

    let url = format!("https://www.wikidata.org/w/api.php?action=wbgetentities&ids={}&props=labels&languages=en&format=json", figure_id);
    let response = client.get(&url).send().await?;
    let json2: Value = response.json().await?;
    match json2["entities"][figure_id]["labels"]["en"]["value"].as_str() {
        Some(figure_name) => push_to_figures(figure_name, &mut figures)?,
        None => return Ok(figures),
    };

    Ok(figures)
}

fn push_to_figures(mut figure_name: &str, figures: &mut Vec<String>) -> Result<()> {
    if figure_name == "Frederik X of Denmark" {
        figure_name = "frederik x";
    } else if figure_name == "Willem-Alexander of the Netherlands" {
        figure_name = "willem-alexander";
    } else if figure_name == "Charles III of the United Kingdom" {
        figure_name = "charles iii,king charles";
    }

    figures.push(figure_name.to_string());

    Ok(())
}

fn get_property_from_iso(iso_code: &str) -> Option<&str> {
    match iso_code {
        "AD" => Some("228"),
        "AE" => Some("878"),
        "AF" => Some("889"),
        "AG" => Some("781"),
        "AI" => Some("25228"),
        "AL" => Some("222"),
        "AM" => Some("399"),
        "AO" => Some("916"),
        "AQ" => Some("51"),
        "AR" => Some("414"),
        "AS" => Some("16641"),
        "AT" => Some("40"),
        "AU" => Some("408"),
        "AW" => Some("21203"),
        "AX" => Some("5689"),
        "AZ" => Some("227"),
        "BA" => Some("225"),
        "BB" => Some("244"),
        "BD" => Some("902"),
        "BE" => Some("31"),
        "BF" => Some("965"),
        "BG" => Some("219"),
        "BH" => Some("398"),
        "BI" => Some("967"),
        "BJ" => Some("962"),
        "BL" => Some("25362"),
        "BM" => Some("23635"),
        "BN" => Some("921"),
        "BO" => Some("750"),
        "BQ" => Some("25396"),
        "BR" => Some("155"),
        "BS" => Some("778"),
        "BT" => Some("917"),
        "BV" => Some("23408"),
        "BW" => Some("963"),
        "BY" => Some("184"),
        "BZ" => Some("242"),
        "CA" => Some("16"),
        "CC" => Some("36004"),
        "CD" => Some("974"),
        "CF" => Some("929"),
        "CG" => Some("971"),
        "CH" => Some("39"),
        "CI" => Some("1008"),
        "CK" => Some("26988"),
        "CL" => Some("298"),
        "CM" => Some("1009"),
        "CN" => Some("148"),
        "CO" => Some("739"),
        "CR" => Some("800"),
        "CU" => Some("241"),
        "CV" => Some("1011"),
        "CW" => Some("25279"),
        "CX" => Some("31063"),
        "CY" => Some("229"),
        "CZ" => Some("213"),
        "DE" => Some("183"),
        "DJ" => Some("977"),
        "DK" => Some("35"),
        "DM" => Some("784"),
        "DO" => Some("786"),
        "DZ" => Some("262"),
        "EC" => Some("736"),
        "EE" => Some("191"),
        "EG" => Some("79"),
        "EH" => Some("6250"),
        "ER" => Some("986"),
        "ES" => Some("29"),
        "ET" => Some("115"),
        "FI" => Some("33"),
        "FJ" => Some("712"),
        "FK" => Some("9648"),
        "FM" => Some("3359409"),
        "FO" => Some("4628"),
        "FR" => Some("142"),
        "GA" => Some("1000"),
        "GB" => Some("145"),
        "GD" => Some("769"),
        "GE" => Some("230"),
        "GF" => Some("3769"),
        "GG" => Some("3311985"),
        "GH" => Some("117"),
        "GI" => Some("1410"),
        "GL" => Some("223"),
        "GM" => Some("1005"),
        "GN" => Some("1006"),
        "GP" => Some("17012"),
        "GQ" => Some("983"),
        "GR" => Some("41"),
        "GS" => Some("35086"),
        "GT" => Some("774"),
        "GU" => Some("16635"),
        "GW" => Some("1007"),
        "GY" => Some("734"),
        "HK" => Some("8646"),
        "HM" => Some("131198"),
        "HN" => Some("783"),
        "HR" => Some("224"),
        "HT" => Some("790"),
        "HU" => Some("28"),
        "ID" => Some("252"),
        "IE" => Some("22890"),
        "IL" => Some("801"),
        "IM" => Some("9676"),
        "IN" => Some("668"),
        "IO" => Some("43448"),
        "IQ" => Some("796"),
        "IR" => Some("794"),
        "IS" => Some("189"),
        "IT" => Some("38"),
        "JE" => Some("785"),
        "JM" => Some("766"),
        "JO" => Some("810"),
        "JP" => Some("17"),
        "KE" => Some("114"),
        "KG" => Some("813"),
        "KH" => Some("424"),
        "KI" => Some("710"),
        "KM" => Some("970"),
        "KN" => Some("763"),
        "KP" => Some("423"),
        "KR" => Some("884"),
        "KW" => Some("817"),
        "KY" => Some("5785"),
        "KZ" => Some("232"),
        "LA" => Some("819"),
        "LB" => Some("822"),
        "LC" => Some("760"),
        "LI" => Some("347"),
        "LK" => Some("854"),
        "LR" => Some("1014"),
        "LS" => Some("1013"),
        "LT" => Some("37"),
        "LU" => Some("32"),
        "LV" => Some("211"),
        "LY" => Some("1016"),
        "MA" => Some("1028"),
        "MC" => Some("235"),
        "MD" => Some("217"),
        "ME" => Some("236"),
        "MF" => Some("25596"),
        "MG" => Some("1019"),
        "MH" => Some("709"),
        "MK" => Some("221"),
        "ML" => Some("912"),
        "MM" => Some("836"),
        "MN" => Some("711"),
        "MO" => Some("14773"),
        "MP" => Some("16644"),
        "MQ" => Some("17054"),
        "MR" => Some("1025"),
        "MS" => Some("732115"),
        "MT" => Some("233"),
        "MU" => Some("1027"),
        "MV" => Some("826"),
        "MW" => Some("1020"),
        "MX" => Some("96"),
        "MY" => Some("833"),
        "MZ" => Some("1029"),
        "NA" => Some("1030"),
        "NC" => Some("33788"),
        "NE" => Some("1032"),
        "NF" => Some("31057"),
        "NG" => Some("1033"),
        "NI" => Some("811"),
        "NL" => Some("55"),
        "NO" => Some("20"),
        "NP" => Some("837"),
        "NR" => Some("697"),
        "NU" => Some("34020"),
        "NZ" => Some("664"),
        "OM" => Some("842"),
        "PA" => Some("804"),
        "PE" => Some("419"),
        "PF" => Some("30971"),
        "PG" => Some("691"),
        "PH" => Some("928"),
        "PK" => Some("843"),
        "PL" => Some("36"),
        "PM" => Some("34617"),
        "PN" => Some("35672"),
        "PR" => Some("1183"),
        "PS" => Some("219060"),
        "PT" => Some("45"),
        "PW" => Some("695"),
        "PY" => Some("733"),
        "QA" => Some("846"),
        "RE" => Some("17070"),
        "RO" => Some("218"),
        "RS" => Some("403"),
        "RU" => Some("159"),
        "RW" => Some("1037"),
        "SA" => Some("851"),
        "SB" => Some("685"),
        "SC" => Some("1042"),
        "SD" => Some("1049"),
        "SE" => Some("34"),
        "SG" => Some("334"),
        "SH" => Some("34497"),
        "SI" => Some("215"),
        "SJ" => Some("842829"),
        "SK" => Some("214"),
        "SL" => Some("1044"),
        "SM" => Some("238"),
        "SN" => Some("1041"),
        "SO" => Some("1045"),
        "SR" => Some("730"),
        "SS" => Some("958"),
        "ST" => Some("1039"),
        "SV" => Some("792"),
        "SX" => Some("26273"),
        "SY" => Some("858"),
        "SZ" => Some("1050"),
        "TC" => Some("18221"),
        "TD" => Some("657"),
        "TF" => Some("129003"),
        "TG" => Some("945"),
        "TH" => Some("869"),
        "TJ" => Some("863"),
        "TK" => Some("36823"),
        "TL" => Some("574"),
        "TM" => Some("874"),
        "TN" => Some("948"),
        "TO" => Some("678"),
        "TR" => Some("43"),
        "TT" => Some("754"),
        "TV" => Some("672"),
        "TW" => Some("865"),
        "TZ" => Some("924"),
        "UA" => Some("212"),
        "UG" => Some("1036"),
        "UM" => Some("16645"),
        "US" => Some("30"),
        "UY" => Some("77"),
        "UZ" => Some("265"),
        "VA" => Some("237"),
        "VC" => Some("757"),
        "VE" => Some("717"),
        "VG" => Some("25305"),
        "VI" => Some("11703"),
        "VN" => Some("881"),
        "VU" => Some("686"),
        "WF" => Some("35555"),
        "WS" => Some("683"),
        "YE" => Some("805"),
        "YT" => Some("17063"),
        "ZA" => Some("258"),
        "ZM" => Some("953"),
        "ZW" => Some("954"),
        _ => None,
    }
}

// You are seeing false values for regions because Anbamap cannot correctly match region ISO codes with Wikidata properties.
// Perhaps the property number and/or region name has changed?
async fn print_result(name: &str, code: &str, result: &bool) {
    if !*result {
        tracing::error!(
            "A Wikidata region code's correctness is false! {}",
            format!("{}, {}: {}", name, code, result)
        );
    }
}

async fn verify_iso_match(client: &Client, wikidata_id: &str, name: &str) -> bool {
    let property = get_property_from_iso(wikidata_id);
    let property = match property {
        Some(property) => property,
        None => return false,
    };

    let url = format!(
        "https://www.wikidata.org/w/api.php?action=wbgetentities&ids=Q{}&format=json",
        property
    );
    let response = client.get(&url).send().await;
    let response = match response {
        Ok(response) => response,
        Err(_) => return false,
    };

    let json = response.json().await;
    let json: serde_json::Value = match json {
        Ok(json) => json,
        Err(_) => return false,
    };

    let label = json["entities"][format!("Q{}", property)]["labels"]["en"]["value"].as_str();
    let label = match label {
        Some(label) => label,
        None => return false,
    };

    label == name
}

pub async fn verify_codes() {
    // Uses Geonames names by default, but this is overrided with Wikidata names when there is a difference. This occurence is tagged with a comment.
    let client = reqwest::Client::new();
    print_result(
        "Andorra",
        "AD",
        &verify_iso_match(&client, "AD", "Andorra").await,
    )
    .await;
    print_result(
        "United Arab Emirates",
        "AE",
        &verify_iso_match(&client, "AE", "United Arab Emirates").await,
    )
    .await;
    print_result(
        "Afghanistan",
        "AF",
        &verify_iso_match(&client, "AF", "Afghanistan").await,
    )
    .await;
    print_result(
        "Antigua and Barbuda",
        "AG",
        &verify_iso_match(&client, "AG", "Antigua and Barbuda").await,
    )
    .await;
    print_result(
        "Anguilla",
        "AI",
        &verify_iso_match(&client, "AI", "Anguilla").await,
    )
    .await;
    print_result(
        "Albania",
        "AL",
        &verify_iso_match(&client, "AL", "Albania").await,
    )
    .await;
    print_result(
        "Armenia",
        "AM",
        &verify_iso_match(&client, "AM", "Armenia").await,
    )
    .await;
    print_result(
        "Angola",
        "AO",
        &verify_iso_match(&client, "AO", "Angola").await,
    )
    .await;
    print_result(
        "Antarctica",
        "AQ",
        &verify_iso_match(&client, "AQ", "Antarctica").await,
    )
    .await;
    print_result(
        "Argentina",
        "AR",
        &verify_iso_match(&client, "AR", "Argentina").await,
    )
    .await;
    print_result(
        "American Samoa",
        "AS",
        &verify_iso_match(&client, "AS", "American Samoa").await,
    )
    .await;
    print_result(
        "Austria",
        "AT",
        &verify_iso_match(&client, "AT", "Austria").await,
    )
    .await;
    print_result(
        "Australia",
        "AU",
        &verify_iso_match(&client, "AU", "Australia").await,
    )
    .await;
    print_result(
        "Aruba",
        "AW",
        &verify_iso_match(&client, "AW", "Aruba").await,
    )
    .await;
    print_result(
        "Åland Islands",
        "AX",
        &verify_iso_match(&client, "AX", "Åland").await,
    )
    .await; //tag
    print_result(
        "Azerbaijan",
        "AZ",
        &verify_iso_match(&client, "AZ", "Azerbaijan").await,
    )
    .await;
    print_result(
        "Bosnia and Herzegovina",
        "BA",
        &verify_iso_match(&client, "BA", "Bosnia and Herzegovina").await,
    )
    .await;
    print_result(
        "Barbados",
        "BB",
        &verify_iso_match(&client, "BB", "Barbados").await,
    )
    .await;
    print_result(
        "Bangladesh",
        "BD",
        &verify_iso_match(&client, "BD", "Bangladesh").await,
    )
    .await;
    print_result(
        "Belgium",
        "BE",
        &verify_iso_match(&client, "BE", "Belgium").await,
    )
    .await;
    print_result(
        "Burkina Faso",
        "BF",
        &verify_iso_match(&client, "BF", "Burkina Faso").await,
    )
    .await;
    print_result(
        "Bulgaria",
        "BG",
        &verify_iso_match(&client, "BG", "Bulgaria").await,
    )
    .await;
    print_result(
        "Bahrain",
        "BH",
        &verify_iso_match(&client, "BH", "Bahrain").await,
    )
    .await;
    print_result(
        "Burundi",
        "BI",
        &verify_iso_match(&client, "BI", "Burundi").await,
    )
    .await;
    print_result(
        "Benin",
        "BJ",
        &verify_iso_match(&client, "BJ", "Benin").await,
    )
    .await;
    print_result(
        "Saint Barthélemy",
        "BL",
        &verify_iso_match(&client, "BL", "Saint Barthélemy").await,
    )
    .await;
    print_result(
        "Bermuda",
        "BM",
        &verify_iso_match(&client, "BM", "Bermuda").await,
    )
    .await;
    print_result(
        "Brunei",
        "BN",
        &verify_iso_match(&client, "BN", "Brunei Darussalam").await,
    )
    .await; //tag
    print_result(
        "Bolivia",
        "BO",
        &verify_iso_match(&client, "BO", "Bolivia").await,
    )
    .await;
    print_result(
        "Bonaire",
        "BQ",
        &verify_iso_match(&client, "BQ", "Bonaire").await,
    )
    .await;
    print_result(
        "Brazil",
        "BR",
        &verify_iso_match(&client, "BR", "Brazil").await,
    )
    .await;
    print_result(
        "Bahamas",
        "BS",
        &verify_iso_match(&client, "BS", "The Bahamas").await,
    )
    .await; //tag
    print_result(
        "Bhutan",
        "BT",
        &verify_iso_match(&client, "BT", "Bhutan").await,
    )
    .await;
    print_result(
        "Bouvet Island",
        "BV",
        &verify_iso_match(&client, "BV", "Bouvet Island").await,
    )
    .await;
    print_result(
        "Botswana",
        "BW",
        &verify_iso_match(&client, "BW", "Botswana").await,
    )
    .await;
    print_result(
        "Belarus",
        "BY",
        &verify_iso_match(&client, "BY", "Belarus").await,
    )
    .await;
    print_result(
        "Belize",
        "BZ",
        &verify_iso_match(&client, "BZ", "Belize").await,
    )
    .await;
    print_result(
        "Canada",
        "CA",
        &verify_iso_match(&client, "CA", "Canada").await,
    )
    .await;
    print_result(
        "Cocos (Keeling) Islands",
        "CC",
        &verify_iso_match(&client, "CC", "Cocos (Keeling) Islands").await,
    )
    .await;
    print_result(
        "Democratic Republic of the Congo",
        "CD",
        &verify_iso_match(&client, "CD", "Democratic Republic of the Congo").await,
    )
    .await;
    print_result(
        "Central African Republic",
        "CF",
        &verify_iso_match(&client, "CF", "Central African Republic").await,
    )
    .await;
    print_result(
        "Republic of the Congo",
        "CG",
        &verify_iso_match(&client, "CG", "Republic of the Congo").await,
    )
    .await;
    print_result(
        "Switzerland",
        "CH",
        &verify_iso_match(&client, "CH", "Switzerland").await,
    )
    .await;
    print_result(
        "Ivory Coast",
        "CI",
        &verify_iso_match(&client, "CI", "Ivory Coast").await,
    )
    .await;
    print_result(
        "Cook Islands",
        "CK",
        &verify_iso_match(&client, "CK", "Cook Islands").await,
    )
    .await;
    print_result(
        "Chile",
        "CL",
        &verify_iso_match(&client, "CL", "Chile").await,
    )
    .await;
    print_result(
        "Cameroon",
        "CM",
        &verify_iso_match(&client, "CM", "Cameroon").await,
    )
    .await;
    print_result(
        "China",
        "CN",
        &verify_iso_match(&client, "CN", "People's Republic of China").await,
    )
    .await; //tag
    print_result(
        "Colombia",
        "CO",
        &verify_iso_match(&client, "CO", "Colombia").await,
    )
    .await;
    print_result(
        "Costa Rica",
        "CR",
        &verify_iso_match(&client, "CR", "Costa Rica").await,
    )
    .await;
    print_result("Cuba", "CU", &verify_iso_match(&client, "CU", "Cuba").await).await;
    print_result(
        "Cape Verde",
        "CV",
        &verify_iso_match(&client, "CV", "Cape Verde").await,
    )
    .await;
    print_result(
        "Curaçao",
        "CW",
        &verify_iso_match(&client, "CW", "Curaçao").await,
    )
    .await;
    print_result(
        "Christmas Island",
        "CX",
        &verify_iso_match(&client, "CX", "Christmas Island").await,
    )
    .await;
    print_result(
        "Cyprus",
        "CY",
        &verify_iso_match(&client, "CY", "Cyprus").await,
    )
    .await;
    print_result(
        "Czech Republic",
        "CZ",
        &verify_iso_match(&client, "CZ", "Czech Republic").await,
    )
    .await;
    print_result(
        "Germany",
        "DE",
        &verify_iso_match(&client, "DE", "Germany").await,
    )
    .await;
    print_result(
        "Djibouti",
        "DJ",
        &verify_iso_match(&client, "DJ", "Djibouti").await,
    )
    .await;
    print_result(
        "Denmark",
        "DK",
        &verify_iso_match(&client, "DK", "Denmark").await,
    )
    .await;
    print_result(
        "Dominica",
        "DM",
        &verify_iso_match(&client, "DM", "Dominica").await,
    )
    .await;
    print_result(
        "Dominican Republic",
        "DO",
        &verify_iso_match(&client, "DO", "Dominican Republic").await,
    )
    .await;
    print_result(
        "Algeria",
        "DZ",
        &verify_iso_match(&client, "DZ", "Algeria").await,
    )
    .await;
    print_result(
        "Ecuador",
        "EC",
        &verify_iso_match(&client, "EC", "Ecuador").await,
    )
    .await;
    print_result(
        "Estonia",
        "EE",
        &verify_iso_match(&client, "EE", "Estonia").await,
    )
    .await;
    print_result(
        "Egypt",
        "EG",
        &verify_iso_match(&client, "EG", "Egypt").await,
    )
    .await;
    print_result(
        "Western Sahara",
        "EH",
        &verify_iso_match(&client, "EH", "Western Sahara").await,
    )
    .await;
    print_result(
        "Eritrea",
        "ER",
        &verify_iso_match(&client, "ER", "Eritrea").await,
    )
    .await;
    print_result(
        "Spain",
        "ES",
        &verify_iso_match(&client, "ES", "Spain").await,
    )
    .await;
    print_result(
        "Ethiopia",
        "ET",
        &verify_iso_match(&client, "ET", "Ethiopia").await,
    )
    .await;
    print_result(
        "Finland",
        "FI",
        &verify_iso_match(&client, "FI", "Finland").await,
    )
    .await;
    print_result("Fiji", "FJ", &verify_iso_match(&client, "FJ", "Fiji").await).await;
    print_result(
        "Falkland Islands",
        "FK",
        &verify_iso_match(&client, "FK", "Falkland Islands").await,
    )
    .await;
    print_result(
        "Micronesia",
        "FM",
        &verify_iso_match(&client, "FM", "Micronesia").await,
    )
    .await;
    print_result(
        "Faroe Islands",
        "FO",
        &verify_iso_match(&client, "FO", "Faroe Islands").await,
    )
    .await;
    print_result(
        "France",
        "FR",
        &verify_iso_match(&client, "FR", "France").await,
    )
    .await;
    print_result(
        "Gabon",
        "GA",
        &verify_iso_match(&client, "GA", "Gabon").await,
    )
    .await;
    print_result(
        "United Kingdom",
        "GB",
        &verify_iso_match(&client, "GB", "United Kingdom").await,
    )
    .await;
    print_result(
        "Grenada",
        "GD",
        &verify_iso_match(&client, "GD", "Grenada").await,
    )
    .await;
    print_result(
        "Georgia",
        "GE",
        &verify_iso_match(&client, "GE", "Georgia").await,
    )
    .await;
    print_result(
        "French Guiana",
        "GF",
        &verify_iso_match(&client, "GF", "French Guiana").await,
    )
    .await;
    print_result(
        "Guernsey",
        "GG",
        &verify_iso_match(&client, "GG", "Guernsey").await,
    )
    .await;
    print_result(
        "Ghana",
        "GH",
        &verify_iso_match(&client, "GH", "Ghana").await,
    )
    .await;
    print_result(
        "Gibraltar",
        "GI",
        &verify_iso_match(&client, "GI", "Gibraltar").await,
    )
    .await;
    print_result(
        "Greenland",
        "GL",
        &verify_iso_match(&client, "GL", "Greenland").await,
    )
    .await;
    print_result(
        "Gambia",
        "GM",
        &verify_iso_match(&client, "GM", "The Gambia").await,
    )
    .await; //tag
    print_result(
        "Guinea",
        "GN",
        &verify_iso_match(&client, "GN", "Guinea").await,
    )
    .await;
    print_result(
        "Guadeloupe",
        "GP",
        &verify_iso_match(&client, "GP", "Guadeloupe").await,
    )
    .await;
    print_result(
        "Equatorial Guinea",
        "GQ",
        &verify_iso_match(&client, "GQ", "Equatorial Guinea").await,
    )
    .await;
    print_result(
        "Greece",
        "GR",
        &verify_iso_match(&client, "GR", "Greece").await,
    )
    .await;
    print_result(
        "South Georgia and the South Sandwich Islands",
        "GS",
        &verify_iso_match(
            &client,
            "GS",
            "South Georgia and the South Sandwich Islands",
        )
        .await,
    )
    .await;
    print_result(
        "Guatemala",
        "GT",
        &verify_iso_match(&client, "GT", "Guatemala").await,
    )
    .await;
    print_result("Guam", "GU", &verify_iso_match(&client, "GU", "Guam").await).await;
    print_result(
        "Guinea-Bissau",
        "GW",
        &verify_iso_match(&client, "GW", "Guinea-Bissau").await,
    )
    .await;
    print_result(
        "Guyana",
        "GY",
        &verify_iso_match(&client, "GY", "Guyana").await,
    )
    .await;
    print_result(
        "Hong Kong",
        "HK",
        &verify_iso_match(&client, "HK", "Hong Kong").await,
    )
    .await;
    print_result(
        "Heard Island and McDonald Islands",
        "HM",
        &verify_iso_match(&client, "HM", "Heard Island and McDonald Islands").await,
    )
    .await;
    print_result(
        "Honduras",
        "HN",
        &verify_iso_match(&client, "HN", "Honduras").await,
    )
    .await;
    print_result(
        "Croatia",
        "HR",
        &verify_iso_match(&client, "HR", "Croatia").await,
    )
    .await;
    print_result(
        "Haiti",
        "HT",
        &verify_iso_match(&client, "HT", "Haiti").await,
    )
    .await;
    print_result(
        "Hungary",
        "HU",
        &verify_iso_match(&client, "HU", "Hungary").await,
    )
    .await;
    print_result(
        "Indonesia",
        "ID",
        &verify_iso_match(&client, "ID", "Indonesia").await,
    )
    .await;
    print_result(
        "Ireland",
        "IE",
        &verify_iso_match(&client, "IE", "Ireland").await,
    )
    .await;
    print_result(
        "Israel",
        "IL",
        &verify_iso_match(&client, "IL", "Israel").await,
    )
    .await;
    print_result(
        "Isle of Man",
        "IM",
        &verify_iso_match(&client, "IM", "Isle of Man").await,
    )
    .await;
    print_result(
        "India",
        "IN",
        &verify_iso_match(&client, "IN", "India").await,
    )
    .await;
    print_result(
        "British Indian Ocean Territory",
        "IO",
        &verify_iso_match(&client, "IO", "British Indian Ocean Territory").await,
    )
    .await;
    print_result("Iraq", "IQ", &verify_iso_match(&client, "IQ", "Iraq").await).await;
    print_result("Iran", "IR", &verify_iso_match(&client, "IR", "Iran").await).await;
    print_result(
        "Iceland",
        "IS",
        &verify_iso_match(&client, "IS", "Iceland").await,
    )
    .await;
    print_result(
        "Italy",
        "IT",
        &verify_iso_match(&client, "IT", "Italy").await,
    )
    .await;
    print_result(
        "Jersey",
        "JE",
        &verify_iso_match(&client, "JE", "Jersey").await,
    )
    .await;
    print_result(
        "Jamaica",
        "JM",
        &verify_iso_match(&client, "JM", "Jamaica").await,
    )
    .await;
    print_result(
        "Jordan",
        "JO",
        &verify_iso_match(&client, "JO", "Jordan").await,
    )
    .await;
    print_result(
        "Japan",
        "JP",
        &verify_iso_match(&client, "JP", "Japan").await,
    )
    .await;
    print_result(
        "Kenya",
        "KE",
        &verify_iso_match(&client, "KE", "Kenya").await,
    )
    .await;
    print_result(
        "Kyrgyzstan",
        "KG",
        &verify_iso_match(&client, "KG", "Kyrgyzstan").await,
    )
    .await;
    print_result(
        "Cambodia",
        "KH",
        &verify_iso_match(&client, "KH", "Cambodia").await,
    )
    .await;
    print_result(
        "Kiribati",
        "KI",
        &verify_iso_match(&client, "KI", "Kiribati").await,
    )
    .await;
    print_result(
        "Comoros",
        "KM",
        &verify_iso_match(&client, "KM", "Comoros").await,
    )
    .await;
    print_result(
        "Saint Kitts and Nevis",
        "KN",
        &verify_iso_match(&client, "KN", "Saint Kitts and Nevis").await,
    )
    .await;
    print_result(
        "North Korea",
        "KP",
        &verify_iso_match(&client, "KP", "North Korea").await,
    )
    .await;
    print_result(
        "South Korea",
        "KR",
        &verify_iso_match(&client, "KR", "South Korea").await,
    )
    .await;
    print_result(
        "Kuwait",
        "KW",
        &verify_iso_match(&client, "KW", "Kuwait").await,
    )
    .await;
    print_result(
        "Cayman Islands",
        "KY",
        &verify_iso_match(&client, "KY", "Cayman Islands").await,
    )
    .await;
    print_result(
        "Kazakhstan",
        "KZ",
        &verify_iso_match(&client, "KZ", "Kazakhstan").await,
    )
    .await;
    print_result("Laos", "LA", &verify_iso_match(&client, "LA", "Laos").await).await;
    print_result(
        "Lebanon",
        "LB",
        &verify_iso_match(&client, "LB", "Lebanon").await,
    )
    .await;
    print_result(
        "Saint Lucia",
        "LC",
        &verify_iso_match(&client, "LC", "Saint Lucia").await,
    )
    .await;
    print_result(
        "Liechtenstein",
        "LI",
        &verify_iso_match(&client, "LI", "Liechtenstein").await,
    )
    .await;
    print_result(
        "Sri Lanka",
        "LK",
        &verify_iso_match(&client, "LK", "Sri Lanka").await,
    )
    .await;
    print_result(
        "Liberia",
        "LR",
        &verify_iso_match(&client, "LR", "Liberia").await,
    )
    .await;
    print_result(
        "Lesotho",
        "LS",
        &verify_iso_match(&client, "LS", "Lesotho").await,
    )
    .await;
    print_result(
        "Lithuania",
        "LT",
        &verify_iso_match(&client, "LT", "Lithuania").await,
    )
    .await;
    print_result(
        "Luxembourg",
        "LU",
        &verify_iso_match(&client, "LU", "Luxembourg").await,
    )
    .await;
    print_result(
        "Latvia",
        "LV",
        &verify_iso_match(&client, "LV", "Latvia").await,
    )
    .await;
    print_result(
        "Libya",
        "LY",
        &verify_iso_match(&client, "LY", "Libya").await,
    )
    .await;
    print_result(
        "Morocco",
        "MA",
        &verify_iso_match(&client, "MA", "Morocco").await,
    )
    .await;
    print_result(
        "Monaco",
        "MC",
        &verify_iso_match(&client, "MC", "Monaco").await,
    )
    .await;
    print_result(
        "Moldova",
        "MD",
        &verify_iso_match(&client, "MD", "Moldova").await,
    )
    .await;
    print_result(
        "Montenegro",
        "ME",
        &verify_iso_match(&client, "ME", "Montenegro").await,
    )
    .await;
    print_result(
        "Saint Martin",
        "MF",
        &verify_iso_match(&client, "MF", "Saint Martin").await,
    )
    .await;
    print_result(
        "Madagascar",
        "MG",
        &verify_iso_match(&client, "MG", "Madagascar").await,
    )
    .await;
    print_result(
        "Marshall Islands",
        "MH",
        &verify_iso_match(&client, "MH", "Marshall Islands").await,
    )
    .await;
    print_result(
        "North Macedonia",
        "MK",
        &verify_iso_match(&client, "MK", "North Macedonia").await,
    )
    .await;
    print_result("Mali", "ML", &verify_iso_match(&client, "ML", "Mali").await).await;
    print_result(
        "Myanmar",
        "MM",
        &verify_iso_match(&client, "MM", "Myanmar").await,
    )
    .await;
    print_result(
        "Mongolia",
        "MN",
        &verify_iso_match(&client, "MN", "Mongolia").await,
    )
    .await;
    print_result(
        "Macao",
        "MO",
        &verify_iso_match(&client, "MO", "Macau").await,
    )
    .await; //tag
    print_result(
        "Northern Mariana Islands",
        "MP",
        &verify_iso_match(&client, "MP", "Northern Mariana Islands").await,
    )
    .await;
    print_result(
        "Martinique",
        "MQ",
        &verify_iso_match(&client, "MQ", "Martinique").await,
    )
    .await;
    print_result(
        "Mauritania",
        "MR",
        &verify_iso_match(&client, "MR", "Mauritania").await,
    )
    .await;
    print_result(
        "Montserrat",
        "MS",
        &verify_iso_match(&client, "MS", "Montserrat").await,
    )
    .await;
    print_result(
        "Malta",
        "MT",
        &verify_iso_match(&client, "MT", "Malta").await,
    )
    .await;
    print_result(
        "Mauritius",
        "MU",
        &verify_iso_match(&client, "MU", "Mauritius").await,
    )
    .await;
    print_result(
        "Maldives",
        "MV",
        &verify_iso_match(&client, "MV", "Maldives").await,
    )
    .await;
    print_result(
        "Malawi",
        "MW",
        &verify_iso_match(&client, "MW", "Malawi").await,
    )
    .await;
    print_result(
        "Mexico",
        "MX",
        &verify_iso_match(&client, "MX", "Mexico").await,
    )
    .await;
    print_result(
        "Malaysia",
        "MY",
        &verify_iso_match(&client, "MY", "Malaysia").await,
    )
    .await;
    print_result(
        "Mozambique",
        "MZ",
        &verify_iso_match(&client, "MZ", "Mozambique").await,
    )
    .await;
    print_result(
        "Namibia",
        "NA",
        &verify_iso_match(&client, "NA", "Namibia").await,
    )
    .await;
    print_result(
        "New Caledonia",
        "NC",
        &verify_iso_match(&client, "NC", "New Caledonia").await,
    )
    .await;
    print_result(
        "Niger",
        "NE",
        &verify_iso_match(&client, "NE", "Niger").await,
    )
    .await;
    print_result(
        "Norfolk Island",
        "NF",
        &verify_iso_match(&client, "NF", "Norfolk Island").await,
    )
    .await;
    print_result(
        "Nigeria",
        "NG",
        &verify_iso_match(&client, "NG", "Nigeria").await,
    )
    .await;
    print_result(
        "Nicaragua",
        "NI",
        &verify_iso_match(&client, "NI", "Nicaragua").await,
    )
    .await;
    print_result(
        "Netherlands",
        "NL",
        &verify_iso_match(&client, "NL", "Netherlands").await,
    )
    .await;
    print_result(
        "Norway",
        "NO",
        &verify_iso_match(&client, "NO", "Norway").await,
    )
    .await;
    print_result(
        "Nepal",
        "NP",
        &verify_iso_match(&client, "NP", "Nepal").await,
    )
    .await;
    print_result(
        "Nauru",
        "NR",
        &verify_iso_match(&client, "NR", "Nauru").await,
    )
    .await;
    print_result("Niue", "NU", &verify_iso_match(&client, "NU", "Niue").await).await;
    print_result(
        "New Zealand",
        "NZ",
        &verify_iso_match(&client, "NZ", "New Zealand").await,
    )
    .await;
    print_result("Oman", "OM", &verify_iso_match(&client, "OM", "Oman").await).await;
    print_result(
        "Panama",
        "PA",
        &verify_iso_match(&client, "PA", "Panama").await,
    )
    .await;
    print_result("Peru", "PE", &verify_iso_match(&client, "PE", "Peru").await).await;
    print_result(
        "French Polynesia",
        "PF",
        &verify_iso_match(&client, "PF", "French Polynesia").await,
    )
    .await;
    print_result(
        "Papua New Guinea",
        "PG",
        &verify_iso_match(&client, "PG", "Papua New Guinea").await,
    )
    .await;
    print_result(
        "Philippines",
        "PH",
        &verify_iso_match(&client, "PH", "Philippines").await,
    )
    .await;
    print_result(
        "Pakistan",
        "PK",
        &verify_iso_match(&client, "PK", "Pakistan").await,
    )
    .await;
    print_result(
        "Poland",
        "PL",
        &verify_iso_match(&client, "PL", "Poland").await,
    )
    .await;
    print_result(
        "Saint Pierre and Miquelon",
        "PM",
        &verify_iso_match(&client, "PM", "Saint Pierre and Miquelon").await,
    )
    .await;
    print_result(
        "Pitcairn Islands",
        "PN",
        &verify_iso_match(&client, "PN", "Pitcairn Islands").await,
    )
    .await;
    print_result(
        "Puerto Rico",
        "PR",
        &verify_iso_match(&client, "PR", "Puerto Rico").await,
    )
    .await;
    print_result(
        "Palestinian Territory",
        "PS",
        &verify_iso_match(&client, "PS", "State of Palestine").await,
    )
    .await;
    print_result(
        "Portugal",
        "PT",
        &verify_iso_match(&client, "PT", "Portugal").await,
    )
    .await;
    print_result(
        "Palau",
        "PW",
        &verify_iso_match(&client, "PW", "Palau").await,
    )
    .await;
    print_result(
        "Paraguay",
        "PY",
        &verify_iso_match(&client, "PY", "Paraguay").await,
    )
    .await;
    print_result(
        "Qatar",
        "QA",
        &verify_iso_match(&client, "QA", "Qatar").await,
    )
    .await;
    print_result(
        "Réunion",
        "RE",
        &verify_iso_match(&client, "RE", "Réunion").await,
    )
    .await;
    print_result(
        "Romania",
        "RO",
        &verify_iso_match(&client, "RO", "Romania").await,
    )
    .await;
    print_result(
        "Serbia",
        "RS",
        &verify_iso_match(&client, "RS", "Serbia").await,
    )
    .await;
    print_result(
        "Russia",
        "RU",
        &verify_iso_match(&client, "RU", "Russia").await,
    )
    .await;
    print_result(
        "Rwanda",
        "RW",
        &verify_iso_match(&client, "RW", "Rwanda").await,
    )
    .await;
    print_result(
        "Saudi Arabia",
        "SA",
        &verify_iso_match(&client, "SA", "Saudi Arabia").await,
    )
    .await;
    print_result(
        "Solomon Islands",
        "SB",
        &verify_iso_match(&client, "SB", "Solomon Islands").await,
    )
    .await;
    print_result(
        "Seychelles",
        "SC",
        &verify_iso_match(&client, "SC", "Seychelles").await,
    )
    .await;
    print_result(
        "Sudan",
        "SD",
        &verify_iso_match(&client, "SD", "Sudan").await,
    )
    .await;
    print_result(
        "Sweden",
        "SE",
        &verify_iso_match(&client, "SE", "Sweden").await,
    )
    .await;
    print_result(
        "Singapore",
        "SG",
        &verify_iso_match(&client, "SG", "Singapore").await,
    )
    .await;
    print_result(
        "Saint Helena",
        "SH",
        &verify_iso_match(&client, "SH", "Saint Helena").await,
    )
    .await;
    print_result(
        "Slovenia",
        "SI",
        &verify_iso_match(&client, "SI", "Slovenia").await,
    )
    .await;
    print_result(
        "Svalbard and Jan Mayen",
        "SJ",
        &verify_iso_match(&client, "SJ", "Svalbard and Jan Mayen").await,
    )
    .await;
    print_result(
        "Slovakia",
        "SK",
        &verify_iso_match(&client, "SK", "Slovakia").await,
    )
    .await;
    print_result(
        "Sierra Leone",
        "SL",
        &verify_iso_match(&client, "SL", "Sierra Leone").await,
    )
    .await;
    print_result(
        "San Marino",
        "SM",
        &verify_iso_match(&client, "SM", "San Marino").await,
    )
    .await;
    print_result(
        "Senegal",
        "SN",
        &verify_iso_match(&client, "SN", "Senegal").await,
    )
    .await;
    print_result(
        "Somalia",
        "SO",
        &verify_iso_match(&client, "SO", "Somalia").await,
    )
    .await;
    print_result(
        "Suriname",
        "SR",
        &verify_iso_match(&client, "SR", "Suriname").await,
    )
    .await;
    print_result(
        "South Sudan",
        "SS",
        &verify_iso_match(&client, "SS", "South Sudan").await,
    )
    .await;
    print_result(
        "São Tomé and Príncipe",
        "ST",
        &verify_iso_match(&client, "ST", "São Tomé and Príncipe").await,
    )
    .await;
    print_result(
        "El Salvador",
        "SV",
        &verify_iso_match(&client, "SV", "El Salvador").await,
    )
    .await;
    print_result(
        "Sint Maarten",
        "SX",
        &verify_iso_match(&client, "SX", "Sint Maarten").await,
    )
    .await;
    print_result(
        "Syria",
        "SY",
        &verify_iso_match(&client, "SY", "Syria").await,
    )
    .await;
    print_result(
        "Swaziland",
        "SZ",
        &verify_iso_match(&client, "SZ", "Eswatini").await,
    )
    .await;
    print_result(
        "Turks and Caicos Islands",
        "TC",
        &verify_iso_match(&client, "TC", "Turks and Caicos Islands").await,
    )
    .await;
    print_result("Chad", "TD", &verify_iso_match(&client, "TD", "Chad").await).await;
    print_result(
        "French Southern Territories",
        "TF",
        &verify_iso_match(&client, "TF", "French Southern and Antarctic Lands").await,
    )
    .await; //tag
    print_result("Togo", "TG", &verify_iso_match(&client, "TG", "Togo").await).await;
    print_result(
        "Thailand",
        "TH",
        &verify_iso_match(&client, "TH", "Thailand").await,
    )
    .await;
    print_result(
        "Tajikistan",
        "TJ",
        &verify_iso_match(&client, "TJ", "Tajikistan").await,
    )
    .await;
    print_result(
        "Tokelau",
        "TK",
        &verify_iso_match(&client, "TK", "Tokelau").await,
    )
    .await;
    print_result(
        "Timor-Leste",
        "TL",
        &verify_iso_match(&client, "TL", "East Timor").await,
    )
    .await; //tag
    print_result(
        "Turkmenistan",
        "TM",
        &verify_iso_match(&client, "TM", "Turkmenistan").await,
    )
    .await;
    print_result(
        "Tunisia",
        "TN",
        &verify_iso_match(&client, "TN", "Tunisia").await,
    )
    .await;
    print_result(
        "Tonga",
        "TO",
        &verify_iso_match(&client, "TO", "Tonga").await,
    )
    .await;
    print_result(
        "Turkey",
        "TR",
        &verify_iso_match(&client, "TR", "Turkey").await,
    )
    .await;
    print_result(
        "Trinidad and Tobago",
        "TT",
        &verify_iso_match(&client, "TT", "Trinidad and Tobago").await,
    )
    .await;
    print_result(
        "Tuvalu",
        "TV",
        &verify_iso_match(&client, "TV", "Tuvalu").await,
    )
    .await;
    print_result(
        "Taiwan",
        "TW",
        &verify_iso_match(&client, "TW", "Taiwan").await,
    )
    .await;
    print_result(
        "Tanzania",
        "TZ",
        &verify_iso_match(&client, "TZ", "Tanzania").await,
    )
    .await;
    print_result(
        "Ukraine",
        "UA",
        &verify_iso_match(&client, "UA", "Ukraine").await,
    )
    .await;
    print_result(
        "Uganda",
        "UG",
        &verify_iso_match(&client, "UG", "Uganda").await,
    )
    .await;
    print_result(
        "United States Minor Outlying Islands",
        "UM",
        &verify_iso_match(&client, "UM", "United States Minor Outlying Islands").await,
    )
    .await;
    print_result(
        "United States",
        "US",
        &verify_iso_match(&client, "US", "United States of America").await,
    )
    .await; //tag
    print_result(
        "Uruguay",
        "UY",
        &verify_iso_match(&client, "UY", "Uruguay").await,
    )
    .await;
    print_result(
        "Uzbekistan",
        "UZ",
        &verify_iso_match(&client, "UZ", "Uzbekistan").await,
    )
    .await;
    print_result(
        "Vatican City",
        "VA",
        &verify_iso_match(&client, "VA", "Vatican City").await,
    )
    .await;
    print_result(
        "Saint Vincent and the Grenadines",
        "VC",
        &verify_iso_match(&client, "VC", "Saint Vincent and the Grenadines").await,
    )
    .await;
    print_result(
        "Venezuela",
        "VE",
        &verify_iso_match(&client, "VE", "Venezuela").await,
    )
    .await;
    print_result(
        "British Virgin Islands",
        "VG",
        &verify_iso_match(&client, "VG", "British Virgin Islands").await,
    )
    .await;
    print_result(
        "United States Virgin Islands",
        "VI",
        &verify_iso_match(&client, "VI", "United States Virgin Islands").await,
    )
    .await;
    print_result(
        "Vietnam",
        "VN",
        &verify_iso_match(&client, "VN", "Vietnam").await,
    )
    .await;
    print_result(
        "Vanuatu",
        "VU",
        &verify_iso_match(&client, "VU", "Vanuatu").await,
    )
    .await;
    print_result(
        "Wallis and Futuna",
        "WF",
        &verify_iso_match(&client, "WF", "Wallis and Futuna").await,
    )
    .await;
    print_result(
        "Samoa",
        "WS",
        &verify_iso_match(&client, "WS", "Samoa").await,
    )
    .await;
    print_result(
        "Yemen",
        "YE",
        &verify_iso_match(&client, "YE", "Yemen").await,
    )
    .await;
    print_result(
        "Mayotte",
        "YT",
        &verify_iso_match(&client, "YT", "Mayotte").await,
    )
    .await;
    print_result(
        "South Africa",
        "ZA",
        &verify_iso_match(&client, "ZA", "South Africa").await,
    )
    .await;
    print_result(
        "Zambia",
        "ZM",
        &verify_iso_match(&client, "ZM", "Zambia").await,
    )
    .await;
    print_result(
        "Zimbabwe",
        "ZW",
        &verify_iso_match(&client, "ZW", "Zimbabwe").await,
    )
    .await;
}
