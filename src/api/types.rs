//! API types module - request/response types for all API endpoints.
//! 
//! Organized by domain:
//! - auth: Authentication types
//! - projects: Project management types
//! - sessions: Session and message types
//! - settings: Settings types
//! - shared: Shared types used across domains

pub mod auth;
pub mod projects;
pub mod sessions;
pub mod settings;
pub mod shared;

// Re-export commonly used types for convenience
pub use auth::{AuthStatusResponse, LoginRequest, LoginResponse};
pub use projects::{AddProjectRequest, AddProjectResponse, ProjectResponse};
pub use sessions::{
    CreateSessionInProjectRequest, CreateSessionInProjectResponse, CreateStandaloneSessionRequest,
    CreateStandaloneSessionResponse, MessageResponse, PromptRequest, SessionResponse,
    SessionStatusResponse, SetModelRequest, SetThinkingLevelRequest, StartSessionResponse,
    StopSessionResponse,
};
pub use settings::{ModelInfo, PikaSettingsResponse, UpdatePikaSettingsRequest};
pub use shared::{
    ErrorResponse, ImageAttachment, ImageAttachmentResponse, PagedResponse, PagedSessionsQuery,
    SessionMessagesPagedQuery, SessionMessagesQuery, SessionsLookupRequest,
};
