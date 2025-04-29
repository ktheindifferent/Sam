use std::env;
use std::process::{Command, Stdio};
use std::fs;
use std::path::Path;
use std::io::{self, Write};
use crate::sam;
#[cfg(target_os = "windows")]
const OS: &str = "windows";
#[cfg(target_os = "macos")]
const OS: &str = "macos";
#[cfg(target_os = "linux")]
const OS: &str = "linux";

fn main() {
    crate::sam::print_banner();
    match OS {
        "windows" => {
      
        },
        _ => {
           
        }
    }
}