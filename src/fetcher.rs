use std::thread;

pub(crate) fn fetch(urls: &[String]) -> String {
    thread::scope(|scope| {
        let mut threads = vec![];

        for url in urls.iter() {
            let thread = scope.spawn(move || {
                eprintln!("fetching {url}");
                let result = fetch_adlist(url);

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

        let mut contents = String::new();
        for thread in threads {
            let x = thread.join().expect("could not join parsing threads");

            contents.push_str(&x);
        }

        contents
    })
}

fn fetch_adlist(url: &str) -> Result<String, reqwest::Error> {
    reqwest::blocking::get(url)?.text()
}
