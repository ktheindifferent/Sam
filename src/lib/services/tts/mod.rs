// ███████     █████     ███    ███
// ██         ██   ██    ████  ████
// ███████    ███████    ██ ████ ██
//      ██    ██   ██    ██  ██  ██
// ███████ ██ ██   ██ ██ ██      ██ ██
// Copyright 2021-2026 The Open Sam Foundation (OSF)
// Developed by Caleb Mitchell Smith (ktheindifferent, PixelCoda, p0indexter)
// Licensed under GPLv3....see LICENSE file.

pub mod enhanced;
pub mod external;
pub mod legacy;

pub use enhanced::{AudioFormat, TtsConfig, TtsRequest, TtsResult, TtsService};

// Re-export legacy functions for backward compatibility
pub use legacy::{
    fetch_local, fetch_online, get, handle, init, tts_cross_platform, tts_cross_platform_wav,
};
