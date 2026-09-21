//! Named, versioned workspaces (Q-053).

pub mod resolve;
pub mod schema;
pub mod store;

pub use schema::{
    default_single, default_trading, parse_workspace, serialize_workspace, LoadError,
    WorkspaceFile, WorkspaceSelection, WorkspaceWindow, CURRENT_SCHEMA_VERSION,
};
pub use store::{default_workspaces_dir, WorkspaceReport, WorkspaceStore};
