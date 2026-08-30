use rusqlite::Connection;
use sqlx::{
    SqlitePool,
    migrate::{MigrateError, Migrator},
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous},
};
use std::{path::Path, str::FromStr, time::Duration};

/// Open an asynchronous SQLite pool with the product baseline applied to every
/// connection. Callers still own their database file and migrations.
pub async fn open_pool(database_url: &str, max_connections: u32) -> anyhow::Result<SqlitePool> {
    if max_connections == 0 {
        anyhow::bail!("max_connections must be greater than zero");
    }
    let options = SqliteConnectOptions::from_str(database_url)?
        .create_if_missing(true)
        .journal_mode(SqliteJournalMode::Wal)
        .foreign_keys(true)
        .busy_timeout(Duration::from_secs(5))
        .synchronous(SqliteSynchronous::Full);
    Ok(SqlitePoolOptions::new()
        .max_connections(max_connections)
        .acquire_timeout(Duration::from_secs(10))
        .connect_with(options)
        .await?)
}

pub async fn migrate(pool: &SqlitePool, migrator: &Migrator) -> Result<(), MigrateError> {
    migrator.run(pool).await
}

pub async fn pool_integrity_check(pool: &SqlitePool) -> Result<bool, sqlx::Error> {
    let result: String = sqlx::query_scalar("PRAGMA integrity_check")
        .fetch_one(pool)
        .await?;
    Ok(result.eq_ignore_ascii_case("ok"))
}

pub async fn pool_foreign_key_check(pool: &SqlitePool) -> Result<bool, sqlx::Error> {
    Ok(sqlx::query("PRAGMA foreign_key_check")
        .fetch_all(pool)
        .await?
        .is_empty())
}

/// Checkpoint the current WAL without silently discarding a busy result.
pub async fn checkpoint(pool: &SqlitePool) -> anyhow::Result<()> {
    let (busy, _log_frames, _checkpointed_frames): (i64, i64, i64) =
        sqlx::query_as("PRAGMA wal_checkpoint(TRUNCATE)")
            .fetch_one(pool)
            .await?;
    if busy != 0 {
        anyhow::bail!("SQLite WAL checkpoint is busy");
    }
    Ok(())
}

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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn sqlx_pool_applies_safety_pragmas_to_every_connection_and_reopens() {
        let directory = tempfile::tempdir().unwrap();
        let database_path = directory.path().join("foundation.sqlite3");
        let database_url = format!("sqlite://{}", database_path.display());
        let pool = open_pool(&database_url, 2).await.unwrap();

        let mut first = pool.acquire().await.unwrap();
        let mut second = pool.acquire().await.unwrap();
        for connection in [&mut first, &mut second] {
            let foreign_keys: i64 = sqlx::query_scalar("PRAGMA foreign_keys")
                .fetch_one(&mut **connection)
                .await
                .unwrap();
            let journal_mode: String = sqlx::query_scalar("PRAGMA journal_mode")
                .fetch_one(&mut **connection)
                .await
                .unwrap();
            let busy_timeout: i64 = sqlx::query_scalar("PRAGMA busy_timeout")
                .fetch_one(&mut **connection)
                .await
                .unwrap();
            let synchronous: i64 = sqlx::query_scalar("PRAGMA synchronous")
                .fetch_one(&mut **connection)
                .await
                .unwrap();
            assert_eq!(foreign_keys, 1);
            assert_eq!(journal_mode, "wal");
            assert_eq!(busy_timeout, 5_000);
            assert_eq!(synchronous, 2);
        }
        drop(first);
        drop(second);

        sqlx::query("CREATE TABLE parents(id INTEGER PRIMARY KEY)")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query(
            "CREATE TABLE children(\
                 id INTEGER PRIMARY KEY,\
                 parent_id INTEGER NOT NULL REFERENCES parents(id)\
             )",
        )
        .execute(&pool)
        .await
        .unwrap();
        assert!(
            sqlx::query("INSERT INTO children(id,parent_id) VALUES(1,999)")
                .execute(&pool)
                .await
                .is_err()
        );
        sqlx::query("INSERT INTO parents(id) VALUES(7)")
            .execute(&pool)
            .await
            .unwrap();
        assert!(pool_integrity_check(&pool).await.unwrap());
        assert!(pool_foreign_key_check(&pool).await.unwrap());
        checkpoint(&pool).await.unwrap();
        pool.close().await;

        let reopened = open_pool(&database_url, 2).await.unwrap();
        let parent: i64 = sqlx::query_scalar("SELECT id FROM parents")
            .fetch_one(&reopened)
            .await
            .unwrap();
        assert_eq!(parent, 7);
        assert!(pool_integrity_check(&reopened).await.unwrap());
        assert!(pool_foreign_key_check(&reopened).await.unwrap());
    }

    #[test]
    fn legacy_synchronous_connection_keeps_the_same_baseline() {
        let directory = tempfile::tempdir().unwrap();
        let connection = open(&directory.path().join("legacy.sqlite3")).unwrap();
        assert_eq!(
            connection
                .pragma_query_value(None, "foreign_keys", |row| row.get::<_, i64>(0))
                .unwrap(),
            1
        );
        assert!(integrity_check(&connection).unwrap());
    }
}
