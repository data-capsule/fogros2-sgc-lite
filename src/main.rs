#[cfg(not(debug_assertions))]
use human_panic::setup_panic;

#[cfg(debug_assertions)]
extern crate better_panic;

use utils::app_config::AppConfig;
use utils::error::Result;
use std::env;
use std::fs;

/// The main entry point of the application.
fn main() -> Result<()> {
    // Human Panic. Only enabled when *not* debugging.
    #[cfg(not(debug_assertions))]
    {
        setup_panic!();
    }

    // Better Panic. Only enabled *when* debugging.
    #[cfg(debug_assertions)]
    {
        better_panic::Settings::debug()
            .most_recent_first(false)
            .lineno_suffix(true)
            .verbosity(better_panic::Verbosity::Full)
            .install();
    }

    let _guard = utils::logger::setup_logging()?;

    // Initialize Configuration
    // Initialize Configuration
    let include_path = match env::var_os("GDP_CONFIG") {
        Some(config_file) => {
            format!("{}{:?}", "./src/resources/", config_file)
        }, 
        None => {
            "./src/resources/default_config.toml".to_owned()
        }
    };
    println!("Using config file : {}", include_path);
    let config_contents = fs::read_to_string(include_path).expect("config file not found!");

    AppConfig::init(Some(&config_contents))?;

    // Match Commands
    cli::cli_match()?;

    Ok(())
}
