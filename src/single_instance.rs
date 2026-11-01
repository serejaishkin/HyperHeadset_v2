//! Keeps one running copy: on start, any other running copy of this app (either front-end) is
//! closed, since two copies fight over the HID device and the audio defaults.

#[cfg(target_os = "windows")]
mod imp {
    #[repr(C)]
    struct ProcessEntry32W {
        size: u32,
        usage: u32,
        pid: u32,
        heap_id: usize,
        module_id: u32,
        threads: u32,
        parent_pid: u32,
        pri_class: i32,
        flags: u32,
        exe: [u16; 260],
    }

    #[link(name = "kernel32")]
    extern "system" {
        fn CreateToolhelp32Snapshot(flags: u32, pid: u32) -> isize;
        fn Process32FirstW(snap: isize, entry: *mut ProcessEntry32W) -> i32;
        fn Process32NextW(snap: isize, entry: *mut ProcessEntry32W) -> i32;
        fn OpenProcess(access: u32, inherit: i32, pid: u32) -> isize;
        fn TerminateProcess(h: isize, code: u32) -> i32;
        fn WaitForSingleObject(h: isize, ms: u32) -> u32;
        fn CloseHandle(h: isize) -> i32;
        fn GetCurrentProcessId() -> u32;
    }

    const TH32CS_SNAPPROCESS: u32 = 0x2;
    const PROCESS_TERMINATE: u32 = 0x0001;
    const SYNCHRONIZE: u32 = 0x0010_0000;

    pub fn kill_duplicates() {
        let mut names: Vec<String> = vec![
            "hyperx_ngenuity_open.exe".into(),
            "hyperx-ngenuity-open.exe".into(),
            "hyperx-ngenuity-open-tauri.exe".into(),
        ];
        if let Some(own) = std::env::current_exe().ok().and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_lowercase())) {
            if !names.contains(&own) {
                names.push(own);
            }
        }
        unsafe {
            let me = GetCurrentProcessId();
            let snap = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
            if snap == -1 || snap == 0 {
                return;
            }
            let mut e: ProcessEntry32W = std::mem::zeroed();
            e.size = std::mem::size_of::<ProcessEntry32W>() as u32;
            let mut ok = Process32FirstW(snap, &mut e);
            while ok != 0 {
                let len = e.exe.iter().position(|&c| c == 0).unwrap_or(e.exe.len());
                let name = String::from_utf16_lossy(&e.exe[..len]).to_lowercase();
                if e.pid != me && names.contains(&name) {
                    let h = OpenProcess(PROCESS_TERMINATE | SYNCHRONIZE, 0, e.pid);
                    if h != 0 {
                        log::info!("[Main] Closing duplicate instance {} (pid {})", name, e.pid);
                        TerminateProcess(h, 0);
                        WaitForSingleObject(h, 2000);
                        CloseHandle(h);
                    } else {
                        log::warn!("[Main] Could not close duplicate {} (pid {})", name, e.pid);
                    }
                }
                ok = Process32NextW(snap, &mut e);
            }
            CloseHandle(snap);
        }
    }
}

#[cfg(all(unix, not(target_os = "macos")))]
mod imp {
    pub fn kill_duplicates() {
        let me = std::process::id();
        let Some(own) = std::env::current_exe().ok().and_then(|p| p.file_name().map(|n| n.to_os_string())) else { return };
        let Ok(dir) = std::fs::read_dir("/proc") else { return };
        for ent in dir.flatten() {
            let Some(pid) = ent.file_name().to_str().and_then(|s| s.parse::<u32>().ok()) else { continue };
            if pid == me {
                continue;
            }
            let same = std::fs::read_link(format!("/proc/{}/exe", pid))
                .ok()
                .and_then(|p| p.file_name().map(|n| n.to_os_string()))
                .map_or(false, |n| n == own);
            if same {
                log::info!("[Main] Closing duplicate instance (pid {})", pid);
                let _ = std::process::Command::new("kill").arg(pid.to_string()).status();
            }
        }
    }
}

#[cfg(not(any(target_os = "windows", all(unix, not(target_os = "macos")))))]
mod imp {
    pub fn kill_duplicates() {}
}

/// Close every other running copy of this app. Call once at startup, after logging is set up.
pub fn kill_duplicates() {
    imp::kill_duplicates();
}
