use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Stdio;
use thiserror::Error;
use tokio::process::Command;
use tokio::sync::broadcast::{self, Receiver, Sender};
use tokio::task::JoinHandle;
use tokio::io::AsyncWriteExt;

/// Maximum number of concurrent pi processes allowed
const MAX_CONCURRENT_PROCESSES: usize = 50;

/// JSON-RPC event emitted by pi process
/// pi-coding-agent uses events with "type" field, not standard JSON-RPC
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcEvent {
    /// Event type (e.g., "message_update", "agent_start", "notify", etc.)
    #[serde(rename = "type")]
    pub event_type: Option<String>,
    /// All other fields from the event
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

/// Manages a pi subprocess
pub struct PiProcess {
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

impl PiProcess {
    /// Spawn a new pi process in RPC mode
    /// If session_file is provided, the process will resume that session
    pub fn spawn(project_path: PathBuf, session_file: Option<PathBuf>) -> Result<Self, PiProcessError> {
        // Validate project path exists
        if !project_path.exists() {
            return Err(PiProcessError::ProjectNotFound { path: project_path });
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

        // Spawn pi process with environment variables inherited
        let mut process = Command::new("npx")
            .args(&args)
            .current_dir(
                project_path.canonicalize().map_err(|_e| PiProcessError::InvalidPath {
                    path: project_path.clone(),
                })?
            )
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .env_clear()
            .envs(std::env::vars())
            .spawn()
            .map_err(|e| PiProcessError::SpawnFailed {
                path: project_path.clone(),
                source: e,
            })?;

        // Get stdin, stdout and stderr handles
        let stdin = process
            .stdin
            .take()
            .ok_or(PiProcessError::PipeFailed)?;
        let stdout = process
            .stdout
            .take()
            .ok_or(PiProcessError::PipeFailed)?;
        let stderr = process
            .stderr
            .take()
            .ok_or(PiProcessError::PipeFailed)?;

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

        Ok(PiProcess {
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

    /// Send a prompt to the pi process via stdin
    pub async fn send_prompt(&mut self, prompt: &str) -> Result<(), PiProcessError> {
        // Create RPC command (pi-coding-agent uses "type" not "method")
        let request = serde_json::json!({
            "type": "prompt",
            "message": prompt
        });

        let request_str = format!("{}\n", request);
        let stdin = &mut self.stdin;

        stdin
            .write_all(request_str.as_bytes())
            .await
            .map_err(|e| PiProcessError::WriteFailed {
                id: self.id.clone(),
                source: e,
            })?;

        stdin
            .flush()
            .await
            .map_err(|e| PiProcessError::WriteFailed {
                id: self.id.clone(),
                source: e,
            })?;

        Ok(())
    }

    /// Kill the pi process
    pub async fn kill(mut self) -> Result<(), PiProcessError> {
        self.process
            .kill()
            .await
            .map_err(|e| PiProcessError::KillFailed {
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
            Ok(None) => true,  // Process is still running
            Ok(Some(_)) => false,  // Process has exited
            Err(_) => false,  // Error checking status, assume not running
        }
    }

    /// Read stdout and parse JSON-RPC events
    async fn read_stdout(
        stdout: tokio::process::ChildStdout,
        tx: Sender<JsonRpcEvent>,
        _id: String,
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
                Err(_) => {
                    // Not valid JSON-RPC, ignore
                    // Could be other output from pi
                }
            }
        }
    }

    /// Read stderr and log (for debugging)
    async fn read_stderr(stderr: tokio::process::ChildStderr) {
        use tokio::io::{AsyncBufReadExt, BufReader};

        let reader = BufReader::new(stderr);
        let mut lines = reader.lines();

        while let Ok(Some(line)) = lines.next_line().await {
            eprintln!("pi stderr: {}", line);
        }
    }
}

/// Manages multiple pi processes
pub struct ProcessManager {
    /// Map of process ID to PiProcess
    processes: HashMap<String, PiProcess>,
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
    ProcessKilled { id: String },
    /// A JSON-RPC event from a process
    JsonRpc { id: String, event: JsonRpcEvent },
    /// A session was started
    SessionStarted { session_id: String, process_id: String },
}

impl Default for ProcessManager {
    fn default() -> Self {
        Self::new()
    }
}

impl ProcessManager {
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

    /// Spawn a new pi process
    /// If session_file is provided, the process will resume that session
    pub fn spawn(&mut self, project_path: PathBuf, session_file: Option<PathBuf>) -> Result<String, PiProcessError> {
        // Check concurrent limit
        if self.processes.len() >= MAX_CONCURRENT_PROCESSES {
            return Err(PiProcessError::TooManyProcesses {
                max: MAX_CONCURRENT_PROCESSES,
            });
        }

        // Create process
        let process = PiProcess::spawn(project_path.clone(), session_file)?;
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
    pub async fn kill(&mut self, id: &str) -> Result<(), PiProcessError> {
        let process = self
            .processes
            .remove(id)
            .ok_or(PiProcessError::ProcessNotFound {
                id: id.to_string(),
            })?;

        process.kill().await?;

        // Remove session-to-process mapping for this process
        self.session_to_process.retain(|_session, process_id| process_id != id);

        // Remove process-to-session mapping for this process
        self.process_to_session.remove(id);

        // Emit ProcessKilled event
        let _ = self.event_tx.send(ProcessManagerEvent::ProcessKilled {
            id: id.to_string(),
        });

        Ok(())
    }

    /// Subscribe to events from a specific process
    pub fn subscribe_to_process(&self, id: &str) -> Result<Receiver<JsonRpcEvent>, PiProcessError> {
        let process = self
            .processes
            .get(id)
            .ok_or(PiProcessError::ProcessNotFound {
                id: id.to_string(),
            })?;

        Ok(process.subscribe())
    }

    /// Get all active process IDs
    pub fn list(&self) -> Vec<String> {
        self.processes.keys().cloned().collect()
    }

    /// Check if a process is running
    pub fn is_running(&mut self, id: &str) -> bool {
        self.processes
            .get_mut(id)
            .map(|p| p.is_running())
            .unwrap_or(false)
    }

    /// Get number of active processes
    pub fn count(&self) -> usize {
        self.processes.len()
    }

    /// Send a prompt to a specific process by ID
    pub async fn send_prompt(&mut self, id: &str, prompt: &str) -> Result<(), PiProcessError> {
        // Remove the process temporarily to get mutable access
        let mut process = self
            .processes
            .remove(id)
            .ok_or(PiProcessError::ProcessNotFound {
                id: id.to_string(),
            })?;

        // Send the prompt
        process.send_prompt(prompt).await?;

        // Put the process back
        self.processes.insert(id.to_string(), process);

        Ok(())
    }

    /// Spawn a new pi process for a specific session
    /// Returns the process ID if spawned, or existing process ID if already running
    /// The process will resume the existing session file if it exists
    pub fn spawn_for_session(
        &mut self,
        session_id: &str,
        project_path: PathBuf,
    ) -> Result<String, PiProcessError> {
        // Check if this session already has a running process
        if let Some(process_id) = self.session_to_process.get(session_id).cloned() {
            // Check if the process is still running
            if self.is_running(&process_id) {
                return Ok(process_id);
            } else {
                // Process is dead, remove the mapping
                self.session_to_process.remove(session_id);
                self.processes.remove(&process_id);
            }
        }

        // Check concurrent limit
        if self.processes.len() >= MAX_CONCURRENT_PROCESSES {
            return Err(PiProcessError::TooManyProcesses {
                max: MAX_CONCURRENT_PROCESSES,
            });
        }

        // Look up the session file to resume
        // Import the helper function from sessions module
        let session_file = crate::sessions::get_session_file_path(session_id, &project_path);
        
        if let Some(ref path) = session_file {
            println!("📂 Resuming session {} from {:?}", session_id, path);
        } else {
            println!("📂 Starting new session {} (no existing session file found)", session_id);
        }

        // Create process with session file if available
        let process = PiProcess::spawn(project_path.clone(), session_file)?;
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
        self.session_to_process.insert(session_id.to_string(), process_id.clone());

        // Track process-to-session mapping (reverse lookup)
        self.process_to_session.insert(process_id.clone(), session_id.to_string());

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

/// Errors related to pi process management
#[derive(Debug, Error)]
pub enum PiProcessError {
    #[error("Project path not found: {path}")]
    ProjectNotFound { path: PathBuf },

    #[error("Invalid project path (not valid UTF-8): {path}")]
    InvalidPath { path: PathBuf },

    #[error("Failed to spawn pi process for {path}: {source}")]
    SpawnFailed {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("Failed to get pipe from pi process")]
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

    #[error("Too many concurrent processes (max {max})")]
    TooManyProcesses { max: usize },
}

#[cfg(test)]
mod tests {
    use super::*;

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
        assert_eq!(event.extra.get("message").and_then(|v| v.as_str()), Some("test notification"));
    }

    #[test]
    fn test_process_manager_new() {
        let manager = ProcessManager::new();
        assert_eq!(manager.count(), 0);
        assert!(manager.list().is_empty());
    }
}
