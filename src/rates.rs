use std::sync::{LazyLock, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use rusqlite::Connection;
use rusqlite_migration::{M, Migrations};
use serde::Deserialize;

use crate::{Currency, ExchangeRate, RateError, RateReader};

static RATE_MIGRATIONS: LazyLock<Migrations> = LazyLock::new(|| {
    Migrations::new(vec![M::up(
        "CREATE TABLE IF NOT EXISTS rates (
            from_currency TEXT NOT NULL,
            to_currency TEXT NOT NULL,
            rate REAL NOT NULL,
            fetched_at INTEGER NOT NULL,
            PRIMARY KEY (from_currency, to_currency)
        )",
    )])
});

pub struct SqliteRateStore {
    conn: Mutex<Connection>,
}

impl SqliteRateStore {
    pub fn new(path: impl AsRef<str>) -> Result<Self, RateError> {
        let mut conn = Connection::open(path.as_ref())
            .map_err(|e| RateError::StoreUnavailable(e.to_string()))?;

        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA busy_timeout=5000;")
            .map_err(|e| RateError::StoreUnavailable(e.to_string()))?;

        RATE_MIGRATIONS
            .to_latest(&mut conn)
            .map_err(|e| RateError::StoreUnavailable(e.to_string()))?;

        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    pub fn upsert_rates(&self, rates: &[ExchangeRate]) -> Result<(), RateError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| RateError::StoreUnavailable(e.to_string()))?;
        let tx = conn
            .unchecked_transaction()
            .map_err(|e| RateError::StoreUnavailable(e.to_string()))?;

        {
            let mut stmt = tx
                .prepare(
                    "INSERT INTO rates (from_currency, to_currency, rate, fetched_at)
                     VALUES (?1, ?2, ?3, ?4)
                     ON CONFLICT (from_currency, to_currency)
                     DO UPDATE SET rate = excluded.rate, fetched_at = excluded.fetched_at",
                )
                .map_err(|e| RateError::StoreUnavailable(e.to_string()))?;

            for rate in rates {
                let from = rate.from.to_string();
                let to = rate.to.to_string();
                let epoch_secs = rate
                    .fetched_at
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or(Duration::ZERO)
                    .as_secs() as i64;

                stmt.execute(rusqlite::params![from, to, rate.rate, epoch_secs])
                    .map_err(|e| RateError::StoreUnavailable(e.to_string()))?;
            }
        }

        tx.commit()
            .map_err(|e| RateError::StoreUnavailable(e.to_string()))?;
        Ok(())
    }
}

impl RateReader for SqliteRateStore {
    fn get_rate(&self, from: Currency, to: Currency) -> Result<ExchangeRate, RateError> {
        if from == to {
            return Ok(ExchangeRate {
                from,
                to,
                rate: 1.0,
                fetched_at: SystemTime::now(),
            });
        }

        let conn = self
            .conn
            .lock()
            .map_err(|e| RateError::StoreUnavailable(e.to_string()))?;
        let mut stmt = conn
            .prepare(
                "SELECT rate, fetched_at FROM rates WHERE from_currency = ?1 AND to_currency = ?2",
            )
            .map_err(|e| RateError::StoreUnavailable(e.to_string()))?;

        let result = stmt
            .query_row(rusqlite::params![from.to_string(), to.to_string()], |row| {
                let rate: f64 = row.get(0)?;
                let epoch_secs: i64 = row.get(1)?;
                Ok((rate, epoch_secs))
            })
            .map_err(|_| RateError::RateNotFound(from, to))?;

        let (rate, epoch_secs) = result;
        let fetched_at = UNIX_EPOCH + Duration::from_secs(epoch_secs as u64);

        Ok(ExchangeRate {
            from,
            to,
            rate,
            fetched_at,
        })
    }

    fn all_rates(&self) -> Result<Vec<ExchangeRate>, RateError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| RateError::StoreUnavailable(e.to_string()))?;
        let mut stmt = conn
            .prepare("SELECT from_currency, to_currency, rate, fetched_at FROM rates")
            .map_err(|e| RateError::StoreUnavailable(e.to_string()))?;

        let rows = stmt
            .query_map([], |row| {
                let from: String = row.get(0)?;
                let to: String = row.get(1)?;
                let rate: f64 = row.get(2)?;
                let epoch_secs: i64 = row.get(3)?;
                Ok((from, to, rate, epoch_secs))
            })
            .map_err(|e| RateError::StoreUnavailable(e.to_string()))?;

        let mut rates = Vec::new();
        for row in rows {
            let (from, to, rate, epoch_secs) =
                row.map_err(|e| RateError::StoreUnavailable(e.to_string()))?;
            let from: Currency = from.parse().map_err(|e: RateError| e)?;
            let to: Currency = to.parse().map_err(|e: RateError| e)?;
            let fetched_at = UNIX_EPOCH + Duration::from_secs(epoch_secs as u64);
            rates.push(ExchangeRate {
                from,
                to,
                rate,
                fetched_at,
            });
        }

        Ok(rates)
    }
}

#[derive(Deserialize)]
struct FrankfurterRate {
    base: String,
    quote: String,
    rate: f64,
}

pub fn fetch_frankfurter() -> Result<Vec<ExchangeRate>, RateError> {
    let bases = [Currency::Usd, Currency::Eur, Currency::Kes];
    let now = SystemTime::now();
    let mut rates = Vec::new();

    for base in &bases {
        let url = format!("https://api.frankfurter.dev/v2/rates?base={base}");
        let body: String = ureq::get(&url)
            .call()
            .map_err(|e| RateError::StoreUnavailable(format!("Frankfurter request failed: {e}")))?
            .body_mut()
            .read_to_string()
            .map_err(|e| RateError::StoreUnavailable(format!("Failed to read response: {e}")))?;

        let resp: Vec<FrankfurterRate> = serde_json::from_str(&body)
            .map_err(|e| RateError::StoreUnavailable(format!("Failed to parse response: {e}")))?;

        for entry in &resp {
            if let (Ok(from), Ok(to)) = (
                entry.base.parse::<Currency>(),
                entry.quote.parse::<Currency>(),
            ) {
                rates.push(ExchangeRate {
                    from,
                    to,
                    rate: entry.rate,
                    fetched_at: now,
                });
            }
        }
    }

    Ok(rates)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_store() -> SqliteRateStore {
        SqliteRateStore::new(":memory:").unwrap()
    }

    fn test_rate(from: Currency, to: Currency, rate: f64) -> ExchangeRate {
        ExchangeRate {
            from,
            to,
            rate,
            fetched_at: SystemTime::now(),
        }
    }

    #[test]
    fn new_store_creates_table() {
        let store = test_store();
        let rates = store.all_rates().unwrap();
        assert!(rates.is_empty());
    }

    #[test]
    fn upsert_and_get_rate() {
        let store = test_store();
        let rates = vec![test_rate(Currency::Eur, Currency::Usd, 1.1234)];
        store.upsert_rates(&rates).unwrap();

        let got = store.get_rate(Currency::Eur, Currency::Usd).unwrap();
        assert_eq!(got.rate, 1.1234);
        assert_eq!(got.from, Currency::Eur);
        assert_eq!(got.to, Currency::Usd);
    }

    #[test]
    fn upsert_updates_existing_rate() {
        let store = test_store();
        store
            .upsert_rates(&[test_rate(Currency::Eur, Currency::Usd, 1.1)])
            .unwrap();
        store
            .upsert_rates(&[test_rate(Currency::Eur, Currency::Usd, 1.2)])
            .unwrap();

        let got = store.get_rate(Currency::Eur, Currency::Usd).unwrap();
        assert_eq!(got.rate, 1.2);
    }

    #[test]
    fn same_currency_returns_one() {
        let store = test_store();
        let got = store.get_rate(Currency::Eur, Currency::Eur).unwrap();
        assert_eq!(got.rate, 1.0);
    }

    #[test]
    fn missing_rate_returns_error() {
        let store = test_store();
        let result = store.get_rate(Currency::Eur, Currency::Usd);
        assert!(matches!(result, Err(RateError::RateNotFound(_, _))));
    }

    #[test]
    fn all_rates_returns_all() {
        let store = test_store();
        store
            .upsert_rates(&[
                test_rate(Currency::Eur, Currency::Usd, 1.1),
                test_rate(Currency::Usd, Currency::Eur, 0.9),
            ])
            .unwrap();

        let rates = store.all_rates().unwrap();
        assert_eq!(rates.len(), 2);
    }

    #[test]
    fn roundtrip_system_time() {
        let store = test_store();
        let ts = UNIX_EPOCH + Duration::from_secs(1_700_000_000);
        let rate = ExchangeRate {
            from: Currency::Eur,
            to: Currency::Usd,
            rate: 1.5,
            fetched_at: ts,
        };
        store.upsert_rates(&[rate]).unwrap();

        let got = store.get_rate(Currency::Eur, Currency::Usd).unwrap();
        assert_eq!(got.fetched_at, ts);
    }
}
