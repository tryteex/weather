//! Weather provider [AccuWeather](https://accuweather.com).
//!
//!

use std::{
    io::{stdin, stdout, Write},
    time::Duration,
};

use chrono::{DateTime, Local, TimeZone, Utc};
use reqwest::blocking::Client;
use serde_json::{Map, Value};

use crate::{geo::Geo, init::Date, wind::WindDeg, work::Provider};

/// Describes 'AccuWeather' credentials.
///
/// * `name: &'static str` - Provider name.
/// * `key: Option<String>` - Api key.
pub struct AccuWeather {
    /// Provider name.
    name: &'static str,
    /// Api key.
    key: Option<String>,
}

/// AccuWeather data format for current item
#[derive(Debug)]
struct AccuWeatherItemCurrent {
    /// Time of data calculation from provider. Local
    date: DateTime<Local>,
    /// Request Address
    address: String,
    /// Geo position
    geo: Geo,
    /// Phrase description of the current weather condition
    weathertext: Option<String>,
    /// Flag indicating the presence or absence of precipitation.
    hasprecipitation: Option<bool>,
    /// The type of precipitation
    precipitationtype: Option<String>,
    /// Temperature
    temperature: Option<f32>,
    /// RealFeel temperature
    realfeeltemperature: Option<f32>,
    /// Relative humidity
    relativehumidity: Option<u32>,
    /// Dew point temperature
    dewpoint: Option<f32>,
    /// Wind direction in Azimuth degrees
    degrees: Option<u16>,
    /// Wind direction (meteorological)
    dir: WindDeg,
    /// Wind Speed
    speed: Option<f32>,
    /// Wind gust speed
    gust: Option<f32>,
    /// Measure of the strength of the ultraviolet radiation from the sun
    uvindex: Option<f32>,
    /// Visibility
    visibility: Option<f32>,
    /// Number representing the percentage of the sky that is covered by clouds
    cloudcover: Option<u8>,
    /// Atmospheric pressure
    pressure: Option<f32>,
}

/// AccuWeather data format for forecast item
#[derive(Debug)]
struct AccuWeatherItemForecast {
    /// Time of data calculation from provider. Local
    date: DateTime<Local>,
    /// Request Address
    address: String,
    /// Geo position
    geo: Geo,
    /// Sun rise
    sunrise: Option<DateTime<Local>>,
    /// Sun set
    sunset: Option<DateTime<Local>>,
    /// Temperature minimum
    temp_min: Option<f32>,
    /// Temperature maximum
    temp_max: Option<f32>,
    /// RealFeel temperature minimum
    realfeel_min: Option<f32>,
    /// RealFeel temperature maximum
    realfeel_max: Option<f32>,
    /// Daytime Flag indicating the presence or absence of precipitation
    day_hasprecipitation: Option<bool>,
    /// Daytime the type of precipitation
    day_precipitationtype: Option<String>,
    /// Daytime description
    day_longphrase: Option<String>,
    /// Daytime Rain probability
    day_rainprobability: Option<u32>,
    /// Daytime Snow probability
    day_snowprobability: Option<u32>,
    /// Daytime Wind Speed
    day_speed: Option<f32>,
    /// Daytime Wind direction in Azimuth degrees
    day_deg: Option<u16>,
    /// Wind direction (meteorological)
    day_dir: WindDeg,
    /// Daytime Wind gust speed
    day_gust: Option<f32>,
    /// Daytime Rain volume
    day_rain: Option<f32>,
    /// Daytime Snow volume
    day_snow: Option<f32>,
    /// Daytime cloud cover
    day_cloudcover: Option<u32>,
    /// Night Flag indicating the presence or absence of precipitation
    night_hasprecipitation: Option<bool>,
    /// Night the type of precipitation
    night_precipitationtype: Option<String>,
    /// Night description
    night_longphrase: Option<String>,
    /// Night Rain probability
    night_rainprobability: Option<u32>,
    /// Night Snow probability
    night_snowprobability: Option<u32>,
    /// Night Wind Speed
    night_speed: Option<f32>,
    /// Night Wind direction in Azimuth degrees
    night_deg: Option<u16>,
    /// Wind direction (meteorological)
    night_dir: WindDeg,
    /// Night Wind gust speed
    night_gust: Option<f32>,
    /// Night Rain volume
    night_rain: Option<f32>,
    /// Night Snow volume
    night_snow: Option<f32>,
    /// Night cloud cover
    night_cloudcover: Option<u32>,
}

impl AccuWeather {
    /// Create new empty provider
    pub fn new() -> AccuWeather {
        AccuWeather {
            name: "AccuWeather",
            key: None,
        }
    }

    /// Load data from provider
    fn get_json(&self, url: &str) -> Option<String> {
        // Client for url query
        let client = match Client::builder().timeout(Duration::from_secs(3)).build() {
            Ok(c) => c,
            Err(e) => {
                println!("The following error occurred: {}", e);
                return None;
            }
        };

        let json_str = match client.get(url).send() {
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
        Some(json_str)
    }

    /// Get citi ID
    fn get_id(&self, address: &str) -> Option<(u32, Geo)> {
        let key = match &self.key {
            Some(key) => key,
            None => {
                println!("OpenWeather server API access key is not set. Please install it first.");
                return None;
            }
        };
        // Find geo coordinates by address
        let mut geo = Geo::get(&address)?;
        let geo = match geo.pop() {
            Some(geo) => geo,
            None => {
                println!("Sorry, we couldn't find your address: {}", address);
                return None;
            }
        };
        let url = format!(
            "https://dataservice.accuweather.com/locations/v1/cities/geoposition/search?apikey={}&q={},{}",
            key, geo.lat, geo.lon
        );
        // Get city ID
        let json_str = self.get_json(&url)?;

        // Parse json
        let json: Map<String, Value> = match serde_json::from_str(&json_str) {
            Ok(json) => json,
            Err(e) => {
                println!(
                    "Unable to recognize json response from server. Error text: {}",
                    e
                );
                return None;
            }
        };
        let id = match json.get("Key")?.as_str()?.parse::<u32>() {
            Ok(id) => id,
            Err(_) => return None,
        };
        Some((id, geo))
    }

    /// Getting weather forecast for now
    fn get_now(&self, address: String) -> Option<AccuWeatherItemCurrent> {
        let (id, geo) = self.get_id(&address)?;
        let key = self.key.as_ref()?;
        let url = format!(
            "https://dataservice.accuweather.com/currentconditions/v1/{}?details=true&apikey={}",
            id, key
        );
        let json_str = self.get_json(&url)?;

        // Parse json
        let json: Vec<Value> = match serde_json::from_str(&json_str) {
            Ok(json) => json,
            Err(e) => {
                println!(
                    "Unable to recognize json response from server. Error text: {}",
                    e
                );
                return None;
            }
        };
        let map = json.get(0)?.as_object()?;
        self.detect_now(map, geo, address)
    }

    /// Getting weather forecast for 'date'
    fn get_date(&self, address: String, date: &DateTime<Local>) -> Option<AccuWeatherItemForecast> {
        let (id, geo) = self.get_id(&address)?;
        let key = self.key.as_ref()?;
        let url = format!(
            "https://dataservice.accuweather.com/forecasts/v1/daily/5day/{}?details=true&metric=true&apikey={}",
            id, key
        );
        let json_str = self.get_json(&url)?;

        // Parse json
        let items: Map<String, Value> = match serde_json::from_str(&json_str) {
            Ok(json) => json,
            Err(e) => {
                println!(
                    "Unable to recognize json response from server. Error text: {}",
                    e
                );
                return None;
            }
        };
        // Get list of AccuWeatherItemForecast
        let its = items
            .get("DailyForecasts")
            .and_then(|i| i.as_array())
            .or_else(|| {
                println!("The AccuWeather server did not provide weather forecast data");
                return None;
            })?;
        // Load all AccuWeatherItemForecast to vector
        let mut list = Vec::with_capacity(24);
        for item in its {
            if let Value::Object(map) = item {
                let res = self.detect_date(map, geo.clone(), address.clone());
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
    fn detect_date(
        &self,
        items: &Map<String, Value>,
        geo: Geo,
        address: String,
    ) -> Option<AccuWeatherItemForecast> {
        let date = items
            .get("EpochDate")
            .and_then(|s| s.as_i64())
            .and_then(|t| Utc.timestamp_opt(t, 0).single())
            .map(|t| Local.from_utc_datetime(&t.naive_utc()))?;
        let sunrise = items
            .get("Sun")
            .and_then(|s| s.get("EpochRise"))
            .and_then(|s| s.as_i64())
            .and_then(|t| Utc.timestamp_opt(t, 0).single())
            .map(|t| Local.from_utc_datetime(&t.naive_utc()));
        let sunset = items
            .get("Sun")
            .and_then(|s| s.get("EpochSet"))
            .and_then(|s| s.as_i64())
            .and_then(|t| Utc.timestamp_opt(t, 0).single())
            .map(|t| Local.from_utc_datetime(&t.naive_utc()));
        let temp_min = items
            .get("Temperature")
            .and_then(|m| m.get("Minimum"))
            .and_then(|m| m.get("Value"))
            .and_then(|s| s.as_f64())
            .map(|s| s as f32);
        let temp_max = items
            .get("Temperature")
            .and_then(|m| m.get("Maximum"))
            .and_then(|m| m.get("Value"))
            .and_then(|s| s.as_f64())
            .map(|s| s as f32);
        let realfeel_min = items
            .get("RealFeelTemperature")
            .and_then(|m| m.get("Minimum"))
            .and_then(|m| m.get("Value"))
            .and_then(|s| s.as_f64())
            .map(|s| s as f32);
        let realfeel_max = items
            .get("RealFeelTemperature")
            .and_then(|m| m.get("Maximum"))
            .and_then(|m| m.get("Value"))
            .and_then(|s| s.as_f64())
            .map(|s| s as f32);

        let day = items.get("Day").and_then(|s| s.as_object())?;
        let day_hasprecipitation = day.get("HasPrecipitation").and_then(|s| s.as_bool());
        let day_precipitationtype = day
            .get("PrecipitationType")
            .and_then(|s| s.as_str())
            .map(|s| s.to_owned());
        let day_longphrase = day
            .get("LongPhrase")
            .and_then(|s| s.as_str())
            .map(|s| s.to_owned());
        let day_rainprobability = day
            .get("RainProbability")
            .and_then(|s| s.as_u64())
            .map(|s| s as u32);
        let day_snowprobability = day
            .get("SnowProbability")
            .and_then(|s| s.as_u64())
            .map(|s| s as u32);
        let day_speed = day
            .get("Wind")
            .and_then(|m| m.get("Speed"))
            .and_then(|m| m.get("Value"))
            .and_then(|s| s.as_f64())
            .map(|s| s as f32);
        let day_deg = day
            .get("Wind")
            .and_then(|m| m.get("Direction"))
            .and_then(|m| m.get("Degrees"))
            .and_then(|s| s.as_u64())
            .map(|s| s as u16);
        let day_dir = WindDeg::get(day_deg);
        let day_gust = day
            .get("WindGust")
            .and_then(|m| m.get("Speed"))
            .and_then(|m| m.get("Value"))
            .and_then(|s| s.as_f64())
            .map(|s| s as f32);
        let day_rain = day
            .get("Rain")
            .and_then(|m| m.get("Value"))
            .and_then(|s| s.as_f64())
            .map(|s| s as f32);
        let day_snow = day
            .get("Snow")
            .and_then(|m| m.get("Value"))
            .and_then(|s| s.as_f64())
            .map(|s| s as f32);
        let day_cloudcover = day
            .get("CloudCover")
            .and_then(|s| s.as_u64())
            .map(|s| s as u32);

        let night = items.get("Night").and_then(|s| s.as_object())?;
        let night_hasprecipitation = night.get("HasPrecipitation").and_then(|s| s.as_bool());
        let night_precipitationtype = night
            .get("PrecipitationType")
            .and_then(|s| s.as_str())
            .map(|s| s.to_owned());
        let night_longphrase = night
            .get("LongPhrase")
            .and_then(|s| s.as_str())
            .map(|s| s.to_owned());
        let night_rainprobability = night
            .get("RainProbability")
            .and_then(|s| s.as_u64())
            .map(|s| s as u32);
        let night_snowprobability = night
            .get("SnowProbability")
            .and_then(|s| s.as_u64())
            .map(|s| s as u32);
        let night_speed = night
            .get("Wind")
            .and_then(|m| m.get("Speed"))
            .and_then(|m| m.get("Value"))
            .and_then(|s| s.as_f64())
            .map(|s| s as f32);
        let night_deg = night
            .get("Wind")
            .and_then(|m| m.get("Direction"))
            .and_then(|m| m.get("Degrees"))
            .and_then(|s| s.as_u64())
            .map(|s| s as u16);
        let night_dir = WindDeg::get(night_deg);
        let night_gust = night
            .get("WindGust")
            .and_then(|m| m.get("Speed"))
            .and_then(|m| m.get("Value"))
            .and_then(|s| s.as_f64())
            .map(|s| s as f32);
        let night_rain = night
            .get("Rain")
            .and_then(|m| m.get("Value"))
            .and_then(|s| s.as_f64())
            .map(|s| s as f32);
        let night_snow = night
            .get("Snow")
            .and_then(|m| m.get("Value"))
            .and_then(|s| s.as_f64())
            .map(|s| s as f32);
        let night_cloudcover = night
            .get("CloudCover")
            .and_then(|s| s.as_u64())
            .map(|s| s as u32);

        Some(AccuWeatherItemForecast {
            date,
            address,
            geo,
            sunrise,
            sunset,
            temp_min,
            temp_max,
            realfeel_min,
            realfeel_max,
            day_hasprecipitation,
            day_precipitationtype,
            day_longphrase,
            day_rainprobability,
            day_snowprobability,
            day_speed,
            day_deg,
            day_dir,
            day_gust,
            day_rain,
            day_snow,
            day_cloudcover,
            night_hasprecipitation,
            night_precipitationtype,
            night_longphrase,
            night_rainprobability,
            night_snowprobability,
            night_speed,
            night_deg,
            night_dir,
            night_gust,
            night_rain,
            night_snow,
            night_cloudcover,
        })
    }

    /// Parse json answer from server
    fn detect_now(
        &self,
        items: &Map<String, Value>,
        geo: Geo,
        address: String,
    ) -> Option<AccuWeatherItemCurrent> {
        let date = items
            .get("EpochTime")
            .and_then(|s| s.as_i64())
            .and_then(|t| Utc.timestamp_opt(t, 0).single())
            .map(|t| Local.from_utc_datetime(&t.naive_utc()))?;
        let weathertext = items
            .get("WeatherText")
            .and_then(|s| s.as_str())
            .map(|s| s.to_owned());
        let hasprecipitation = items.get("hasprecipitation").and_then(|s| s.as_bool());
        let precipitationtype = items
            .get("precipitationtype")
            .and_then(|s| s.as_str())
            .map(|s| s.to_owned());
        let temperature = items
            .get("Temperature")
            .and_then(|m| m.get("Metric"))
            .and_then(|m| m.get("Value"))
            .and_then(|s| s.as_f64())
            .map(|s| s as f32);
        let realfeeltemperature = items
            .get("RealFeelTemperature")
            .and_then(|m| m.get("Metric"))
            .and_then(|m| m.get("Value"))
            .and_then(|s| s.as_f64())
            .map(|s| s as f32);
        let relativehumidity = items
            .get("RelativeHumidity")
            .and_then(|s| s.as_u64())
            .map(|s| s as u32);
        let dewpoint = items
            .get("DewPoint")
            .and_then(|m| m.get("Metric"))
            .and_then(|m| m.get("Value"))
            .and_then(|s| s.as_f64())
            .map(|s| s as f32);
        let degrees = items
            .get("Wind")
            .and_then(|m| m.get("Direction"))
            .and_then(|m| m.get("Degrees"))
            .and_then(|s| s.as_u64())
            .map(|s| s as u16);
        let dir = WindDeg::get(degrees);
        let speed = items
            .get("Wind")
            .and_then(|m| m.get("Speed"))
            .and_then(|m| m.get("Metric"))
            .and_then(|m| m.get("Value"))
            .and_then(|s| s.as_f64())
            .map(|s| s as f32);
        let gust = items
            .get("WindGust")
            .and_then(|m| m.get("Speed"))
            .and_then(|m| m.get("Metric"))
            .and_then(|m| m.get("Value"))
            .and_then(|s| s.as_f64())
            .map(|s| s as f32);
        let uvindex = items
            .get("UVIndex")
            .and_then(|s| s.as_f64())
            .map(|s| s as f32);
        let visibility = items
            .get("Visibility")
            .and_then(|m| m.get("Metric"))
            .and_then(|m| m.get("Value"))
            .and_then(|s| s.as_f64())
            .map(|s| s as f32);
        let cloudcover = items
            .get("CloudCover")
            .and_then(|s| s.as_u64())
            .map(|s| s as u8);
        let pressure = items
            .get("Pressure")
            .and_then(|m| m.get("Metric"))
            .and_then(|m| m.get("Value"))
            .and_then(|s| s.as_f64())
            .map(|s| s as f32);

        Some(AccuWeatherItemCurrent {
            date,
            address,
            geo,
            weathertext,
            hasprecipitation,
            precipitationtype,
            temperature,
            realfeeltemperature,
            relativehumidity,
            dewpoint,
            degrees,
            dir,
            speed,
            gust,
            uvindex,
            visibility,
            cloudcover,
            pressure,
        })
    }

    /// Display result
    #[rustfmt::skip]
    fn show_current(&self, item: &AccuWeatherItemCurrent, duration: i64, date: &str) {
        println!("Weather for '{}'. OpenWeather server. Request time {} ms.", date, duration);
        println!("Request address: {}.", item.address);
        println!("Found address: {} ({},{}).", item.geo.address, item.geo.lat, item.geo.lon);
        println!("Forecast date on the server: {}", item.date.format("%Y-%m-%d %H:%M:%S (%:z)"));
        println!("{}", "-".repeat(40));
        println!("Description of weather       : {}", item.weathertext.as_ref().map_or("None".to_owned(), |s| s.to_owned()));
        println!("Presence of precipitation    : {}", item.hasprecipitation.map_or("None".to_owned(), |s| format!("{}", s)));
        println!("The type of precipitation    : {}", item.precipitationtype.as_ref().map_or("None".to_owned(), |s| s.to_owned()));
        println!("Temperature                  : {}", item.temperature.map_or("None".to_owned(), |s| format!("{:#.1} °C", s)));
        println!("Real feel temperature        : {}", item.realfeeltemperature.map_or("None".to_owned(), |s| format!("{:#.1} °C", s)));
        println!("Humidity                     : {}", item.relativehumidity.map_or("None".to_owned(), |s| s.to_string() + " %"));
        println!("Atmospheric pressure         : {}", item.pressure.map_or("None".to_owned(), |s| s.to_string() + " hPa"));
        println!("Dew point temperature        : {}", item.dewpoint.map_or("None".to_owned(), |s| format!("{:#.1} °C", s)));
        println!("Wind direction and degrees   : {:?} ({})", item.dir, item.degrees.map_or("None".to_owned(), |s| s.to_string() + "°"));
        println!("Wind speed                   : {}", item.speed.map_or("None".to_owned(), |s| format!("{:#.1} km/h", s)));
        println!("Wind gust                    : {}", item.gust.map_or("None".to_owned(), |s| format!("{:#.1} km/h", s)));
        println!("UV index                     : {}", item.uvindex.map_or("None".to_owned(), |s| format!("{:#.1}", s)));
        println!("Visibility                   : {}", item.visibility.map_or("None".to_owned(), |s| s.to_string() + " km"));
        println!("Cloud cover                  : {}", item.cloudcover.map_or("None".to_owned(), |s| s.to_string() + " %"));
    }

    /// Display result
    #[rustfmt::skip]
    fn show_date(&self, item: &AccuWeatherItemForecast, duration: i64, date: &str) {
        println!("Weather for '{}'. OpenWeather server. Request time {} ms.", date, duration);
        println!("Request address: {}.", item.address);
        println!("Found address: {} ({},{}).", item.geo.address, item.geo.lat, item.geo.lon);
        println!("Forecast date on the server: {}", item.date.format("%Y-%m-%d %H:%M:%S (%:z)"));
        println!("{}", "-".repeat(40));
        println!("Sunrise time                 : {}", item.sunrise.map_or("None".to_owned(), |dt| dt.format("%Y-%m-%d %H:%M:%S (%:z)").to_string()));
        println!("Sunset time                  : {}", item.sunset.map_or("None".to_owned(), |dt| dt.format("%Y-%m-%d %H:%M:%S (%:z)").to_string()));
        println!("Temperature min              : {}", item.temp_min.map_or("None".to_owned(), |s| format!("{:#.1} °C", s)));
        println!("Temperature max              : {}", item.temp_max.map_or("None".to_owned(), |s| format!("{:#.1} °C", s)));
        println!("Real feel temperature        : {}", item.realfeel_min.map_or("None".to_owned(), |s| format!("{:#.1} °C", s)));
        println!("Real feel temperature        : {}", item.realfeel_max.map_or("None".to_owned(), |s| format!("{:#.1} °C", s)));
        println!("{}", "-".repeat(40));
        println!("Daytime forecast");
        println!("{}", "-".repeat(40));
        println!("Description of weather       : {}", item.day_longphrase.as_ref().map_or("None".to_owned(), |s| s.to_owned()));
        println!("Presence of precipitation    : {}", item.day_hasprecipitation.map_or("None".to_owned(), |s| format!("{}", s)));
        println!("The type of precipitation    : {}", item.day_precipitationtype.as_ref().map_or("None".to_owned(), |s| s.to_owned()));
        println!("Rain probability             : {}", item.day_rainprobability.map_or("None".to_owned(), |s| s.to_string() + " %"));
        println!("Rain volume                  : {}", item.day_rain.map_or("None".to_owned(), |s| format!("{:#.1} mm", s)));
        println!("Snow probability             : {}", item.day_snowprobability.map_or("None".to_owned(), |s| s.to_string() + " %"));
        println!("Snow volume                  : {}", item.day_snow.map_or("None".to_owned(), |s| format!("{:#.1} sm", s)));
        println!("Wind direction and degrees   : {:?} ({})", item.day_dir, item.day_deg.map_or("None".to_owned(), |s| s.to_string() + "°"));
        println!("Wind speed                   : {}", item.day_speed.map_or("None".to_owned(), |s| format!("{:#.1} km/h", s)));
        println!("Wind gust                    : {}", item.day_gust.map_or("None".to_owned(), |s| format!("{:#.1} km/h", s)));
        println!("Cloud cover                  : {}", item.day_cloudcover.map_or("None".to_owned(), |s| s.to_string() + " %"));
        println!("{}", "-".repeat(40));
        println!("Night forecast");
        println!("{}", "-".repeat(40));
        println!("Description of weather       : {}", item.night_longphrase.as_ref().map_or("None".to_owned(), |s| s.to_owned()));
        println!("Presence of precipitation    : {}", item.night_hasprecipitation.map_or("None".to_owned(), |s| format!("{}", s)));
        println!("The type of precipitation    : {}", item.night_precipitationtype.as_ref().map_or("None".to_owned(), |s| s.to_owned()));
        println!("Rain probability             : {}", item.night_rainprobability.map_or("None".to_owned(), |s| s.to_string() + " %"));
        println!("Rain volume                  : {}", item.night_rain.map_or("None".to_owned(), |s| format!("{:#.1} mm", s)));
        println!("Snow probability             : {}", item.night_snowprobability.map_or("None".to_owned(), |s| s.to_string() + " %"));
        println!("Snow volume                  : {}", item.night_snow.map_or("None".to_owned(), |s| format!("{:#.1} sm", s)));
        println!("Wind direction and degrees   : {:?} ({})", item.night_dir, item.night_deg.map_or("None".to_owned(), |s| s.to_string() + "°"));
        println!("Wind speed                   : {}", item.night_speed.map_or("None".to_owned(), |s| format!("{:#.1} km/h", s)));
        println!("Wind gust                    : {}", item.night_gust.map_or("None".to_owned(), |s| format!("{:#.1} km/h", s)));
        println!("Cloud cover                  : {}", item.night_cloudcover.map_or("None".to_owned(), |s| s.to_string() + " %"));

    }
}

impl Provider for AccuWeather {
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
        // https://dataservice.accuweather.com/forecasts/v1/daily/5day/324505?apikey=hHWnLgUfUGzr0KQFbSOcKQYkNPM8GlVL
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
                self.show_current(&now, duration.num_milliseconds(), "now");
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
                self.show_date(
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

impl Default for AccuWeather {
    fn default() -> AccuWeather {
        AccuWeather::new()
    }
}
