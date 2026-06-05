// Modular organization of misc commands
pub mod archive;
pub mod file_ops;
pub mod filesystem;
pub mod pager;
pub mod permissions;
pub mod system;
pub mod textproc;

// Re-export all public functions for backward compatibility
pub use archive::{handle_gunzip, handle_gzip, handle_tar};
pub use file_ops::{handle_find, handle_head, handle_tail, handle_touch};
pub use filesystem::{
    handle_cat, handle_clear, handle_cp, handle_ls, handle_mkdir, handle_mv, handle_pwd, handle_rm,
    handle_rmdir,
};
pub use pager::{handle_less, handle_less_nav};
pub use permissions::{handle_chmod, handle_chown};
pub use system::{
    handle_date, handle_default, handle_df, handle_du, handle_kill, handle_man, handle_ps,
    handle_setup, handle_top, handle_uname, handle_version, handle_whoami,
};
pub use textproc::{
    grep_file, handle_echo, handle_grep, handle_grep_with_input, handle_sort,
    handle_sort_with_input, handle_wc, handle_wc_with_input, GrepOptions,
};
