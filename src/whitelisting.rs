use std::{
    collections::HashSet,
    sync::mpsc::{self, Sender},
    thread::{self, available_parallelism},
};

use crate::parser::HostEntry;

pub struct Whitelister<'a> {
    whitelisted_hosts: &'a HashSet<String>,
}

impl<'a> Whitelister<'a> {
    pub fn new(whitelisted_hosts: &'a HashSet<String>) -> Self {
        Self { whitelisted_hosts }
    }

    pub fn evaluate(&self, hosts: &[HostEntry]) -> Vec<HostEntry> {
        let chunk_size = available_parallelism()
            .map(|count| hosts.len().div_ceil(count.into()))
            .unwrap_or(hosts.len());

        let chunks = hosts.chunks(chunk_size);

        // we will at best need a vector with the capacity to hold all hosts we have pre-filtering
        let mut entries = Vec::with_capacity(hosts.len());

        let rx = thread::scope(|scope| {
            let (tx, rx) = mpsc::channel::<Vec<HostEntry>>();

            for chunk in chunks {
                let worker = WhitelistingWorker {
                    entries_to_check: chunk,
                    whitelisted_hosts: self.whitelisted_hosts,
                    tx: tx.clone(),
                };

                scope.spawn(|| worker.run());
            }
            rx
        });

        rx.iter().for_each(|mut subset| entries.append(&mut subset));

        entries
    }
}

/// A simple worker that performs
/// filtering of host entries on a subset that it is created with.
/// It sends back the result over a channel to the main thread
struct WhitelistingWorker<'a> {
    entries_to_check: &'a [HostEntry],
    whitelisted_hosts: &'a HashSet<String>,
    tx: Sender<Vec<HostEntry>>,
}

impl<'a> WhitelistingWorker<'a> {
    fn run(self) {
        let result = self
            .entries_to_check
            .iter()
            .filter(|entry| !self.whitelisted_hosts.contains(&entry.hostname))
            .cloned()
            .collect::<Vec<HostEntry>>();

        self.tx.send(result).expect("cannot send results to queue");
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_whitelister() {
        let whitelisted_hosts = HashSet::from_iter(vec![String::from("kagi.com")]);
        let whitelister = Whitelister::new(&whitelisted_hosts);

        let entries = vec![
            HostEntry::new(
                std::net::IpAddr::V4("127.0.0.1".parse().unwrap()),
                "kagi.com".to_owned(),
            ),
            HostEntry::new(
                std::net::IpAddr::V4("127.0.0.1".parse().unwrap()),
                "eff.org".to_owned(),
            ),
        ];

        let result = whitelister.evaluate(&entries);
        assert!(result.len() == 1);
        assert_eq!(result.first().unwrap().hostname, "eff.org");
    }
}
