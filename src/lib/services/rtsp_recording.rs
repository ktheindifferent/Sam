// RTSP Recording Module - Test Implementation that re-exports rtsp_dl_test
//! Test stub for rtsp_recording module to allow testing without dependencies

#[cfg(test)]
pub use super::rtsp_dl_test::*;

#[cfg(not(test))]
// For non-test builds, provide minimal stubs
pub struct RecordingManager;
#[cfg(not(test))]
pub struct RecordingConfig;
