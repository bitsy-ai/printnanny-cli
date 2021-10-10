use anyhow::{ Result };
use log::{ info };
use env_logger::Builder;
use log::LevelFilter;
use clap::{ Arg, App, SubCommand };
use printnanny::config:: { LocalConfig };


// Basic flow goess
// if <field> not exist -> prompt for config
// if <field> exist, print config -> prompt to use Y/n -> prompt for config OR proceed
async fn handle_setup(config: LocalConfig) -> Result<()>{
    if config.api_token.is_none() {
        config.auth();
    } else {
        config.print_user();
    }
    Ok(())
}

// resets config back to default values
async fn handle_reset(config: LocalConfig) -> Result<LocalConfig>{
    let defaults = LocalConfig::new();
    defaults.save();
    Ok(defaults)
}


#[tokio::main]
async fn main() -> Result<()> {
    let mut builder = Builder::new();
    let app_name = "printnanny";
    let app = App::new(app_name)
        .version("0.1.0")
        .author("Leigh Johnson <leigh@bitsy.ai>")
        .about("Official Print Nanny CLI https://print-nanny.com")
        .arg(Arg::with_name("api-url")
            .long("api-url")
            .help("Specify api_url")
            .value_name("API_URL")
            .takes_value(true))
        .arg(Arg::with_name("config")
            .short("c")
            .long("config")
            .help("Load custom config file")
            .value_name("FILE")
            .takes_value(true))
        .arg(Arg::with_name("v")
        .short("v")
        .multiple(true)
        .help("Sets the level of verbosity"))
        .subcommand(SubCommand::with_name("setup")
            .about("Connect your Print Nanny account"))
        .subcommand(SubCommand::with_name("reset")
        .about("Reset your Print Nanny setup"))
        .subcommand(SubCommand::with_name("update")
        .about("Update Print Nanny system"));    
    let app_m = app.get_matches();

    let default_config_name = "default";
    let config_name = app_m.value_of("config").unwrap_or(default_config_name);
    info!("Using config file: {}", config_name);

    // Vary the output based on how many times the user used the "verbose" flag
    // (i.e. 'printnanny -v -v -v' or 'printnanny -vvv' vs 'printnanny -v'
    let verbosity = app_m.occurrences_of("v");
    match verbosity {
        0 => builder.filter_level(LevelFilter::Warn).init(),
        1 => builder.filter_level(LevelFilter::Info).init(),
        2 => builder.filter_level(LevelFilter::Debug).init(),
        _ => builder.filter_level(LevelFilter::Trace).init(),
    };
    
    let config = LocalConfig::load(app_name)?;

    match app_m.subcommand() {
        ("setup", Some(_sub_m)) => {
            handle_setup(config).await?;
        },
        ("reset", Some(_sub_m)) => {
            handle_reset(config).await?;
        },
        ("update", Some(_sub_m)) => {
            unimplemented!();
        },
        _ => {}
    }
    Ok(())
}
