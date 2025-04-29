// ███████     █████     ███    ███    
// ██         ██   ██    ████  ████    
// ███████    ███████    ██ ████ ██    
//      ██    ██   ██    ██  ██  ██    
// ███████ ██ ██   ██ ██ ██      ██ ██ 
// Copyright 2021-2026 The Open Sam Foundation (OSF)
// Developed by Caleb Mitchell Smith (ktheindifferent, PixelCoda, p0indexter)
// Licensed under GPLv3....see LICENSE file.

pub mod http;
pub mod memory;
pub mod services;
pub mod setup;
pub mod tools;
pub mod cli;

pub fn print_banner() {
    println!("███████     █████     ███    ███    ");
    println!("██         ██   ██    ████  ████    ");
    println!("███████    ███████    ██ ████ ██    ");
    println!("     ██    ██   ██    ██  ██  ██    ");
    println!("███████ ██ ██   ██ ██ ██      ██ ██ ");
    println!("Smart Artificial Mind");
    println!("VERSION: {:?}", VERSION);
    println!("Copyright 2021-2026 The Open Sam Foundation (OSF)");
    println!("Developed by Caleb Mitchell Smith (ktheindifferent, PixelCoda, p0indexter)");
    println!("Licensed under GPLv3....see LICENSE file.");
    println!("================================================");
    println!("Hello {}....SAM is starting up...", user);
    println!("================================================");
}