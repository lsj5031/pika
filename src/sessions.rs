use crate::config::ProjectConfig;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::{HashMap, VecDeque};
use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Read as _, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use thiserror::Error;
use futures::future::join_all;
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

/// In-memory index of sessions for fast lookup and pagination
#[derive(Debug, Clone, Default)]
pub struct SessionIndex {
    sessions_by_id: HashMap<String, SessionInfo>,
}

/// Paged session results
#[derive(Debug, Clone)]
pub struct SessionPage {
    pub sessions: Vec<SessionInfo>,
    pub next_cursor: Option<String>,
    pub total: usize,
}

impl SessionIndex {
    pub fn empty() -> Self {
        Self {
            sessions_by_id: HashMap::new(),
        }
    }

    pub fn from_sessions(sessions: Vec<SessionInfo>) -> Self {
        let mut sessions_by_id = HashMap::new();
        for session in sessions {
            sessions_by_id.insert(session.id.clone(), session);
        }

        Self { sessions_by_id }
    }

    pub fn rebuild(&mut self, sessions: Vec<SessionInfo>) {
        *self = Self::from_sessions(sessions);
    }

    pub fn upsert(&mut self, session: SessionInfo) {
        self.sessions_by_id.insert(session.id.clone(), session);
    }

    pub fn remove(&mut self, session_id: &str) {
        self.sessions_by_id.remove(session_id);
    }

    pub fn get(&self, session_id: &str) -> Option<&SessionInfo> {
        self.sessions_by_id.get(session_id)
    }

    pub fn lookup(&self, session_ids: &[String]) -> Vec<SessionInfo> {
        session_ids
            .iter()
            .filter_map(|id| self.sessions_by_id.get(id))
            .cloned()
            .collect()
    }

    pub fn list_sorted(
        &self,
        project_path: Option<&PathBuf>,
        query: Option<&str>,
    ) -> Vec<SessionInfo> {
        let query_lower = query.map(|q| q.to_lowercase());
        let mut sessions: Vec<&SessionInfo> = self
            .sessions_by_id
            .values()
            .filter(|session| {
                if let Some(project_path) = project_path {
                    if &session.project_path != project_path {
                        return false;
                    }
                }

                if let Some(ref q) = query_lower {
                    let project_path_str = session.project_path.to_string_lossy().to_lowercase();
                    let name_lower = session.name.to_lowercase();
                    let id_lower = session.id.to_lowercase();
                    if !name_lower.contains(q)
                        && !project_path_str.contains(q)
                        && !id_lower.contains(q)
                    {
                        return false;
                    }
                }

                true
            })
            .collect();

        sessions.sort_by(|a, b| compare_sessions(*a, *b));
        sessions.into_iter().cloned().collect()
    }

    pub fn paged(
        &self,
        project_path: Option<&PathBuf>,
        query: Option<&str>,
        limit: usize,
        cursor: Option<&str>,
    ) -> SessionPage {
        let all_sessions = self.list_sorted(project_path, query);
        let total = all_sessions.len();

        let filtered: Vec<SessionInfo> = if let Some(cursor) = cursor {
            match parse_cursor(cursor) {
                Some((cursor_time, cursor_id)) => all_sessions
                    .into_iter()
                    .filter(|session| is_after_cursor(session, &cursor_time, &cursor_id))
                    .collect(),
                None => all_sessions,
            }
        } else {
            all_sessions
        };

        let has_more = filtered.len() > limit;
        let sessions: Vec<SessionInfo> = filtered.into_iter().take(limit).collect();
        let next_cursor = if has_more {
            sessions
                .last()
                .map(|session| build_cursor(session))
        } else {
            None
        };

        SessionPage {
            sessions,
            next_cursor,
            total,
        }
    }

    pub fn project_counts(&self) -> HashMap<PathBuf, usize> {
        let mut counts = HashMap::new();
        for session in self.sessions_by_id.values() {
            *counts.entry(session.project_path.clone()).or_insert(0) += 1;
        }
        counts
    }
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

fn session_sort_key(session: &SessionInfo) -> (&str, &str) {
    let time = session
        .last_message_time
        .as_deref()
        .unwrap_or(&session.created_at);
    (time, session.id.as_str())
}

fn compare_sessions(a: &SessionInfo, b: &SessionInfo) -> Ordering {
    let (time_a, id_a) = session_sort_key(a);
    let (time_b, id_b) = session_sort_key(b);
    match time_b.cmp(time_a) {
        Ordering::Equal => id_b.cmp(id_a),
        ordering => ordering,
    }
}

fn build_cursor(session: &SessionInfo) -> String {
    let (time, id) = session_sort_key(session);
    format!("{}|{}", time, id)
}

fn parse_cursor(cursor: &str) -> Option<(String, String)> {
    let mut parts = cursor.splitn(2, '|');
    let time = parts.next()?.to_string();
    let id = parts.next()?.to_string();
    Some((time, id))
}

fn is_after_cursor(session: &SessionInfo, cursor_time: &str, cursor_id: &str) -> bool {
    let (time_a, id_a) = session_sort_key(session);
    match cursor_time.cmp(time_a) {
        Ordering::Equal => id_a.cmp(cursor_id) == Ordering::Less,
        Ordering::Greater => true,
        Ordering::Less => false,
    }
}

fn extract_session_id_and_timestamp(file_stem: &str) -> (String, String) {
    let parts: Vec<&str> = file_stem.rsplitn(2, '_').collect();
    if parts.len() == 2 {
        (parts[0].to_string(), parts[1].to_string())
    } else {
        (Uuid::new_v4().to_string(), "Unknown".to_string())
    }
}

pub async fn load_session_info_from_file(
    project_path: &Path,
    session_file: &Path,
) -> Result<SessionInfo, SessionError> {
    let file_stem = session_file
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("");
    let (session_id, timestamp) = extract_session_id_and_timestamp(file_stem);

    let metadata = tokio::fs::metadata(session_file)
        .await
        .map_err(|e| SessionError::IoError {
            path: session_file.to_path_buf(),
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

    let last_message_time = get_last_message_timestamp(session_file)
        .await
        .unwrap_or_else(|_| modified.clone());

    Ok(SessionInfo {
        id: session_id,
        project_path: project_path.to_path_buf(),
        name: file_stem.to_string(),
        created_at: modified,
        is_active: false,
        last_message_time: Some(last_message_time),
    })
}

pub async fn build_session_index(config: &ProjectConfig) -> SessionIndex {
    let sessions = scan_sessions(config).await;
    SessionIndex::from_sessions(sessions)
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

    let mut entries =
        tokio::fs::read_dir(sessions_dir)
            .await
            .map_err(|e| SessionError::IoError {
                path: sessions_dir.to_path_buf(),
                source: e,
            })?;

    let mut jsonl_paths = Vec::new();
    while let Some(entry) = entries.next_entry().await.map_err(|e| SessionError::IoError {
        path: sessions_dir.to_path_buf(),
        source: e,
    })? {
        let path = entry.path();
        if path.is_dir() {
            continue;
        }
        if path.extension().and_then(|s| s.to_str()) != Some("jsonl") {
            continue;
        }
        jsonl_paths.push(path);
    }

    let project_path = project_path.to_path_buf();
    let futures: Vec<_> = jsonl_paths
        .into_iter()
        .map(|path| {
            let project = project_path.clone();
            async move { load_session_info_from_file(&project, &path).await }
        })
        .collect();

    let results = join_all(futures).await;
    for result in results {
        sessions.push(result?);
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

fn parse_session_message_line(line: &str) -> Option<SessionMessage> {
    if line.trim().is_empty() {
        return None;
    }

    let value = serde_json::from_str::<serde_json::Value>(line).ok()?;
    if value.get("type").and_then(|t| t.as_str()) != Some("message") {
        return None;
    }

    let message_obj = value.get("message").and_then(|m| m.as_object())?;

    let role = message_obj
        .get("role")
        .and_then(|r| r.as_str())
        .unwrap_or("unknown")
        .to_string();

    let content = if let Some(content_array) = message_obj.get("content").and_then(|c| c.as_array())
    {
        let thinking_parts: Vec<String> = content_array
            .iter()
            .filter_map(|part| {
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

        let text_parts: Vec<String> = content_array
            .iter()
            .filter_map(|part| part.get("text").and_then(|t| t.as_str()).map(|s| s.to_string()))
            .collect();

        let mut all_parts = thinking_parts;
        all_parts.extend(text_parts);

        if !all_parts.is_empty() {
            all_parts.join("\n")
        } else {
            let tool_parts: Vec<String> = content_array
                .iter()
                .filter_map(|part| {
                    if let Some(tool_use) = part.get("tool_use").and_then(|t| t.as_object()) {
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
                    } else if let Some(tool_result) =
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
                format!(
                    "Tool call: {}",
                    serde_json::to_string(content_array).unwrap_or_default()
                )
            }
        }
    } else if let Some(content_str) = message_obj.get("content").and_then(|c| c.as_str()) {
        content_str.to_string()
    } else {
        String::from("[Tool call or system message - no text content]")
    };

    let timestamp = value
        .get("timestamp")
        .and_then(|t| t.as_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| {
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

    Some(SessionMessage {
        role,
        content,
        timestamp: Some(timestamp),
        images: None,
    })
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

        if let Some(message) = parse_session_message_line(&line) {
            messages.push(message);
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

/// Read up to `limit` parseable messages from the end of a JSONL file by reading
/// chunks backwards. Returns messages in reverse chronological order (newest first);
/// the caller must reverse if chronological order is needed.
fn read_last_messages_reverse(
    path: &Path,
    limit: usize,
) -> Result<Vec<SessionMessage>, SessionError> {
    const CHUNK_SIZE: u64 = 8192;

    let mut file = fs::File::open(path).map_err(|e| SessionError::IoError {
        path: path.to_path_buf(),
        source: e,
    })?;

    let file_len = file
        .metadata()
        .map_err(|e| SessionError::IoError {
            path: path.to_path_buf(),
            source: e,
        })?
        .len();

    if file_len == 0 {
        return Ok(Vec::new());
    }

    let mut messages: Vec<SessionMessage> = Vec::with_capacity(limit);
    let mut pos = file_len;
    let mut trailing = String::new(); // leftover bytes from the previous (later) chunk

    while pos > 0 && messages.len() < limit {
        let read_start = pos.saturating_sub(CHUNK_SIZE);
        let to_read = (pos - read_start) as usize;

        file.seek(SeekFrom::Start(read_start)).map_err(|e| SessionError::IoError {
            path: path.to_path_buf(),
            source: e,
        })?;

        let mut buf = vec![0u8; to_read];
        file.read_exact(&mut buf).map_err(|e| SessionError::IoError {
            path: path.to_path_buf(),
            source: e,
        })?;

        let chunk_str = String::from_utf8_lossy(&buf);

        let combined = if trailing.is_empty() {
            chunk_str.into_owned()
        } else {
            let mut s = chunk_str.into_owned();
            s.push_str(&trailing);
            s
        };

        let mut lines: Vec<&str> = combined.split('\n').collect();

        if read_start > 0 {
            trailing = lines.remove(0).to_owned();
        } else {
            trailing.clear();
        }

        for line in lines.iter().rev() {
            if messages.len() >= limit {
                break;
            }
            if let Some(msg) = parse_session_message_line(line) {
                messages.push(msg);
            }
        }

        pos = read_start;
    }

    if messages.len() < limit && !trailing.is_empty() {
        if let Some(msg) = parse_session_message_line(&trailing) {
            messages.push(msg);
        }
    }

    Ok(messages)
}

/// Get messages before a timestamp (paged history loading)
pub fn get_session_messages_before(
    session_id: &str,
    project_path: &Path,
    limit: usize,
    before: Option<&str>,
) -> Result<Vec<SessionMessage>, SessionError> {
    if limit == 0 {
        return Ok(Vec::new());
    }

    let pi_sessions_dir = dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".pi")
        .join("agent")
        .join("sessions");

    let encoded_path = encode_project_path(project_path);
    let project_sessions_dir = pi_sessions_dir.join(&encoded_path);

    if !project_sessions_dir.exists() {
        return Ok(Vec::new());
    }

    let session_file = fs::read_dir(&project_sessions_dir)
        .map_err(|e| SessionError::IoError {
            path: project_sessions_dir.clone(),
            source: e,
        })?
        .find_map(|entry| {
            let entry = entry.ok()?;
            let path = entry.path();
            let file_name = path.file_stem()?.to_str()?;
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

    if before.is_none() {
        let mut messages =
            read_last_messages_reverse(&session_file, limit)?;
        messages.reverse();
        return Ok(messages);
    }

    let file = fs::File::open(&session_file).map_err(|e| SessionError::IoError {
        path: session_file.clone(),
        source: e,
    })?;

    let reader = BufReader::new(file);
    let mut buffer: VecDeque<SessionMessage> = VecDeque::new();
    let before_ts = before.unwrap();

    for line in reader.lines() {
        let line = line.map_err(|e| SessionError::IoError {
            path: session_file.clone(),
            source: e,
        })?;

        if let Some(message) = parse_session_message_line(&line) {
            let message_ts = message.timestamp.as_deref();
            if message_ts.is_none() || message_ts >= Some(before_ts) {
                continue;
            }

            buffer.push_back(message);
            if buffer.len() > limit {
                buffer.pop_front();
            }
        }
    }

    Ok(buffer.into_iter().collect())
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
