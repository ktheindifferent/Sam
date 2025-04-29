use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;

pub async fn handle_status(
    output_lines: &Arc<Mutex<Vec<String>>>,
    current_dir: &PathBuf,
    human_name: &str,
) {
    // ...existing code from original status match arm...
    let mut sys = sysinfo::System::new_all();
    sys.refresh_all();
    let pid = sysinfo::get_current_pid().ok();
    let process = pid.and_then(|p| sys.process(p));
    let mem_total = sys.total_memory();
    let mem_used = sys.used_memory();
    let cpu_usage = process.map(|proc| proc.cpu_usage()).unwrap_or(0.0);
    let mem_proc = process.map(|proc| proc.memory()).unwrap_or(0);
    let os = sysinfo::System::name().unwrap_or_else(|| "Unknown".to_string());
    let os_ver = sysinfo::System::os_version().unwrap_or_default();
    let kernel = sysinfo::System::kernel_version().unwrap_or_default();
    let arch = std::env::consts::ARCH;
    let exe = std::env::current_exe().ok().and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_string())).unwrap_or_else(|| "Unknown".to_string());
    let version = format!("{:?}", libsam::VERSION);

    let lines = vec![
        format!("Executable: {}", exe),
        format!("User: {}", human_name),
        format!("Current Directory: {}", current_dir.display()),
        format!("PID: {}", pid.map(|p| p.as_u32()).unwrap_or(0)),
        format!("Version: {}", version),
        format!("OS: {} {} ({})", os, os_ver, arch),
        format!("Kernel: {}", kernel),
        format!("CPU Usage: {:.2}%", cpu_usage),
        format!("Process Memory: {} MiB", mem_proc / 1024 / 1024),
        format!("System Memory: {} MiB used / {} MiB total", mem_used / 1024, mem_total / 1024),
        format!("PID: {}", pid.map(|p| p.as_u32()).unwrap_or(0)),
    ];
    let mut out = output_lines.lock().await;
    out.extend(lines);
}
