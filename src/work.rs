//! Module responsible for program logic.
//!
use std::{
    fs::File,
    io::{stdin, stdout, BufRead, BufReader, ErrorKind, Write},
};

use crate::init::Date;

/// Interaction with weather forecast provider.
///
/// * `fn serialize(&self) -> String` - Serialize provider credentials.
/// * `fn deserialize(&mut self, data: &str) -> bool` - Deserialize provider credentials.
/// * `fn get_weather(&self, address: String, date: Date)` - Displays weather for the provided address.
/// * `fn name(&self) -> &'static str` - Get provider name..
/// * `fn configure(&mut self)` - Configures credentials for the selected provider.
pub trait Provider {
    /// Serialize provider credentials.
    fn serialize(&self) -> String;
    /// Deserialize provider credentials.
    fn deserialize(&mut self, data: &str) -> bool;
    /// Displays weather for the provided address.
    fn get_weather(&self, address: String, date: Date);
    /// Get provider name.
    fn name(&self) -> &'static str;
    /// Configures credentials for the selected provider
    fn configure(&mut self);
}

/// Work struct with list of providers and default provider.
///
/// * `providers: Vec<Box<dyn Provider>>` - List of weather providers.
/// * `default: usize` - Default provider.
pub struct Work {
    /// List of weather providers.
    providers: Vec<Box<dyn Provider>>,
    /// Default provider.
    default: usize,
}

impl Work {
    /// Create empty work structure.
    pub fn new() -> Work {
        let providers: Vec<Box<dyn Provider>> = vec![
            Box::new(crate::provider::openweather::OpenWeather::new()),
            Box::new(crate::provider::weatherapi::WeatherAPI::new()),
            Box::new(crate::provider::accuweather::AccuWeather::new()),
            Box::new(crate::provider::aerisweather::AerisWeather::new()),
        ];

        let mut work = Work {
            providers,
            default: 0,
        };
        work.load();
        work.save();
        work
    }

    /// Displays a list of available providers and allows to set the default.
    pub fn list(&mut self) {
        // Display header
        println!("Weather can be obtained through the following providers:");
        for (index, vec) in self.providers.iter().enumerate() {
            if self.default == index {
                println!("  *{} - {}", index + 1, vec.name());
            } else {
                println!("   {} - {}", index + 1, vec.name());
            }
        }
        print!(
            "* - default provider.\nPlease set the new default provider [Integer from 1 to {}]: ",
            self.providers.len()
        );
        if let Err(e) = stdout().flush() {
            eprint!("System error: {}\n\nFailed to set default provider.", e);
            return;
        };

        // Input default index
        let mut input = String::new();
        if let Err(e) = stdin().read_line(&mut input) {
            print!(
                "The key must be only integer from 1 to {}. Error: {}.",
                self.providers.len(),
                e
            );
            return;
        }
        let input = input.trim();
        // Don't change provider
        if input.is_empty() {
            let provider = &self.providers[self.default];
            println!(
                "The '{}' provider was successfully left as the default.",
                provider.name()
            );
            return;
        }
        // Check entered number
        let num = match input.parse::<usize>() {
            Ok(num) => num,
            Err(e) => {
                print!(
                    "The key must be only integer from 1 to {}. Error: {}.",
                    self.providers.len(),
                    e
                );
                return;
            }
        };
        if num == 0 || num > self.providers.len() {
            print!(
                "The key must be only integer from 1 to {}.",
                self.providers.len()
            );
            return;
        }
        self.default = num - 1;

        // Display footer
        let provider = &self.providers[self.default];
        println!(
            "The '{}' provider was successfully installed by default.",
            provider.name()
        );
        self.save();
    }

    /// Configures credentials for the selected provider
    pub fn configure(&mut self, provider: String) {
        let mut res = None;
        for vec in self.providers.iter_mut() {
            if vec.name() == provider {
                res = Some(vec);
                break;
            }
        }
        match res {
            Some(provider) => provider.configure(),
            None => println!("Weather provider {} not found.", provider),
        }
        self.save();
    }

    /// Displays weather for the provided address.
    ///
    /// * `provider: Option<String>` - Using the default provider.
    /// * `address: String` - The provided address.
    /// * `date: Date` - Displays weather for the specified date.
    pub fn get(&self, provider: Option<String>, address: String, date: Date) {
        match provider {
            Some(provider) => {
                let mut res = None;
                for vec in &self.providers {
                    if vec.name() == provider {
                        res = Some(vec);
                        break;
                    }
                }
                match res {
                    Some(provider) => provider.get_weather(address, date),
                    None => println!("Weather provider {} not found.", provider),
                }
            }
            None => {
                let provider = &self.providers[self.default];
                provider.get_weather(address, date);
            }
        }
    }

    /// Load credentials from text file
    fn load(&mut self) {
        let file = match File::open("key.txt") {
            Ok(file) => file,
            Err(e) => {
                match e.kind() {
                    ErrorKind::NotFound => {}
                    _ => println!("Could not open the key file. Error: {}.", e),
                }
                return;
            }
        };
        let buf_reader = BufReader::new(file);
        let vec = match buf_reader.lines().collect::<std::io::Result<Vec<String>>>() {
            Ok(vec) => vec,
            Err(e) => {
                println!("Could not read the key file. Error: {}.", e);
                return;
            }
        };
        if vec.is_empty() {
            return;
        }
        let default = &vec[0];
        for keys in &vec[1..] {
            for (index, vec) in self.providers.iter_mut().enumerate() {
                if vec.deserialize(keys) && default == vec.name() {
                    self.default = index;
                    break;
                }
            }
        }
    }

    /// Save credentials to text file
    fn save(&self) {
        let mut data = Vec::with_capacity(self.providers.len() + 1);
        data.push(self.providers[self.default].name().to_owned());
        for provider in &self.providers {
            data.push(provider.serialize());
        }
        let mut file = match File::create("key.txt") {
            Ok(file) => file,
            Err(e) => {
                println!(
                    "An error occurred while writing the keys to the file. Error: {}.",
                    e
                );
                return;
            }
        };
        if let Err(e) = file.write_all(data.join("\n").as_bytes()) {
            println!("An error occurred while writing these keys. Error: {}.", e);
        }
    }
}

impl Default for Work {
    fn default() -> Work {
        Work::new()
    }
}
