use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::time::{Instant, Duration};

#[cfg(target_os = "windows")]
use windows::Win32::System::ProcessStatus::{GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS};
#[cfg(target_os = "windows")]
use windows::Win32::Foundation::CloseHandle;
#[cfg(target_os = "windows")]
use windows::Win32::System::Threading::{OpenProcess, PROCESS_QUERY_INFORMATION};
#[cfg(target_os = "windows")]
use windows::Win32::System::ProcessStatus::EmptyWorkingSet;
#[cfg(target_os = "windows")]
use windows::Win32::System::Threading::GetCurrentProcess;

pub struct MemoryMonitor {
    start_time: Instant,
    peak_memory: AtomicU64,
    current_memory: AtomicU64,
    update_count: AtomicUsize,
}

impl MemoryMonitor {
    pub fn new() -> Self {
        Self {
            start_time: Instant::now(),
            peak_memory: AtomicU64::new(0),
            current_memory: AtomicU64::new(0),
            update_count: AtomicUsize::new(0),
        }
    }

    #[cfg(target_os = "windows")]
    pub fn update(&self) -> u64 {
        let memory_used = self.get_process_memory_windows();
        self.current_memory.store(memory_used, Ordering::SeqCst);
        
        let current = self.current_memory.load(Ordering::SeqCst);
        let peak = self.peak_memory.load(Ordering::SeqCst);
        if current > peak {
            self.peak_memory.store(current, Ordering::SeqCst);
        }
        
        self.update_count.fetch_add(1, Ordering::SeqCst);
        memory_used
    }

    #[cfg(not(target_os = "windows"))]
    pub fn update(&self) -> u64 {
        0
    }

    #[cfg(target_os = "windows")]
    fn get_process_memory_windows(&self) -> u64 {
        unsafe {
            let pid = windows::Win32::System::Threading::GetCurrentProcessId();
            let handle = OpenProcess(PROCESS_QUERY_INFORMATION, false, pid).ok();
            
            if let Some(handle) = handle {
                let mut counters = PROCESS_MEMORY_COUNTERS::default();
                counters.cb = std::mem::size_of::<PROCESS_MEMORY_COUNTERS>() as u32;
                
                if GetProcessMemoryInfo(handle, &mut counters, counters.cb).is_ok() {
                    let working_set = counters.WorkingSetSize as u64;
                    CloseHandle(handle).ok();
                    return working_set;
                }
                CloseHandle(handle).ok();
            }
            0
        }
    }

    #[cfg(not(target_os = "windows"))]
    fn get_process_memory_windows(&self) -> u64 {
        0
    }

    pub fn get_current_memory(&self) -> u64 {
        self.current_memory.load(Ordering::SeqCst)
    }

    pub fn get_peak_memory(&self) -> u64 {
        self.peak_memory.load(Ordering::SeqCst)
    }

    pub fn get_update_count(&self) -> usize {
        self.update_count.load(Ordering::SeqCst)
    }

    pub fn get_uptime(&self) -> Duration {
        self.start_time.elapsed()
    }

    pub fn format_memory(bytes: u64) -> String {
        const KB: u64 = 1024;
        const MB: u64 = KB * 1024;
        const GB: u64 = MB * 1024;

        if bytes >= GB {
            format!("{:.2} GB", bytes as f64 / GB as f64)
        } else if bytes >= MB {
            format!("{:.2} MB", bytes as f64 / MB as f64)
        } else if bytes >= KB {
            format!("{:.2} KB", bytes as f64 / KB as f64)
        } else {
            format!("{} B", bytes)
        }
    }
}

impl MemoryMonitor {
    /// Try to release the process working set back to the OS (Windows only).
    /// Returns true on success.
    #[cfg(target_os = "windows")]
    pub fn minimize_working_set() -> bool {
        unsafe {
            let handle = GetCurrentProcess();
            EmptyWorkingSet(handle).is_ok()
        }
    }

    #[cfg(not(target_os = "windows"))]
    pub fn minimize_working_set() -> bool {
        // No-op on non-windows platforms
        false
    }
}

impl Default for MemoryMonitor {
    fn default() -> Self {
        Self::new()
    }
}
