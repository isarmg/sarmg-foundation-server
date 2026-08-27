//! PostgreSQL connection, migration and readiness primitives without business SQL.

use std::time::Duration;

use sqlx::{PgPool, migrate::Migrator, postgres::PgPoolOptions};
use thiserror::Error;

#[derive(Debug, Clone)]
pub struct PostgresConfig<'a> {
    pub database_url: &'a str,
    pub max_connections: u32,
    pub acquire_timeout: Duration,
}

impl<'a> PostgresConfig<'a> {
    pub fn new(database_url: &'a str) -> Self {
        Self {
            database_url,
            max_connections: 20,
            acquire_timeout: Duration::from_secs(10),
        }
    }

    fn validate(&self) -> Result<(), PostgresSupportError> {
        if self.database_url.trim().is_empty() {
            return Err(PostgresSupportError::EmptyDatabaseUrl);
        }
        if self.max_connections == 0 {
            return Err(PostgresSupportError::ZeroConnections);
        }
        if self.acquire_timeout.is_zero() {
            return Err(PostgresSupportError::ZeroAcquireTimeout);
        }
        Ok(())
    }
}

#[derive(Debug, Error)]
pub enum PostgresSupportError {
    #[error("PostgreSQL database URL must not be empty")]
    EmptyDatabaseUrl,
    #[error("PostgreSQL max connections must be greater than zero")]
    ZeroConnections,
    #[error("PostgreSQL acquire timeout must be greater than zero")]
    ZeroAcquireTimeout,
    #[error("PostgreSQL connection failed")]
    Connect(#[source] sqlx::Error),
    #[error("PostgreSQL migration failed")]
    Migration(#[source] sqlx::migrate::MigrateError),
    #[error("PostgreSQL schema name is invalid: {0}")]
    InvalidSchema(String),
    #[error("PostgreSQL schema does not exist: {0}")]
    MissingSchema(String),
}

pub async fn connect(config: &PostgresConfig<'_>) -> Result<PgPool, PostgresSupportError> {
    config.validate()?;
    PgPoolOptions::new()
        .max_connections(config.max_connections)
        .acquire_timeout(config.acquire_timeout)
        .connect(config.database_url)
        .await
        .map_err(PostgresSupportError::Connect)
}

pub async fn run_migrations(
    pool: &PgPool,
    migrator: &Migrator,
) -> Result<(), PostgresSupportError> {
    migrator
        .run(pool)
        .await
        .map_err(PostgresSupportError::Migration)
}

/// Run a module-owned migrator with its own `_sqlx_migrations` table.
///
/// The deployment must provision the schema and grants first. This function deliberately does not
/// create schemas because a runtime module role should not have cluster-level DDL privileges.
pub async fn run_schema_migrations(
    pool: &PgPool,
    schema: &str,
    migrator: &Migrator,
) -> Result<(), PostgresSupportError> {
    if !valid_schema_name(schema) {
        return Err(PostgresSupportError::InvalidSchema(schema.to_string()));
    }
    let exists = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM information_schema.schemata WHERE schema_name = $1)",
    )
    .bind(schema)
    .fetch_one(pool)
    .await
    .map_err(PostgresSupportError::Connect)?;
    if !exists {
        return Err(PostgresSupportError::MissingSchema(schema.to_string()));
    }

    let mut connection = pool
        .acquire()
        .await
        .map_err(PostgresSupportError::Connect)?;
    // The identifier was restricted to lowercase ASCII, digits and underscores above. Keeping the
    // dynamic fragment here local and audited prevents a module name from becoming arbitrary SQL.
    sqlx::query(&format!("SET search_path TO {schema}, pg_catalog"))
        .execute(&mut *connection)
        .await
        .map_err(PostgresSupportError::Connect)?;
    migrator
        .run(&mut *connection)
        .await
        .map_err(PostgresSupportError::Migration)
}

pub async fn ready(pool: &PgPool) -> bool {
    sqlx::query_scalar::<_, i32>("SELECT 1")
        .fetch_one(pool)
        .await
        .is_ok()
}

fn valid_schema_name(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 63
        && value.starts_with(|character: char| character.is_ascii_lowercase())
        && value.chars().all(|character| {
            character.is_ascii_lowercase() || character.is_ascii_digit() || character == '_'
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalid_pool_limits_fail_before_network_access() {
        let mut config = PostgresConfig::new("postgresql://example.invalid/database");
        config.max_connections = 0;
        assert!(matches!(
            config.validate(),
            Err(PostgresSupportError::ZeroConnections)
        ));
        config.max_connections = 1;
        config.acquire_timeout = Duration::ZERO;
        assert!(matches!(
            config.validate(),
            Err(PostgresSupportError::ZeroAcquireTimeout)
        ));
    }

    #[test]
    fn schema_names_are_safe_to_use_in_a_search_path_statement() {
        for valid in ["core", "sunshine", "host_monitoring", "module2"] {
            assert!(valid_schema_name(valid));
        }
        for invalid in [
            "",
            "Core",
            "2module",
            "module-name",
            "public,pg_catalog",
            "a;drop schema core",
        ] {
            assert!(!valid_schema_name(invalid));
        }
    }
}
