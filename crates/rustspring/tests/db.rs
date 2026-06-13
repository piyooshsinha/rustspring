//! Transaction tests against a real Postgres. They run when `DATABASE_URL`
//! is set (CI provides a service container) and skip silently otherwise, so
//! `cargo test` works on machines without a database.

#![cfg(feature = "postgres")]

use rustspring::{db::build_pool, transactional, AppError};
use sqlx::{PgPool, Row};

async fn test_pool() -> Option<PgPool> {
    let url = std::env::var("DATABASE_URL").ok()?;
    // Reuse the framework's own pool construction path.
    let cfg = rustspring::config::DatabaseConfig {
        url,
        max_connections: 2,
    };
    let pool = build_pool(&cfg).expect("pool");
    Some(pool)
}

async fn count(pool: &PgPool, table: &str) -> i64 {
    sqlx::query(&format!("SELECT count(*) AS n FROM {table}"))
        .fetch_one(pool)
        .await
        .expect("count")
        .get("n")
}

#[tokio::test]
async fn transaction_commits_on_ok() {
    let Some(pool) = test_pool().await else {
        eprintln!("skipping: DATABASE_URL not set");
        return;
    };
    sqlx::query("DROP TABLE IF EXISTS tx_commit_test")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("CREATE TABLE tx_commit_test (id BIGSERIAL PRIMARY KEY, v TEXT)")
        .execute(&pool)
        .await
        .unwrap();

    transactional(&pool, |tx| {
        Box::pin(async move {
            sqlx::query("INSERT INTO tx_commit_test (v) VALUES ('a')")
                .execute(&mut **tx)
                .await?;
            sqlx::query("INSERT INTO tx_commit_test (v) VALUES ('b')")
                .execute(&mut **tx)
                .await?;
            Ok::<_, AppError>(())
        })
    })
    .await
    .expect("transaction should commit");

    assert_eq!(count(&pool, "tx_commit_test").await, 2);
}

#[tokio::test]
async fn transaction_rolls_back_on_err() {
    let Some(pool) = test_pool().await else {
        eprintln!("skipping: DATABASE_URL not set");
        return;
    };
    sqlx::query("DROP TABLE IF EXISTS tx_rollback_test")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("CREATE TABLE tx_rollback_test (id BIGSERIAL PRIMARY KEY, v TEXT)")
        .execute(&pool)
        .await
        .unwrap();

    let result = transactional(&pool, |tx| {
        Box::pin(async move {
            sqlx::query("INSERT INTO tx_rollback_test (v) VALUES ('a')")
                .execute(&mut **tx)
                .await?;
            // Second statement fails (no such table) -> whole tx rolls back.
            sqlx::query("INSERT INTO no_such_table (v) VALUES ('b')")
                .execute(&mut **tx)
                .await?;
            Ok::<_, AppError>(())
        })
    })
    .await;

    assert!(result.is_err());
    assert_eq!(
        count(&pool, "tx_rollback_test").await,
        0,
        "first insert must be rolled back"
    );
}
