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

    // Get the pi sessions directory
    let pi_sessions_dir = dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".pi")
        .join("agent")
        .join("sessions");

    // If pi sessions directory doesn't exist, return empty list
    if !pi_sessions_dir.exists() {
        return sessions;
    }

    for project_path in &config.project_root_paths {
        // Encode the project path to match pi-coding-agent's naming convention
        // e.g., /home/leo/appifex/appifex -> --home-leo-appifex-appifex--
        let encoded_path = encode_project_path(project_path);
        let project_sessions_dir = pi_sessions_dir.join(&encoded_path);

        // Scan each project's sessions directory
        if let Ok(project_sessions) = scan_project_sessions(project_path, &project_sessions_dir) {
            sessions.extend(project_sessions);
        }
    }

    sessions
}

/// Encode a project path to match pi-coding-agent's directory naming convention
fn encode_project_path(path: &Path) -> String {
    // Convert path to string, remove leading /, replace / with -, wrap with --
    let path_str = path.to_string_lossy();
    let normalized = path_str
        .trim_start_matches('/')
        .replace('/', "-")
        .replace('\\', "-"); // Handle Windows paths too
    format!("--{}--", normalized)
}

/// Scan a single project directory for sessions
fn scan_project_sessions(project_path: &Path, sessions_dir: &Path) -> Result<Vec<SessionInfo>, SessionError> {
    let mut sessions = Vec::new();

    // If sessions directory doesn't exist, return empty list (not an error)
    if !sessions_dir.exists() {
        return Ok(sessions);
    }

    // Read all session files (*.jsonl)
    let entries = fs::read_dir(sessions_dir)
        .map_err(|e| SessionError::IoError {
            path: sessions_dir.to_path_buf(),
            source: e,
        })?;

    for entry in entries {
        let entry = entry.map_err(|e| SessionError::IoError {
            path: sessions_dir.to_path_buf(),
            source: e,
        })?;

        let path = entry.path();

        // Skip directories
        if path.is_dir() {
            continue;
        }

        // Only process .jsonl files
        if path.extension().and_then(|s| s.to_str()) != Some("jsonl") {
            continue;
        }

        // Extract session ID from filename
        // Format: 2025-12-19T23-02-19-917Z_0e4ffe0f-899b-4730-a576-73ee542d84b4.jsonl
        let file_name = path.file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("");

        // Split by last underscore to get UUID
        let parts: Vec<&str> = file_name.rsplitn(2, '_').collect();
        let session_id = if parts.len() == 2 {
            parts[0].to_string()
        } else {
            // Fallback: generate UUID from filename
            Uuid::new_v4().to_string()
        };

        // Extract timestamp from filename (first part before underscore)
        let timestamp = if parts.len() == 2 {
            parts[1].to_string()
        } else {
            "Unknown".to_string()
        };

        // Get file modification time as created_at
        let metadata = fs::metadata(&path)
            .map_err(|e| SessionError::IoError {
                path: path.clone(),
                source: e,
            })?;
        let modified = metadata.modified()
            .ok()
            .and_then(|t| {
                let datetime: chrono::DateTime<chrono::Utc> = t.into();
                Some(datetime.format("%Y-%m-%d %H:%M:%S").to_string())
            })
            .unwrap_or_else(|| timestamp.clone());

        sessions.push(SessionInfo {
            id: session_id,
            project_path: project_path.to_path_buf(),
            name: file_name.to_string(),
            created_at: modified,
            is_active: false,
        });
    }

    Ok(sessions)
}

/// Get messages for a specific session
pub fn get_session_messages(session_id: &str, project_path: &Path) -> Result<Vec<SessionMessage>, SessionError> {
    // Get the pi sessions directory
    let pi_sessions_dir = dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".pi")
        .join("agent")
        .join("sessions");

    // Encode the project path to match pi-coding-agent's naming convention
    let encoded_path = encode_project_path(project_path);
    let project_sessions_dir = pi_sessions_dir.join(&encoded_path);

    // If sessions directory doesn't exist, return empty list
    if !project_sessions_dir.exists() {
        return Ok(Vec::new());
    }

    // Find the session file with the given ID
    let session_file = fs::read_dir(&project_sessions_dir)
        .map_err(|e| SessionError::IoError {
            path: project_sessions_dir.clone(),
            source: e,
        })?
        .find_map(|entry| {
            let entry = entry.ok()?;
            let path = entry.path();
            let file_name = path.file_stem()?.to_str()?;
            // Check if the file name contains the session ID
            if file_name.contains(session_id) {
                Some(path)
            } else {
                None
            }
        });

    let session_file = match session_file {
        Some(file) => file,
        None => return Ok(Vec::new()),
    };

    // Parse the session.jsonl file
    let file = fs::File::open(&session_file)
        .map_err(|e| SessionError::IoError {
            path: session_file.clone(),
            source: e,
        })?;

    let reader = BufReader::new(file);
    let mut messages = Vec::new();

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

        // Try to parse as a pi-coding-agent session entry
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(&line) {
            // Only process entries with type "message"
            if value.get("type").and_then(|t| t.as_str()) != Some("message") {
                continue;
            }

            // Extract message data
            let message_obj = value.get("message").and_then(|m| m.as_object());
            if message_obj.is_none() {
                continue;
            }

            let message_obj = message_obj.unwrap();

            // Get role
            let role = message_obj.get("role")
                .and_then(|r| r.as_str())
                .unwrap_or("unknown")
                .to_string();

            // Get content from message.content array
            let content = if let Some(content_array) = message_obj.get("content").and_then(|c| c.as_array()) {
                // Concatenate all text parts
                content_array
                    .iter()
                    .filter_map(|part| {
                        part.get("text")
                            .and_then(|t| t.as_str())
                            .map(|s| s.to_string())
                    })
                    .collect::<Vec<_>>()
                    .join("\n")
            } else {
                continue;
            };

            // Get timestamp
            let timestamp = value.get("timestamp")
                .and_then(|t| t.as_str())
                .map(|s| s.to_string())
                .unwrap_or_else(|| {
                    // Fallback to message timestamp if available
                    message_obj.get("timestamp")
                        .and_then(|t| t.as_i64())
                        .map(|ts| {
                            let datetime: chrono::DateTime<chrono::Utc> = chrono::DateTime::from_timestamp(ts, 0).unwrap();
                            datetime.format("%Y-%m-%d %H:%M:%S").to_string()
                        })
                        .unwrap_or_else(|| "Unknown".to_string())
                });

            messages.push(SessionMessage {
                role,
                content,
                timestamp: Some(timestamp),
            });
        }
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
