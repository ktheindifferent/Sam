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
    SshManager, SshConfig, AuthMethod, CommandResult, TransferResult,
    FileInfo, ConnectionInfo, SshCommandBuilder, SshSession
};

pub use server::{
    RemoteAccessServer, start_ssh_server, is_ssh_server_running, 
    get_ssh_server_status, TuiHandle
};