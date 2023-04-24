//! Weather provider [WeatherAPI](https://weatherapi.com).
//!

use std::{
    io::{stdin, stdout, Write},
    time::Duration,
};

use chrono::{DateTime, Local, TimeZone, Utc};
use reqwest::blocking::Client;
use serde_json::{Map, Value};

use crate::{geo::Geo, init::Date, wind::WindDeg, work::Provider};

/// Describes 'WeatherAPI' credentials
///
/// * `name: &'static str` - Provider name.
/// * `key: Option<String>` - Api key.
pub struct WeatherAPI {
    /// Provider name.
    name: &'static str,
    /// Api key
    key: Option<String>,
}

/// WeatherAPI data format for one item
#[derive(Debug)]
struct WeatherAPIItem {
    /// Time of data calculation from provider. Local
    date: DateTime<Local>,
    /// Request Address
    address: String,
    /// Geo position
    geo: Geo,
    /// Weather condition text
    condition: Option<String>,
    /// Temperature in celsius
    temp: Option<f32>,
    /// Feels like temperature in celsius
    feelslike: Option<f32>,
    /// Windchill temperature in celcius
    windchill: Option<f32>,
    /// Heat index in celcius
    heatindex: Option<f32>,
    /// Dew point in celcius
    dewpoint: Option<f32>,
    /// Wind speed in kilometer per hour
    wind: Option<f32>,
    /// Wind speed in kilometer per hour
    dir: WindDeg,
    /// Wind direction in degrees
    degree: Option<u16>,
    /// Wind gust in kilometer per hour
    gust: Option<f32>,
    /// Pressure in millibars
    pressure: Option<f32>,
    /// Precipitation amount in millimeters
    precip: Option<f32>,
    /// Humidity as percentage
    humidity: Option<u8>,
    /// Cloud cover as percentage
    cloud: Option<u8>,
    /// Will it will rain or not
    will_it_rain: Option<bool>,
    /// Chance of rain as percentage
    chance_of_rain: Option<u8>,
    /// Will it will snow or not
    will_it_snow: Option<bool>,
    /// Chance of snow as percentage
    chance_of_snow: Option<u8>,
    /// Visibility in kilometer
    vis: Option<f32>,
    /// UV Index
    uv: Option<f32>,
}

impl WeatherAPI {
    /// Create new empty provider
    pub fn new() -> WeatherAPI {
        WeatherAPI {
            name: "WeatherAPI",
            key: None,
        }
    }

    /// Load data from provider
    fn get_json(
        &self,
        url: &str,
        address: &str,
        date: Option<&str>,
    ) -> Option<(Map<String, Value>, Geo)> {
        let key = match &self.key {
            Some(key) => key,
            None => {
                println!("WeatherAPI server API access key is not set. Please install it first.");
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
        let url = match date {
            Some(d) => format!("{}?key={}&q={},{}&dt={}", url, key, geo.lat, geo.lon, d),
            None => format!("{}?key={}&q={},{}", url, key, geo.lat, geo.lon),
        };

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
    fn get_now(&self, address: String) -> Option<WeatherAPIItem> {
        let (items, geo) =
            self.get_json("https://api.weatherapi.com/v1/current.json", &address, None)?;
        let items = items
            .get("current")
            .and_then(|its| its.as_object())
            .or_else(|| {
                println!("The WeatherAPI server did not provide weather forecast data");
                return None;
            })?;
        self.detect(items, geo, address)
    }

    /// Getting weather forecast for `date`
    fn get_date(&self, address: String, date: &DateTime<Local>) -> Option<WeatherAPIItem> {
        // Load json from provider
        let dt = date.format("%Y-%m-%d").to_string();
        let (items, geo) = self.get_json(
            "https://api.weatherapi.com/v1/forecast.json",
            &address,
            Some(&dt),
        )?;
        // Get list of WeatherAPIItem
        let its = items
            .get("forecast")
            .and_then(|i| i.get("forecastday"))
            .and_then(|i| i.get(0))
            .and_then(|i| i.get("hour"))
            .and_then(|i| i.as_array())
            .or_else(|| {
                println!("The WeatherAPI server did not provide weather forecast data");
                return None;
            })?;
        // Load all WeatherAPIItem to vector
        let mut list = Vec::with_capacity(24);
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
    ) -> Option<WeatherAPIItem> {
        let condition = items
            .get("condition")
            .and_then(|m| m.get("text"))
            .and_then(|s| s.as_str())
            .map(|s| s.to_owned());
        let temp = items
            .get("temp_c")
            .and_then(|s| s.as_f64())
            .map(|s| s as f32);
        let feelslike = items
            .get("feelslike_c")
            .and_then(|s| s.as_f64())
            .map(|s| s as f32);
        let windchill = items
            .get("windchill_c")
            .and_then(|s| s.as_f64())
            .map(|s| s as f32);
        let heatindex = items
            .get("heatindex_c")
            .and_then(|s| s.as_f64())
            .map(|s| s as f32);
        let dewpoint = items
            .get("dewpoint_c")
            .and_then(|s| s.as_f64())
            .map(|s| s as f32);
        let wind = items
            .get("wind_kph")
            .and_then(|s| s.as_f64())
            .map(|s| s as f32);
        let degree = items
            .get("wind_degree")
            .and_then(|s| s.as_u64())
            .map(|s| s as u16);
        let dir = WindDeg::get(degree);
        let gust = items
            .get("gust_kph")
            .and_then(|s| s.as_f64())
            .map(|s| s as f32);
        let pressure = items
            .get("pressure_mb")
            .and_then(|s| s.as_f64())
            .map(|s| s as f32);
        let precip = items
            .get("precip_mm")
            .and_then(|s| s.as_f64())
            .map(|s| s as f32);
        let humidity = items
            .get("humidity")
            .and_then(|s| s.as_u64())
            .map(|s| s as u8);
        let cloud = items.get("cloud").and_then(|s| s.as_u64()).map(|s| s as u8);
        let chance_of_rain = items
            .get("chance_of_rain")
            .and_then(|s| s.as_u64())
            .map(|s| s as u8);
        let will_it_rain = items.get("will_it_rain").and_then(|s| s.as_u64()).map(|s| {
            if s == 1 {
                true
            } else {
                false
            }
        });
        let chance_of_snow = items
            .get("chance_of_snow")
            .and_then(|s| s.as_u64())
            .map(|s| s as u8);
        let will_it_snow = items.get("will_it_snow").and_then(|s| s.as_u64()).map(|s| {
            if s == 1 {
                true
            } else {
                false
            }
        });
        let vis = items
            .get("vis_km")
            .and_then(|s| s.as_f64())
            .map(|s| s as f32);
        let uv = items.get("uv").and_then(|s| s.as_f64()).map(|s| s as f32);
        let date = items
            .get("time_epoch")
            .and_then(|s| s.as_i64())
            .and_then(|t| Utc.timestamp_opt(t, 0).single())
            .map(|t| Local.from_utc_datetime(&t.naive_utc()));
        let date = match date {
            Some(date) => date,
            None => items
                .get("last_updated_epoch")
                .and_then(|s| s.as_i64())
                .and_then(|t| Utc.timestamp_opt(t, 0).single())
                .map(|t| Local.from_utc_datetime(&t.naive_utc()))?,
        };

        Some(WeatherAPIItem {
            date,
            address,
            geo,
            condition,
            temp,
            feelslike,
            windchill,
            heatindex,
            dewpoint,
            wind,
            dir,
            degree,
            gust,
            pressure,
            precip,
            humidity,
            cloud,
            will_it_rain,
            chance_of_rain,
            will_it_snow,
            chance_of_snow,
            vis,
            uv,
        })
    }

    /// Display result
    #[rustfmt::skip]
    fn show(&self, item: &WeatherAPIItem, duration: i64, date: &str) {
        println!("Weather for '{}'. WeatherAPI server. Request time {} ms.", date, duration);
        println!("Request address: {}.", item.address);
        println!("Found address: {} ({},{}).", item.geo.address, item.geo.lat, item.geo.lon);
        println!("Forecast date on the server: {}", item.date.format("%Y-%m-%d %H:%M:%S (%:z)"));
        println!("{}", "-".repeat(40));
        println!("Weather condition text       : {}", item.condition.as_ref().map_or("None".to_owned(), |s| s.to_owned()));
        println!("Temperature                  : {}", item.temp.map_or("None".to_owned(), |s| format!("{:#.1} °C", s)));
        println!("Feels like temperature       : {}", item.feelslike.map_or("None".to_owned(), |s| format!("{:#.1} °C", s)));
        println!("Windchill temperature        : {}", item.windchill.map_or("None".to_owned(), |s| format!("{:#.1} °C", s)));
        println!("Heat index                   : {}", item.heatindex.map_or("None".to_owned(), |s| format!("{:#.1} °C", s)));
        println!("Dew point                    : {}", item.dewpoint.map_or("None".to_owned(), |s| format!("{:#.1} °C", s)));
        println!("Wind speed                   : {}", item.wind.map_or("None".to_owned(), |s| format!("{:#.1} km/hour", s)));
        println!("Wind direction in degrees    : {:?} ({})", item.dir, item.degree.map_or("None".to_owned(), |s| s.to_string() + "°"));
        println!("Wind gust                    : {}", item.gust.map_or("None".to_owned(), |s| format!("{:#.1} km/hour", s)));
        println!("Atmospheric pressure         : {}", item.pressure.map_or("None".to_owned(), |s| format!("{:#.1} mbar", s)));
        println!("Precipitation amount         : {}", item.precip.map_or("None".to_owned(), |s| format!("{:#.1} mm", s)));
        println!("Humidity                     : {}", item.humidity.map_or("None".to_owned(), |s| s.to_string() + " %"));
        println!("Cloud cover                  : {}", item.cloud.map_or("None".to_owned(), |s| s.to_string() + " %"));
        println!("Will it will rain or not     : {}", item.will_it_rain.map_or("None".to_owned(), |s| format!("{}", s)));
        println!("Chance of rain               : {}", item.chance_of_rain.map_or("None".to_owned(), |s| s.to_string() + " %"));
        println!("Will it will snow or not     : {}", item.will_it_snow.map_or("None".to_owned(), |s| format!("{}", s)));
        println!("Chance of snow               : {}", item.chance_of_snow.map_or("None".to_owned(), |s| s.to_string() + " %"));
        println!("Visibility                   : {}", item.vis.map_or("None".to_owned(), |s| format!("{:#.1} km", s)));
        println!("UV Index                     : {}", item.uv.map_or("None".to_owned(), |s| format!("{:#.1}", s)));
    }
}

impl Provider for WeatherAPI {
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

impl Default for WeatherAPI {
    fn default() -> WeatherAPI {
        WeatherAPI::new()
    }
}
