//! File watching module for real-time session updates
//!
//! Watches the ~/.pi/agent/sessions/ directory for changes and broadcasts
//! WebSocket events when session files are created or modified.

use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::mpsc;
use tokio::sync::broadcast;

/// Events from the file watcher
#[derive(Debug, Clone)]
pub enum SessionFileEvent {
    /// A session file was created
    SessionFileCreated {
        project_path: PathBuf,
        session_id: String,
        file_path: PathBuf,
    },
    /// A session file was modified (new messages)
    SessionFileModified {
        project_path: PathBuf,
        session_id: String,
        file_path: PathBuf,
    },
    /// A session file was removed
    SessionFileRemoved {
        project_path: PathBuf,
        session_id: String,
        file_path: PathBuf,
    },
}

/// Watches the pi sessions directory for changes
pub struct SessionFileWatcher {
    _watcher: RecommendedWatcher,
    event_tx: broadcast::Sender<SessionFileEvent>,
    /// Map of encoded project names to their original paths (for lossless decoding)
    _encoded_project_map: HashMap<String, PathBuf>,
}

impl SessionFileWatcher {
    /// Create a new session file watcher
    /// Watches ~/.pi/agent/sessions/ for changes
    /// Takes a map of encoded project names to their original paths for lossless path resolution
    pub fn new(encoded_project_map: HashMap<String, PathBuf>) -> Result<Self, notify::Error> {
        let (event_tx, _) = broadcast::channel(1000);
        let event_tx_clone = event_tx.clone();

        // Create a channel for the synchronous file watcher
        let (sync_tx, sync_rx) = mpsc::channel::<Result<Event, notify::Error>>();

        // Create the watcher
        let watcher = RecommendedWatcher::new(
            move |res| {
                let _ = sync_tx.send(res);
            },
            Config::default(),
        )?;

        // Get the sessions directory
        let sessions_dir = dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".pi")
            .join("agent")
            .join("sessions");

        // Start a thread to process file events and forward to async channel
        let sessions_dir_clone = sessions_dir.clone();
        let encoded_project_map_clone = encoded_project_map.clone();
        std::thread::spawn(move || {
            Self::process_events(
                sync_rx,
                event_tx_clone,
                sessions_dir_clone,
                encoded_project_map_clone,
            );
        });

        Ok(SessionFileWatcher {
            _watcher: watcher,
            event_tx,
            _encoded_project_map: encoded_project_map,
        })
    }

    /// Start watching the sessions directory
    pub fn start_watching(&mut self) -> Result<(), notify::Error> {
        let sessions_dir = dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".pi")
            .join("agent")
            .join("sessions");

        // Create directory if it doesn't exist
        if !sessions_dir.exists()
            && let Err(e) = std::fs::create_dir_all(&sessions_dir)
        {
            eprintln!(
                "Failed to create sessions directory {:?}: {}",
                sessions_dir, e
            );
        }

        if sessions_dir.exists() {
            self._watcher
                .watch(&sessions_dir, RecursiveMode::Recursive)?;
            println!("👀 Watching sessions directory: {:?}", sessions_dir);
        } else {
            println!("⚠️ Sessions directory does not exist: {:?}", sessions_dir);
        }

        Ok(())
    }

    /// Subscribe to file events
    pub fn subscribe(&self) -> broadcast::Receiver<SessionFileEvent> {
        self.event_tx.subscribe()
    }

    /// Process file events from the synchronous watcher
    fn process_events(
        rx: mpsc::Receiver<Result<Event, notify::Error>>,
        tx: broadcast::Sender<SessionFileEvent>,
        sessions_dir: PathBuf,
        encoded_project_map: HashMap<String, PathBuf>,
    ) {
        for result in rx {
            match result {
                Ok(event) => {
                    // Only process .jsonl files
                    for path in &event.paths {
                        if path.extension().and_then(|s| s.to_str()) != Some("jsonl") {
                            continue;
                        }

                        // Extract project path and session ID from the file path
                        // Path format: ~/.pi/agent/sessions/--{encoded-project-path}--/{timestamp}_{session_id}.jsonl
                        if let Some((project_path, session_id)) =
                            Self::parse_session_path(path, &sessions_dir, &encoded_project_map)
                        {
                            let file_event = match event.kind {
                                EventKind::Create(_) => {
                                    Some(SessionFileEvent::SessionFileCreated {
                                        project_path,
                                        session_id,
                                        file_path: path.clone(),
                                    })
                                }
                                EventKind::Modify(_) => {
                                    Some(SessionFileEvent::SessionFileModified {
                                        project_path,
                                        session_id,
                                        file_path: path.clone(),
                                    })
                                }
                                EventKind::Remove(_) => {
                                    Some(SessionFileEvent::SessionFileRemoved {
                                        project_path,
                                        session_id,
                                        file_path: path.clone(),
                                    })
                                }
                                _ => None,
                            };

                            if let Some(evt) = file_event {
                                let _ = tx.send(evt);
                            }
                        }
                    }
                }
                Err(e) => {
                    eprintln!("File watcher error: {}", e);
                }
            }
        }
    }

    /// Parse a session file path to extract project path and session ID
    fn parse_session_path(
        path: &std::path::Path,
        sessions_dir: &std::path::Path,
        encoded_project_map: &HashMap<String, PathBuf>,
    ) -> Option<(PathBuf, String)> {
        // Get the relative path from sessions_dir
        let relative = path.strip_prefix(sessions_dir).ok()?;

        // First component should be the encoded project path (e.g., --home-leo-code-project--)
        let encoded_project = relative.components().next()?.as_os_str().to_str()?;

        // Look up the project path from the map (lossless)
        let project_path = encoded_project_map.get(encoded_project)?.clone();

        // Get the filename and extract session ID
        let filename = path.file_stem()?.to_str()?;

        // Filename format: {timestamp}_{session_id}
        let parts: Vec<&str> = filename.rsplitn(2, '_').collect();
        let session_id = if parts.len() == 2 {
            parts[0].to_string()
        } else {
            filename.to_string()
        };

        Some((project_path, session_id))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_session_path_with_lookup_map() {
        let sessions_dir = PathBuf::from("/home/youruser/.pi/agent/sessions");
        let mut encoded_map = HashMap::new();
        encoded_map.insert(
            "--home-leo-code-my-project--".to_string(),
            PathBuf::from("/home/youruser/code/my-project"),
        );

        let file_path = PathBuf::from(
            "/home/youruser/.pi/agent/sessions/--home-leo-code-my-project--/2025-01-13T00-00-00-000Z_abc123.jsonl",
        );

        let result =
            SessionFileWatcher::parse_session_path(&file_path, &sessions_dir, &encoded_map);
        assert!(result.is_some());
        let (project_path, session_id) = result.unwrap();
        assert_eq!(project_path, PathBuf::from("/home/youruser/code/my-project"));
        assert_eq!(session_id, "abc123");
    }
}
