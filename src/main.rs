//! # Weather
//!
//! Elastio Rust Test Task from [link](https://gist.github.com/anelson/0029f620105a19702b5eed5935880a28).
//!
//! This application displays weather information for CLI on Windows, Linux, and macOS.
//!
pub mod geo;
pub mod help;
pub mod init;
pub mod provider;
pub mod wind;
pub mod work;

use init::Init;

use crate::{help::Help, work::Work};

/// Program entry point
fn main() {
    let init = Init::new();
    match init.command {
        init::Command::Help { error } => Help::show(error, &init.args),
        com => {
            let mut work = Work::new();
            match com {
                init::Command::List => work.list(),
                init::Command::Configure { provider } => work.configure(provider),
                init::Command::Get {
                    provider,
                    address,
                    date,
                } => work.get(provider, address, date),
                _ => {}
            }
        }
    }
}
