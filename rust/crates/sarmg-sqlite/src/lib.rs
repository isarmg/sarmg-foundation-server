//! Strict SQLx adapters for the Sarmg SQLite baseline.
//!
//! This crate does not own product DDL, migrations, locking, backup, restore or
//! file-permission policy. Opening an existing database and allowing SQLite to
//! create a missing file are deliberately separate operations.

use sarmg_schema_identity::{
    SQLITE_SCHEMA_ROWS_QUERY, schema_fingerprint as fingerprint_rows,
    schema_identity_from_metadata_rows, validate_product_metadata_columns,
    validate_product_metadata_ddl,
};
use sqlx::{
    Executor, Row, Sqlite, SqliteConnection, SqlitePool,
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous},
};
use std::{io, path::Path, path::PathBuf, time::Duration};
use thiserror::Error;

pub use sarmg_schema_identity::{
    Error as SchemaIdentityError, IdentityField, PRODUCT_METADATA_DDL, ProductMetadataColumn,
    ProductMetadataRow, SCHEMA_FINGERPRINT_ALGORITHM_VERSION, SchemaIdentity, SchemaRow,
};

pub const BUSY_TIMEOUT: Duration = Duration::from_secs(5);
pub const DEFAULT_ACQUIRE_TIMEOUT: Duration = Duration::from_secs(10);

/// The intentionally small set of product-selectable pool settings.
///
/// Durability and correctness PRAGMAs are not configurable: every connection
/// uses WAL, foreign keys, a five-second busy timeout and FULL synchronous.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PoolOptions {
    max_connections: u32,
    min_connections: u32,
    acquire_timeout: Duration,
}

impl PoolOptions {
    pub const fn new(max_connections: u32) -> Self {
        Self {
            max_connections,
            min_connections: 0,
            acquire_timeout: DEFAULT_ACQUIRE_TIMEOUT,
        }
    }

    pub const fn with_min_connections(mut self, min_connections: u32) -> Self {
        self.min_connections = min_connections;
        self
    }

    pub const fn with_acquire_timeout(mut self, acquire_timeout: Duration) -> Self {
        self.acquire_timeout = acquire_timeout;
        self
    }

    pub const fn max_connections(&self) -> u32 {
        self.max_connections
    }

    pub const fn min_connections(&self) -> u32 {
        self.min_connections
    }

    pub const fn acquire_timeout(&self) -> Duration {
        self.acquire_timeout
    }

    pub fn validate(&self) -> Result<(), PoolOptionsError> {
        if self.max_connections == 0 {
            return Err(PoolOptionsError::ZeroMaxConnections);
        }
        if self.min_connections > self.max_connections {
            return Err(PoolOptionsError::MinConnectionsExceedMax {
                min_connections: self.min_connections,
                max_connections: self.max_connections,
            });
        }
        if self.acquire_timeout.is_zero() {
            return Err(PoolOptionsError::ZeroAcquireTimeout);
        }
        Ok(())
    }
}

impl Default for PoolOptions {
    fn default() -> Self {
        Self::new(10)
    }
}

/// Open an existing SQLite file. This operation never asks SQLite to create a
/// missing file and reports an initially missing path as a typed error.
pub async fn open_existing(
    database_path: impl AsRef<Path>,
    options: PoolOptions,
) -> Result<SqlitePool, Error> {
    options.validate()?;
    let database_path = database_path.as_ref();
    match database_path.try_exists() {
        Ok(true) => {}
        Ok(false) => {
            return Err(Error::DatabaseDoesNotExist {
                path: database_path.to_path_buf(),
            });
        }
        Err(source) => {
            return Err(Error::InspectDatabase {
                path: database_path.to_path_buf(),
                source,
            });
        }
    }
    connect(database_path, false, options).await
}

/// Open a SQLite file, explicitly allowing SQLite to create it when missing.
/// This is not a schema initializer and may also open an already-existing file.
pub async fn create_if_missing(
    database_path: impl AsRef<Path>,
    options: PoolOptions,
) -> Result<SqlitePool, Error> {
    options.validate()?;
    connect(database_path.as_ref(), true, options).await
}

async fn connect(
    database_path: &Path,
    create_if_missing: bool,
    options: PoolOptions,
) -> Result<SqlitePool, Error> {
    let connect_options = SqliteConnectOptions::new()
        .filename(database_path)
        .create_if_missing(create_if_missing)
        .journal_mode(SqliteJournalMode::Wal)
        .foreign_keys(true)
        .busy_timeout(BUSY_TIMEOUT)
        .synchronous(SqliteSynchronous::Full);
    let pool = SqlitePoolOptions::new()
        .max_connections(options.max_connections)
        .min_connections(options.min_connections)
        .acquire_timeout(options.acquire_timeout)
        .connect_with(connect_options)
        .await?;
    Ok(pool)
}

/// Require the complete integrity-check result to be the single value `ok`.
pub async fn integrity_check(pool: &SqlitePool) -> Result<(), Error> {
    let messages: Vec<String> = sqlx::query_scalar("PRAGMA integrity_check")
        .fetch_all(pool)
        .await?;
    if messages.len() != 1 || !messages[0].eq_ignore_ascii_case("ok") {
        return Err(Error::IntegrityCheckFailed { messages });
    }
    Ok(())
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ForeignKeyViolation {
    pub table: String,
    pub row_id: Option<i64>,
    pub parent: String,
    pub foreign_key_index: i64,
}

/// Require `PRAGMA foreign_key_check` to return no violations.
pub async fn foreign_key_check(pool: &SqlitePool) -> Result<(), Error> {
    let violations = sqlx::query("PRAGMA foreign_key_check")
        .fetch_all(pool)
        .await?
        .into_iter()
        .map(|row| {
            Ok(ForeignKeyViolation {
                table: row.try_get(0)?,
                row_id: row.try_get(1)?,
                parent: row.try_get(2)?,
                foreign_key_index: row.try_get(3)?,
            })
        })
        .collect::<Result<Vec<_>, sqlx::Error>>()?;
    if !violations.is_empty() {
        return Err(Error::ForeignKeyViolations { violations });
    }
    Ok(())
}

/// Truncate-checkpoint the WAL and reject both busy and incomplete results.
pub async fn checkpoint(pool: &SqlitePool) -> Result<(), Error> {
    let result: (i64, i64, i64) = sqlx::query_as("PRAGMA wal_checkpoint(TRUNCATE)")
        .fetch_one(pool)
        .await?;
    validate_checkpoint_result(result)
}

fn validate_checkpoint_result(
    (busy, log_frames, checkpointed_frames): (i64, i64, i64),
) -> Result<(), Error> {
    if busy != 0 {
        return Err(Error::CheckpointBusy {
            log_frames,
            checkpointed_frames,
        });
    }
    if log_frames != checkpointed_frames {
        return Err(Error::CheckpointIncomplete {
            log_frames,
            checkpointed_frames,
        });
    }
    Ok(())
}

/// Read the canonically filtered and ordered rows using any SQLx SQLite
/// executor, including a pool, connection or transaction connection.
pub async fn schema_rows<'executor, E>(executor: E) -> Result<Vec<SchemaRow>, Error>
where
    E: Executor<'executor, Database = Sqlite>,
{
    let rows = sqlx::query(SQLITE_SCHEMA_ROWS_QUERY)
        .fetch_all(executor)
        .await?
        .into_iter()
        .map(|row| {
            Ok(SchemaRow::new(
                row.try_get::<String, _>(0)?,
                row.try_get::<String, _>(1)?,
                row.try_get::<String, _>(2)?,
                row.try_get::<String, _>(3)?,
            ))
        })
        .collect::<Result<Vec<_>, sqlx::Error>>()?;
    Ok(rows)
}

/// Calculate the canonical schema fingerprint using an arbitrary SQLx SQLite
/// executor. The pure algorithm lives in `sarmg-schema-identity`.
pub async fn schema_fingerprint<'executor, E>(executor: E) -> Result<String, Error>
where
    E: Executor<'executor, Database = Sqlite>,
{
    Ok(fingerprint_rows(&schema_rows(executor).await?)?)
}

/// Validate the metadata table and return an identity only after its declared
/// fingerprint has been verified against the actual schema.
pub async fn read_schema_identity(
    connection: &mut SqliteConnection,
) -> Result<SchemaIdentity, Error> {
    validate_metadata_table(connection).await?;
    let metadata_rows = read_metadata_rows(connection).await?;
    let identity = schema_identity_from_metadata_rows(&metadata_rows)?;
    let actual_fingerprint = schema_fingerprint(&mut *connection).await?;
    identity.verify_fingerprint(&actual_fingerprint)?;
    Ok(identity)
}

/// Validate an identity against exact compiled current values.
pub async fn require_current_schema(
    connection: &mut SqliteConnection,
    expected: &SchemaIdentity,
) -> Result<SchemaIdentity, Error> {
    let actual = read_schema_identity(connection).await?;
    actual.require_exact(expected)?;
    Ok(actual)
}

/// Pool convenience wrapper around [`read_schema_identity`].
pub async fn read_pool_schema_identity(pool: &SqlitePool) -> Result<SchemaIdentity, Error> {
    let mut connection = pool.acquire().await?;
    read_schema_identity(&mut connection).await
}

/// Pool convenience wrapper around [`require_current_schema`].
pub async fn require_pool_current_schema(
    pool: &SqlitePool,
    expected: &SchemaIdentity,
) -> Result<SchemaIdentity, Error> {
    let mut connection = pool.acquire().await?;
    require_current_schema(&mut connection, expected).await
}

async fn validate_metadata_table(connection: &mut SqliteConnection) -> Result<(), Error> {
    let ddl: Option<String> = sqlx::query_scalar(
        "SELECT sql FROM sqlite_schema WHERE type='table' AND name='product_metadata'",
    )
    .fetch_optional(&mut *connection)
    .await?;
    let ddl = ddl.ok_or(Error::ProductMetadataTableMissing)?;
    validate_product_metadata_ddl(&ddl)?;

    let columns = sqlx::query(
        "SELECT cid, name, type, \"notnull\", dflt_value, pk \
         FROM pragma_table_info('product_metadata') ORDER BY cid",
    )
    .fetch_all(&mut *connection)
    .await?
    .into_iter()
    .map(|row| {
        Ok(ProductMetadataColumn {
            cid: row.try_get(0)?,
            name: row.try_get(1)?,
            declared_type: row.try_get(2)?,
            not_null: row.try_get(3)?,
            default_sql: row.try_get(4)?,
            primary_key_position: row.try_get(5)?,
        })
    })
    .collect::<Result<Vec<_>, sqlx::Error>>()?;
    validate_product_metadata_columns(&columns)?;
    Ok(())
}

async fn read_metadata_rows(
    connection: &mut SqliteConnection,
) -> Result<Vec<ProductMetadataRow>, Error> {
    let rows = sqlx::query(
        "SELECT typeof(singleton), singleton, typeof(application), application, \
                typeof(application_version), application_version, \
                typeof(schema_revision), schema_revision, \
                typeof(schema_sha256), schema_sha256 \
         FROM product_metadata ORDER BY singleton",
    )
    .fetch_all(connection)
    .await?;

    let mut metadata = Vec::with_capacity(rows.len());
    for row in rows {
        for (field, index, expected) in [
            ("singleton", 0, "integer"),
            ("application", 2, "text"),
            ("application_version", 4, "text"),
            ("schema_revision", 6, "integer"),
            ("schema_sha256", 8, "text"),
        ] {
            let actual: String = row.try_get(index)?;
            if actual != expected {
                return Err(Error::ProductMetadataStorageClass {
                    field,
                    expected,
                    actual,
                });
            }
        }
        metadata.push(ProductMetadataRow {
            singleton: row.try_get(1)?,
            application: row.try_get(3)?,
            application_version: row.try_get(5)?,
            schema_revision: row.try_get(7)?,
            schema_sha256: row.try_get(9)?,
        });
    }
    Ok(metadata)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Error)]
pub enum PoolOptionsError {
    #[error("max_connections must be greater than zero")]
    ZeroMaxConnections,
    #[error(
        "min_connections ({min_connections}) must not exceed max_connections ({max_connections})"
    )]
    MinConnectionsExceedMax {
        min_connections: u32,
        max_connections: u32,
    },
    #[error("acquire_timeout must be greater than zero")]
    ZeroAcquireTimeout,
}

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    InvalidPoolOptions(#[from] PoolOptionsError),
    #[error("SQLite database does not exist: {path}", path = .path.display())]
    DatabaseDoesNotExist { path: PathBuf },
    #[error("cannot inspect SQLite database {path}: {source}", path = .path.display())]
    InspectDatabase { path: PathBuf, source: io::Error },
    #[error("SQLite operation failed: {0}")]
    Sqlx(#[from] sqlx::Error),
    #[error("SQLite integrity check failed: {messages:?}")]
    IntegrityCheckFailed { messages: Vec<String> },
    #[error("SQLite foreign-key check found violations: {violations:?}")]
    ForeignKeyViolations {
        violations: Vec<ForeignKeyViolation>,
    },
    #[error(
        "SQLite WAL checkpoint is busy ({checkpointed_frames}/{log_frames} frames checkpointed)"
    )]
    CheckpointBusy {
        log_frames: i64,
        checkpointed_frames: i64,
    },
    #[error(
        "SQLite WAL checkpoint was incomplete ({checkpointed_frames}/{log_frames} frames checkpointed)"
    )]
    CheckpointIncomplete {
        log_frames: i64,
        checkpointed_frames: i64,
    },
    #[error("database has no product_metadata table")]
    ProductMetadataTableMissing,
    #[error("product_metadata {field} must use SQLite storage class {expected}, found {actual}")]
    ProductMetadataStorageClass {
        field: &'static str,
        expected: &'static str,
        actual: String,
    },
    #[error(transparent)]
    SchemaIdentity(#[from] SchemaIdentityError),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_identity(schema_sha256: String) -> Result<SchemaIdentity, SchemaIdentityError> {
        SchemaIdentity::new("test-product", "0.3.0", 1, schema_sha256)
    }

    #[tokio::test]
    async fn explicit_creation_applies_pragmas_to_every_connection_and_reopens()
    -> Result<(), Box<dyn std::error::Error>> {
        let directory = tempfile::tempdir()?;
        let database_path = directory.path().join("foundation.sqlite3");
        let options = PoolOptions::new(2).with_min_connections(2);
        let pool = create_if_missing(&database_path, options.clone()).await?;

        let mut first = pool.acquire().await?;
        let mut second = pool.acquire().await?;
        for connection in [&mut first, &mut second] {
            let foreign_keys: i64 = sqlx::query_scalar("PRAGMA foreign_keys")
                .fetch_one(&mut **connection)
                .await?;
            let journal_mode: String = sqlx::query_scalar("PRAGMA journal_mode")
                .fetch_one(&mut **connection)
                .await?;
            let busy_timeout: i64 = sqlx::query_scalar("PRAGMA busy_timeout")
                .fetch_one(&mut **connection)
                .await?;
            let synchronous: i64 = sqlx::query_scalar("PRAGMA synchronous")
                .fetch_one(&mut **connection)
                .await?;
            assert_eq!(foreign_keys, 1);
            assert_eq!(journal_mode, "wal");
            assert_eq!(busy_timeout, 5_000);
            assert_eq!(synchronous, 2);
        }
        drop(first);
        drop(second);

        sqlx::query("CREATE TABLE parents(id INTEGER PRIMARY KEY)")
            .execute(&pool)
            .await?;
        sqlx::query(
            "CREATE TABLE children(\
                 id INTEGER PRIMARY KEY,\
                 parent_id INTEGER NOT NULL REFERENCES parents(id)\
             )",
        )
        .execute(&pool)
        .await?;
        assert!(
            sqlx::query("INSERT INTO children(id,parent_id) VALUES(1,999)")
                .execute(&pool)
                .await
                .is_err()
        );
        sqlx::query("INSERT INTO parents(id) VALUES(7)")
            .execute(&pool)
            .await?;
        integrity_check(&pool).await?;
        foreign_key_check(&pool).await?;
        checkpoint(&pool).await?;
        pool.close().await;

        let reopened = open_existing(&database_path, options).await?;
        let parent: i64 = sqlx::query_scalar("SELECT id FROM parents")
            .fetch_one(&reopened)
            .await?;
        assert_eq!(parent, 7);
        integrity_check(&reopened).await?;
        foreign_key_check(&reopened).await?;
        Ok(())
    }

    #[tokio::test]
    async fn missing_existing_and_invalid_options_are_typed_and_do_not_create()
    -> Result<(), Box<dyn std::error::Error>> {
        let directory = tempfile::tempdir()?;
        let missing = directory.path().join("missing.sqlite3");
        assert!(matches!(
            open_existing(&missing, PoolOptions::default()).await,
            Err(Error::DatabaseDoesNotExist { .. })
        ));
        assert!(!missing.exists());
        assert!(matches!(
            create_if_missing(&missing, PoolOptions::new(0)).await,
            Err(Error::InvalidPoolOptions(
                PoolOptionsError::ZeroMaxConnections
            ))
        ));
        assert!(!missing.exists());
        assert!(matches!(
            PoolOptions::new(1).with_min_connections(2).validate(),
            Err(PoolOptionsError::MinConnectionsExceedMax { .. })
        ));
        Ok(())
    }

    #[tokio::test]
    async fn sqlx_adapter_verifies_metadata_shape_values_and_fingerprint()
    -> Result<(), Box<dyn std::error::Error>> {
        let directory = tempfile::tempdir()?;
        let database_path = directory.path().join("identity.sqlite3");
        let pool = create_if_missing(&database_path, PoolOptions::new(2)).await?;
        sqlx::raw_sql(PRODUCT_METADATA_DDL).execute(&pool).await?;
        sqlx::query("CREATE TABLE items(id INTEGER PRIMARY KEY, value TEXT NOT NULL)")
            .execute(&pool)
            .await?;
        let fingerprint = schema_fingerprint(&pool).await?;
        let expected = test_identity(fingerprint.clone())?;
        sqlx::query(
            "INSERT INTO product_metadata(\
               singleton, application, application_version, schema_revision, schema_sha256\
             ) VALUES(1, ?, ?, ?, ?)",
        )
        .bind(&expected.application)
        .bind(&expected.application_version)
        .bind(i64::try_from(expected.schema_revision)?)
        .bind(&expected.schema_sha256)
        .execute(&pool)
        .await?;

        assert_eq!(read_pool_schema_identity(&pool).await?, expected);
        assert_eq!(
            require_pool_current_schema(&pool, &expected).await?,
            expected
        );

        sqlx::query("UPDATE product_metadata SET application_version='0.2.0'")
            .execute(&pool)
            .await?;
        assert!(matches!(
            require_pool_current_schema(&pool, &expected).await,
            Err(Error::SchemaIdentity(
                SchemaIdentityError::IdentityMismatch {
                    field: IdentityField::ApplicationVersion,
                    ..
                }
            ))
        ));
        sqlx::query("UPDATE product_metadata SET application_version='0.3.0'")
            .execute(&pool)
            .await?;
        sqlx::query("CREATE TABLE drift(id INTEGER PRIMARY KEY)")
            .execute(&pool)
            .await?;
        assert!(matches!(
            read_pool_schema_identity(&pool).await,
            Err(Error::SchemaIdentity(
                SchemaIdentityError::SchemaFingerprintMismatch { .. }
            ))
        ));
        Ok(())
    }

    #[tokio::test]
    async fn foreign_key_check_reports_the_violating_rows() -> Result<(), Box<dyn std::error::Error>>
    {
        let directory = tempfile::tempdir()?;
        let database_path = directory.path().join("foreign-keys.sqlite3");
        let pool = create_if_missing(&database_path, PoolOptions::new(1)).await?;
        sqlx::raw_sql(
            "CREATE TABLE parents(id INTEGER PRIMARY KEY);\
             CREATE TABLE children(\
               id INTEGER PRIMARY KEY,\
               parent_id INTEGER NOT NULL REFERENCES parents(id)\
             );",
        )
        .execute(&pool)
        .await?;
        let mut connection = pool.acquire().await?;
        sqlx::query("PRAGMA foreign_keys=OFF")
            .execute(&mut *connection)
            .await?;
        sqlx::query("INSERT INTO children(id,parent_id) VALUES(9,99)")
            .execute(&mut *connection)
            .await?;
        sqlx::query("PRAGMA foreign_keys=ON")
            .execute(&mut *connection)
            .await?;
        drop(connection);

        assert!(matches!(
            foreign_key_check(&pool).await,
            Err(Error::ForeignKeyViolations { violations })
                if violations == vec![ForeignKeyViolation {
                    table: "children".to_owned(),
                    row_id: Some(9),
                    parent: "parents".to_owned(),
                    foreign_key_index: 0,
                }]
        ));
        Ok(())
    }

    #[test]
    fn checkpoint_result_distinguishes_busy_and_incomplete() {
        assert!(matches!(
            validate_checkpoint_result((1, 8, 3)),
            Err(Error::CheckpointBusy {
                log_frames: 8,
                checkpointed_frames: 3
            })
        ));
        assert!(matches!(
            validate_checkpoint_result((0, 8, 3)),
            Err(Error::CheckpointIncomplete {
                log_frames: 8,
                checkpointed_frames: 3
            })
        ));
        assert!(validate_checkpoint_result((0, 8, 8)).is_ok());
    }

    #[tokio::test]
    async fn live_reader_makes_truncate_checkpoint_report_busy()
    -> Result<(), Box<dyn std::error::Error>> {
        let directory = tempfile::tempdir()?;
        let database_path = directory.path().join("busy.sqlite3");
        let pool = create_if_missing(&database_path, PoolOptions::new(3)).await?;
        sqlx::query("CREATE TABLE values_table(value INTEGER NOT NULL)")
            .execute(&pool)
            .await?;
        sqlx::query("INSERT INTO values_table(value) VALUES(1)")
            .execute(&pool)
            .await?;
        checkpoint(&pool).await?;

        let mut reader = pool.acquire().await?;
        sqlx::query("BEGIN").execute(&mut *reader).await?;
        let _: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM values_table")
            .fetch_one(&mut *reader)
            .await?;
        sqlx::query("INSERT INTO values_table(value) VALUES(2)")
            .execute(&pool)
            .await?;

        assert!(matches!(
            checkpoint(&pool).await,
            Err(Error::CheckpointBusy { .. })
        ));
        sqlx::query("ROLLBACK").execute(&mut *reader).await?;
        checkpoint(&pool).await?;
        Ok(())
    }
}
