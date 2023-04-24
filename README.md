# Weather CLI

Elastio Rust Test Task
Version: 0.1.0

Weather CLI is an application that displays weather information for CLI on Windows, Linux, and macOS.

## Usage

`weather help | configure [provider] | get [provider] <address> [date=format]`


### Commands

- `help` - Shows the help message
- `configure` - Displays a list of available providers and allows setting the default
- `configure <provider>` - Configures credentials for the selected provider
- `get <address>` - Displays weather for the provided address using the default provider
- `get [provider] <address>` - Displays weather for the provided address using the specified provider
- `[date=format]` - Displays weather for the specified date

#### Date Format

- `now` - Displays weather for the current date and time
- `yyyy-mm-dd` - Displays weather for the specified date and current time
- `yyyy-mm-ddThh:mm:ss` - Displays weather for the specified date and time

## Examples

- `"weather get Kyiv, Ukraine"`: Displays weather for Kyiv, Ukraine for the current date and time
- `"weather get provider=AccuWeather Kyiv, Ukraine date=2023-05-11"`: Displays weather for Kyiv, Ukraine on May 11, 2023 using the AccuWeather provider
- `"weather get provider=AccuWeather Kyiv, Ukraine date=2023-05-11T11:00:20"`: Displays weather for Kyiv, Ukraine on May 11, 2023 at 11:00:20 using the AccuWeather provider

## Note

We would like to note separately that not all weather providers provide a forecast for the specified date, so the program searches for the closest date to the entered one.

## The result of the application

```
user@laptop:~$ weather cargo run get Kyiv
Weather for 'now'. OpenWeather server. Request time 560 ms.
Request address: Kyiv.
Found address: Київ, Україна (50.4500336,30.5241361).
Forecast date on the server: 2023-04-24 14:14:53 (+03:00)
----------------------------------------
Group of weather parameters  : Clouds
Temperature                  : 19.7 °C
Human perception temperature : 18.6 °C
Atmospheric pressure         : 1012 hPa
Humidity                     : 35 %
Wind speed                   : 4.3 meter/sec
Wind direction and degrees   : SouthSouthEast (160°)
Wind gust                    : 4.5 meter/sec
Rain volume (last 1 hour)    : None
Rain volume (last 3 hour)    : None
Snow volume (last 1 hour)    : None
Snow volume (last 3 hour)    : None
Visibility                   : 10000 meter
Sunrise time                 : 2023-04-24 05:46:52 (+03:00)
Sunset time                  : 2023-04-24 20:05:04 (+03:00)
```

## Safety Warnings

The saved keys are stored in the file `key.txt` in an unencrypted form in the same directory as this application.