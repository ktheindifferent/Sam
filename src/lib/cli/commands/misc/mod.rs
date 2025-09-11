// Modular organization of misc commands
pub mod filesystem;
pub mod pager;
pub mod textproc;
pub mod system;
pub mod file_ops;
pub mod permissions;
pub mod archive;

// Re-export all public functions for backward compatibility
pub use filesystem::{handle_clear, handle_ls, handle_cat, handle_pwd, handle_mkdir, handle_rmdir, handle_cp, handle_mv, handle_rm};
pub use pager::{handle_less, handle_less_nav};
pub use textproc::{handle_grep, handle_grep_with_input, grep_file, GrepOptions, handle_echo, handle_sort, handle_sort_with_input, handle_wc, handle_wc_with_input};
pub use system::{handle_setup, handle_version, handle_default, handle_uname, handle_whoami, handle_date, handle_df, handle_du, handle_ps, handle_top, handle_kill, handle_man};
pub use file_ops::{handle_touch, handle_head, handle_tail, handle_find};
pub use permissions::{handle_chmod, handle_chown};
pub use archive::{handle_tar, handle_gzip, handle_gunzip};
