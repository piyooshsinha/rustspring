//! Database-backed routes showing constructor injection, the pool, and
//! transactions.
//!
//! `UserService` is a `#[derive(Component)]`: its `Arc<PgPool>` field is
//! resolved automatically at startup from the `[database]` config. Requires
//! a running Postgres; the pool connects lazily, so the app boots without
//! one — these endpoints just return errors until the database is reachable.
//!
//! Quick start:
//!   docker run -d -p 5432:5432 -e POSTGRES_PASSWORD=postgres postgres:16
//! The tables are created on first request.

use std::sync::Arc;

use rustspring::{
    axum::{
        routing::{get, post},
        Json, Router,
    },
    transactional, AppError, Component, Inject,
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

/// The framework constructs this at startup (`.component::<UserService>()`),
/// injecting the managed pool — no hand-wiring in main().
#[derive(Component)]
pub struct UserService {
    pool: Arc<PgPool>,
}

impl UserService {
    async fn list(&self) -> Result<Vec<User>, AppError> {
        self.ensure_schema().await?;
        let users = sqlx::query_as::<_, User>("SELECT id, name FROM users ORDER BY id")
            .fetch_all(&*self.pool)
            .await?;
        Ok(users)
    }

    /// Two inserts in one transaction: if the audit insert fails, the user
    /// insert rolls back too. This is the `@Transactional` pattern.
    async fn create(&self, name: String) -> Result<User, AppError> {
        self.ensure_schema().await?;
        transactional(&self.pool, |tx| {
            Box::pin(async move {
                let user = sqlx::query_as::<_, User>(
                    "INSERT INTO users (name) VALUES ($1) RETURNING id, name",
                )
                .bind(name)
                .fetch_one(&mut **tx)
                .await?;

                sqlx::query("INSERT INTO audit_log (action) VALUES ($1)")
                    .bind(format!("created user {}", user.id))
                    .execute(&mut **tx)
                    .await?;

                Ok::<_, AppError>(user)
            })
        })
        .await
    }

    async fn ensure_schema(&self) -> Result<(), AppError> {
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS users (
                id BIGSERIAL PRIMARY KEY,
                name TEXT NOT NULL
            )",
        )
        .execute(&*self.pool)
        .await?;
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS audit_log (
                id BIGSERIAL PRIMARY KEY,
                action TEXT NOT NULL,
                at TIMESTAMPTZ NOT NULL DEFAULT now()
            )",
        )
        .execute(&*self.pool)
        .await?;
        Ok(())
    }
}

pub fn routes() -> Router {
    Router::new()
        .route("/api/users", get(list_users))
        .route("/api/users", post(create_user))
}

async fn list_users(Inject(service): Inject<UserService>) -> Result<Json<Vec<User>>, AppError> {
    Ok(Json(service.list().await?))
}

async fn create_user(
    Inject(service): Inject<UserService>,
    Json(body): Json<CreateUser>,
) -> Result<Json<User>, AppError> {
    if body.name.trim().is_empty() {
        return Err(AppError::BadRequest("name must not be empty".into()));
    }
    Ok(Json(service.create(body.name.trim().to_string()).await?))
}
