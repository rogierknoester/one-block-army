use std::{fs::File, io::Read, path::PathBuf};

use gumdrop::Options;
use serde::Deserialize;

use crate::parser::{HostEntry, HostsRenderer, Parser};

mod parser;

fn main() {
    let opts = Opts::parse_args_default_or_exit();

    let mut file = File::open(opts.config).expect("Unable to open config file. Does it exist?");

    let mut config_contents = String::new();
    file.read_to_string(&mut config_contents)
        .expect("Cannot read config file");

    let config: Config = toml::from_str(&config_contents).expect("Unable parse config");

    let mut adlists = config
        .adlists
        .iter()
        .map(|url| {
            fetch_adlist(url)
                .map_err(|err| format!("{err}"))
                .and_then(|content| Parser::parse(content.as_str()))
                .unwrap_or(Vec::new())
        })
        .flatten()
        .collect::<Vec<HostEntry>>();

    adlists.retain(|entry| !config.whitelisted_domains.contains(&entry.hostname));

    println!("{}", adlists.render());
}

#[derive(Debug, Options)]
struct Opts {
    #[options(help = "Path to the config file")]
    config: PathBuf,
}

#[derive(Deserialize, Debug)]
struct Config {
    adlists: Vec<String>,
    whitelisted_domains: Vec<String>,
}

fn fetch_adlist(url: &str) -> Result<String, reqwest::Error> {
    reqwest::blocking::get(url)?.text()
}
