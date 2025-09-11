// Modular organization of misc commands
pub mod filesystem;
pub mod pager;
pub mod textproc;
pub mod system;

// Re-export all public functions for backward compatibility
pub use filesystem::{handle_clear, handle_ls, handle_cat, handle_pwd, handle_mkdir, handle_rmdir, handle_cp, handle_mv, handle_rm};
pub use pager::{handle_less, handle_less_nav};
pub use textproc::{handle_grep, handle_grep_with_input, grep_file, GrepOptions};
pub use system::{handle_setup, handle_version, handle_default};
