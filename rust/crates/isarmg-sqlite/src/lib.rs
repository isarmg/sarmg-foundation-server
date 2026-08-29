use rusqlite::Connection;
use std::path::Path;

pub fn open(path: &Path) -> Result<Connection, rusqlite::Error> {
    let connection = Connection::open(path)?;
    connection.pragma_update(None, "journal_mode", "WAL")?;
    connection.pragma_update(None, "foreign_keys", "ON")?;
    connection.pragma_update(None, "busy_timeout", 5000)?;
    connection.pragma_update(None, "synchronous", "FULL")?;
    Ok(connection)
}

pub fn integrity_check(connection: &Connection) -> Result<bool, rusqlite::Error> {
    let result: String = connection.query_row("PRAGMA integrity_check", [], |row| row.get(0))?;
    Ok(result.eq_ignore_ascii_case("ok"))
}
