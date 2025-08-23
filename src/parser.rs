use std::{
    collections::HashSet,
    fmt::Display,
    net::IpAddr,
    ops::Index,
    sync::mpsc,
    thread::{self, available_parallelism},
};

pub(crate) struct Parser {}

struct Line {
    content: String,
}

impl From<&str> for Line {
    fn from(value: &str) -> Self {
        Self {
            content: value.trim().to_string(),
        }
    }
}

impl Line {
    fn is_only_comment(&self) -> bool {
        self.content.starts_with('#')
    }

    fn is_empty(&self) -> bool {
        self.content.is_empty()
    }

    fn filter_out_comments(self) -> Self {
        if let Some(index) = self.content.find('#') {
            self.content.split_at(index).0.trim().into()
        } else {
            self
        }
    }

    fn parts(&self) -> Vec<&str> {
        self.content.split_whitespace().collect()
    }
}

impl Parser {
    pub(super) fn parse(input: &str) -> Result<Vec<HostEntry>, String> {
        let space = available_parallelism().unwrap();
        let lines = input.lines().collect::<Vec<&str>>();
        let chunk_size = lines.len().div_ceil(space.into());
        let chunks = lines.chunks(chunk_size);

        let rx = thread::scope(|scope| {
            let (tx, rx) = mpsc::channel::<Vec<HostEntry>>();

            for chunk in chunks {
                let tx = tx.clone();
                scope.spawn(move || {
                    let mut entries = vec![];
                    for raw_line_content in chunk {
                        let line = Line::from(*raw_line_content);

                        if line.is_empty() || line.is_only_comment() {
                            continue;
                        }

                        let line = line.filter_out_comments();

                        if line.is_empty() {
                            continue;
                        }

                        let parts = line.parts();

                        // we need at least two parts
                        if parts.len() < 2 {
                            continue;
                        }

                        let ip: IpAddr = match parts.index(0).parse() {
                            Ok(ip) => ip,
                            Err(err) => {
                                let part = parts.index(0);
                                println!("unable to parse ip addr \"{part}\": {err}");
                                continue;
                            }
                        };

                        let mut iter = parts.into_iter();
                        iter.next();

                        for hostname in iter {
                            if !hostname_validator::is_valid(hostname) {
                                println!("hostname \"{hostname}\" is invalid");
                            }

                            entries.push(HostEntry::new(ip, hostname.to_string()));
                        }
                    }

                    tx.send(entries).expect("cannot send parsed");
                });
            }
            rx
        });

        let mut entries = vec![];
        rx.iter().for_each(|mut subset| entries.append(&mut subset));

        Ok(entries)
    }
}

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub(crate) struct HostEntry {
    pub(super) ip: IpAddr,
    pub(super) hostname: String,
}

impl HostEntry {
    pub(crate) fn new(ip: IpAddr, hostname: String) -> Self {
        Self { ip, hostname }
    }
}

impl Display for HostEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.ip, self.hostname)
    }
}

pub(crate) trait HostsRenderer {
    fn render(self) -> String;
}

impl HostsRenderer for Vec<HostEntry> {
    fn render(self) -> String {
        let mut list = String::new();

        for entry in self.into_iter() {
            use std::fmt::Write;
            writeln!(list, "{entry}").expect("unable to render host");
        }

        list
    }
}

#[cfg(test)]
mod test {

    use super::*;

    #[test]
    fn test_parser() {
        let parse_result = Parser::parse(
            r#"
            127.0.0.1 localhost
            127.0.0.1 localhost.localdomain
            0.0.0.0 0.0.0.0
            # Start of list

            0.0.0.0 domain-a.com
            0.0.0.0 domain-b.com
            0.0.0.0 domain-c.com
        "#,
        );

        match parse_result {
            Err(err) => panic!("{err}"),
            Ok(entries) => {
                assert!(entries.len() == 6);
            }
        }
    }
}
