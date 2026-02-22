//! Static file serving for the React frontend

use axum::{
    extract::Request,
    http::{HeaderMap, StatusCode, Uri, header},
    response::{IntoResponse, Response},
};
use std::path::{Component, Path};

/// Path to the frontend build directory
const FRONTEND_DIST: &str = "frontend-web/dist";

/// Serves static files from the frontend build directory.
/// Falls back to index.html for SPA routing.
pub async fn serve_static_files(uri: Uri, _request: Request) -> Response {
    let requested_path = uri.path();

    // API routes and WebSocket should return 404 since they're handled elsewhere
    if requested_path.starts_with("/api") || requested_path.starts_with("/ws") {
        return StatusCode::NOT_FOUND.into_response();
    }

    let dist_root = match tokio::fs::canonicalize(FRONTEND_DIST).await {
        Ok(path) => path,
        Err(_) => return StatusCode::NOT_FOUND.into_response(),
    };

    let relative_path = requested_path.trim_start_matches('/');
    let relative = Path::new(relative_path);
    if relative
        .components()
        .any(|component| matches!(component, Component::ParentDir))
    {
        return StatusCode::NOT_FOUND.into_response();
    }

    // Build the full path to the requested file
    let full_path = dist_root.join(relative);

    // Check if the requested file exists
    let canonical_file = tokio::fs::canonicalize(&full_path).await.ok();
    let is_file = canonical_file
        .as_ref()
        .map(|resolved| resolved.starts_with(&dist_root))
        .unwrap_or(false)
        && tokio::fs::metadata(&full_path)
            .await
            .map(|m| m.is_file())
            .unwrap_or(false);

    if is_file {
        // File exists - determine mime type and serve
        let safe_path = match canonical_file {
            Some(path) => path,
            None => return StatusCode::NOT_FOUND.into_response(),
        };

        let mime = mime_guess::from_path(&safe_path)
            .first_or_octet_stream()
            .to_string();

        match tokio::fs::read(&safe_path).await {
            Ok(contents) => {
                let mut headers = HeaderMap::new();
                headers.insert(header::CONTENT_TYPE, mime.parse().unwrap());
                let cache_value = if requested_path.starts_with("/assets/") {
                    "public, max-age=31536000, immutable"
                } else {
                    "public, max-age=3600"
                };
                headers.insert(header::CACHE_CONTROL, cache_value.parse().unwrap());

                (headers, contents).into_response()
            }
            Err(_) => StatusCode::NOT_FOUND.into_response(),
        }
    } else {
        // File doesn't exist or is a directory - serve index.html for SPA routing
        // This allows client-side routing to work
        let index_path = dist_root.join("index.html");
        match tokio::fs::read(&index_path).await {
            Ok(contents) => {
                let mime = mime_guess::from_path("index.html")
                    .first_or_octet_stream()
                    .to_string();

                let mut headers = HeaderMap::new();
                headers.insert(header::CONTENT_TYPE, mime.parse().unwrap());
                headers.insert(header::CACHE_CONTROL, "no-cache".parse().unwrap());

                (headers, contents).into_response()
            }
            Err(_) => {
                // index.html not found - return 404
                StatusCode::NOT_FOUND.into_response()
            }
        }
    }
}
