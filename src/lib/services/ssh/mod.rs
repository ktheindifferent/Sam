//! SSH module containing both client and server functionality
//!
//! This module provides comprehensive SSH capabilities for the SAM system:
//! - SSH client for connecting to and managing remote systems
//! - SSH server for providing remote access to SAM
//!
//! ## Client Module
//! The client module (`ssh::client`) provides:
//! - SSH connection management with multiple authentication methods
//! - Command execution (single, batch, and streaming)
//! - File transfer capabilities (upload/download via SFTP)
//! - Directory listing and file operations
//! - SSH tunneling support
//! - Connection pooling and cleanup
//!
//! ## Server Module
//! The server module (`ssh::server`) provides:
//! - Simple SSH-like server for remote access to SAM
//! - Authentication with configurable credentials
//! - Command interface for basic system operations
//! - Session management
//! - Integration hooks for TUI access

pub mod client;
pub mod server;

// Re-export commonly used types for convenience
pub use client::{
    AuthMethod, CommandResult, ConnectionInfo, FileInfo, SshCommandBuilder, SshConfig, SshManager,
    SshSession, TransferResult,
};

pub use server::{
    get_ssh_server_status, is_ssh_server_running, start_ssh_server, RemoteAccessServer, TuiHandle,
};
