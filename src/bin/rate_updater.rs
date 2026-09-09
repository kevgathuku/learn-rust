use std::time::Duration;

use hello_world::rates::{SqliteRateStore, fetch_frankfurter};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let db_path = args.get(1).map(|s| s.as_str()).unwrap_or("ledger.db");
    let interval_secs: u64 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(3600);

    println!("rate_updater: db={db_path}, interval={interval_secs}s");

    let store = match SqliteRateStore::new(db_path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("failed to open rate store: {e}");
            std::process::exit(1);
        }
    };

    loop {
        match fetch_frankfurter() {
            Ok(rates) => {
                let count = rates.len();
                match store.upsert_rates(&rates) {
                    Ok(()) => println!("updated {count} rates"),
                    Err(e) => eprintln!("failed to upsert rates: {e}"),
                }
            }
            Err(e) => eprintln!("failed to fetch rates: {e}"),
        }

        std::thread::sleep(Duration::from_secs(interval_secs));
    }
}
