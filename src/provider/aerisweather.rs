//! Weather provider [AerisWeather](https://aerisapi.com).
//!

use std::{
    io::{stdin, stdout, Write},
    time::Duration,
};

use chrono::{DateTime, Local, TimeZone, Utc};
use reqwest::blocking::Client;
use serde_json::{Map, Value};

use crate::{geo::Geo, init::Date, wind::WindDeg, work::Provider};

/// Describes 'AerisWeather' credentials
///
/// * `name: &'static str` - Provider name.
/// * `key: Option<(String, String)>` - Turple of client_id and client_secret.
pub struct AerisWeather {
    /// Provider name.
    name: &'static str,
    /// Api key.
    key: Option<(String, String)>,
}

/// Temperature representation
#[derive(Debug)]
enum TempView {
    /// None
    None,
    /// One value
    Single(f32),
    // Min and max value
    MinMax((f32, f32)),
}

/// AerisWeather data format for one item
#[derive(Debug)]
struct AerisWeatherItem {
    /// Time of data calculation from provider. Local
    date: DateTime<Local>,
    /// Request Address
    address: String,
    /// Geo position
    geo: Geo,
    /// weather phrase
    weather: Option<String>,
    /// Temperature
    temp_c: TempView,
    /// The dew point temperature
    dewpoint_c: Option<f32>,
    /// Humidity percentage
    humidity: Option<u16>,
    /// Barometric pressure in millibars
    pressure_mb: Option<u32>,
    /// Wind Speed
    wind_speed_kph: Option<f32>,
    /// Wind direction in Azimuth degrees
    wind_dir_deg: Option<u16>,
    /// Wind direction (meteorological)
    dir: WindDeg,
    /// Wind gust speed
    wind_gust_kph: Option<f32>,
    /// Visibility
    visibility_km: Option<f32>,
    /// RealFeel temperature
    feelslike_c: Option<f32>,
    /// Snow
    snow_depth_cm: Option<u16>,
    /// Precipitation
    precip_mm: Option<u16>,
    /// Measure of the strength of the ultraviolet radiation from the sun
    uvi: Option<u16>,
    /// Number representing the percentage of the sky that is covered by clouds
    sky: Option<u16>,
    /// Sun rise
    sunrise: Option<DateTime<Local>>,
    /// Sun set
    sunset: Option<DateTime<Local>>,
}

impl AerisWeather {
    pub fn new() -> AerisWeather {
        AerisWeather {
            name: "AerisWeather",
            key: None,
        }
    }
    /// Load data from provider
    fn get_json(&self, url: &str, address: &str) -> Option<(Map<String, Value>, Geo)> {
        let (id, secret) = match &self.key {
            Some(key) => key,
            None => {
                println!("AerisWeather server API access key is not set. Please install it first.");
                return None;
            }
        };
        // Find geo coordinates by address
        let geo = match Geo::get(address) {
            Some(mut geos) => match geos.pop() {
                Some(geo) => geo,
                None => {
                    println!("Sorry, we couldn't find your address: {}", address);
                    return None;
                }
            },
            None => return None,
        };
        let url = format!(
            "{}/{},{}?&format=json&client_id={}&client_secret={}",
            url, geo.lat, geo.lon, id, secret
        );
        // Client for url query
        let client = match Client::builder().timeout(Duration::from_secs(3)).build() {
            Ok(c) => c,
            Err(e) => {
                println!("The following error occurred while requesting coordinates for your address: {}", e);
                return None;
            }
        };

        let json_str = match client.get(&url).send() {
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
        match serde_json::from_str(&json_str) {
            Ok(json) => Some((json, geo)),
            Err(e) => {
                println!(
                    "Unable to recognize json response from server. Error text: {}",
                    e
                );
                None
            }
        }
    }

    /// Getting weather forecast for now
    fn get_now(&self, address: String) -> Option<AerisWeatherItem> {
        let (items, geo) = self.get_json("https://api.aerisapi.com/observations", &address)?;
        let item = items
            .get("response")
            .and_then(|s| s.get("ob"))
            .and_then(|s| s.as_object())?;
        self.detect(item, geo, address)
    }

    /// Getting weather forecast for `date`
    fn get_date(&self, address: String, date: &DateTime<Local>) -> Option<AerisWeatherItem> {
        // Load json from provider
        let (items, geo) = self.get_json("https://api.aerisapi.com/forecasts", &address)?;

        // Get list of AerisWeatherItem
        let its = items
            .get("response")
            .and_then(|its| its.get(0))
            .and_then(|its| its.get("periods"))
            .and_then(|its| its.as_array())
            .or_else(|| {
                println!("The AerisWeather server did not provide weather forecast data");
                return None;
            })?;
        // Load all AerisWeatherItem to vector
        let mut list = Vec::with_capacity(40);
        for item in its {
            if let Value::Object(map) = item {
                let res = self.detect(map, geo.clone(), address.clone());
                if let Some(item) = res {
                    list.push(item);
                }
            }
        }
        if list.is_empty() {
            return None;
        }
        // Find item with the closest date
        list.into_iter().min_by(|item_a, item_b| {
            let diff_a = item_a.date.signed_duration_since(*date).num_seconds().abs();
            let diff_b = item_b.date.signed_duration_since(*date).num_seconds().abs();

            diff_a.cmp(&diff_b)
        })
    }

    /// Parse json answer from server
    fn detect(
        &self,
        items: &Map<String, Value>,
        geo: Geo,
        address: String,
    ) -> Option<AerisWeatherItem> {
        let date = items
            .get("timestamp")
            .and_then(|s| s.as_i64())
            .and_then(|t| Utc.timestamp_opt(t, 0).single())
            .map(|t| Local.from_utc_datetime(&t.naive_utc()))?;
        let weather = items
            .get("weather")
            .and_then(|s| s.as_str())
            .map(|s| s.to_owned());
        let temp = items
            .get("tempC")
            .and_then(|s| s.as_f64())
            .map(|s| s as f32);
        let min = items
            .get("minTempC")
            .and_then(|s| s.as_f64())
            .map(|s| s as f32);
        let max = items
            .get("maxTempC")
            .and_then(|s| s.as_f64())
            .map(|s| s as f32);
        let temp_c = match (temp, min, max) {
            (Some(temp), _, _) => TempView::Single(temp),
            (None, Some(min), Some(max)) => TempView::MinMax((min, max)),
            _ => TempView::None,
        };
        let dewpoint_c = items
            .get("dewpointC")
            .and_then(|s| s.as_f64())
            .map(|s| s as f32);
        let humidity = items
            .get("humidity")
            .and_then(|s| s.as_u64())
            .map(|s| s as u16);
        let pressure_mb = items
            .get("pressureMB")
            .and_then(|s| s.as_u64())
            .map(|s| s as u32);
        let wind_speed_kph = items
            .get("windSpeedKPH")
            .and_then(|s| s.as_f64())
            .map(|s| s as f32);
        let wind_dir_deg = items
            .get("windDirDEG")
            .and_then(|s| s.as_u64())
            .map(|s| s as u16);
        let dir = WindDeg::get(wind_dir_deg);
        let wind_gust_kph = items
            .get("windGustKPH")
            .and_then(|s| s.as_f64())
            .map(|s| s as f32);
        let visibility_km = items
            .get("visibilityKM")
            .and_then(|s| s.as_f64())
            .map(|s| s as f32);
        let feelslike_c = items
            .get("feelslikeC")
            .and_then(|s| s.as_f64())
            .map(|s| s as f32);
        let snow_depth_cm = items
            .get("snowDepthCM")
            .and_then(|s| s.as_u64())
            .map(|s| s as u16);
        let precip_mm = items
            .get("precipMM")
            .and_then(|s| s.as_u64())
            .map(|s| s as u16);
        let uvi = items.get("uvi").and_then(|s| s.as_u64()).map(|s| s as u16);
        let sky = items.get("sky").and_then(|s| s.as_u64()).map(|s| s as u16);
        let sunrise = items
            .get("sunrise")
            .and_then(|s| s.as_i64())
            .and_then(|t| Utc.timestamp_opt(t, 0).single())
            .map(|t| Local.from_utc_datetime(&t.naive_utc()));
        let sunset = items
            .get("sunset")
            .and_then(|s| s.as_i64())
            .and_then(|t| Utc.timestamp_opt(t, 0).single())
            .map(|t| Local.from_utc_datetime(&t.naive_utc()));
        Some(AerisWeatherItem {
            date,
            address,
            geo,
            weather,
            temp_c,
            dewpoint_c,
            humidity,
            pressure_mb,
            wind_speed_kph,
            wind_dir_deg,
            dir,
            wind_gust_kph,
            visibility_km,
            feelslike_c,
            snow_depth_cm,
            precip_mm,
            uvi,
            sky,
            sunrise,
            sunset,
        })
    }

    /// Display result
    #[rustfmt::skip]
    fn show(&self, item: &AerisWeatherItem, duration: i64, date: &str) {
        println!("Weather for '{}'. AerisWeather server. Request time {} ms.", date, duration);
        println!("Request address: {}.", item.address);
        println!("Found address: {} ({},{}).", item.geo.address, item.geo.lat, item.geo.lon);
        println!("Forecast date on the server: {}", item.date.format("%Y-%m-%d %H:%M:%S (%:z)"));
        println!("{}", "-".repeat(40));
        println!("Sunrise time                 : {}", item.sunrise.map_or("None".to_owned(), |dt| dt.format("%Y-%m-%d %H:%M:%S (%:z)").to_string()));
        println!("Sunset time                  : {}", item.sunset.map_or("None".to_owned(), |dt| dt.format("%Y-%m-%d %H:%M:%S (%:z)").to_string()));
        println!("Weather description          : {}", item.weather.as_ref().map_or("None".to_owned(), |s| s.to_owned()));
        match item.temp_c {
            TempView::None =>              println!("Temperature                  : None"),
            TempView::Single(temp) => println!("Temperature                  : {}", format!("{:#.1} °C", temp)),
            TempView::MinMax((min, max)) => {
                                           println!("Temperature min              : {}", format!("{:#.1} °C", min));
                                           println!("Temperature max              : {}", format!("{:#.1} °C", max));
            },
        }
        println!("Dew point                    : {}", item.dewpoint_c.map_or("None".to_owned(), |s| format!("{:#.1} °C", s)));
        println!("Humidity                     : {}", item.humidity.map_or("None".to_owned(), |s| s.to_string() + " %"));
        println!("Atmospheric pressure         : {}", item.pressure_mb.map_or("None".to_owned(), |s| format!("{:#.1} mbar", s)));
        println!("Wind speed                   : {}", item.wind_speed_kph.map_or("None".to_owned(), |s| format!("{:#.1} km/hour", s)));
        println!("Wind direction and degrees   : {:?} ({})", item.dir, item.wind_dir_deg.map_or("None".to_owned(), |s| s.to_string() + "°"));
        println!("Wind gust                    : {}", item.wind_gust_kph.map_or("None".to_owned(), |s| format!("{:#.1} km/hou", s)));
        println!("Visibility                   : {}", item.visibility_km.map_or("None".to_owned(), |s| s.to_string() + " km"));
        println!("Human perception temperature : {}", item.feelslike_c.map_or("None".to_owned(), |s| format!("{:#.1} °C", s)));
        println!("Snow depth                   : {}", item.snow_depth_cm.map_or("None".to_owned(), |s| format!("{:#.1} sm", s)));
        println!("Precipitation depth          : {}", item.precip_mm.map_or("None".to_owned(), |s| format!("{:#.1} mm", s)));
        println!("UV Index                     : {}", item.uvi.map_or("None".to_owned(), |s| format!("{:#.1}", s)));
        println!("Cloud cover                  : {}", item.sky.map_or("None".to_owned(), |s| s.to_string() + " %"));

    }
}

impl Provider for AerisWeather {
    fn serialize(&self) -> String {
        match &self.key {
            Some((id, key)) => format!("{}:{}:{}", self.name, id, key),
            None => format!("{}::", self.name),
        }
    }

    fn deserialize(&mut self, data: &str) -> bool {
        let mut input = data.split(':');
        match input.next() {
            Some(name) => {
                if name != self.name {
                    return false;
                }
            }
            None => {
                println!("The data file structure is damaged. The data file will be deleted.");
                return false;
            }
        };
        let id = match input.next() {
            Some(id) => id.to_owned(),
            None => {
                println!("The data file structure is damaged. The data file will be deleted.");
                return false;
            }
        };
        let key = match input.next() {
            Some(key) => key.to_owned(),
            None => {
                println!("The data file structure is damaged. The data file will be deleted.");
                return false;
            }
        };
        if input.next().is_some() {
            println!("The data file structure is damaged. The data file will be deleted.");
            return false;
        }
        if id.is_empty() && key.is_empty() {
            self.key = None;
            return true;
        } else if !id.is_empty() && !key.is_empty() {
            self.key = Some((id, key));
            return true;
        }
        println!("The data file structure is damaged. The data file will be deleted.");
        false
    }

    fn get_weather(&self, address: String, date: Date) {
        // https://api.aerisapi.com/observations/50.468071,30.484137576584864?client_id=MoWpgnVwCeEqjy9bSFf2P&client_secret=n1KUHGW0i7ncFRw638p1ewsskPpA6c1GKi9G9SYT&format=json
        // https://api.aerisapi.com/forecasts/50.468071,30.484137576584864?client_id=MoWpgnVwCeEqjy9bSFf2P&client_secret=n1KUHGW0i7ncFRw638p1ewsskPpA6c1GKi9G9SYT&format=json
        match date {
            Date::Now => {
                let start = Local::now();
                let now = match self.get_now(address) {
                    Some(now) => now,
                    None => {
                        println!("It is not possible to determine the date of the weather forecast sent by the provider");
                        return;
                    }
                };
                let duration = Local::now() - start;
                self.show(&now, duration.num_milliseconds(), "now");
            }
            Date::Set(dt) => {
                let start = Local::now();
                let now = match self.get_date(address, &dt) {
                    Some(now) => now,
                    None => {
                        println!("It is not possible to determine the date of the weather forecast sent by the provider");
                        return;
                    }
                };
                let duration = Local::now() - start;
                self.show(
                    &now,
                    duration.num_milliseconds(),
                    &dt.format("%Y-%m-%d %H:%M:%S (%:z)").to_string(),
                );
            }
            _ => {}
        }
    }

    fn name(&self) -> &'static str {
        self.name
    }

    fn configure(&mut self) {
        println!("Configure credentials for {}: \n", self.name);
        // get client_id
        match &self.key {
            Some((client_id, _)) => print!(
                "Please enter the client_id to access the weather forecast. Current client_id={}: ",
                client_id
            ),
            None => print!("Please enter the client_id to access the weather forecast: "),
        }
        if let Err(e) = stdout().flush() {
            print!("System error: {}\n\nFailed to set client_id.", e);
            return;
        };
        let mut input = String::new();
        if let Err(e) = stdin().read_line(&mut input) {
            print!(
                "The key must be only printed characters. Error: {}\n\nFailed to set client_id.",
                e
            );
            return;
        }
        let client_id = input.trim().to_string();
        if client_id.is_empty() {
            print!("The client_id and client_secret was removed successfully.");
            self.key = None;
            return;
        }

        // get client_secret
        match &self.key {
            Some((_, client_secret)) => print!("Please enter the client_secret to access the weather forecast. Current client_secret={}: ", client_secret),
            None => print!("Please enter the client_secret to access the weather forecast: "),
        }
        if let Err(e) = stdout().flush() {
            print!("System error: {}\n\nFailed to set client_secret.", e);
            return;
        };
        let mut input = String::new();
        if let Err(e) = stdin().read_line(&mut input) {
            print!("The key must be only printed characters. Error: {}\n\nFailed to set client_secret.", e);
            return;
        }
        let client_secret = input.trim().to_string();
        if client_secret.is_empty() {
            print!("The client_secret can't be empty.");
            return;
        }
        print!(
            "The client_id '{}' and client_secret '{}' was setted successfully.",
            client_id, client_secret
        );
        self.key = Some((client_id, client_secret))
    }
}

impl Default for AerisWeather {
    fn default() -> AerisWeather {
        AerisWeather::new()
    }
}
