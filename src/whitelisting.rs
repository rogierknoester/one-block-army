use std::{
    collections::HashSet,
    sync::mpsc::{self, Sender},
    thread::{self, available_parallelism},
};

use bumpalo::Bump;
use bumpalo::collections::Vec as BVec;
use glob_match::glob_match;

use crate::parser::HostEntry;

pub struct Whitelister<'a> {
    whitelisted_hosts: &'a HashSet<String>,
}

impl<'a> Whitelister<'a> {
    pub fn new(whitelisted_hosts: &'a HashSet<String>) -> Self {
        Self { whitelisted_hosts }
    }

    pub fn evaluate(&self, hosts: &[HostEntry], collector: &mut BVec<HostEntry>) {
        let chunk_size = available_parallelism()
            .map(|count| hosts.len().div_ceil(count.into()))
            .unwrap_or(hosts.len());

        let chunks = hosts.chunks(chunk_size);

        let rx = thread::scope(|scope| {
            let (tx, rx) = mpsc::channel::<HostEntry>();

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

        rx.iter().for_each(|entry| collector.push(entry));
    }
}

/// A simple worker that performs
/// filtering of host entries on a subset that it is created with.
/// It sends back the result over a channel to the main thread
struct WhitelistingWorker<'a> {
    entries_to_check: &'a [HostEntry],
    whitelisted_hosts: &'a HashSet<String>,
    tx: Sender<HostEntry>,
}

#[derive(PartialEq)]
enum EvaluationResult {
    Remove,
    Keep,
}

impl<'a> WhitelistingWorker<'a> {
    fn run(self) {
         self
            .entries_to_check
            .iter()
            .filter(|entry| self.evaluate(entry) == EvaluationResult::Keep)
            .for_each(|entry| self.tx.send(entry.to_owned()).expect("cannot send whitelisting results to queue"));
    }

    fn evaluate(&self, host: &HostEntry) -> EvaluationResult {
        for whitelist_entry in self.whitelisted_hosts {
            if glob_match(whitelist_entry, &host.0) {
                return EvaluationResult::Remove;
            }
        }

        EvaluationResult::Keep
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_whitelister() {
        let whitelisted_hosts = HashSet::from_iter(vec![String::from("kagi.com")]);
        let whitelister = Whitelister::new(&whitelisted_hosts);

        let entries = vec!["kagi.com".into(), "eff.org".into()];

        let result = whitelister.evaluate(&entries);
        assert!(result.len() == 1);
        assert_eq!(result.first().unwrap().0, "eff.org");
    }

    #[test]
    fn test_globbing_in_whitelisted_hosts() {
        let whitelisted_hosts = HashSet::from_iter(vec![String::from("*.kagi.com")]);
        let whitelister = Whitelister::new(&whitelisted_hosts);

        let entries = vec![
            "kagi.com".into(),
            "assistant.kagi.com".into(),
            "settings.kagi.com".into(),
            "eff.org".into(),
        ];

        let result = whitelister.evaluate(&entries);
        assert!(result.len() == 2);
        assert_eq!(result.first().unwrap().0, "kagi.com");
        assert_eq!(result[1].0, "eff.org");
    }
}
