use crate::config::ProjectConfig;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use thiserror::Error;
use tokio::io::{AsyncReadExt, AsyncSeekExt};
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
    /// Timestamp of the most recent message (if any)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_message_time: Option<String>,
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
    /// Image attachments (for user messages with images)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub images: Option<Vec<ImageAttachmentStored>>,
}

/// Session discovery errors
#[derive(Debug, Error)]
pub enum SessionError {
    #[error("Failed to read session file {path}: {source}")]
    IoError {
        path: PathBuf,
        source: std::io::Error,
    },
}

/// Scan for pi sessions in configured project directories
pub async fn scan_sessions(config: &ProjectConfig) -> Vec<SessionInfo> {
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
        // Encode the project path to match Pika's naming convention
        // e.g., /home/youruser/appifex/appifex -> --home-leo-appifex-appifex--
        let encoded_path = encode_project_path(project_path);
        let project_sessions_dir = pi_sessions_dir.join(&encoded_path);

        // Scan each project's sessions directory
        if let Ok(project_sessions) =
            scan_project_sessions(project_path, &project_sessions_dir).await
        {
            sessions.extend(project_sessions);
        }
    }

    sessions
}

/// Encode a project path to match Pika's directory naming convention
pub fn encode_project_path(path: &Path) -> String {
    let path_str = path.to_string_lossy();
    let normalized = path_str.trim_start_matches('/').replace(['/', '\\'], "-");
    format!("--{}--", normalized)
}

/// Build a lookup map from encoded project names to their original paths
/// This is needed because decoding is lossy (e.g., paths with '-' in them)
#[allow(dead_code)]
pub fn build_encoded_project_map(config: &ProjectConfig) -> HashMap<String, PathBuf> {
    let mut map = HashMap::new();
    for path in &config.project_root_paths {
        let encoded = encode_project_path(path);
        map.insert(encoded, path.clone());
    }
    map
}

/// Get the pi sessions directory for a project path
/// Uses the standard ~/.pi/agent/sessions/{encoded-path}/ structure
pub fn get_pi_sessions_dir(project_path: &Path) -> PathBuf {
    let pi_sessions_base = dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".pi")
        .join("agent")
        .join("sessions");

    let encoded_path = encode_project_path(project_path);
    pi_sessions_base.join(encoded_path)
}

/// Get the full path to a session file
pub fn get_session_file_path(session_id: &str, project_path: &Path) -> Option<PathBuf> {
    let sessions_dir = get_pi_sessions_dir(project_path);

    if !sessions_dir.exists() {
        return None;
    }

    // Find the session file with the given ID
    fs::read_dir(&sessions_dir).ok()?.find_map(|entry| {
        let entry = entry.ok()?;
        let path = entry.path();
        let file_name = path.file_stem()?.to_str()?;
        if file_name.contains(session_id) {
            Some(path)
        } else {
            None
        }
    })
}

/// Get the timestamp of the most recent message in a session file
/// Optimized to read only the last ~4KB of the file instead of the entire file
async fn get_last_message_timestamp(session_file: &Path) -> Result<String, SessionError> {
    let mut file =
        tokio::fs::File::open(session_file)
            .await
            .map_err(|e| SessionError::IoError {
                path: session_file.to_path_buf(),
                source: e,
            })?;

    let metadata = file.metadata().await.map_err(|e| SessionError::IoError {
        path: session_file.to_path_buf(),
        source: e,
    })?;
    let file_size = metadata.len();

    // Read only the last ~4KB of the file
    const TAIL_SIZE: u64 = 4096;
    let seek_pos = file_size.saturating_sub(TAIL_SIZE);

    file.seek(std::io::SeekFrom::Start(seek_pos))
        .await
        .map_err(|e| SessionError::IoError {
            path: session_file.to_path_buf(),
            source: e,
        })?;

    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)
        .await
        .map_err(|e| SessionError::IoError {
            path: session_file.to_path_buf(),
            source: e,
        })?;

    let content = String::from_utf8_lossy(&buffer);
    let mut last_timestamp: Option<String> = None;

    // Parse each line from the tail, looking for the last valid timestamp
    for line in content.lines() {
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(line) {
            if let Some(ts) = json.get("timestamp").and_then(|t| t.as_str()) {
                last_timestamp = Some(ts.to_string());
            }
        }
    }

    last_timestamp.ok_or_else(|| SessionError::IoError {
        path: session_file.to_path_buf(),
        source: std::io::Error::new(std::io::ErrorKind::NotFound, "No messages found"),
    })
}

/// Scan a single project directory for sessions
async fn scan_project_sessions(
    project_path: &Path,
    sessions_dir: &Path,
) -> Result<Vec<SessionInfo>, SessionError> {
    let mut sessions = Vec::new();

    // If sessions directory doesn't exist, return empty list (not an error)
    if !sessions_dir.exists() {
        return Ok(sessions);
    }

    // Read all session files (*.jsonl)
    let mut entries =
        tokio::fs::read_dir(sessions_dir)
            .await
            .map_err(|e| SessionError::IoError {
                path: sessions_dir.to_path_buf(),
                source: e,
            })?;

    while let Some(entry) = entries.next_entry().await.map_err(|e| SessionError::IoError {
        path: sessions_dir.to_path_buf(),
        source: e,
    })? {
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
        let file_name = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");

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
        let metadata =
            tokio::fs::metadata(&path)
                .await
                .map_err(|e| SessionError::IoError {
                    path: path.clone(),
                    source: e,
                })?;
        let modified = metadata
            .modified()
            .ok()
            .map(|t| {
                let datetime: chrono::DateTime<chrono::Utc> = t.into();
                datetime.format("%Y-%m-%d %H:%M:%S").to_string()
            })
            .unwrap_or_else(|| timestamp.clone());

        // Try to get the last message timestamp by parsing the session file
        let last_message_time = get_last_message_timestamp(&path)
            .await
            .unwrap_or_else(|_| modified.clone());

        sessions.push(SessionInfo {
            id: session_id,
            project_path: project_path.to_path_buf(),
            name: file_name.to_string(),
            created_at: modified,
            is_active: false,
            last_message_time: Some(last_message_time),
        });
    }

    Ok(sessions)
}

/// Get messages for a specific session
#[allow(dead_code)]
pub fn get_session_messages(
    session_id: &str,
    project_path: &Path,
) -> Result<Vec<SessionMessage>, SessionError> {
    get_session_messages_limited(session_id, project_path, None, false)
}

pub fn get_session_messages_limited(
    session_id: &str,
    project_path: &Path,
    limit: Option<usize>,
    from_start: bool,
) -> Result<Vec<SessionMessage>, SessionError> {
    if let Some(0) = limit {
        return Ok(Vec::new());
    }
    // Get the pi sessions directory
    let pi_sessions_dir = dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".pi")
        .join("agent")
        .join("sessions");

    // Encode the project path to match Pika's naming convention
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
    let file = fs::File::open(&session_file).map_err(|e| SessionError::IoError {
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

        // Try to parse as a Pika session entry
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
            let role = message_obj
                .get("role")
                .and_then(|r| r.as_str())
                .unwrap_or("unknown")
                .to_string();

            // Get content from message.content array
            // Handle text content, thinking blocks, and tool call results
            let content = if let Some(content_array) =
                message_obj.get("content").and_then(|c| c.as_array())
            {
                // Extract thinking blocks first
                let thinking_parts: Vec<String> = content_array
                    .iter()
                    .filter_map(|part| {
                        // Check for type: "thinking" with thinking field
                        if part.get("type").and_then(|t| t.as_str()) == Some("thinking") {
                            part.get("thinking")
                                .and_then(|t| t.as_str())
                                .filter(|s| !s.is_empty())
                                .map(|s| format!("<thinking>{}</thinking>", s))
                        } else {
                            None
                        }
                    })
                    .collect();

                // Try to get text parts
                let text_parts: Vec<String> = content_array
                    .iter()
                    .filter_map(|part| {
                        part.get("text")
                            .and_then(|t| t.as_str())
                            .map(|s| s.to_string())
                    })
                    .collect();

                // Combine thinking and text parts
                let mut all_parts = thinking_parts;
                all_parts.extend(text_parts);

                if !all_parts.is_empty() {
                    all_parts.join("\n")
                } else {
                    // Try to extract tool call information
                    let tool_parts: Vec<String> = content_array
                        .iter()
                        .filter_map(|part| {
                            // Handle tool_use type
                            if let Some(tool_use) = part.get("tool_use").and_then(|t| t.as_object())
                            {
                                let name = tool_use
                                    .get("name")
                                    .and_then(|n| n.as_str())
                                    .unwrap_or("unknown_tool");
                                let input = tool_use
                                    .get("input")
                                    .map(|i| {
                                        if i.is_string() {
                                            i.as_str().unwrap_or("").to_string()
                                        } else if i.is_object() {
                                            serde_json::to_string(i).unwrap_or_default()
                                        } else {
                                            String::new()
                                        }
                                    })
                                    .unwrap_or_default();
                                Some(format!("Tool Call: {}({})", name, input))
                            }
                            // Handle tool_result type
                            else if let Some(tool_result) =
                                part.get("tool_result").and_then(|t| t.as_object())
                            {
                                let is_error = tool_result
                                    .get("is_error")
                                    .and_then(|e| e.as_bool())
                                    .unwrap_or(false);
                                let content = tool_result
                                    .get("content")
                                    .map(|c| {
                                        if c.is_string() {
                                            c.as_str().unwrap_or("").to_string()
                                        } else if c.is_array() {
                                            serde_json::to_string(c).unwrap_or_default()
                                        } else {
                                            String::new()
                                        }
                                    })
                                    .unwrap_or_default();
                                Some(format!(
                                    "Tool Result{}: {}",
                                    if is_error { " (Error)" } else { "" },
                                    content
                                ))
                            } else {
                                None
                            }
                        })
                        .collect();

                    if !tool_parts.is_empty() {
                        tool_parts.join("\n")
                    } else {
                        // Fallback: show entire content array as JSON for debugging
                        format!(
                            "Tool call: {}",
                            serde_json::to_string(content_array).unwrap_or_default()
                        )
                    }
                }
            } else if let Some(content_str) = message_obj.get("content").and_then(|c| c.as_str()) {
                // Handle string content directly
                content_str.to_string()
            } else {
                // Empty or unparseable content - don't skip, show placeholder
                String::from("[Tool call or system message - no text content]")
            };

            // Get timestamp
            let timestamp = value
                .get("timestamp")
                .and_then(|t| t.as_str())
                .map(|s| s.to_string())
                .unwrap_or_else(|| {
                    // Fallback to message timestamp if available
                    message_obj
                        .get("timestamp")
                        .and_then(|t| t.as_i64())
                        .map(|ts| {
                            let datetime: chrono::DateTime<chrono::Utc> =
                                chrono::DateTime::from_timestamp(ts, 0).unwrap();
                            datetime.format("%Y-%m-%d %H:%M:%S").to_string()
                        })
                        .unwrap_or_else(|| "Unknown".to_string())
                });

            messages.push(SessionMessage {
                role,
                content,
                timestamp: Some(timestamp),
                images: None,
            });
        }
    }

    if let Some(limit) = limit {
        if from_start {
            messages.truncate(limit.min(messages.len()));
            return Ok(messages);
        }

        if messages.len() > limit {
            return Ok(messages[messages.len() - limit..].to_vec());
        }
    }

    Ok(messages)
}

/// Stored user prompt (for prompts sent via our API that pi-agent doesn't persist)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredUserPrompt {
    /// The prompt text
    pub prompt: String,
    /// Timestamp when the prompt was sent
    pub timestamp: String,
}

/// Image attachment stored with user prompt
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageAttachmentStored {
    pub id: String,
    pub filename: String,
    pub content_type: String,
    pub size: usize,
    pub url: String,
}

/// Stored user prompt with optional image attachments
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredUserPromptWithImages {
    pub prompt: String,
    pub timestamp: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub images: Option<Vec<ImageAttachmentStored>>,
}

/// Get the path to the user prompts file for a session
fn get_user_prompts_path(session_id: &str, project_path: &Path) -> Option<PathBuf> {
    let pi_sessions_dir = dirs::home_dir()?
        .join(".pi")
        .join("agent")
        .join("sessions");
    
    let encoded_path = encode_project_path(project_path);
    let project_sessions_dir = pi_sessions_dir.join(&encoded_path);
    
    Some(project_sessions_dir.join(format!(".user-prompts-{}.jsonl", session_id)))
}

/// Store a user prompt with optional image attachments for later retrieval
pub fn store_user_prompt_with_images(
    session_id: &str,
    project_path: &Path,
    prompt: &str,
    images: Option<Vec<ImageAttachmentStored>>,
) -> Result<(), SessionError> {
    let prompts_path = match get_user_prompts_path(session_id, project_path) {
        Some(p) => p,
        None => return Ok(()),
    };

    if let Some(parent) = prompts_path.parent() {
        fs::create_dir_all(parent).map_err(|e| SessionError::IoError {
            path: parent.to_path_buf(),
            source: e,
        })?;
    }

    let stored_prompt = StoredUserPromptWithImages {
        prompt: prompt.to_string(),
        timestamp: chrono::Utc::now().to_rfc3339(),
        images,
    };

    let line = serde_json::to_string(&stored_prompt).unwrap_or_default();

    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&prompts_path)
        .map_err(|e| SessionError::IoError {
            path: prompts_path.clone(),
            source: e,
        })?;

    writeln!(file, "{}", line).map_err(|e| SessionError::IoError {
        path: prompts_path,
        source: e,
    })?;

    Ok(())
}

/// Load stored user prompts for a session
pub fn load_user_prompts(
    session_id: &str,
    project_path: &Path,
) -> Vec<SessionMessage> {
    let prompts_path = match get_user_prompts_path(session_id, project_path) {
        Some(p) => p,
        None => return Vec::new(),
    };

    if !prompts_path.exists() {
        return Vec::new();
    }

    let file = match fs::File::open(&prompts_path) {
        Ok(f) => f,
        Err(_) => return Vec::new(),
    };

    let reader = BufReader::new(file);
    let mut prompts = Vec::new();

    for line in reader.lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => continue,
        };

        if line.trim().is_empty() {
            continue;
        }

        // Try parsing with images first (new format)
        if let Ok(stored) = serde_json::from_str::<StoredUserPromptWithImages>(&line) {
            prompts.push(SessionMessage {
                role: "user".to_string(),
                content: stored.prompt,
                timestamp: Some(stored.timestamp),
                images: stored.images,
            });
            continue;
        }

        // Fallback to parsing without images (backward compatibility)
        if let Ok(stored) = serde_json::from_str::<StoredUserPrompt>(&line) {
            prompts.push(SessionMessage {
                role: "user".to_string(),
                content: stored.prompt,
                timestamp: Some(stored.timestamp),
                images: None,
            });
        }
    }

    prompts
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
/// Sessions are stored in ~/.pi/agent/sessions/{encoded-project-path}/ to match Pika
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
    let _name = request.name.unwrap_or_else(|| {
        // Default name: timestamp
        chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string()
    });

    // Generate timestamp in Pika format: 2026-01-13T00-20-44-881Z
    let now = chrono::Utc::now();
    let timestamp_for_filename = now.format("%Y-%m-%dT%H-%M-%S").to_string();
    let millis = now.timestamp_subsec_millis();
    let timestamp_str = format!("{}-{:03}Z", timestamp_for_filename, millis);
    let created_at = now.format("%Y-%m-%d %H:%M:%S").to_string();

    // Use the standard ~/.pi/agent/sessions/{encoded-path}/ directory
    let sessions_dir = get_pi_sessions_dir(project_path);

    // Create sessions directory if it doesn't exist
    fs::create_dir_all(&sessions_dir).map_err(|e| SessionError::IoError {
        path: sessions_dir.clone(),
        source: e,
    })?;

    // Create the session file with Pika naming convention:
    // {timestamp}_{session_id}.jsonl
    let session_filename = format!("{}_{}.jsonl", timestamp_str, session_id);
    let session_file = sessions_dir.join(&session_filename);

    // Create empty session file (Pika will populate it when used)
    fs::File::create(&session_file).map_err(|e| SessionError::IoError {
        path: session_file.clone(),
        source: e,
    })?;

    Ok(CreateSessionResponse {
        session_id,
        name: session_filename.trim_end_matches(".jsonl").to_string(),
        project_path: project_path.to_path_buf(),
        created_at,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_scan_sessions_empty_config() {
        let config = ProjectConfig::default();
        let sessions = scan_sessions(&config).await;
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
            last_message_time: None,
        };

        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains("test-session-123"));
        assert!(json.contains("Test Session"));
    }
}
