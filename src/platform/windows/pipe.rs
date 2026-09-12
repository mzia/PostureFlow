// Windows Named Pipe IPC constants & protocol
pub const POSTUREFLOW_PIPE_NAME: &str = r"\\.\pipe\PostureFlowPipe";

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub enum PipeRequest {
    GetActiveProfile,
    ApplyProfile(String),
    GetStatus,
    ResetDefaults,
    BlockPort { port: u16, proto: String },
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub enum PipeResponse {
    ActiveProfile(String),
    Status(String),
    Success(String),
    Error(String),
}
