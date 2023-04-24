//! The module responsible for detecting Geo data be user address via [Nominatim](https://nominatim.openstreetmap.org).
//!

use std::time::Duration;

use reqwest::blocking::Client;
use serde::Deserialize;
use urlencoding::encode;

/// Determine geographic coordinates by address string.
///
/// * `pub lat: String` - Latitude.
/// * `pub lon: String` - Longitude.
/// * `pub address: String` - Full address.
#[derive(Debug, Deserialize, PartialEq, Clone)]
pub struct Geo {
    /// Latitude.
    pub lat: String,
    /// Longitude.
    pub lon: String,
    /// Full address.
    #[serde(rename = "display_name")]
    pub address: String,
}

impl Geo {
    /// Get geographic coordinates by address string.
    pub fn get(address: &str) -> Option<Vec<Geo>> {
        let url = format!(
            "https://nominatim.openstreetmap.org/search?q={}&format=json&limit=1",
            encode(address)
        );

        // Client for url query
        let client = match Client::builder().timeout(Duration::from_secs(3)).build() {
            Ok(c) => c,
            Err(e) => {
                println!("The following error occurred while requesting coordinates for your address: {}", e);
                return None;
            }
        };
        let json_str = match client.get(&url).header("User-Agent", "weather bot").send() {
            Ok(s) => {
                let status = s.status();
                if status != 200 {
                    println!("Error connecting to {}. Status code: {}", &url, status);
                    return None;
                }
                match s.text() {
                    Ok(s) => s,
                    Err(e) => {
                        println!("Error getting answer from {}. Error text: {}", &url, e);
                        return None;
                    }
                }
            }
            Err(e) => {
                println!("Error connecting to {}. Error text: {}", &url, e);
                return None;
            }
        };
        // Parse json
        let geo: Option<Vec<Geo>> = match serde_json::from_str(&json_str) {
            Ok(geo) => geo,
            Err(e) => {
                println!(
                    "Unable to recognize json response from server. Error text: {}",
                    e
                );
                return None;
            }
        };
        geo
    }
}

#[cfg(test)]
mod tests {
    use crate::geo::Geo;

    #[test]
    fn test_geo() {
        assert_eq!(
            Geo::get("Kyiv, Ukraine"),
            Some(vec![Geo {
                lat: "50.4500336".to_owned(),
                lon: "30.5241361".to_owned(),
                address: "Київ, Україна".to_owned()
            }])
        );
        assert_eq!(Geo::get("Дніпро, Україна"), Some(vec![ Geo { lat: "48.4680221".to_owned(), lon: "35.0417711".to_owned(), address: "Дніпро, Дніпровська міська громада, Дніпровський район, Дніпропетровська область, 49000, Україна".to_owned() }]));
        assert_eq!(Geo::get("unknown galaxy"), Some(vec![]));
    }
}
