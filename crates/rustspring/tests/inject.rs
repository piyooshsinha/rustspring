//! Integration tests: the `Inject` extractor and `AppError` responses
//! through a real axum router, exactly as the framework wires them.

use axum::{
    body::Body,
    http::{Request, StatusCode},
    routing::get,
    Extension, Json, Router,
};
use http_body_util::BodyExt;
use rustspring::{AppContext, AppError, Inject};
use tower::ServiceExt;

struct Greeter {
    name: &'static str,
}

async fn hello(Inject(g): Inject<Greeter>) -> String {
    format!("hello, {}", g.name)
}

async fn boom() -> Result<Json<()>, AppError> {
    Err(AppError::NotFound("no such thing".into()))
}

fn app(ctx: AppContext) -> Router {
    Router::new()
        .route("/hello", get(hello))
        .route("/boom", get(boom))
        .layer(Extension(ctx))
}

async fn get_response(router: Router, uri: &str) -> (StatusCode, String) {
    let response = router
        .oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let body = response.into_body().collect().await.unwrap().to_bytes();
    (status, String::from_utf8(body.to_vec()).unwrap())
}

#[tokio::test]
async fn inject_resolves_managed_component() {
    let mut ctx = AppContext::default();
    ctx.register(Greeter { name: "rustspring" });

    let (status, body) = get_response(app(ctx), "/hello").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, "hello, rustspring");
}

#[tokio::test]
async fn inject_of_unregistered_component_names_the_type() {
    let (status, body) = get_response(app(AppContext::default()), "/hello").await;
    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
    assert!(
        body.contains("Greeter"),
        "error names the missing type: {body}"
    );
    assert!(body.contains(".manage("), "error tells you the fix: {body}");
}

#[tokio::test]
async fn app_error_maps_to_status_and_json() {
    let (status, body) = get_response(app(AppContext::default()), "/boom").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body, r#"{"error":"not found: no such thing"}"#);
}
