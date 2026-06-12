//! Database support: a managed connection pool (the HikariCP role) and a
//! closure-based transaction wrapper (the `@Transactional` role).

use futures::future::BoxFuture;
use sqlx::{postgres::PgPoolOptions, PgPool, Postgres, Transaction};

use crate::config::DatabaseConfig;

/// Build a lazily-connecting pool from config. Lazy connection means the
/// application boots even if the database is down, and connects on first use.
pub fn build_pool(cfg: &DatabaseConfig) -> Result<PgPool, sqlx::Error> {
    PgPoolOptions::new()
        .max_connections(cfg.max_connections)
        .connect_lazy(&cfg.url)
}

/// Run a closure inside a transaction. Commits on `Ok`, rolls back on `Err`
/// — and also rolls back if the future is dropped mid-flight, because an
/// uncommitted `sqlx::Transaction` rolls back on drop. This is the
/// `@Transactional` of the framework, with rollback semantics you can't forget.
///
/// ```ignore
/// let user = transactional(&pool, |tx| Box::pin(async move {
///     let user = sqlx::query_as::<_, User>(
///         "INSERT INTO users (name) VALUES ($1) RETURNING *")
///         .bind(&name)
///         .fetch_one(&mut **tx)
///         .await?;
///     sqlx::query("INSERT INTO audit_log (action) VALUES ($1)")
///         .bind(format!("created user {}", user.id))
///         .execute(&mut **tx)
///         .await?;
///     Ok(user)
/// })).await?;
/// ```
pub async fn transactional<T, E, F>(pool: &PgPool, f: F) -> Result<T, E>
where
    E: From<sqlx::Error>,
    F: for<'t> FnOnce(&'t mut Transaction<'static, Postgres>) -> BoxFuture<'t, Result<T, E>>,
{
    let mut tx = pool.begin().await?;
    let result = f(&mut tx).await;
    match result {
        Ok(value) => {
            tx.commit().await?;
            Ok(value)
        }
        Err(err) => {
            // Explicit rollback so errors surface; drop would roll back anyway.
            let _ = tx.rollback().await;
            Err(err)
        }
    }
}
