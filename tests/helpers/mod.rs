use std::sync::Once;
use std::path::PathBuf;
use tokio::runtime::Runtime;

static INIT: Once = Once::new();

pub fn setup_test_environment() {
    INIT.call_once(|| {
        std::env::set_var("RUST_LOG", "debug");
        env_logger::init();
    });
}

pub fn get_test_runtime() -> Runtime {
    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .expect("Failed to build test runtime")
}

pub fn get_test_data_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests").join("data")
}

pub fn create_temp_dir() -> tempfile::TempDir {
    tempfile::tempdir().expect("Failed to create temp directory")
}

#[macro_export]
macro_rules! assert_err {
    ($result:expr) => {
        assert!($result.is_err(), "Expected an error but got: {:?}", $result);
    };
}

#[macro_export]
macro_rules! assert_ok {
    ($result:expr) => {
        assert!($result.is_ok(), "Expected Ok but got error: {:?}", $result);
    };
}