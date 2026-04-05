use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use thiserror::Error;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;
use tokio::sync::broadcast::{self, Receiver, Sender};
use tokio::task::JoinHandle;
use tracing::{debug, info};

/// Maximum number of concurrent Pika processes allowed
const MAX_CONCURRENT_PROCESSES: usize = 50;

#[cfg(windows)]
const NPX_CANDIDATES: &[&str] = &["npx.cmd", "npx.exe", "npx"];
#[cfg(not(windows))]
const NPX_CANDIDATES: &[&str] = &["npx"];

fn find_executable_in_path(names: &[&str]) -> Option<PathBuf> {
    let path_var = env::var_os("PATH")?;

    for dir in env::split_paths(&path_var) {
        for name in names {
            let candidate = dir.join(name);
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }

    None
}

fn find_nvm_npx() -> Option<PathBuf> {
    let home = env::var_os("HOME")?;
    let versions_root = PathBuf::from(home).join(".nvm/versions/node");
    find_nvm_npx_under(&versions_root)
}

fn find_nvm_npx_under(versions_root: &Path) -> Option<PathBuf> {
    let mut version_dirs: Vec<PathBuf> = fs::read_dir(versions_root)
        .ok()?
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let file_type = entry.file_type().ok()?;
            file_type.is_dir().then_some(entry.path())
        })
        .collect();

    // Prefer newest semver-like directory names first (e.g. v22 > v20 > v18).
    version_dirs.sort_by(|a, b| b.file_name().cmp(&a.file_name()));

    for version_dir in version_dirs {
        for name in NPX_CANDIDATES {
            let candidate = version_dir.join("bin").join(name);
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }

    None
}

fn resolve_npx_executable() -> PathBuf {
    if let Ok(path) = env::var("PIKA_NPX_PATH") {
        let trimmed = path.trim();
        if !trimmed.is_empty() {
            return PathBuf::from(trimmed);
        }
    }

    find_executable_in_path(NPX_CANDIDATES)
        .or_else(find_nvm_npx)
        .unwrap_or_else(|| PathBuf::from("npx"))
}

/// JSON-RPC event emitted by Pika process
/// Pika uses events with "type" field, not standard JSON-RPC
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcEvent {
    /// Event type (e.g., "message_update", "agent_start", "notify", etc.)
    #[serde(rename = "type")]
    pub event_type: Option<String>,
    /// All other fields from the event
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

/// Image attachment for sending to pika-agent
#[derive(Debug, Clone)]
pub struct ImageUpload {
    pub filename: String,
    pub content_type: String,
    pub data: String,
}

/// Manages a agent subprocess
pub struct PikaProcess {
    /// The subprocess handle
    process: tokio::process::Child,
    /// Unique process identifier
    pub id: String,

    /// Channel sender for broadcasting JSON-RPC events
    tx: Sender<JsonRpcEvent>,
    /// stdin handle for sending commands
    stdin: tokio::process::ChildStdin,
    /// Task handle for stdout reader
    _stdout_task: JoinHandle<()>,
    /// Task handle for stderr reader
    _stderr_task: JoinHandle<()>,
}

impl PikaProcess {
    /// Spawn a new Pika process in RPC mode
    /// If session_file is provided, the process will resume that session
    pub fn spawn(
        project_path: PathBuf,
        session_file: Option<PathBuf>,
    ) -> Result<Self, PikaProcessError> {
        // Validate project path exists
        if !project_path.exists() {
            return Err(PikaProcessError::ProjectNotFound { path: project_path });
        }

        // Generate unique process ID
        let id = uuid::Uuid::new_v4().to_string();

        // Create broadcast channel for events
        let (tx, _rx) = broadcast::channel(1000);

        // Build command arguments
        let mut args = vec![
            "@mariozechner/pi-coding-agent".to_string(),
            "--mode".to_string(),
            "rpc".to_string(),
        ];

        // If session file is provided, resume that session
        if let Some(ref session_path) = session_file {
            args.push("--session".to_string());
            args.push(session_path.to_string_lossy().to_string());
        }

        let npx_executable = resolve_npx_executable();

        info!(
            process_id = %id,
            npx = %npx_executable.display(),
            project = %project_path.display(),
            session_file = session_file.as_ref().map(|p| p.display().to_string()).unwrap_or_default(),
            "Spawning pika-agent process"
        );

        // Spawn Pika process with environment variables inherited
        let mut process =
            Command::new(&npx_executable)
                .args(&args)
                .current_dir(project_path.canonicalize().map_err(|_e| {
                    PikaProcessError::InvalidPath {
                        path: project_path.clone(),
                    }
                })?)
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .map_err(|e| match e.kind() {
                    std::io::ErrorKind::NotFound => PikaProcessError::NpxNotFound {
                        path: project_path.clone(),
                        executable: npx_executable.to_string_lossy().to_string(),
                        source: e,
                    },
                    _ => PikaProcessError::SpawnFailed {
                        path: project_path.clone(),
                        source: e,
                    },
                })?;

        // Get stdin, stdout and stderr handles
        let stdin = process.stdin.take().ok_or(PikaProcessError::PipeFailed)?;
        let stdout = process.stdout.take().ok_or(PikaProcessError::PipeFailed)?;
        let stderr = process.stderr.take().ok_or(PikaProcessError::PipeFailed)?;

        // Spawn task to read JSON-RPC events from stdout
        let tx_clone = tx.clone();
        let id_clone = id.clone();
        let stdout_task = tokio::spawn(async move {
            Self::read_stdout(stdout, tx_clone, id_clone).await;
        });

        // Spawn task to read logs from stderr
        let stderr_task = tokio::spawn(async move {
            Self::read_stderr(stderr).await;
        });

        Ok(PikaProcess {
            process,
            id,
            tx,
            stdin,
            _stdout_task: stdout_task,
            _stderr_task: stderr_task,
        })
    }

    /// Subscribe to JSON-RPC events from this process
    pub fn subscribe(&self) -> Receiver<JsonRpcEvent> {
        self.tx.subscribe()
    }

    /// Send a prompt with optional images to the Pika process via stdin
    pub async fn send_prompt_with_images(
        &mut self,
        prompt: &str,
        images: &[ImageUpload],
    ) -> Result<(), PikaProcessError> {
        if !self.is_running() {
            return Err(PikaProcessError::ProcessNotRunning {
                id: self.id.clone(),
            });
        }

        let image_contents: Vec<serde_json::Value> = images
            .iter()
            .map(|img| {
                serde_json::json!({
                    "type": "image_url",
                    "image_url": {
                        "url": format!("data:{};base64,{}", img.content_type, img.data)
                    }
                })
            })
            .collect();

        let mut request = serde_json::json!({
            "type": "prompt",
            "message": prompt
        });

        if !image_contents.is_empty() {
            request["images"] = serde_json::Value::Array(image_contents);
        }

        let request_str = format!("{}\n", request);
        let stdin = &mut self.stdin;

        stdin
            .write_all(request_str.as_bytes())
            .await
            .map_err(|e| PikaProcessError::WriteFailed {
                id: self.id.clone(),
                source: e,
            })?;

        stdin
            .flush()
            .await
            .map_err(|e| PikaProcessError::WriteFailed {
                id: self.id.clone(),
                source: e,
            })?;

        Ok(())
    }

    /// Send a raw JSON command to the Pika process via stdin
    pub async fn send_command(&mut self, command: serde_json::Value) -> Result<(), PikaProcessError> {
        if !self.is_running() {
            return Err(PikaProcessError::ProcessNotRunning {
                id: self.id.clone(),
            });
        }

        let request_str = format!("{}\n", command);
        let stdin = &mut self.stdin;

        stdin
            .write_all(request_str.as_bytes())
            .await
            .map_err(|e| PikaProcessError::WriteFailed {
                id: self.id.clone(),
                source: e,
            })?;

        stdin
            .flush()
            .await
            .map_err(|e| PikaProcessError::WriteFailed {
                id: self.id.clone(),
                source: e,
            })?;

        Ok(())
    }

    /// Kill the Pika process
    pub async fn kill(mut self) -> Result<(), PikaProcessError> {
        self.process
            .kill()
            .await
            .map_err(|e| PikaProcessError::KillFailed {
                id: self.id.clone(),
                source: e,
            })?;

        // Abort the reader tasks
        self._stdout_task.abort();
        self._stderr_task.abort();

        Ok(())
    }

    /// Check if the process is still running
    pub fn is_running(&mut self) -> bool {
        match self.process.try_wait() {
            Ok(None) => true,     // Process is still running
            Ok(Some(_)) => false, // Process has exited
            Err(_) => false,      // Error checking status, assume not running
        }
    }

    /// Read stdout and parse JSON-RPC events
    async fn read_stdout(
        stdout: tokio::process::ChildStdout,
        tx: Sender<JsonRpcEvent>,
        id: String,
    ) {
        use tokio::io::{AsyncBufReadExt, BufReader};

        let reader = BufReader::new(stdout);
        let mut lines = reader.lines();

        while let Ok(Some(line)) = lines.next_line().await {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }

            // Try to parse as JSON-RPC event
            match serde_json::from_str::<JsonRpcEvent>(trimmed) {
                Ok(event) => {
                    // Broadcast to all subscribers (ignore errors if no listeners)
                    let _ = tx.send(event);
                }
                Err(e) => {
                    // Not valid JSON-RPC — log at debug level with truncated content
                    let preview: String = trimmed.chars().take(80).collect();
                    debug!(
                        process_id = %id,
                        error = %e,
                        preview = %preview,
                        "Non-JSON stdout line from pika-agent"
                    );
                }
            }
        }

        debug!(process_id = %id, "pika-agent stdout reader ended");
    }

    /// Read stderr, logging metadata without raw payload content.
    async fn read_stderr(stderr: tokio::process::ChildStderr) {
        use tokio::io::{AsyncBufReadExt, BufReader};

        let reader = BufReader::new(stderr);
        let mut lines = reader.lines();
        let mut line_count: u64 = 0;

        while let Ok(Some(_line)) = lines.next_line().await {
            line_count += 1;
            // Intentionally suppress raw stderr content to avoid leaking prompts/secrets.
        }

        if line_count > 0 {
            debug!(stderr_lines = line_count, "pika-agent stderr output");
        }
    }
}

/// Manages multiple Pika processes
pub struct ProcessManager {
    /// Map of process ID to PikaProcess
    processes: HashMap<String, PikaProcess>,
    /// Combined event sender that forwards events from all processes
    event_tx: Sender<ProcessManagerEvent>,
    /// Map of session ID to process ID (for tracking which sessions are active)
    session_to_process: HashMap<String, String>,
    /// Map of process ID to session ID (reverse mapping)
    process_to_session: HashMap<String, String>,
}

/// Events from the ProcessManager
#[derive(Debug, Clone)]
pub enum ProcessManagerEvent {
    /// A process was spawned
    ProcessSpawned { id: String, project_path: PathBuf },
    /// A process was killed
    ProcessKilled {
        id: String,
        session_id: Option<String>,
    },
    /// A JSON-RPC event from a process
    JsonRpc { id: String, event: JsonRpcEvent },
    /// A session was started
    SessionStarted {
        session_id: String,
        process_id: String,
    },
}

impl Default for ProcessManager {
    fn default() -> Self {
        Self::new()
    }
}

impl ProcessManager {
    fn remove_process_mappings(&mut self, id: &str) -> Option<String> {
        let session_id = self.process_to_session.remove(id);

        if let Some(session_id) = session_id.as_ref() {
            self.session_to_process.remove(session_id);
        } else {
            self.session_to_process
                .retain(|_session, process_id| process_id != id);
        }

        session_id
    }

    fn cleanup_exited_processes(&mut self) {
        let exited_ids: Vec<String> = self
            .processes
            .iter_mut()
            .filter_map(|(id, process)| (!process.is_running()).then_some(id.clone()))
            .collect();

        for id in exited_ids {
            self.processes.remove(&id);
            let session_id = self.remove_process_mappings(&id);
            let _ = self
                .event_tx
                .send(ProcessManagerEvent::ProcessKilled { id, session_id });
        }
    }

    /// Create a new process manager
    pub fn new() -> Self {
        let (event_tx, _) = broadcast::channel(1000);
        ProcessManager {
            processes: HashMap::new(),
            event_tx,
            session_to_process: HashMap::new(),
            process_to_session: HashMap::new(),
        }
    }

    /// Subscribe to events from the process manager
    pub fn subscribe(&self) -> Receiver<ProcessManagerEvent> {
        self.event_tx.subscribe()
    }

    /// Spawn a new Pika process
    /// If session_file is provided, the process will resume that session
    pub fn spawn(
        &mut self,
        project_path: PathBuf,
        session_file: Option<PathBuf>,
    ) -> Result<String, PikaProcessError> {
        self.cleanup_exited_processes();

        // Check concurrent limit
        if self.processes.len() >= MAX_CONCURRENT_PROCESSES {
            return Err(PikaProcessError::TooManyProcesses {
                max: MAX_CONCURRENT_PROCESSES,
            });
        }

        // Create process
        let process = PikaProcess::spawn(project_path.clone(), session_file)?;
        let id = process.id.clone();

        // Subscribe to this process's events
        let mut rx = process.subscribe();
        let event_tx = self.event_tx.clone();
        let id_clone = id.clone();

        // Spawn a task to forward events from this process
        tokio::spawn(async move {
            while let Ok(event) = rx.recv().await {
                let _ = event_tx.send(ProcessManagerEvent::JsonRpc {
                    id: id_clone.clone(),
                    event,
                });
            }
        });

        // Store in map
        self.processes.insert(id.clone(), process);

        // Emit ProcessSpawned event
        let _ = self.event_tx.send(ProcessManagerEvent::ProcessSpawned {
            id: id.clone(),
            project_path,
        });

        Ok(id)
    }

    /// Kill a process by ID
    pub async fn kill(&mut self, id: &str) -> Result<(), PikaProcessError> {
        let process = self
            .processes
            .remove(id)
            .ok_or(PikaProcessError::ProcessNotFound { id: id.to_string() })?;

        let session_id = self.remove_process_mappings(id);

        process.kill().await?;

        // Emit ProcessKilled event
        let _ = self.event_tx.send(ProcessManagerEvent::ProcessKilled {
            id: id.to_string(),
            session_id,
        });

        Ok(())
    }

    /// Subscribe to events from a specific process
    pub fn subscribe_to_process(&self, id: &str) -> Result<Receiver<JsonRpcEvent>, PikaProcessError> {
        let process = self
            .processes
            .get(id)
            .ok_or(PikaProcessError::ProcessNotFound { id: id.to_string() })?;

        Ok(process.subscribe())
    }

    /// Get all active process IDs
    pub fn list(&self) -> Vec<String> {
        self.processes.keys().cloned().collect()
    }

    /// Check if a process is running
    pub fn is_running(&mut self, id: &str) -> bool {
        let Some(process) = self.processes.get_mut(id) else {
            return false;
        };

        if process.is_running() {
            true
        } else {
            self.processes.remove(id);
            let session_id = self.remove_process_mappings(id);
            let _ = self.event_tx.send(ProcessManagerEvent::ProcessKilled {
                id: id.to_string(),
                session_id,
            });
            false
        }
    }

    /// Get number of active processes
    pub fn count(&self) -> usize {
        self.processes.len()
    }

    /// Send a prompt to a specific process by ID
    pub async fn send_prompt(&mut self, id: &str, prompt: &str) -> Result<(), PikaProcessError> {
        self.send_prompt_with_images(id, prompt, &[]).await
    }

    /// Send a prompt with images to a specific process by ID
    pub async fn send_prompt_with_images(
        &mut self,
        id: &str,
        prompt: &str,
        images: &[ImageUpload],
    ) -> Result<(), PikaProcessError> {
        let process = self
            .processes
            .get_mut(id)
            .ok_or(PikaProcessError::ProcessNotFound { id: id.to_string() })?;

        process.send_prompt_with_images(prompt, images).await
    }

    /// Send a raw JSON command to a specific process by ID
    pub async fn send_command(
        &mut self,
        id: &str,
        command: serde_json::Value,
    ) -> Result<(), PikaProcessError> {
        let process = self
            .processes
            .get_mut(id)
            .ok_or(PikaProcessError::ProcessNotFound { id: id.to_string() })?;

        process.send_command(command).await
    }

    /// Spawn a new Pika process for a specific session
    /// Returns the process ID if spawned, or existing process ID if already running
    /// The process will resume the existing session file if it exists
    pub fn spawn_for_session(
        &mut self,
        session_id: &str,
        project_path: PathBuf,
    ) -> Result<String, PikaProcessError> {
        self.cleanup_exited_processes();

        // Check if this session already has a running process
        if let Some(process_id) = self.session_to_process.get(session_id).cloned() {
            // Check if the process is still running
            if self.is_running(&process_id) {
                return Ok(process_id);
            }
        }

        // Check concurrent limit
        if self.processes.len() >= MAX_CONCURRENT_PROCESSES {
            return Err(PikaProcessError::TooManyProcesses {
                max: MAX_CONCURRENT_PROCESSES,
            });
        }

        // Look up the session file to resume
        // Import the helper function from sessions module
        let session_file = crate::sessions::get_session_file_path(session_id, &project_path);

        if let Some(ref path) = session_file {
            info!(session_id = %session_id, path = ?path, "Resuming existing session");
        } else {
            info!(session_id = %session_id, "Starting new session without existing session file");
        }

        // Create process with session file if available
        let process = PikaProcess::spawn(project_path.clone(), session_file)?;
        let process_id = process.id.clone();

        // Subscribe to this process's events
        let mut rx = process.subscribe();
        let event_tx = self.event_tx.clone();
        let id_clone = process_id.clone();

        // Spawn a task to forward events from this process
        tokio::spawn(async move {
            while let Ok(event) = rx.recv().await {
                let _ = event_tx.send(ProcessManagerEvent::JsonRpc {
                    id: id_clone.clone(),
                    event,
                });
            }
        });

        // Store in map
        self.processes.insert(process_id.clone(), process);

        // Track session-to-process mapping
        self.session_to_process
            .insert(session_id.to_string(), process_id.clone());

        // Track process-to-session mapping (reverse lookup)
        self.process_to_session
            .insert(process_id.clone(), session_id.to_string());

        // Emit ProcessSpawned event
        let _ = self.event_tx.send(ProcessManagerEvent::ProcessSpawned {
            id: process_id.clone(),
            project_path: project_path.clone(),
        });

        // Emit SessionStarted event
        let _ = self.event_tx.send(ProcessManagerEvent::SessionStarted {
            session_id: session_id.to_string(),
            process_id: process_id.clone(),
        });

        Ok(process_id)
    }

    /// Check if a session has a running process
    pub fn is_session_running(&mut self, session_id: &str) -> bool {
        if let Some(process_id) = self.session_to_process.get(session_id).cloned() {
            self.is_running(&process_id)
        } else {
            false
        }
    }

    /// Get the process ID for a session (if running)
    pub fn get_process_id_for_session(&self, session_id: &str) -> Option<String> {
        self.session_to_process.get(session_id).cloned()
    }

    /// Get the session ID for a process (if running)
    pub fn get_session_id_for_process(&self, process_id: &str) -> Option<String> {
        self.process_to_session.get(process_id).cloned()
    }
}

/// Errors related to Pika process management
#[derive(Debug, Error)]
pub enum PikaProcessError {
    #[error("Project path not found: {path}")]
    ProjectNotFound { path: PathBuf },

    #[error("Invalid project path (not valid UTF-8): {path}")]
    InvalidPath { path: PathBuf },

    #[error("Failed to spawn Pika process for {path}: {source}")]
    SpawnFailed {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error(
        "Failed to spawn Pika process for {path}: unable to execute '{executable}'. Install Node.js and ensure npx is on PATH, or set PIKA_NPX_PATH to the full npx path"
    )]
    NpxNotFound {
        path: PathBuf,
        executable: String,
        #[source]
        source: std::io::Error,
    },

    #[error("Failed to get pipe from Pika process")]
    PipeFailed,

    #[error("Failed to write to process {id}: {source}")]
    WriteFailed {
        id: String,
        #[source]
        source: std::io::Error,
    },

    #[error("Failed to kill process {id}: {source}")]
    KillFailed {
        id: String,
        #[source]
        source: std::io::Error,
    },

    #[error("Process not found: {id}")]
    ProcessNotFound { id: String },

    #[error("Process {id} is no longer running")]
    ProcessNotRunning { id: String },

    #[error("Too many concurrent processes (max {max})")]
    TooManyProcesses { max: usize },
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{self, File};
    use std::time::{SystemTime, UNIX_EPOCH};

    fn unique_temp_dir(prefix: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("{}_{}", prefix, nanos))
    }

    #[test]
    fn test_json_rpc_event_parsing() {
        let json = r#"{"type":"message_update","message":{"role":"assistant"}}"#;
        let event: JsonRpcEvent = serde_json::from_str(json).unwrap();
        assert_eq!(event.event_type, Some("message_update".to_string()));
        assert!(event.extra.contains_key("message"));
    }

    #[test]
    fn test_json_rpc_event_with_extra_fields() {
        let json = r#"{"type":"notify","message":"test notification"}"#;
        let event: JsonRpcEvent = serde_json::from_str(json).unwrap();
        assert_eq!(event.event_type, Some("notify".to_string()));
        assert_eq!(
            event.extra.get("message").and_then(|v| v.as_str()),
            Some("test notification")
        );
    }

    #[test]
    fn test_process_manager_new() {
        let manager = ProcessManager::new();
        assert_eq!(manager.count(), 0);
        assert!(manager.list().is_empty());
    }

    #[test]
    fn test_process_not_running_error_display() {
        let err = PikaProcessError::ProcessNotRunning {
            id: "test-proc-123".to_string(),
        };
        let msg = format!("{}", err);
        assert!(
            msg.contains("test-proc-123"),
            "Error message should contain the process ID"
        );
        assert!(
            msg.to_lowercase().contains("no longer running")
                || msg.to_lowercase().contains("not running")
                || msg.to_lowercase().contains("stopped"),
            "Error message should indicate process is not running: {}",
            msg
        );
    }

    #[test]
    fn test_process_not_running_error_is_distinct_from_write_failed() {
        // ProcessNotRunning should be a separate variant from WriteFailed
        let not_running = PikaProcessError::ProcessNotRunning {
            id: "proc-1".to_string(),
        };
        let write_failed = PikaProcessError::WriteFailed {
            id: "proc-1".to_string(),
            source: std::io::Error::new(std::io::ErrorKind::BrokenPipe, "Broken pipe"),
        };

        // They should have different display messages
        assert_ne!(
            format!("{}", not_running),
            format!("{}", write_failed),
            "ProcessNotRunning and WriteFailed should produce different error messages"
        );
    }

    #[test]
    fn test_find_nvm_npx_under_prefers_latest_version_dir() {
        let root = unique_temp_dir("pika_nvm_test");
        let old_bin = root.join("v20.1.0").join("bin");
        let new_bin = root.join("v22.2.0").join("bin");
        fs::create_dir_all(&old_bin).unwrap();
        fs::create_dir_all(&new_bin).unwrap();
        File::create(old_bin.join(NPX_CANDIDATES[0])).unwrap();
        File::create(new_bin.join(NPX_CANDIDATES[0])).unwrap();

        let resolved = find_nvm_npx_under(&root).unwrap();
        assert!(resolved.starts_with(root.join("v22.2.0")));

        fs::remove_dir_all(root).unwrap();
    }
}
