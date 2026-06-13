//! A database-backed CRUD slice using SQLite — runs with zero setup.
//!
//! `ItemService` is a `#[derive(Component)]`: its `Arc<SqlitePool>` field is
//! resolved automatically at startup from the `[database]` config. Try it:
//!
//!   curl localhost:8080/api/items
//!   curl -X POST localhost:8080/api/items -H 'content-type: application/json' -d '{"title":"Buy milk"}'

use std::sync::Arc;

use rustspring::{
    axum::{
        routing::{get, post},
        Json, Router,
    },
    transactional, AppError, Component, Inject,
};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

#[derive(Serialize, sqlx::FromRow)]
struct Item {
    id: i64,
    title: String,
}

#[derive(Deserialize)]
struct NewItem {
    title: String,
}

/// Constructed at startup by `.component::<ItemService>()`, with the managed
/// pool injected — no hand-wiring in main().
#[derive(Component)]
pub struct ItemService {
    pool: Arc<SqlitePool>,
}

impl ItemService {
    async fn list(&self) -> Result<Vec<Item>, AppError> {
        self.ensure_schema().await?;
        let items = sqlx::query_as::<_, Item>("SELECT id, title FROM items ORDER BY id")
            .fetch_all(&*self.pool)
            .await?;
        Ok(items)
    }

    /// Insert the item and an audit row in one transaction: if either fails,
    /// both roll back. This is the `@Transactional` pattern.
    async fn create(&self, title: String) -> Result<Item, AppError> {
        self.ensure_schema().await?;
        transactional(&self.pool, |tx| {
            Box::pin(async move {
                let item = sqlx::query_as::<_, Item>(
                    "INSERT INTO items (title) VALUES (?) RETURNING id, title",
                )
                .bind(title)
                .fetch_one(&mut **tx)
                .await?;

                sqlx::query("INSERT INTO audit_log (action) VALUES (?)")
                    .bind(format!("created item {}", item.id))
                    .execute(&mut **tx)
                    .await?;

                Ok::<_, AppError>(item)
            })
        })
        .await
    }

    async fn ensure_schema(&self) -> Result<(), AppError> {
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS items (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                title TEXT NOT NULL
            )",
        )
        .execute(&*self.pool)
        .await?;
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS audit_log (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                action TEXT NOT NULL
            )",
        )
        .execute(&*self.pool)
        .await?;
        Ok(())
    }
}

pub fn routes() -> Router {
    Router::new()
        .route("/api/items", get(list_items))
        .route("/api/items", post(create_item))
}

async fn list_items(Inject(service): Inject<ItemService>) -> Result<Json<Vec<Item>>, AppError> {
    Ok(Json(service.list().await?))
}

async fn create_item(
    Inject(service): Inject<ItemService>,
    Json(body): Json<NewItem>,
) -> Result<Json<Item>, AppError> {
    if body.title.trim().is_empty() {
        return Err(AppError::BadRequest("title must not be empty".into()));
    }
    Ok(Json(service.create(body.title.trim().to_string()).await?))
}
