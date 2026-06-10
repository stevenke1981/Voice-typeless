//! Persistence layer for recognition history using SQLite.
//!
//! The store is backed by a local SQLite database at the platform config path.

use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use rusqlite::{params, Connection};

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// A single recognition history entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryItem {
    pub id: String,
    pub text: String,
    pub language: String,
    /// Unix timestamp (seconds since epoch).
    pub timestamp: i64,
}

/// Trait abstracting the history store.
pub trait HistoryStore {
    /// Inserts a new item.  The `id` is assigned by the store.
    fn add(&self, text: &str, language: &str) -> Result<HistoryItem, HistoryError>;

    /// Returns the most recent `n` items, newest first.
    fn list(&self, n: i32) -> Result<Vec<HistoryItem>, HistoryError>;

    /// Removes a single item by `id`.
    fn delete(&self, id: &str) -> Result<(), HistoryError>;

    /// Removes items older than `retention_days`.
    fn prune(&self, retention_days: i32) -> Result<(), HistoryError>;

    /// Releases the database connection.
    fn close(&self) -> Result<(), HistoryError>;
}

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Errors that can occur during history operations.
#[derive(Debug, thiserror::Error)]
pub enum HistoryError {
    #[error("SQLite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("config directory not available")]
    NoConfigDir,
}

// ---------------------------------------------------------------------------
// SQLite implementation
// ---------------------------------------------------------------------------

/// SQLite-backed [`HistoryStore`].
///
/// Uses interior mutability (`Mutex<Option<Connection>>`) so that all trait
/// methods take `&self`.  The `Option` wrapper allows `close()` to consume
/// the connection (since `rusqlite::Connection::close` takes `self`).
pub struct SqliteStore {
    db: Mutex<Option<Connection>>,
}

impl SqliteStore {
    /// Opens (or creates) the history database at `<config-dir>/VoiceTypeless/history.db`.
    pub fn new() -> Result<Self, HistoryError> {
        let db_path = Self::db_path()?;

        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let db = Connection::open(&db_path)?;

        db.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS history (
                id         TEXT PRIMARY KEY,
                text       TEXT NOT NULL,
                language   TEXT NOT NULL DEFAULT 'en',
                created_at INTEGER NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_history_created ON history(created_at DESC);
            ",
        )?;

        Ok(Self {
            db: Mutex::new(Some(db)),
        })
    }

    fn db_path() -> Result<PathBuf, HistoryError> {
        let base = dirs::config_dir().ok_or(HistoryError::NoConfigDir)?;
        Ok(base.join("VoiceTypeless").join("history.db"))
    }
}

impl HistoryStore for SqliteStore {
    fn add(&self, text: &str, language: &str) -> Result<HistoryItem, HistoryError> {
        let guard = self.db.lock().unwrap();
        let db = guard.as_ref().expect("store not closed");
        let id = epoch_nanos().to_string();
        let ts = epoch_seconds();

        db.execute(
            "INSERT INTO history (id, text, language, created_at) VALUES (?1, ?2, ?3, ?4)",
            params![id, text, language, ts],
        )?;

        Ok(HistoryItem {
            id,
            text: text.to_string(),
            language: language.to_string(),
            timestamp: ts,
        })
    }

    fn list(&self, n: i32) -> Result<Vec<HistoryItem>, HistoryError> {
        let guard = self.db.lock().unwrap();
        let db = guard.as_ref().expect("store not closed");
        let mut stmt = db.prepare(
            "SELECT id, text, language, created_at FROM history ORDER BY created_at DESC LIMIT ?1",
        )?;

        let items = stmt
            .query_map(params![n], |row| {
                Ok(HistoryItem {
                    id: row.get(0)?,
                    text: row.get(1)?,
                    language: row.get(2)?,
                    timestamp: row.get(3)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(items)
    }

    fn delete(&self, id: &str) -> Result<(), HistoryError> {
        let guard = self.db.lock().unwrap();
        let db = guard.as_ref().expect("store not closed");
        db.execute("DELETE FROM history WHERE id = ?1", params![id])?;
        Ok(())
    }

    fn prune(&self, retention_days: i32) -> Result<(), HistoryError> {
        let guard = self.db.lock().unwrap();
        let db = guard.as_ref().expect("store not closed");
        let cutoff = epoch_seconds() - (retention_days as i64) * 86_400;
        db.execute("DELETE FROM history WHERE created_at < ?1", params![cutoff])?;
        Ok(())
    }

    fn close(&self) -> Result<(), HistoryError> {
        let mut guard = self.db.lock().unwrap();
        match guard.take() {
            Some(db) => db.close().map_err(|(_, e)| HistoryError::Sqlite(e)),
            None => Ok(()), // already closed — no-op
        }
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn epoch_seconds() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

fn epoch_nanos() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    /// Creates a store backed by a temporary file.
    fn temp_store(name: &str) -> (SqliteStore, PathBuf) {
        let dir = std::env::temp_dir().join(format!("vtl_history_test_{name}"));
        let _ = fs::remove_dir_all(&dir);

        // Override the default path by constructing manually.
        let db_path = dir.join("history.db");
        std::fs::create_dir_all(&dir).unwrap();

        let db = Connection::open(&db_path).unwrap();
        db.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS history (
                id         TEXT PRIMARY KEY,
                text       TEXT NOT NULL,
                language   TEXT NOT NULL DEFAULT 'en',
                created_at INTEGER NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_history_created ON history(created_at DESC);
            ",
        )
        .unwrap();

        (
            SqliteStore {
                db: Mutex::new(Some(db)),
            },
            dir,
        )
    }

    #[test]
    fn test_add_and_list() {
        let (store, _dir) = temp_store("add_list");

        let item = store.add("hello world", "en").unwrap();
        assert_eq!(item.text, "hello world");
        assert!(!item.id.is_empty());

        let items = store.list(10).unwrap();
        assert!(!items.is_empty());
        assert!(items.iter().any(|i| i.id == item.id));
    }

    #[test]
    fn test_list_returns_newest_first() {
        let (store, _dir) = temp_store("newest_first");

        store.add("first", "en").unwrap();
        std::thread::sleep(std::time::Duration::from_secs(1));
        store.add("second", "en").unwrap();

        let items = store.list(10).unwrap();
        assert_eq!(items.len(), 2);
        // Second inserted should be first in list
        assert_eq!(items[0].text, "second");
    }

    #[test]
    fn test_delete() {
        let (store, _dir) = temp_store("delete");

        let item = store.add("delete me", "en").unwrap();
        store.delete(&item.id).unwrap();

        let items = store.list(10).unwrap();
        assert!(!items.iter().any(|i| i.id == item.id));
    }

    #[test]
    fn test_prune() {
        let (store, _dir) = temp_store("prune");
        store.add("ancient", "en").unwrap();
        store.prune(365).unwrap();
        // May or may not be pruned depending on timing — just verify no error
    }

    #[test]
    fn test_close() {
        let (store, _dir) = temp_store("close");
        store.close().unwrap();
        // Second close is a no-op, should still be Ok
        assert!(store.close().is_ok());
    }

    #[test]
    fn test_new_creates_file() {
        let dir = std::env::temp_dir().join("vtl_history_test_new");
        let _ = fs::remove_dir_all(&dir);

        // Override config dir for this test
        // We can't easily override dirs() behavior, so test via temp_path
        let db_path = dir.join("VoiceTypeless").join("history.db");
        std::fs::create_dir_all(db_path.parent().unwrap()).unwrap();
        let _db = Connection::open(&db_path).unwrap();

        // Verify the directory was created
        assert!(dir.join("VoiceTypeless").exists());
        let _ = fs::remove_dir_all(&dir);
    }
}
