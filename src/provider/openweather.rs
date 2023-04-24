//! Weather provider [OpenWeather](https://openweathermap.org).
//!

use std::{
    io::{stdin, stdout, Write},
    time::Duration,
};

use chrono::{DateTime, Local, TimeZone, Utc};
use reqwest::blocking::Client;
use serde_json::{Map, Value};

use crate::{geo::Geo, init::Date, wind::WindDeg, work::Provider};

/// Describes 'OpenWeather' credentials
///
/// * `name: &'static str` - Provider name.
/// * `key: Option<String>` - Api key.
pub struct OpenWeather {
    /// Provider name.
    name: &'static str,
    /// Api key.
    key: Option<String>,
}

/// OpenWeather data format for one item
#[derive(Debug)]
struct OpenWeatherItem {
    /// Time of data calculation from provider. Local
    date: DateTime<Local>,
    /// Request Address
    address: String,
    /// Geo position
    geo: Geo,
    /// Group of weather parameters (Rain, Snow, Extreme etc.)
    group: Option<String>,
    /// Temperature. Metric: Celsius
    temp: Option<f32>,
    /// Temperature. This temperature parameter accounts for the human perception of weather. Metric: Celsius
    feels_like: Option<f32>,
    /// Atmospheric pressure (on the sea level, if there is no sea_level or grnd_level data), hPa
    pressure: Option<u32>,
    /// Humidity, %
    humidity: Option<u32>,
    /// Visibility, meter.
    visibility: Option<u32>,
    /// Wind speed. Metric: meter/sec
    speed: Option<f32>,
    /// Wind degrees (meteorological)
    deg: Option<u16>,
    /// Wind direction (meteorological)
    dir: WindDeg,
    /// Wind gust. Metric: meter/sec
    gust: Option<f32>,
    /// Rain volume for the last 1 hour, mm
    rain1: Option<f32>,
    /// Rain volume for the last 3 hour, mm
    rain3: Option<f32>,
    /// Snow volume for the last 1 hour, mm
    snow1: Option<f32>,
    /// Snow volume for the last 3 hour, mm
    snow3: Option<f32>,
    /// Sunrise time. Local
    sunrise: Option<DateTime<Local>>,
    /// Sunset time. Local
    sunset: Option<DateTime<Local>>,
}

impl OpenWeather {
    /// Create new empty provider
    pub fn new() -> OpenWeather {
        OpenWeather {
            name: "OpenWeather",
            key: None,
        }
    }

    /// Load data from provider
    fn get_json(&self, url: &str, address: &str) -> Option<(Map<String, Value>, Geo)> {
        let key = match &self.key {
            Some(key) => key,
            None => {
                println!("OpenWeather server API access key is not set. Please install it first.");
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
            "{}?lat={}&lon={}&appid={}&units=metric",
            url, geo.lat, geo.lon, key
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
    fn get_now(&self, address: String) -> Option<OpenWeatherItem> {
        let (items, geo) =
            self.get_json("https://api.openweathermap.org/data/2.5/weather", &address)?;
        self.detect(&items, geo, address, None, None)
    }

    /// Getting weather forecast for `date`
    fn get_date(&self, address: String, date: &DateTime<Local>) -> Option<OpenWeatherItem> {
        // Load json from provider
        let (items, geo) =
            self.get_json("https://api.openweathermap.org/data/2.5/forecast", &address)?;
        // Detect sunrise and sunset, because provider returns different jsons for 'now' and 'date'
        let sunrise = items
            .get("city")
            .and_then(|m| m.get("sunrise"))
            .and_then(|s| s.as_i64())
            .and_then(|t| Utc.timestamp_opt(t, 0).single())
            .map(|t| Local.from_utc_datetime(&t.naive_utc()));
        let sunset = items
            .get("city")
            .and_then(|m| m.get("sunset"))
            .and_then(|s| s.as_i64())
            .and_then(|t| Utc.timestamp_opt(t, 0).single())
            .map(|t| Local.from_utc_datetime(&t.naive_utc()));

        // Get list of OpenWeatherItem
        let its = items
            .get("list")
            .and_then(|its| its.as_array())
            .or_else(|| {
                println!("The OpenWeather server did not provide weather forecast data");
                None
            })?;
        // Load all OpenWeatherItem to vector
        let mut list = Vec::with_capacity(40);
        for item in its {
            if let Value::Object(map) = item {
                let res = self.detect(map, geo.clone(), address.clone(), sunset, sunrise);
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
        sunrise: Option<DateTime<Local>>,
        sunset: Option<DateTime<Local>>,
    ) -> Option<OpenWeatherItem> {
        let group = items
            .get("weather")
            .and_then(|a| a.get(0))
            .and_then(|m| m.get("main"))
            .and_then(|s| s.as_str())
            .map(|s| s.to_owned());
        let temp = items
            .get("main")
            .and_then(|m| m.get("temp"))
            .and_then(|s| s.as_f64())
            .map(|s| s as f32);
        let feels_like = items
            .get("main")
            .and_then(|m| m.get("feels_like"))
            .and_then(|s| s.as_f64())
            .map(|s| s as f32);
        let pressure = items
            .get("main")
            .and_then(|m| m.get("pressure"))
            .and_then(|s| s.as_u64())
            .map(|s| s as u32);
        let humidity = items
            .get("main")
            .and_then(|m| m.get("humidity"))
            .and_then(|s| s.as_u64())
            .map(|s| s as u32);
        let visibility = items
            .get("visibility")
            .and_then(|s| s.as_u64())
            .map(|s| s as u32);
        let speed = items
            .get("wind")
            .and_then(|m| m.get("speed"))
            .and_then(|s| s.as_f64())
            .map(|s| s as f32);
        let deg = items
            .get("wind")
            .and_then(|m| m.get("deg"))
            .and_then(|s| s.as_u64())
            .map(|s| s as u16);
        let dir = WindDeg::get(deg);
        let gust = items
            .get("wind")
            .and_then(|m| m.get("gust"))
            .and_then(|s| s.as_f64())
            .map(|s| s as f32);
        let rain1 = items
            .get("rain")
            .and_then(|m| m.get("1h"))
            .and_then(|s| s.as_f64())
            .map(|s| s as f32);
        let rain3 = items
            .get("rain")
            .and_then(|m| m.get("3h"))
            .and_then(|s| s.as_f64())
            .map(|s| s as f32);
        let snow1 = items
            .get("snow")
            .and_then(|m| m.get("1h"))
            .and_then(|s| s.as_f64())
            .map(|s| s as f32);
        let snow3 = items
            .get("snow")
            .and_then(|m| m.get("3h"))
            .and_then(|s| s.as_f64())
            .map(|s| s as f32);
        let date = items
            .get("dt")
            .and_then(|s| s.as_i64())
            .and_then(|t| Utc.timestamp_opt(t, 0).single())
            .map(|t| Local.from_utc_datetime(&t.naive_utc()))?;
        let sunrise = sunrise.or_else(|| {
            items
                .get("sys")
                .and_then(|m| m.get("sunrise"))
                .and_then(|s| s.as_i64())
                .and_then(|t| Utc.timestamp_opt(t, 0).single())
                .map(|t| Local.from_utc_datetime(&t.naive_utc()))
        });
        let sunset = sunset.or_else(|| {
            items
                .get("sys")
                .and_then(|m| m.get("sunset"))
                .and_then(|s| s.as_i64())
                .and_then(|t| Utc.timestamp_opt(t, 0).single())
                .map(|t| Local.from_utc_datetime(&t.naive_utc()))
        });

        Some(OpenWeatherItem {
            date,
            address,
            geo,
            group,
            temp,
            feels_like,
            pressure,
            humidity,
            visibility,
            speed,
            deg,
            dir,
            gust,
            rain1,
            rain3,
            snow1,
            snow3,
            sunrise,
            sunset,
        })
    }

    /// Display result
    #[rustfmt::skip]
    fn show(&self, item: &OpenWeatherItem, duration: i64, date: &str) {
        println!("Weather for '{}'. OpenWeather server. Request time {} ms.", date, duration);
        println!("Request address: {}.", item.address);
        println!("Found address: {} ({},{}).", item.geo.address, item.geo.lat, item.geo.lon);
        println!("Forecast date on the server: {}", item.date.format("%Y-%m-%d %H:%M:%S (%:z)"));
        println!("{}", "-".repeat(40));
        println!("Group of weather parameters  : {}", item.group.as_ref().map_or("None".to_owned(), |s| s.to_owned()));
        println!("Temperature                  : {}", item.temp.map_or("None".to_owned(), |s| format!("{:#.1} °C", s)));
        println!("Human perception temperature : {}", item.feels_like.map_or("None".to_owned(), |s| format!("{:#.1} °C", s)));
        println!("Atmospheric pressure         : {}", item.pressure.map_or("None".to_owned(), |s| s.to_string() + " hPa"));
        println!("Humidity                     : {}", item.humidity.map_or("None".to_owned(), |s| s.to_string() + " %"));
        println!("Wind speed                   : {}", item.speed.map_or("None".to_owned(), |s| format!("{:#.1} meter/sec", s)));
        println!("Wind direction and degrees   : {:?} ({})", item.dir, item.deg.map_or("None".to_owned(), |s| s.to_string() + "°"));
        println!("Wind gust                    : {}", item.gust.map_or("None".to_owned(), |s| format!("{:#.1} meter/sec", s)));
        println!("Rain volume (last 1 hour)    : {}", item.rain1.map_or("None".to_owned(), |s| format!("{:#.1} mm", s)));
        println!("Rain volume (last 3 hour)    : {}", item.rain3.map_or("None".to_owned(), |s| format!("{:#.1} mm", s)));
        println!("Snow volume (last 1 hour)    : {}", item.snow1.map_or("None".to_owned(), |s| format!("{:#.1} mm", s)));
        println!("Snow volume (last 3 hour)    : {}", item.snow3.map_or("None".to_owned(), |s| format!("{:#.1} mm", s)));
        println!("Visibility                   : {}", item.visibility.map_or("None".to_owned(), |s| s.to_string() + " meter"));
        println!("Sunrise time                 : {}", item.sunrise.map_or("None".to_owned(), |dt| dt.format("%Y-%m-%d %H:%M:%S (%:z)").to_string()));
        println!("Sunset time                  : {}", item.sunset.map_or("None".to_owned(), |dt| dt.format("%Y-%m-%d %H:%M:%S (%:z)").to_string()));
    }
}

impl Provider for OpenWeather {
    fn serialize(&self) -> String {
        match &self.key {
            Some(key) => format!("{}:{}", self.name, key),
            None => format!("{}:", self.name),
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
        let key = match input.next() {
            Some(key) => key.to_owned(),
            None => {
                println!("The data file structure is damaged. The data file will be deleted.");
                return false;
            }
        };
        if key.is_empty() {
            self.key = None;
            return true;
        }
        self.key = Some(key);
        true
    }

    fn get_weather(&self, address: String, date: Date) {
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
        match &self.key {
            Some(key) => print!(
                "Please enter the API key to access the weather forecast. Current key={}: ",
                key
            ),
            None => print!("Please enter the API key to access the weather forecast: "),
        }
        if let Err(e) = stdout().flush() {
            print!("System error: {}\n\nFailed to set key.", e);
            return;
        };
        let mut input = String::new();
        if let Err(e) = stdin().read_line(&mut input) {
            print!(
                "The key must be only printed characters. Error: {}\n\nFailed to set key.",
                e
            );
            return;
        }
        let key = input.trim().to_string();
        if key.is_empty() {
            print!("The key was removed successfully.");
            self.key = None;
        } else {
            print!("The key '{}' was setted successfully.", key);
            self.key = Some(key);
        }
    }
}

impl Default for OpenWeather {
    fn default() -> OpenWeather {
        OpenWeather::new()
    }
}
