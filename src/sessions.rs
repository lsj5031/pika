use crate::config::ProjectConfig;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use thiserror::Error;
use uuid::Uuid;

/// Session information extracted from session.jsonl files
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionInfo {
    /// Unique session identifier
    pub id: String,
    /// Path to the project containing this session
    pub project_path: PathBuf,
    /// Human-readable session name
    pub name: String,
    /// Session creation timestamp
    pub created_at: String,
    /// Whether the session is currently active
    pub is_active: bool,
}

/// Message in a session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMessage {
    /// Message role ("user" or "assistant")
    pub role: String,
    /// Message content
    pub content: String,
    /// Message timestamp (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<String>,
}

/// Raw session entry from session.jsonl file
#[derive(Debug, Serialize, Deserialize)]
struct SessionEntry {
    #[serde(rename = "sessionId")]
    session_id: String,
    #[serde(default)]
    name: String,
    #[serde(default)]
    timestamp: String,
}

/// Session discovery errors
#[derive(Debug, Error)]
pub enum SessionError {
    #[error("Failed to read session file {path}: {source}")]
    IoError {
        path: PathBuf,
        source: std::io::Error,
    },
    #[allow(dead_code)]
    #[error("Failed to parse session file {path}: {source}")]
    ParseError {
        path: PathBuf,
        source: serde_json::Error,
    },
}

/// Scan for pi sessions in configured project directories
pub fn scan_sessions(config: &ProjectConfig) -> Vec<SessionInfo> {
    let mut sessions = Vec::new();

    for project_path in &config.project_root_paths {
        // Scan each project root for sessions
        if let Ok(project_sessions) = scan_project_sessions(project_path) {
            sessions.extend(project_sessions);
        }
        // If scan fails (e.g., directory doesn't exist), we skip it gracefully
    }

    sessions
}

/// Scan a single project directory for sessions
fn scan_project_sessions(project_path: &Path) -> Result<Vec<SessionInfo>, SessionError> {
    let mut sessions = Vec::new();

    // Construct path to .pi/agent/sessions/
    let sessions_dir = project_path.join(".pi").join("agent").join("sessions");

    // If sessions directory doesn't exist, return empty list (not an error)
    if !sessions_dir.exists() {
        return Ok(sessions);
    }

    // Look for session.jsonl file
    let session_file = sessions_dir.join("session.jsonl");
    if !session_file.exists() {
        return Ok(sessions);
    }

    // Parse the session.jsonl file
    let file = fs::File::open(&session_file)
        .map_err(|e| SessionError::IoError {
            path: session_file.clone(),
            source: e,
        })?;

    let reader = BufReader::new(file);

    // JSONL format: one JSON object per line
    for line in reader.lines() {
        let line = line.map_err(|e| SessionError::IoError {
            path: session_file.clone(),
            source: e,
        })?;

        // Skip empty lines
        if line.trim().is_empty() {
            continue;
        }

        // Parse each line as a session entry
        if let Ok(entry) = serde_json::from_str::<SessionEntry>(&line) {
            sessions.push(SessionInfo {
                id: entry.session_id.clone(),
                project_path: project_path.to_path_buf(),
                name: if entry.name.is_empty() {
                    entry.session_id.clone()
                } else {
                    entry.name
                },
                created_at: if entry.timestamp.is_empty() {
                    "Unknown".to_string()
                } else {
                    entry.timestamp
                },
                is_active: false, // Will be determined by checking active session marker
            });
        }
        // If parsing fails, skip that line (graceful degradation)
    }

    Ok(sessions)
}

/// Get messages for a specific session
pub fn get_session_messages(_session_id: &str, project_path: &Path) -> Result<Vec<SessionMessage>, SessionError> {
    // Construct path to .pi/agent/sessions/
    let sessions_dir = project_path.join(".pi").join("agent").join("sessions");

    // If sessions directory doesn't exist, return empty list
    if !sessions_dir.exists() {
        return Ok(Vec::new());
    }

    // Look for session.jsonl file
    let session_file = sessions_dir.join("session.jsonl");
    if !session_file.exists() {
        return Ok(Vec::new());
    }

    // Parse the session.jsonl file
    let file = fs::File::open(&session_file)
        .map_err(|e| SessionError::IoError {
            path: session_file.clone(),
            source: e,
        })?;

    let reader = BufReader::new(file);
    let mut messages = Vec::new();

    // JSONL format: one JSON object per line
    // For messages, we'll parse each line as a potential message
    for line in reader.lines() {
        let line = line.map_err(|e| SessionError::IoError {
            path: session_file.clone(),
            source: e,
        })?;

        // Skip empty lines
        if line.trim().is_empty() {
            continue;
        }

        // Try to parse as a message
        // Messages typically have role and content fields
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(&line)
            && let Some(obj) = value.as_object()
            && obj.contains_key("role") && obj.contains_key("content")
            && let Ok(message) = serde_json::from_value::<SessionMessage>(value)
        {
            messages.push(message);
        }
        // If parsing fails, skip that line (graceful degradation)
    }

    Ok(messages)
}

/// Request to create a new session
#[derive(Debug, Deserialize)]
pub struct CreateSessionRequest {
    /// Optional session name (defaults to timestamp if not provided)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

/// Response when creating a new session
#[derive(Debug, Serialize)]
pub struct CreateSessionResponse {
    /// The newly created session ID
    pub session_id: String,
    /// The session name
    pub name: String,
    /// The project path where the session was created
    pub project_path: PathBuf,
    /// The session creation timestamp
    pub created_at: String,
}

/// Create a new session in the specified project
pub fn create_session(
    project_path: &Path,
    request: CreateSessionRequest,
) -> Result<CreateSessionResponse, SessionError> {
    // Validate project path exists
    if !project_path.exists() {
        return Err(SessionError::IoError {
            path: project_path.to_path_buf(),
            source: std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "Project path does not exist",
            ),
        });
    }

    // Generate a unique session ID
    let session_id = Uuid::new_v4().to_string();

    // Generate session name (use provided name or default to timestamp)
    let name = request.name.unwrap_or_else(|| {
        // Default name: timestamp
        use std::time::{SystemTime, UNIX_EPOCH};
        let duration = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap();
        format!("Session {}", duration.as_secs())
    });

    // Generate timestamp
    let created_at = {
        use std::time::{SystemTime, UNIX_EPOCH};
        let duration = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap();
        format!("{:.3?}", duration)
    };

    // Construct path to .pi/agent/sessions/
    let sessions_dir = project_path.join(".pi").join("agent").join("sessions");

    // Create sessions directory if it doesn't exist
    fs::create_dir_all(&sessions_dir)
        .map_err(|e| SessionError::IoError {
            path: sessions_dir.clone(),
            source: e,
        })?;

    // Create or append to session.jsonl file
    let session_file = sessions_dir.join("session.jsonl");
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&session_file)
        .map_err(|e| SessionError::IoError {
            path: session_file.clone(),
            source: e,
        })?;

    // Write the session entry as JSONL
    let entry = SessionEntry {
        session_id: session_id.clone(),
        name: name.clone(),
        timestamp: created_at.clone(),
    };

    let json_line = serde_json::to_string(&entry)
        .map_err(|e| SessionError::ParseError {
            path: session_file.clone(),
            source: e,
        })?;

    writeln!(file, "{}", json_line)
        .map_err(|e| SessionError::IoError {
            path: session_file.clone(),
            source: e,
        })?;

    // Flush to ensure data is written
    file.flush()
        .map_err(|e| SessionError::IoError {
            path: session_file.clone(),
            source: e,
        })?;

    Ok(CreateSessionResponse {
        session_id,
        name,
        project_path: project_path.to_path_buf(),
        created_at,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scan_sessions_empty_config() {
        let config = ProjectConfig::default();
        let sessions = scan_sessions(&config);
        assert!(sessions.is_empty());
    }

    #[test]
    fn test_session_info_serialization() {
        let info = SessionInfo {
            id: "test-session-123".to_string(),
            project_path: PathBuf::from("/test/project"),
            name: "Test Session".to_string(),
            created_at: "2025-01-13T00:00:00Z".to_string(),
            is_active: false,
        };

        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains("test-session-123"));
        assert!(json.contains("Test Session"));
    }
}
