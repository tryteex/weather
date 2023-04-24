//! The module responsible for displaying error and help information.
//!

/// Unit struct for help system
pub struct Help;

impl Help {
    /// Shows the help message.
    ///
    /// * `is_error: bool` - True: an error occurred while recognizing the launch command.
    /// * `args: &str` - Parameters for starting the application..
    pub fn show(is_error: bool, args: &str) {
        if is_error {
            println!(
                "weather: {}: unrecognized command
For help information, type: \"weather help\"",
                args
            );
        } else {
            println!(
"weather: {} v:{}
Usage: weather help | configure [provider] | get [provider] <address> [date=format]

This application displays weather information for CLI on Windows, Linux, and macOS:

  help                      - Shows this help message
  configure                 - Displays a list of available providers and allows to set the default
  configure <provider>      - Configures credentials for the selected provider
  get <address>             - Displays weather for the provided address using the default provider
  get [provider] <address>  - Displays weather for the provided address using the specified provider
      [date=format]         - Displays weather for the specified date

  format = now | yyyy-mm-dd | yyyy-mm-ddThh:mm:ss
    now                     - Displays weather for the current date and time
    yyyy-mm-dd              - Displays weather for the specified date and current time
    yyyy-mm-ddThh:mm:ss     - Displays weather for the specified date and time

Examples:
  \"weather get Kyiv, Ukraine\"
    Displays weather for Kyiv, Ukraine for the current date and time

  \"weather get provider=AccuWeather Kyiv, Ukraine date=2023-05-11\"
    Displays weather for Kyiv, Ukraine on May 11, 2023 using the AccuWeather provider

  \"weather get provider=AccuWeather Kyiv, Ukraine date=2023-05-11T11:00:20\"
    Displays weather for Kyiv, Ukraine on May 11, 2023 on time 11:00:20 using the AccuWeather provider

Note:
    We would like to note separately that not all weather providers provide a forecast for the specified date,
    so the program searches for the closest date to the entered one.

Please report any bugs to {}"
            , env!("CARGO_PKG_DESCRIPTION"), env!("CARGO_PKG_VERSION"), env!("CARGO_PKG_AUTHORS"));
        };
    }
}
