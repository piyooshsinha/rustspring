//! Database-backed routes showing the pool + transaction story.
//!
//! Requires `[database]` in config and a running Postgres. The pool connects
//! lazily, so the app boots fine without one — these endpoints just return
//! errors until the database is reachable.
//!
//! Quick start:
//!   docker run -d -p 5432:5432 -e POSTGRES_PASSWORD=postgres postgres:16
//! The table is created on first call to POST /api/users.

use rustspring::{
    axum::{
        routing::{get, post},
        Json, Router,
    },
    transactional, AppError, Inject,
};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

#[derive(Serialize, sqlx::FromRow)]
struct User {
    id: i64,
    name: String,
}

#[derive(Deserialize)]
struct CreateUser {
    name: String,
}

pub fn routes() -> Router {
    Router::new()
        .route("/api/users", get(list_users))
        .route("/api/users", post(create_user))
}

async fn list_users(Inject(pool): Inject<PgPool>) -> Result<Json<Vec<User>>, AppError> {
    ensure_schema(&pool).await?;
    let users = sqlx::query_as::<_, User>("SELECT id, name FROM users ORDER BY id")
        .fetch_all(&*pool)
        .await?;
    Ok(Json(users))
}

/// Two inserts in one transaction: if the audit insert fails, the user
/// insert rolls back too. This is the `@Transactional` pattern.
async fn create_user(
    Inject(pool): Inject<PgPool>,
    Json(body): Json<CreateUser>,
) -> Result<Json<User>, AppError> {
    if body.name.trim().is_empty() {
        return Err(AppError::BadRequest("name must not be empty".into()));
    }
    ensure_schema(&pool).await?;

    let user = transactional(&pool, |tx| {
        Box::pin(async move {
            let user = sqlx::query_as::<_, User>(
                "INSERT INTO users (name) VALUES ($1) RETURNING id, name",
            )
            .bind(body.name.trim())
            .fetch_one(&mut **tx)
            .await?;

            sqlx::query("INSERT INTO audit_log (action) VALUES ($1)")
                .bind(format!("created user {}", user.id))
                .execute(&mut **tx)
                .await?;

            Ok::<_, AppError>(user)
        })
    })
    .await?;

    Ok(Json(user))
}

async fn ensure_schema(pool: &PgPool) -> Result<(), AppError> {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS users (
            id BIGSERIAL PRIMARY KEY,
            name TEXT NOT NULL
        )",
    )
    .execute(pool)
    .await?;
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS audit_log (
            id BIGSERIAL PRIMARY KEY,
            action TEXT NOT NULL,
            at TIMESTAMPTZ NOT NULL DEFAULT now()
        )",
    )
    .execute(pool)
    .await?;
    Ok(())
}
