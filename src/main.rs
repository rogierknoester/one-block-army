use std::process::ExitCode;
use std::str::FromStr;
use std::{collections::HashSet, fs::File, io::Read, path::PathBuf, time::Duration};

use blocking_http_server::{Response, Server};
use bumpalo::collections::Vec as BVec;
use bumpalo::Bump;
use cached::proc_macro::once;
use gumdrop::Options;
use reqwest::{Method, StatusCode};
use serde::Deserialize;

use crate::parser::{HostEntry, HostsRenderer, Parser};
use crate::whitelisting::Whitelister;

mod fetcher;
mod parser;
mod whitelisting;

fn main() -> ExitCode {
    let opts = Opts::parse_args_default_or_exit();

    let mut file = File::open(opts.config).expect("Unable to open config file. Does it exist?");

    let mut config_contents = String::new();
    file.read_to_string(&mut config_contents)
        .expect("Cannot read config file");

    let config: Config = toml::from_str(&config_contents).expect("Unable parse config");

    if opts.mode == Mode::Cli {
        let bump = Bump::new();
        print!("{}", build_adlist(&config, &bump).render());
        return ExitCode::SUCCESS;
    }

    let address = opts.listen;
    let port = opts.port;

    eprintln!("starting HTTP server on {address}:{port}");
    let mut server =
        Server::bind(format!("{address}:{port}")).expect("unable to create http server");

    for req in server.incoming() {
        let req = match req {
            Ok(req) => req,
            Err(e) => {
                eprintln!("error with request: {e}");
                continue;
            }
        };

        match (req.method(), req.uri().path()) {
            (&Method::GET, "/") => {
                eprintln!("received request from {}", req.peer_addr);
        let bump = Bump::new();
                let _ = req.respond(
                    Response::builder()
                        .status(StatusCode::OK)
                        .header("Content-Type".to_owned(), "text/plain".to_owned())
                        .body(build_adlist(&config, &bump).render())
                        .unwrap(),
                );
            }
            (&Method::HEAD, "/") => {
        let bump = Bump::new();
                let content_length = build_adlist(&config, &bump).render().len();
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

    ExitCode::SUCCESS
}

fn build_adlist<'a>(config: &'a Config, bump: &'a Bump) -> BVec<'a, HostEntry> {
    let whitelister = Whitelister::new(&config.whitelisted_hosts);

    let fetcher = fetcher::Fetcher::new_with_reqwest();
    let adlist_contents= bump.alloc(String::new());
    fetcher.fetch(&config.adlists, adlist_contents);

    let mut collector: BVec<HostEntry> = BVec::new_in(bump);
    Parser::parse(adlist_contents, &mut collector).expect("cannot parse");

    
    let mut result : BVec<HostEntry> = BVec::with_capacity_in(collector.len(), bump);
    whitelister.evaluate(collector.into_bump_slice(), &mut result);
    result
}

#[derive(Debug, Options)]
struct Opts {
    #[options(help = "Path to the config file")]
    config: PathBuf,

    #[options(help = "The address on which oba will listen", default = "127.0.0.1")]
    listen: String,

    #[options(help = "The port that oba binds to", default = "8000")]
    port: u16,

    #[options(help = "The mode to run in", default = "webserver")]
    mode: Mode,
}

#[derive(Debug, PartialEq, Eq)]
enum Mode {
    Webserver,
    Cli,
}

impl FromStr for Mode {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "webserver" => Ok(Mode::Webserver),
            "cli" => Ok(Mode::Cli),
            _ => Err("cannot parse mode"),
        }
    }
}

#[derive(Deserialize, Debug)]
struct Config {
    adlists: Vec<String>,
    whitelisted_hosts: HashSet<String>,
}
