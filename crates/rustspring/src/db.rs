//! Database support: a managed connection pool (the HikariCP role) and a
//! closure-based transaction wrapper (the `@Transactional` role).
//!
//! Everything is generic over the backend — enable the `postgres`, `mysql`,
//! or `sqlite` feature and configure a matching `database.url`; the
//! framework registers the right pool type by URL scheme at startup
//! (`Inject<PgPool>`, `Inject<MySqlPool>`, or `Inject<SqlitePool>`).

use futures::future::BoxFuture;
use sqlx::{pool::PoolOptions, Database, Pool, Transaction};

use crate::config::DatabaseConfig;

/// Build a lazily-connecting pool from config. Lazy connection means the
/// application boots even if the database is down, and connects on first use.
pub fn build_pool<DB: Database>(cfg: &DatabaseConfig) -> Result<Pool<DB>, sqlx::Error> {
    PoolOptions::<DB>::new()
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
pub async fn transactional<DB, T, E, F>(pool: &Pool<DB>, f: F) -> Result<T, E>
where
    DB: Database,
    E: From<sqlx::Error>,
    F: for<'t> FnOnce(&'t mut Transaction<'static, DB>) -> BoxFuture<'t, Result<T, E>>,
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
