use std::thread;

pub(crate) struct Fetcher<Client: HttpClient> {
    client: Client,
}

impl Fetcher<ReqwestClient> {
    pub(super) fn new_with_reqwest() -> Self {
        Self {
            client: ReqwestClient {},
        }
    }
}

impl<Client: HttpClient> Fetcher<Client> {
    pub(crate) fn fetch(&self, urls: &[String], contents: &mut String)  {
        thread::scope(|scope| {
            let mut threads = vec![];

            for url in urls.iter() {
                let thread = scope.spawn(move || {
                    eprintln!("fetching {url}");
                    let result = self.client.get_text(url);

                    match result {
                        Ok(contents) => contents,
                        Err(err) => {
                            eprintln!("could not fetch contents at adlist \"{url}\"");
                            eprintln!("{err}");
                            String::new()
                        }
                    }
                });

                threads.push(thread);
            }

            for thread in threads {
                let adlist_contents = thread.join().expect("could not join parsing threads");

                contents.push_str(&adlist_contents);
            }
        })
    }
}

#[derive(Debug, thiserror::Error, Clone )]
pub(crate) enum HttpError {
    #[error("A timeout occurred when requesting {0}")]
    Timeout(String),
    #[error("Unable to connect to {0}")]
    Connection(String),
    #[error("Received http error with status code {0}")]
    Status(String),
    #[error("Unknown error occurred while requesting {0}")]
    Unknown(String),
}

pub(crate) trait HttpClient: Sync + Send {
    fn get_text(&self, url: &str) -> Result<String, HttpError>;
}

pub(crate) struct ReqwestClient {}

impl HttpClient for ReqwestClient {
    fn get_text(&self, url: &str) -> Result<String, HttpError> {
        let response = reqwest::blocking::get(url).and_then(|x| x.text());

        match response {
            Ok(text) => Ok(text),
            Err(error) => {
                if error.is_timeout() {
                    Err(HttpError::Timeout(url.to_string()))
                } else if error.is_connect() {
                    Err(HttpError::Connection(url.to_string()))
                } else if error.is_status()
                    && let Some(status_code) = error.status()
                {
                    Err(HttpError::Status(status_code.to_string()))
                } else {
                    Err(HttpError::Unknown(url.to_string()))
                }
            }
        }
    }
}

#[cfg(test)]
mod test {
    use std::collections::HashMap;

    use crate::fetcher;

    use super::*;

    struct MockClient {
        responses: HashMap<String, Result<String, HttpError>>,
    }

    impl MockClient {
        fn new() -> Self {
            Self {
                responses: HashMap::new(),
            }
        }

        fn add_response(&mut self, url: &str, response: &str) {
            self.responses
                .insert(url.to_string(), Ok(response.to_string()));
        }

        fn add_error(&mut self, url: &str, error: HttpError) {
            self.responses.insert(url.to_string(), Err(error));
        }
    }
    impl HttpClient for MockClient {
        fn get_text(&self, url: &str) -> Result<String, HttpError> {
            self.responses.get(url).cloned().unwrap()
        }
    }

    #[test]
    fn fetcher_retrieves_texts_from_urls() {
        let mut client = MockClient::new();
        client.add_response("example.com", "0.0.0.0 ads.com\n");
        client.add_response("other.com", "0.0.0.0 ads.com\n");

        let urls = vec!["example.com".to_string(), "other.com".to_string()];

        let adlists = fetcher::Fetcher { client }.fetch(&urls);

        assert_eq!("0.0.0.0 ads.com\n0.0.0.0 ads.com\n", adlists.as_str());
    }

    #[test]
    fn fetcher_should_ignore_errors() {
        let mut client = MockClient::new();
        client.add_response("example.com", "0.0.0.0 ads.com\n");
        client.add_error("other.com", HttpError::Unknown("other.com".to_string()));

        let urls = vec!["example.com".to_string(), "other.com".to_string()];

        let adlists = fetcher::Fetcher { client }.fetch(&urls);

        assert_eq!("0.0.0.0 ads.com\n", adlists.as_str());
    }
}
