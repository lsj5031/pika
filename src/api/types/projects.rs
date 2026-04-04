//! Project API types.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Project information response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectResponse {
    /// Unique project ID (based on path hash for now)
    pub id: String,
    /// Project root path
    pub path: PathBuf,
    /// Project name (extracted from path)
    pub name: String,
    /// Number of sessions found in this project
    pub session_count: usize,
}

/// Request to add a new project
#[derive(Debug, Deserialize)]
pub struct AddProjectRequest {
    /// Path to the project root
    pub path: String,
}

/// Response when adding a project
#[derive(Debug, Serialize)]
pub struct AddProjectResponse {
    /// The project ID
    pub id: String,
    /// The project name
    pub name: String,
    /// The project path
    pub path: PathBuf,
}