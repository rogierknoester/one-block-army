use std::{collections::HashSet, fs::File, io::Read, path::PathBuf, time::Duration};

use blocking_http_server::{Response, Server};
use cached::proc_macro::once;
use gumdrop::Options;
use reqwest::{Method, StatusCode};
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

    let address = opts.listen;
    let port = opts.port;

    eprintln!("starting HTTP server on {address}:{port}");
    let mut server =
        Server::bind(format!("{address}:{port}")).expect("unable to create http server");

    for req in server.incoming() {
        let req = match req {
            Ok(req) => req,
            Err(e) => {
                eprintln!("Error with request: {e}");
                continue;
            }
        };

        match (req.method(), req.uri().path()) {
            (&Method::GET, "/") => {
                eprintln!("received request from {}", req.peer_addr);
                let _ = req.respond(
                    Response::builder()
                        .status(StatusCode::OK)
                        .header("Content-Type".to_owned(), "text/plain".to_owned())
                        .body(build_adlist(&config).render())
                        .unwrap(),
                );
            }
            (&Method::HEAD, "/") => {
                let content_length = build_adlist(&config).render().len();
                let _ = req.respond(
                    Response::builder()
                        .status(StatusCode::OK)
                        .header("Content-Type".to_owned(), "text/plain".to_owned())
                        .header("Content-Length".to_owned(), content_length)
                        .body("")
                        .unwrap(),
                );
            }
            _ => {
                let _ = req.respond(
                    Response::builder()
                        .status(StatusCode::NOT_FOUND)
                        .body("404 Not Found")
                        .unwrap(),
                );
            }
        }
    }

    let adlist = build_adlist(&config);

    println!("{}", adlist.render());
}

#[once(time = 900)]
fn build_adlist(config: &Config) -> HashSet<HostEntry> {
    let whitelister = Whitelister::new(&config.whitelisted_hosts);

    config
        .adlists
        .iter()
        .flat_map(|url| {
            fetch_adlist(url)
                .map_err(|err| format!("{err}"))
                .and_then(|content| Parser::parse(content.as_str()))
                .unwrap_or_default()
        })
        .filter(|host| !whitelister.evaluate(host))
        .collect::<HashSet<HostEntry>>()
}

#[derive(Debug, Options)]
struct Opts {
    #[options(help = "Path to the config file")]
    config: PathBuf,

    #[options(help = "The address on which oba will listen", default = "127.0.0.1")]
    listen: String,

    #[options(help = "The port that oba binds to", default = "8000")]
    port: u16,
}

#[derive(Deserialize, Debug)]
struct Config {
    adlists: Vec<String>,
    whitelisted_hosts: HashSet<String>,
}

fn fetch_adlist(url: &str) -> Result<String, reqwest::Error> {
    reqwest::blocking::get(url)?.text()
}

struct Whitelister<'a> {
    whitelisted_hosts: &'a HashSet<String>,
}

impl<'a> Whitelister<'a> {
    fn new(whitelisted_hosts: &'a HashSet<String>) -> Self {
        Self { whitelisted_hosts }
    }

    fn evaluate(&self, host: &HostEntry) -> bool {
        self.whitelisted_hosts.contains(&host.hostname)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_whitelister() {
        let whitelisted_hosts = HashSet::from_iter(vec![String::from("kagi.com")]);
        let whitelister = Whitelister::new(&whitelisted_hosts);

        let blocked_host_entry = HostEntry::new(
            std::net::IpAddr::V4("127.0.0.1".parse().unwrap()),
            "kagi.com".to_owned(),
        );

        let non_blocked_host_entry = HostEntry::new(
            std::net::IpAddr::V4("127.0.0.1".parse().unwrap()),
            "eff.org".to_owned(),
        );

        assert!(whitelister.evaluate(&blocked_host_entry));
        assert!(!whitelister.evaluate(&non_blocked_host_entry));
    }
}
