//! The module responsible for initialization parameters, as well as saving and loading keys to a file.
//!

use std::env;

use chrono::{DateTime, Local, NaiveDateTime, TimeZone};

const PROVIDER: &str = "provider=";
const DATE: &str = "date=";

/// Describes date value.
///
/// * `Now` - Current data and time (now).
/// * `Error` - Error set data.
/// * `Set(DateTime<Local>)` - The given date.
#[derive(Debug, PartialEq)]
pub enum Date {
    /// Current data and time (now).
    Now,
    /// Error set data
    Error,
    /// The given date.
    Set(DateTime<Local>),
}

/// The command to launch the application.
///
/// * `List` - Displays a list of available providers and allows to set the default.
/// * `Configure { provider }` - Configures credentials for the selected provider.
///   * `provider: String` - The selected provider.
/// * `Get { provider, address, date }` - Displays weather for the provided address.
///   * `provider: Option<String>` - Using the default provider.
///   * `address: String` - The provided address.
///   * `date: Date` - Displays weather for the specified date.
/// * `Help { error}` - Shows the help message.
///   * `error: bool` - True: an error occurred while recognizing the launch command.
#[derive(Debug, PartialEq)]
pub enum Command {
    /// Displays a list of available providers and allows to set the default.
    List,
    /// Configures credentials for the selected provider.
    /// * `provider` - The selected provider.
    Configure { provider: String },
    /// Displays weather for the provided address.
    /// * `provider` - Using the default provider.
    /// * `address` - The provided address.
    /// * `date` - Displays weather for the specified date.
    Get {
        provider: Option<String>,
        address: String,
        date: Date,
    },
    /// Shows the help message.
    /// * `error` - True: an error occurred while recognizing the launch command.
    Help { error: bool },
}

/// Initialization structure.
///
/// * `pub args: String` - Arguments for starting the application.
/// * `pub command: Command` - The command to launch the application.
#[derive(Debug, PartialEq)]
pub struct Init {
    /// Parameters for starting the application.
    pub args: String,
    /// The command to launch the application.
    pub command: Command,
}

impl Init {
    /// Create empty initialization structure.
    pub fn new() -> Init {
        let list: Vec<String> = env::args().skip(1).collect();
        let args = list.join(" ");
        let command = Init::parse_args(&list);

        Init { args, command }
    }

    /// Parsing of the launch parameters
    ///
    /// * `list: &[String]` - Non empty array with launch parameters
    ///
    /// Return
    ///
    /// `Command` - The command to launch the application.
    fn parse_args(list: &[String]) -> Command {
        let first = match list.get(0) {
            None => return Command::Help { error: false },
            Some(first) => first.as_ref(),
        };
        match first {
            "help" => Command::Help { error: false },
            "configure" => match list.get(1) {
                Some(provider) => Command::Configure {
                    provider: provider.to_string(),
                },
                None => Command::List,
            },
            "get" => match Init::parse_get_command(&list[1..]) {
                Some((provider, address, date)) => Command::Get {
                    provider,
                    address,
                    date,
                },
                None => Command::Help { error: true },
            },
            _ => Command::Help { error: true },
        }
    }

    /// Detail parsing 'get' command.
    ///
    /// * `parts: &[String]` - Non empty array with launch parameters from `get` command.
    ///
    /// Return
    ///
    /// `Option<(provider, address, date)>` - Turple with provider, address and date.
    ///   * `Option::None` - Error recognizing the parameters.
    ///   * `Option::Some` - Parameters recognized successfully.
    ///     * `provider: Option<String>` - Weather provider.
    ///     * `address: String` - The address to which you need to receive a weather forecast.
    ///     * `date: Date` - Forecast date.
    fn parse_get_command(parts: &[String]) -> Option<(Option<String>, String, Date)> {
        // First parameter
        let first = parts.first();
        // Last parameter
        let mut last = if parts.len() > 1 { parts.last() } else { None };
        // Middle part
        let middle = if parts.len() > 2 {
            Some(parts[1..parts.len() - 1].join(" "))
        } else {
            last.take().cloned()
        };
        match (first, middle, last) {
            // Nothing
            (None, _, _) => None,
            // Only one part
            (Some(first), None, None) | (Some(first), None, Some(_)) => {
                if first.starts_with(PROVIDER) || first.starts_with(DATE) {
                    None
                } else {
                    Some((None, first.to_owned(), Date::Now))
                }
            }
            // Two parts
            (Some(first), Some(middle), None) => {
                if first.starts_with(PROVIDER) {
                    if middle.starts_with(DATE) {
                        None
                    } else {
                        Some((Init::set_provider(first), middle, Date::Now))
                    }
                } else if middle.starts_with(DATE) {
                    let dt = match Init::set_date(&middle) {
                        Date::Error => return None,
                        dt => dt,
                    };
                    Some((None, first.to_owned(), dt))
                } else {
                    Some((None, first.to_owned() + " " + &middle, Date::Now))
                }
            }
            // All parts
            (Some(first), Some(middle), Some(last)) => {
                if first.starts_with(PROVIDER) {
                    if last.starts_with(DATE) {
                        let dt = match Init::set_date(last) {
                            Date::Error => return None,
                            dt => dt,
                        };
                        Some((Init::set_provider(first), middle, dt))
                    } else {
                        Some((Init::set_provider(first), middle + " " + last, Date::Now))
                    }
                } else if last.starts_with(DATE) {
                    let dt = match Init::set_date(last) {
                        Date::Error => return None,
                        dt => dt,
                    };
                    Some((None, first.to_owned() + " " + &middle, dt))
                } else {
                    Some((
                        None,
                        first.to_owned() + " " + &middle + " " + last,
                        Date::Now,
                    ))
                }
            }
        }
    }

    /// Checking for an empty provider
    #[inline]
    fn set_provider(provider: &str) -> Option<String> {
        if provider == PROVIDER {
            None
        } else {
            Some(provider[PROVIDER.len()..].to_owned())
        }
    }

    /// Checking for an empty date
    #[inline]
    fn set_date(date: &str) -> Date {
        if date == DATE || date.to_lowercase() == format!("{}now", DATE) {
            Date::Now
        } else {
            let mut dt = date[DATE.len()..].to_owned();
            // Add curent time to date without time
            if dt.len() == 10 {
                let now: DateTime<Local> = Local::now();
                dt.push_str(&now.format("T%H:%M:%S").to_string());
            }
            match NaiveDateTime::parse_from_str(&dt, "%Y-%m-%dT%H:%M:%S") {
                Ok(dt) => match Local.from_local_datetime(&dt).single() {
                    Some(dt) => Date::Set(dt),
                    None => Date::Error,
                },
                Err(e) => {
                    println!("Unable to determine date: {}. Error: {}.", dt, e);
                    Date::Error
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use chrono::{Local, NaiveDateTime, TimeZone};

    use super::Init;
    use crate::init::{Command, Date};

    fn setup_args(args: &str) -> Command {
        let args: Vec<String> = args
            .split(' ')
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .collect();
        Init::parse_args(&args)
    }

    #[test]
    fn test_parse_args_help() {
        assert_eq!(setup_args(""), Command::Help { error: false });
        assert_eq!(setup_args("help"), Command::Help { error: false });
        assert_eq!(
            setup_args("help and  something else"),
            Command::Help { error: false }
        );
        assert_eq!(setup_args("unknown command"), Command::Help { error: true });
    }

    #[test]
    fn test_parse_args_configure() {
        assert_eq!(setup_args("configure"), Command::List);
        assert_eq!(
            setup_args("configure AccuWeather"),
            Command::Configure {
                provider: "AccuWeather".to_owned()
            }
        );
    }

    #[test]
    fn test_parse_args_get() {
        assert_eq!(setup_args("get"), Command::Help { error: true });
        assert_eq!(
            setup_args("get address"),
            Command::Get {
                provider: None,
                address: "address".to_owned(),
                date: Date::Now
            }
        );
        assert_eq!(
            setup_args("get some    address else"),
            Command::Get {
                provider: None,
                address: "some address else".to_owned(),
                date: Date::Now
            }
        );
        assert_eq!(
            setup_args("get provider= some address"),
            Command::Get {
                provider: None,
                address: "some address".to_owned(),
                date: Date::Now
            }
        );
        assert_eq!(
            setup_args("get provider= some address date="),
            Command::Get {
                provider: None,
                address: "some address".to_owned(),
                date: Date::Now
            }
        );
        assert_eq!(
            setup_args("get provider=AccuWeather some address"),
            Command::Get {
                provider: Some("AccuWeather".to_owned()),
                address: "some address".to_owned(),
                date: Date::Now
            }
        );
        assert_eq!(
            setup_args("get provider=AccuWeather some   address date="),
            Command::Get {
                provider: Some("AccuWeather".to_owned()),
                address: "some address".to_owned(),
                date: Date::Now
            }
        );
        assert_eq!(
            setup_args("get provider=AccuWeather some address date=now"),
            Command::Get {
                provider: Some("AccuWeather".to_owned()),
                address: "some address".to_owned(),
                date: Date::Now
            }
        );
        assert_eq!(
            setup_args("get provider=AccuWeather some   address date=2023-05-01T10:12:50"),
            Command::Get {
                provider: Some("AccuWeather".to_owned()),
                address: "some address".to_owned(),
                date: Date::Set(
                    Local
                        .from_local_datetime(
                            &NaiveDateTime::parse_from_str(
                                "2023-05-01T10:12:50",
                                "%Y-%m-%dT%H:%M:%S"
                            )
                            .unwrap()
                        )
                        .single()
                        .unwrap()
                )
            }
        );
        assert_eq!(
            setup_args("get some address date="),
            Command::Get {
                provider: None,
                address: "some address".to_owned(),
                date: Date::Now
            }
        );
        assert_eq!(
            setup_args("get some address date=now"),
            Command::Get {
                provider: None,
                address: "some address".to_owned(),
                date: Date::Now
            }
        );
    }
}

impl Default for Init {
    fn default() -> Init {
        Init::new()
    }
}
