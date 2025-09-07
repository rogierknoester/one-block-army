use std::{
    fmt::Display,
    sync::mpsc::{self, Sender},
    thread::{self, available_parallelism},
};

use bumpalo::collections::Vec as BVec;

pub(crate) struct Parser {}

struct Line<'a> {
    content: &'a str,
}

impl<'a> From<&'a str> for Line<'a> {
    fn from(value: &'a str) -> Self {
        Self {
            content: value.trim(),
        }
    }
}

impl<'a> Line<'a> {
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
    pub(super) fn parse(input: &str, collector: &mut BVec<HostEntry>) -> Result<(), String> {
        let orchestrator = Orchestrator::new();

        let results = orchestrator.orchestrate(input, collector);

        Ok(results)
    }
}

struct Orchestrator {}

impl Orchestrator {
    fn new() -> Self {
        Self {}
    }

    /// Orchestrate the parsing in a multithreaded manner
    fn orchestrate(&self, contents: &str, collector: &mut BVec<HostEntry>)  {
        let lines = contents.lines().collect::<Vec<&str>>();
        let chunks = lines.chunks(self.resolve_chunk_size(contents));

        let rx = thread::scope(|scope| {
            let (tx, rx) = mpsc::channel::<HostEntry>();

            for chunk in chunks {
                let worker = Worker::new(tx.clone());

                scope.spawn(move || worker.work(chunk));
            }

            rx
        });

        for x in rx.into_iter() {
            collector.push(x);
        }
    }

    /// Resolve how many chunks should be created from the contents
    fn resolve_chunk_size(&self, contents: &str) -> usize {
        let max_thread_count = available_parallelism().unwrap();
        let line_count = contents.lines().count();
        line_count.div_ceil(max_thread_count.into())
    }
}

struct Worker {
    tx: Sender<HostEntry>,
}

impl Worker {
    fn new(tx: Sender<HostEntry>) -> Self {
        Self { tx }
    }

    fn work(&self, line: &[&str]) {
        for raw_line_content in line {
            if let Some(hostname) = parse_line(raw_line_content) {
                self.tx.send(hostname).expect("cannot send parsed");
            }
        }

    }
}

fn parse_line(line: &str) -> Option<HostEntry> {
    let line: Line = line.into();

    if line.is_empty() || line.is_only_comment() {
        return None;
    }

    let line = line.filter_out_comments();

    if line.is_empty() {
        return None;
    }

    let parts = line.parts();

    // it should be an ip and hostname part only
    if parts.len() < 2 {
        return None;
    }

    let hostname = unsafe { *parts.get_unchecked(1) };

    if !hostname_validator::is_valid(hostname) {
        eprintln!("hostname \"{hostname}\" is invalid");
    }

    Some(hostname.into())
}

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub(crate) struct HostEntry(pub(crate) String);

impl From<&str> for HostEntry {
    fn from(value: &str) -> Self {
        Self(value.to_string())
    }
}

impl Display for HostEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "0.0.0.0 {}", &self.0)
    }
}

pub(crate) trait HostsRenderer {
    fn render(self) -> String;
}

impl<'a> HostsRenderer for BVec<'a, HostEntry> {
    fn render(self) -> String {
        let mut list = String::with_capacity(self.len());

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
