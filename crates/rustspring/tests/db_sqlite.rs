//! Transaction tests against SQLite — exercises the backend-generic
//! `transactional` with no database server. Run with:
//!   cargo test --features sqlite

#![cfg(feature = "sqlite")]

use rustspring::{
    config::DatabaseConfig,
    db::{build_pool, transactional},
    AppError,
};
use sqlx::{Row, Sqlite};

fn test_pool(name: &str) -> sqlx::Pool<Sqlite> {
    let path = std::env::temp_dir().join(format!("rustspring-test-{name}.db"));
    let _ = std::fs::remove_file(&path);
    let cfg = DatabaseConfig {
        // mode=rwc creates the file on first connect.
        url: format!("sqlite://{}?mode=rwc", path.display()),
        max_connections: 1,
    };
    build_pool(&cfg).expect("pool")
}

async fn count(pool: &sqlx::Pool<Sqlite>, table: &str) -> i64 {
    sqlx::query(&format!("SELECT count(*) AS n FROM {table}"))
        .fetch_one(pool)
        .await
        .expect("count")
        .get("n")
}

#[tokio::test]
async fn transaction_commits_on_ok() {
    let pool = test_pool("commit");
    sqlx::query("CREATE TABLE items (id INTEGER PRIMARY KEY, v TEXT)")
        .execute(&pool)
        .await
        .unwrap();

    transactional(&pool, |tx| {
        Box::pin(async move {
            sqlx::query("INSERT INTO items (v) VALUES ('a')")
                .execute(&mut **tx)
                .await?;
            sqlx::query("INSERT INTO items (v) VALUES ('b')")
                .execute(&mut **tx)
                .await?;
            Ok::<_, AppError>(())
        })
    })
    .await
    .expect("transaction should commit");

    assert_eq!(count(&pool, "items").await, 2);
}

#[tokio::test]
async fn transaction_rolls_back_on_err() {
    let pool = test_pool("rollback");
    sqlx::query("CREATE TABLE items (id INTEGER PRIMARY KEY, v TEXT)")
        .execute(&pool)
        .await
        .unwrap();

    let result = transactional(&pool, |tx| {
        Box::pin(async move {
            sqlx::query("INSERT INTO items (v) VALUES ('a')")
                .execute(&mut **tx)
                .await?;
            // Fails (no such table) -> the whole transaction rolls back.
            sqlx::query("INSERT INTO no_such_table (v) VALUES ('b')")
                .execute(&mut **tx)
                .await?;
            Ok::<_, AppError>(())
        })
    })
    .await;

    assert!(result.is_err());
    assert_eq!(
        count(&pool, "items").await,
        0,
        "first insert must be rolled back"
    );
}
