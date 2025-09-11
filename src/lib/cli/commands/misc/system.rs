use std::sync::Arc;

use tokio::sync::Mutex;
use sysinfo::System;

pub async fn handle_setup() {
    // tokio::spawn(crate::setup::install());
}

pub async fn handle_version(output_lines: &Arc<Mutex<Vec<String>>>) {
    let lines = vec![
        "███████     █████     ███    ███    ".to_string(),
        "██         ██   ██    ████  ████    ".to_string(),
        "███████    ███████    ██ ████ ██    ".to_string(),
        "     ██    ██   ██    ██  ██  ██    ".to_string(),
        "███████ ██ ██   ██ ██ ██      ██ ██ ".to_string(),
        "Smart Artificial Mind".to_string(),
        format!("VERSION: {:?}", crate::VERSION),
        "Copyright 2021-2026 The Open Sam Foundation (OSF)".to_string(),
        "Developed by Caleb Mitchell Smith (ktheindifferent, PixelCoda, p0indexter)".to_string(),
        "Licensed under GPLv3....see LICENSE file.".to_string(),
    ];
    let mut out = output_lines.lock().await;
    out.extend(lines);
}

pub async fn handle_default(cmd: &str, output_lines: &Arc<Mutex<Vec<String>>>) {
    match crate::services::rivescript::query(cmd) {
        Ok(reply) => {
            let text = reply.text.clone();
            let output_lines = output_lines.clone();
            tokio::spawn(crate::cli::helpers::append_and_tts(
                output_lines,
                format!("┌─[sam]─> {text}"),
            ));
        }
        Err(e) => {
            let mut out = output_lines.lock().await;
            out.push(format!("┌─[sam]─> [error: {e}]"));
        }
    }
}

// System Information Commands

/// Display system information (uname equivalent)
pub async fn handle_uname(cmd: &str, output_lines: &Arc<Mutex<Vec<String>>>) {
    let args: Vec<&str> = cmd.split_whitespace().collect();
    let mut show_all = false;
    let mut show_kernel = false;
    let mut show_nodename = false;
    let mut show_release = false;
    let mut show_version = false;
    let mut show_machine = false;
    let mut show_os = false;

    // Parse arguments
    for arg in args.iter().skip(1) {
        match *arg {
            "-a" | "--all" => show_all = true,
            "-s" | "--kernel-name" => show_kernel = true,
            "-n" | "--nodename" => show_nodename = true,
            "-r" | "--kernel-release" => show_release = true,
            "-v" | "--kernel-version" => show_version = true,
            "-m" | "--machine" => show_machine = true,
            "-o" | "--operating-system" => show_os = true,
            _ => {}
        }
    }

    // If no flags specified, show kernel name by default
    if !show_all && !show_kernel && !show_nodename && !show_release && !show_version && !show_machine && !show_os {
        show_kernel = true;
    }

    let mut result = Vec::new();
    
    if show_all || show_kernel {
        result.push(std::env::consts::OS.to_string());
    }
    if show_all || show_nodename {
        result.push(whoami::hostname());
    }
    if show_all || show_release {
        result.push(std::env::consts::OS.to_string());
    }
    if show_all || show_version {
        result.push("SAM".to_string());
    }
    if show_all || show_machine {
        result.push(std::env::consts::ARCH.to_string());
    }
    if show_all || show_os {
        result.push(format!("{}", std::env::consts::OS));
    }

    let mut out = output_lines.lock().await;
    out.push(result.join(" "));
}

/// Display current username (whoami equivalent)
pub async fn handle_whoami(output_lines: &Arc<Mutex<Vec<String>>>) {
    let mut out = output_lines.lock().await;
    out.push(whoami::username());
}

/// Display current date and time
pub async fn handle_date(cmd: &str, output_lines: &Arc<Mutex<Vec<String>>>) {
    use chrono::{Local, Utc};
    
    let args: Vec<&str> = cmd.split_whitespace().collect();
    let mut utc = false;
    let mut format = None;

    // Parse arguments
    for (i, arg) in args.iter().skip(1).enumerate() {
        match *arg {
            "-u" | "--utc" => utc = true,
            _ if arg.starts_with('+') => {
                format = Some(arg[1..].to_string());
            }
            _ => {}
        }
    }

    let mut out = output_lines.lock().await;
    
    if let Some(fmt) = format {
        // Custom format
        let formatted = if utc {
            Utc::now().format(&fmt).to_string()
        } else {
            Local::now().format(&fmt).to_string()
        };
        out.push(formatted);
    } else {
        // Default format
        let datetime_str = if utc {
            Utc::now().format("%a %b %e %H:%M:%S UTC %Y").to_string()
        } else {
            Local::now().format("%a %b %e %H:%M:%S %Z %Y").to_string()
        };
        out.push(datetime_str);
    }
}

/// Display disk space usage (df equivalent)
pub async fn handle_df(cmd: &str, output_lines: &Arc<Mutex<Vec<String>>>) {
    use sysinfo::System;
    
    let args: Vec<&str> = cmd.split_whitespace().collect();
    let mut human_readable = false;
    let mut show_inodes = false;
    let mut show_type = false;

    // Parse arguments
    for arg in args.iter().skip(1) {
        match *arg {
            "-h" | "--human-readable" => human_readable = true,
            "-i" | "--inodes" => show_inodes = true,
            "-T" | "--print-type" => show_type = true,
            _ => {}
        }
    }

    let mut sys = System::new_all();
    // sys.refresh_disks_list(); // Not available in current sysinfo version

    let mut out = output_lines.lock().await;
    
    // Header
    if show_inodes {
        out.push("Filesystem      Inodes   IUsed   IFree IUse% Mounted on".to_string());
    } else if show_type {
        out.push("Filesystem     Type     1K-blocks    Used Available Use% Mounted on".to_string());
    } else {
        out.push("Filesystem     1K-blocks    Used Available Use% Mounted on".to_string());
    }

    // Disk information - disabled due to sysinfo API changes
    // TODO: Update for newer sysinfo API
    out.push("Note: Disk information requires sysinfo API update".to_string());
}

/// Display disk usage of files and directories (du equivalent)
pub async fn handle_du(cmd: &str, output_lines: &Arc<Mutex<Vec<String>>>) {
    let args: Vec<&str> = cmd.split_whitespace().collect();
    let mut human_readable = false;
    let mut summarize = false;
    let mut max_depth = None;
    let mut paths = Vec::new();

    // Parse arguments
    let mut i = 1;
    while i < args.len() {
        match args[i] {
            "-h" | "--human-readable" => human_readable = true,
            "-s" | "--summarize" => summarize = true,
            "--max-depth" => {
                if i + 1 < args.len() {
                    max_depth = args[i + 1].parse::<usize>().ok();
                    i += 1;
                }
            }
            _ if !args[i].starts_with('-') => {
                paths.push(args[i]);
            }
            _ => {}
        }
        i += 1;
    }

    // Default to current directory if no paths specified
    if paths.is_empty() {
        paths.push(".");
    }

    let mut out = output_lines.lock().await;
    
    for path in paths {
        match calculate_directory_size(path, max_depth.unwrap_or(usize::MAX), 0) {
            Ok(size) => {
                let size_str = if human_readable {
                    format_bytes(size)
                } else {
                    (size / 1024).to_string() // Convert to KB
                };
                out.push(format!("{}\t{}", size_str, path));
            }
            Err(e) => {
                out.push(format!("du: cannot access '{}': {}", path, e));
            }
        }
    }
}

// Process Control Commands

/// Display running processes (ps equivalent)
pub async fn handle_ps(cmd: &str, output_lines: &Arc<Mutex<Vec<String>>>) {
    let args: Vec<&str> = cmd.split_whitespace().collect();
    let mut show_all = false;
    let mut show_full = false;
    let mut show_threads = false;

    // Parse arguments
    for arg in args.iter().skip(1) {
        match *arg {
            "-a" | "-A" | "--all" => show_all = true,
            "-f" | "--full" => show_full = true,
            "-T" | "--show-threads" => show_threads = true,
            "aux" => {
                show_all = true;
                show_full = true;
            }
            _ => {}
        }
    }

    let mut sys = System::new_all();
    sys.refresh_processes(sysinfo::ProcessesToUpdate::All, true);

    let mut out = output_lines.lock().await;
    
    // Header
    if show_full {
        out.push("  PID  PPID USER     %CPU %MEM    VSZ   RSS TTY      STAT START   TIME COMMAND".to_string());
    } else {
        out.push("  PID TTY          TIME CMD".to_string());
    }

    let processes: Vec<_> = sys.processes().iter().collect();
    
    for (pid, process) in processes {
        if !show_all && process.parent().is_none() {
            continue; // Skip processes without parent unless showing all
        }

        if show_full {
            let cpu = process.cpu_usage();
            let memory = process.memory();
            let virtual_memory = process.virtual_memory();
            let parent_pid = process.parent().map(|p| p.to_string()).unwrap_or_else(|| "    ".to_string());
            let user = whoami::username(); // Simplified - would need actual process owner
            let cmd = process.name();
            
            out.push(format!("{:5} {:5} {:<8} {:4.1} {:4.1} {:7} {:5} ?        S    00:00 {:8} {}", 
                pid, parent_pid, user, cpu, memory as f64 / 1024.0 / 1024.0, 
                virtual_memory / 1024, memory / 1024, "00:00", cmd.to_string_lossy()));
        } else {
            out.push(format!("{:5} pts/0    00:00:00 {}", pid, process.name().to_string_lossy()));
        }
    }
}

/// Display dynamic view of processes (top equivalent - simplified)
pub async fn handle_top(output_lines: &Arc<Mutex<Vec<String>>>) {
    let mut sys = System::new_all();
    sys.refresh_all();

    let mut out = output_lines.lock().await;
    
    // System summary
    out.push(format!("Tasks: {} total", sys.processes().len()));
    out.push(format!("CPU usage: {:.1}%", sys.global_cpu_usage()));
    out.push(format!("Memory: {} MB used, {} MB total", 
        sys.used_memory() / 1024 / 1024, 
        sys.total_memory() / 1024 / 1024));
    out.push("".to_string());
    
    // Header
    out.push("  PID USER      PR  NI    VIRT    RES    SHR S  %CPU %MEM     TIME+ COMMAND".to_string());
    
    // Get processes sorted by CPU usage
    let mut processes: Vec<_> = sys.processes().iter().collect();
    processes.sort_by(|a, b| b.1.cpu_usage().partial_cmp(&a.1.cpu_usage()).unwrap_or(std::cmp::Ordering::Equal));
    
    // Show top 20 processes
    for (pid, process) in processes.iter().take(20) {
        let cpu = process.cpu_usage();
        let memory = process.memory();
        let virtual_memory = process.virtual_memory();
        let user = whoami::username();
        let cmd = process.name();
        
        out.push(format!("{:5} {:<9} {:2} {:3} {:7} {:6} {:6} S {:5.1} {:4.1} {:8} {}", 
            pid, user, 20, 0, virtual_memory / 1024, memory / 1024, memory / 1024,
            cpu, memory as f64 / sys.total_memory() as f64 * 100.0, "0:00.00", cmd.to_string_lossy()));
    }
    
    out.push("".to_string());
    out.push("Note: This is a snapshot. Real top command updates continuously.".to_string());
}

/// Send signals to processes (kill equivalent)
pub async fn handle_kill(cmd: &str, output_lines: &Arc<Mutex<Vec<String>>>) {
    let args: Vec<&str> = cmd.split_whitespace().collect();
    
    if args.len() < 2 {
        let mut out = output_lines.lock().await;
        out.push("Usage: kill [-signal] pid [pid ...]".to_string());
        out.push("       kill -l [signal]".to_string());
        return;
    }

    let mut signal = "TERM"; // Default signal
    let mut pids = Vec::new();
    let mut list_signals = false;

    // Parse arguments
    let mut i = 1;
    while i < args.len() {
        let arg = args[i];
        if arg == "-l" || arg == "--list" {
            list_signals = true;
        } else if arg.starts_with('-') && arg.len() > 1 {
            signal = &arg[1..];
        } else if let Ok(pid) = arg.parse::<u32>() {
            pids.push(pid);
        } else {
            let mut out = output_lines.lock().await;
            out.push(format!("kill: invalid argument: {}", arg));
            return;
        }
        i += 1;
    }

    let mut out = output_lines.lock().await;
    
    if list_signals {
        out.push("Available signals:".to_string());
        out.push(" 1) SIGHUP       2) SIGINT       3) SIGQUIT      4) SIGILL       5) SIGTRAP".to_string());
        out.push(" 6) SIGABRT      7) SIGBUS       8) SIGFPE       9) SIGKILL     10) SIGUSR1".to_string());
        out.push("11) SIGSEGV     12) SIGUSR2     13) SIGPIPE     14) SIGALRM     15) SIGTERM".to_string());
        return;
    }

    if pids.is_empty() {
        out.push("Usage: kill [-signal] pid [pid ...]".to_string());
        return;
    }

    // Simplified kill implementation - in a real system you'd use platform-specific APIs
    for pid in pids {
        // Note: This is a simplified implementation
        // In a real cross-platform implementation, you'd need to use:
        // - Windows: OpenProcess, TerminateProcess APIs
        // - Unix/Linux: kill() system call
        // - macOS: Same as Unix/Linux
        
        out.push(format!("kill: sending {} signal to process {}", signal, pid));
        out.push("Note: Actual process termination requires platform-specific implementation.".to_string());
    }
}

/// Manual pages command (man equivalent)
pub async fn handle_man(cmd: &str, output_lines: &Arc<Mutex<Vec<String>>>) {
    let args: Vec<&str> = cmd.split_whitespace().collect();
    
    if args.len() < 2 {
        let mut out = output_lines.lock().await;
        out.push("Usage: man command".to_string());
        return;
    }

    let command = args[1];
    let mut out = output_lines.lock().await;
    
    let manual = get_command_manual(command);
    out.extend(manual);
}

// Helper functions

fn format_bytes(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "K", "M", "G", "T"];
    let mut size = bytes as f64;
    let mut unit_index = 0;
    
    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }
    
    if unit_index == 0 {
        format!("{:.0}{}", size, UNITS[unit_index])
    } else {
        format!("{:.1}{}", size, UNITS[unit_index])
    }
}

fn calculate_directory_size(path: &str, max_depth: usize, current_depth: usize) -> Result<u64, Box<dyn std::error::Error>> {
    use std::fs;
    use std::path::Path;

    let path = Path::new(path);
    let mut total_size = 0;

    if path.is_file() {
        return Ok(path.metadata()?.len());
    }

    if current_depth >= max_depth {
        return Ok(0);
    }

    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        
        if file_type.is_file() {
            total_size += entry.metadata()?.len();
        } else if file_type.is_dir() {
            let subdir_path = entry.path();
            if let Some(path_str) = subdir_path.to_str() {
                match calculate_directory_size(path_str, max_depth, current_depth + 1) {
                    Ok(size) => total_size += size,
                    Err(_) => {} // Skip directories we can't access
                }
            }
        }
    }

    Ok(total_size)
}

fn get_command_manual(command: &str) -> Vec<String> {
    match command {
        "uname" => vec![
            "NAME".to_string(),
            "    uname - print system information".to_string(),
            "".to_string(),
            "SYNOPSIS".to_string(),
            "    uname [OPTION]...".to_string(),
            "".to_string(),
            "DESCRIPTION".to_string(),
            "    Print certain system information. With no OPTION, same as -s.".to_string(),
            "".to_string(),
            "    -a, --all                print all information".to_string(),
            "    -s, --kernel-name        print the kernel name".to_string(),
            "    -n, --nodename          print the network node hostname".to_string(),
            "    -r, --kernel-release    print the kernel release".to_string(),
            "    -v, --kernel-version    print the kernel version".to_string(),
            "    -m, --machine           print the machine hardware name".to_string(),
            "    -o, --operating-system  print the operating system".to_string(),
        ],
        "whoami" => vec![
            "NAME".to_string(),
            "    whoami - print effective userid".to_string(),
            "".to_string(),
            "SYNOPSIS".to_string(),
            "    whoami".to_string(),
            "".to_string(),
            "DESCRIPTION".to_string(),
            "    Print the user name associated with the current effective user ID.".to_string(),
        ],
        "ps" => vec![
            "NAME".to_string(),
            "    ps - report a snapshot of the current processes".to_string(),
            "".to_string(),
            "SYNOPSIS".to_string(),
            "    ps [options]".to_string(),
            "".to_string(),
            "DESCRIPTION".to_string(),
            "    ps displays information about a selection of the active processes.".to_string(),
            "".to_string(),
            "    -a     show processes for all users".to_string(),
            "    -f     do full-format listing".to_string(),
            "    -T     show threads".to_string(),
            "    aux    show all processes in user-oriented format".to_string(),
        ],
        "top" => vec![
            "NAME".to_string(),
            "    top - display running processes".to_string(),
            "".to_string(),
            "SYNOPSIS".to_string(),
            "    top".to_string(),
            "".to_string(),
            "DESCRIPTION".to_string(),
            "    The top program provides a dynamic real-time view of a running system.".to_string(),
            "    It can display system summary information as well as a list of processes.".to_string(),
        ],
        "kill" => vec![
            "NAME".to_string(),
            "    kill - send a signal to a process".to_string(),
            "".to_string(),
            "SYNOPSIS".to_string(),
            "    kill [-signal] pid...".to_string(),
            "    kill -l [signal]".to_string(),
            "".to_string(),
            "DESCRIPTION".to_string(),
            "    The kill utility sends a signal to the processes specified by pid.".to_string(),
            "".to_string(),
            "    -l     list available signal names".to_string(),
            "    -signal specify which signal to send (default: TERM)".to_string(),
        ],
        "date" => vec![
            "NAME".to_string(),
            "    date - print or set the system date".to_string(),
            "".to_string(),
            "SYNOPSIS".to_string(),
            "    date [OPTION]... [+FORMAT]".to_string(),
            "".to_string(),
            "DESCRIPTION".to_string(),
            "    Display the current time in the given FORMAT, or set the system date.".to_string(),
            "".to_string(),
            "    -u, --utc          display or set time in UTC".to_string(),
            "    +FORMAT           specify display format".to_string(),
        ],
        "df" => vec![
            "NAME".to_string(),
            "    df - display filesystem disk space usage".to_string(),
            "".to_string(),
            "SYNOPSIS".to_string(),
            "    df [OPTION]... [FILE]...".to_string(),
            "".to_string(),
            "DESCRIPTION".to_string(),
            "    Show information about the file system on which each FILE resides,".to_string(),
            "    or all file systems by default.".to_string(),
            "".to_string(),
            "    -h, --human-readable  print sizes in human readable format".to_string(),
            "    -T, --print-type     print file system type".to_string(),
        ],
        "du" => vec![
            "NAME".to_string(),
            "    du - estimate file space usage".to_string(),
            "".to_string(),
            "SYNOPSIS".to_string(),
            "    du [OPTION]... [FILE]...".to_string(),
            "".to_string(),
            "DESCRIPTION".to_string(),
            "    Summarize disk usage of each FILE, recursively for directories.".to_string(),
            "".to_string(),
            "    -h, --human-readable     print sizes in human readable format".to_string(),
            "    -s, --summarize         display only a total for each argument".to_string(),
            "    --max-depth=N           limit recursion depth".to_string(),
        ],
        _ => vec![
            format!("No manual entry for {}", command),
            "".to_string(),
            "Available commands with manuals:".to_string(),
            "    uname, whoami, ps, top, kill, date, df, du".to_string(),
        ]
    }
}
