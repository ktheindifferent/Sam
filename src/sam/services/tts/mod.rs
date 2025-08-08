// ███████     █████     ███    ███
// ██         ██   ██    ████  ████
// ███████    ███████    ██ ████ ██
//      ██    ██   ██    ██  ██  ██
// ███████ ██ ██   ██ ██ ██      ██ ██
// Copyright 2021-2026 The Open Sam Foundation (OSF)
// Developed by Caleb Mitchell Smith (ktheindifferent, PixelCoda, p0indexter)
// Licensed under GPLv3....see LICENSE file.

pub mod enhanced;
pub mod legacy;

pub use enhanced::{TtsConfig, TtsService, TtsRequest, TtsResult, AudioFormat};

// Re-export legacy functions for backward compatibility
pub use legacy::{handle, init, get, fetch_online, fetch_local, tts_cross_platform, tts_cross_platform_wav};