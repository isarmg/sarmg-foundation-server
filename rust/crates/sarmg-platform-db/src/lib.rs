//! Current-only lifecycle and metadata checks for a Sarmg platform database.
//!
//! Historical DDL and migration remain the responsibility of `sarmg-upgrade`.
//! This crate opens only an existing database whose complete schema identity
//! and platform component are exactly what the running product compiled for.

use sarmg_schema_identity::SchemaIdentity;
use sarmg_sqlite::PoolOptions;
use sarmg_state_file::{InstanceLock, PrivateStateDirectory, SecureStateFile};
use sqlx::{Row, SqlitePool};
use thiserror::Error;

pub const PLATFORM_GENERATION: u32 = 1;
pub const PLATFORM_SCHEMA_REVISION: u32 = 1;
pub const PLATFORM_METADATA_DDL: &str = include_str!("../../../../schemas/platform/v1.sql");

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProductDescriptor {
    product_id: String,
    profile: String,
    database_file: String,
}

impl ProductDescriptor {
    pub fn new(
        product_id: impl Into<String>,
        profile: impl Into<String>,
        database_file: impl Into<String>,
    ) -> Result<Self, Error> {
        let descriptor = Self {
            product_id: product_id.into(),
            profile: profile.into(),
            database_file: database_file.into(),
        };
        require_product_id(&descriptor.product_id)?;
        if descriptor.profile != "server-control-plane" {
            return Err(Error::UnsupportedProfile {
                actual: descriptor.profile,
            });
        }
        if descriptor.database_file.is_empty() {
            return Err(Error::InvalidDatabaseFile);
        }
        Ok(descriptor)
    }

    pub fn product_id(&self) -> &str {
        &self.product_id
    }

    pub fn profile(&self) -> &str {
        &self.profile
    }

    pub fn database_file(&self) -> &str {
        &self.database_file
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlatformMetadata {
    pub platform_generation: u32,
    pub platform_schema_revision: u32,
    pub profile: String,
    pub created_at_micros: u64,
}

#[derive(Debug)]
pub struct PlatformDatabase {
    pool: SqlitePool,
    metadata: PlatformMetadata,
    descriptor: ProductDescriptor,
    _secure_file: SecureStateFile,
    _instance_lock: InstanceLock,
}

impl PlatformDatabase {
    pub async fn open(
        state_directory: PrivateStateDirectory,
        product_descriptor: ProductDescriptor,
        expected_schema: SchemaIdentity,
    ) -> Result<Self, Error> {
        Self::open_with_options(
            state_directory,
            product_descriptor,
            expected_schema,
            PoolOptions::default(),
        )
        .await
    }

    pub async fn open_with_options(
        state_directory: PrivateStateDirectory,
        product_descriptor: ProductDescriptor,
        expected_schema: SchemaIdentity,
        pool_options: PoolOptions,
    ) -> Result<Self, Error> {
        if expected_schema.application != product_descriptor.product_id {
            return Err(Error::ProductIdentityMismatch {
                descriptor: product_descriptor.product_id.clone(),
                schema: expected_schema.application,
            });
        }
        state_directory.verify_identity()?;
        let instance_lock = state_directory.try_instance_lock()?;
        let secure_file = state_directory.open_existing(&product_descriptor.database_file)?;
        let pool = sarmg_sqlite::open_existing(secure_file.path(), pool_options).await?;
        secure_file.verify_identity()?;
        sarmg_sqlite::require_pool_current_schema(&pool, &expected_schema).await?;
        let metadata = read_platform_metadata(&pool).await?;
        require_current_platform(&metadata, &product_descriptor)?;
        secure_file.verify_identity()?;
        Ok(Self {
            pool,
            metadata,
            descriptor: product_descriptor,
            _secure_file: secure_file,
            _instance_lock: instance_lock,
        })
    }

    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    pub fn metadata(&self) -> &PlatformMetadata {
        &self.metadata
    }

    pub fn descriptor(&self) -> &ProductDescriptor {
        &self.descriptor
    }

    pub async fn close(self) {
        self.pool.close().await;
    }
}

pub async fn read_platform_metadata(pool: &SqlitePool) -> Result<PlatformMetadata, Error> {
    validate_platform_table(pool).await?;
    let rows = sqlx::query(
        "SELECT typeof(singleton), singleton, \
                typeof(platform_generation), platform_generation, \
                typeof(platform_schema_revision), platform_schema_revision, \
                typeof(profile), profile, \
                typeof(created_at_micros), created_at_micros \
         FROM _sarmg_platform_metadata ORDER BY singleton",
    )
    .fetch_all(pool)
    .await?;
    if rows.len() != 1 {
        return Err(Error::PlatformMetadataRowCount { actual: rows.len() });
    }
    let row = &rows[0];
    for (field, index, expected) in [
        ("singleton", 0, "integer"),
        ("platform_generation", 2, "integer"),
        ("platform_schema_revision", 4, "integer"),
        ("profile", 6, "text"),
        ("created_at_micros", 8, "integer"),
    ] {
        let actual: String = row.try_get(index)?;
        if actual != expected {
            return Err(Error::PlatformMetadataStorageClass {
                field,
                expected,
                actual,
            });
        }
    }
    let singleton: i64 = row.try_get(1)?;
    if singleton != 1 {
        return Err(Error::InvalidPlatformSingleton { actual: singleton });
    }
    Ok(PlatformMetadata {
        platform_generation: positive_u32("platform_generation", row.try_get(3)?)?,
        platform_schema_revision: positive_u32("platform_schema_revision", row.try_get(5)?)?,
        profile: row.try_get(7)?,
        created_at_micros: nonnegative_u64("created_at_micros", row.try_get(9)?)?,
    })
}

async fn validate_platform_table(pool: &SqlitePool) -> Result<(), Error> {
    let ddl: Option<String> = sqlx::query_scalar(
        "SELECT sql FROM sqlite_schema WHERE type='table' AND name='_sarmg_platform_metadata'",
    )
    .fetch_optional(pool)
    .await?;
    let ddl = ddl.ok_or(Error::PlatformMetadataTableMissing)?;
    if normalize_sql(&ddl) != normalize_sql(PLATFORM_METADATA_DDL) {
        return Err(Error::PlatformMetadataDdlMismatch);
    }
    Ok(())
}

fn normalize_sql(sql: &str) -> String {
    sql.chars()
        .filter(|character| !character.is_ascii_whitespace() && *character != ';')
        .flat_map(char::to_lowercase)
        .collect()
}

fn require_current_platform(
    metadata: &PlatformMetadata,
    descriptor: &ProductDescriptor,
) -> Result<(), Error> {
    if metadata.platform_generation != PLATFORM_GENERATION {
        return Err(Error::PlatformGenerationMismatch {
            expected: PLATFORM_GENERATION,
            actual: metadata.platform_generation,
        });
    }
    if metadata.platform_schema_revision != PLATFORM_SCHEMA_REVISION {
        return Err(Error::PlatformSchemaRevisionMismatch {
            expected: PLATFORM_SCHEMA_REVISION,
            actual: metadata.platform_schema_revision,
        });
    }
    if metadata.profile != descriptor.profile {
        return Err(Error::PlatformProfileMismatch {
            expected: descriptor.profile.clone(),
            actual: metadata.profile.clone(),
        });
    }
    Ok(())
}

fn require_product_id(value: &str) -> Result<(), Error> {
    let bytes = value.as_bytes();
    if bytes.is_empty()
        || bytes.len() > 63
        || !bytes[0].is_ascii_lowercase()
        || !bytes
            .iter()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || *byte == b'-')
    {
        return Err(Error::InvalidProductId);
    }
    Ok(())
}

fn positive_u32(field: &'static str, value: i64) -> Result<u32, Error> {
    let value = u32::try_from(value).map_err(|_| Error::InvalidPlatformInteger { field, value })?;
    if value == 0 {
        Err(Error::InvalidPlatformInteger {
            field,
            value: i64::from(value),
        })
    } else {
        Ok(value)
    }
}

fn nonnegative_u64(field: &'static str, value: i64) -> Result<u64, Error> {
    u64::try_from(value).map_err(|_| Error::InvalidPlatformInteger { field, value })
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("product ID is not canonical")]
    InvalidProductId,
    #[error("platform database requires profile server-control-plane, found {actual}")]
    UnsupportedProfile { actual: String },
    #[error("database file name is empty")]
    InvalidDatabaseFile,
    #[error("descriptor product {descriptor} differs from schema application {schema}")]
    ProductIdentityMismatch { descriptor: String, schema: String },
    #[error(transparent)]
    StateFile(#[from] sarmg_state_file::Error),
    #[error(transparent)]
    Sqlite(#[from] sarmg_sqlite::Error),
    #[error("platform metadata table is missing")]
    PlatformMetadataTableMissing,
    #[error("platform metadata DDL differs from the current Foundation DDL")]
    PlatformMetadataDdlMismatch,
    #[error("platform metadata must contain exactly one row, found {actual}")]
    PlatformMetadataRowCount { actual: usize },
    #[error("platform metadata singleton must be 1, found {actual}")]
    InvalidPlatformSingleton { actual: i64 },
    #[error("platform metadata {field} must use storage class {expected}, found {actual}")]
    PlatformMetadataStorageClass {
        field: &'static str,
        expected: &'static str,
        actual: String,
    },
    #[error("platform metadata {field} has invalid integer {value}")]
    InvalidPlatformInteger { field: &'static str, value: i64 },
    #[error("platform generation must be {expected}, found {actual}")]
    PlatformGenerationMismatch { expected: u32, actual: u32 },
    #[error("platform schema revision must be {expected}, found {actual}")]
    PlatformSchemaRevisionMismatch { expected: u32, actual: u32 },
    #[error("platform profile must be {expected}, found {actual}")]
    PlatformProfileMismatch { expected: String, actual: String },
    #[error("SQLite platform query failed: {0}")]
    Sqlx(#[from] sqlx::Error),
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{fs, os::unix::fs::PermissionsExt};

    async fn current_database(
        profile: &str,
    ) -> Result<
        (tempfile::TempDir, PrivateStateDirectory, SchemaIdentity),
        Box<dyn std::error::Error>,
    > {
        let parent = tempfile::tempdir()?;
        fs::set_permissions(parent.path(), fs::Permissions::from_mode(0o700))?;
        let state = PrivateStateDirectory::create(parent.path().join("state"))?;
        state.create_file("state.sqlite3")?;
        let pool =
            sarmg_sqlite::open_existing(state.path().join("state.sqlite3"), PoolOptions::new(1))
                .await?;
        sqlx::raw_sql(sarmg_sqlite::PRODUCT_METADATA_DDL)
            .execute(&pool)
            .await?;
        sqlx::raw_sql(PLATFORM_METADATA_DDL).execute(&pool).await?;
        sqlx::query(
            "INSERT INTO _sarmg_platform_metadata(\
               singleton, platform_generation, platform_schema_revision, profile, created_at_micros\
             ) VALUES(1, 1, 1, ?, 7)",
        )
        .bind(profile)
        .execute(&pool)
        .await?;
        let fingerprint = sarmg_sqlite::schema_fingerprint(&pool).await?;
        let identity = SchemaIdentity::new("example-product", "0.5.0", 1, fingerprint)?;
        sqlx::query(
            "INSERT INTO product_metadata(\
               singleton, application, application_version, schema_revision, schema_sha256\
             ) VALUES(1, ?, ?, ?, ?)",
        )
        .bind(&identity.application)
        .bind(&identity.application_version)
        .bind(i64::try_from(identity.schema_revision)?)
        .bind(&identity.schema_sha256)
        .execute(&pool)
        .await?;
        pool.close().await;
        Ok((parent, state, identity))
    }

    #[tokio::test]
    async fn opens_only_the_exact_current_platform_database()
    -> Result<(), Box<dyn std::error::Error>> {
        let (_parent, state, identity) = current_database("server-control-plane").await?;
        let descriptor =
            ProductDescriptor::new("example-product", "server-control-plane", "state.sqlite3")?;
        let database = PlatformDatabase::open(state.clone(), descriptor, identity).await?;
        assert_eq!(database.metadata().created_at_micros, 7);
        assert!(matches!(
            PlatformDatabase::open(
                state,
                ProductDescriptor::new(
                    "example-product",
                    "server-control-plane",
                    "state.sqlite3",
                )?,
                SchemaIdentity::new("example-product", "9.9.9", 1, "0".repeat(64))?,
            )
            .await,
            Err(Error::StateFile(sarmg_state_file::Error::LockUnavailable { .. }))
        ));
        database.close().await;
        Ok(())
    }

    #[tokio::test]
    async fn rejects_wrong_platform_profile() -> Result<(), Box<dyn std::error::Error>> {
        let (_parent, state, identity) = current_database("server-filesystem").await?;
        let error = PlatformDatabase::open(
            state,
            ProductDescriptor::new("example-product", "server-control-plane", "state.sqlite3")?,
            identity,
        )
        .await
        .unwrap_err();
        assert!(matches!(error, Error::PlatformProfileMismatch { .. }));
        Ok(())
    }
}
