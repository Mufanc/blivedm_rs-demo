use anyhow::Result;
use rusqlite::Connection;
use std::path::Path;

pub struct LivePersist {
    conn: Connection,
}

impl LivePersist {
    pub fn new(file: &dyn AsRef<Path>) -> Result<LivePersist> {
        let conn = Connection::open(file)?;

        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "synchronous", "NORMAL")?;
        conn.pragma_update(None, "busy_timeout", "5000")?;

        // Todo: create tables

        Ok(Self { conn })
    }
}
