use axum::{body::Body, http::StatusCode};
use http_body_util::BodyExt;
use serde_json::Value;
use tower::util::ServiceExt;

#[tokio::test]
async fn health_check_returns_200() {
    let app = pika::create_test_app().await;

    let response = app
        .oneshot(
            axum::http::Request::builder()
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["status"], "ok");
    assert_eq!(json["service"], "pika");
}

#[tokio::test]
async fn health_check_returns_valid_json() {
    let app = pika::create_test_app().await;

    let response = app
        .oneshot(
            axum::http::Request::builder()
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&body).unwrap();

    assert!(json.get("status").is_some());
    assert!(json.get("service").is_some());
    assert!(json.get("version").is_some());
}

#[tokio::test]
async fn static_files_route_works() {
    let app = pika::create_test_app().await;

    let response = app
        .oneshot(
            axum::http::Request::builder()
                .uri("/")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert!(response.status().is_success() || response.status().is_redirection());
}

#[tokio::test]
async fn auth_status_returns_shape() {
    let app = pika::create_test_app().await;

    let response = app
        .oneshot(
            axum::http::Request::builder()
                .uri("/api/auth/status")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert!(json.get("enabled").is_some());
    assert!(json.get("authenticated").is_some());
}

#[tokio::test]
async fn projects_endpoint_returns_array() {
    let app = pika::create_test_app().await;

    let response = app
        .oneshot(
            axum::http::Request::builder()
                .uri("/api/projects")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert!(json.is_array());
}

#[tokio::test]
async fn get_unknown_session_returns_404() {
    let app = pika::create_test_app().await;

    let response = app
        .oneshot(
            axum::http::Request::builder()
                .uri("/api/sessions/does-not-exist")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["error"], "NOT_FOUND");
}

#[tokio::test]
async fn static_file_traversal_is_blocked() {
    let app = pika::create_test_app().await;

    let response = app
        .oneshot(
            axum::http::Request::builder()
                .uri("/../Cargo.toml")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}
