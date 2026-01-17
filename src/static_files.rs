//! Static file serving for the React frontend

use axum::{
    extract::Request,
    http::{HeaderMap, StatusCode, Uri, header},
    response::{IntoResponse, Response},
};
use std::path::PathBuf;

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

    // Build the full path to the requested file
    let mut full_path = PathBuf::from(FRONTEND_DIST);
    full_path.push(requested_path.trim_start_matches('/'));

    // Check if the requested file exists
    let metadata = std::fs::metadata(&full_path);

    if metadata.is_ok() && metadata.unwrap().is_file() {
        // File exists - determine mime type and serve
        let mime = mime_guess::from_path(&full_path)
            .first_or_octet_stream()
            .to_string();

        match tokio::fs::read(&full_path).await {
            Ok(contents) => {
                let mut headers = HeaderMap::new();
                headers.insert(header::CONTENT_TYPE, mime.parse().unwrap());
                headers.insert(
                    header::CACHE_CONTROL,
                    "public, max-age=3600".parse().unwrap(),
                );

                (headers, contents).into_response()
            }
            Err(_) => StatusCode::NOT_FOUND.into_response(),
        }
    } else {
        // File doesn't exist or is a directory - serve index.html for SPA routing
        // This allows client-side routing to work
        let index_path = format!("{}/index.html", FRONTEND_DIST);
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
