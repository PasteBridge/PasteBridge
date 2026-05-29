use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::time::{Instant, Duration};

#[cfg(target_os = "windows")]
use windows::Win32::System::ProcessStatus::{GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS};
#[cfg(target_os = "windows")]
use windows::Win32::System::Threading::GetCurrentProcess;
#[cfg(target_os = "windows")]
use windows::Win32::System::Memory::{SetProcessWorkingSetSizeEx, SETPROCESSWORKINGSETSIZEEX_FLAGS};

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

    pub fn update(&self) -> u64 {
        #[cfg(target_os = "windows")]
        {
            let memory_used = self.get_process_memory_windows();
            // 使用 Relaxed 排序对于性能更好，因为这些指标不控制逻辑流程
            self.current_memory.store(memory_used, Ordering::Relaxed);
            
            // 原子性地更新最大值
            self.peak_memory.fetch_max(memory_used, Ordering::Relaxed);
            self.update_count.fetch_add(1, Ordering::Relaxed);
            
            memory_used
        }
        #[cfg(not(target_os = "windows"))]
        {
            0
        }
    }

    #[cfg(target_os = "windows")]
    fn get_process_memory_windows(&self) -> u64 {
        unsafe {
            let handle = GetCurrentProcess(); // 获取伪句柄，无需 OpenProcess
            let mut counters = PROCESS_MEMORY_COUNTERS::default();
            counters.cb = std::mem::size_of::<PROCESS_MEMORY_COUNTERS>() as u32;

            if GetProcessMemoryInfo(handle, &mut counters, counters.cb).is_ok() {
                return counters.WorkingSetSize as u64;
            }
            0
        }
    }

    /// 释放闲置内存回操作系统
    #[cfg(target_os = "windows")]
    pub fn trim_working_set(&self) -> Option<u64> {
        let before = self.get_current_memory();
        unsafe {
            // 将工作集大小设为 -1 (usize::MAX) 会触发内存页清理
            if SetProcessWorkingSetSizeEx(
                GetCurrentProcess(),
                usize::MAX,
                usize::MAX,
                SETPROCESSWORKINGSETSIZEEX_FLAGS(0),
            ).is_ok() {
                let after = self.update();
                return Some(before.saturating_sub(after));
            }
        }
        None
    }

    #[cfg(not(target_os = "windows"))]
    pub fn trim_working_set(&self) -> Option<u64> { None }

    // 获取方法建议使用 Relaxed 排序
    pub fn get_current_memory(&self) -> u64 { self.current_memory.load(Ordering::Relaxed) }
    pub fn get_peak_memory(&self) -> u64 { self.peak_memory.load(Ordering::Relaxed) }
    pub fn get_update_count(&self) -> usize { self.update_count.load(Ordering::Relaxed) }
    pub fn get_uptime(&self) -> Duration { self.start_time.elapsed() }

    pub fn format_memory(bytes: u64) -> String {
        let kb = 1024.0;
        let mb = kb * 1024.0;
        let gb = mb * 1024.0;
        let b = bytes as f64;

        if b >= gb { format!("{:.2} GB", b / gb) }
        else if b >= mb { format!("{:.2} MB", b / mb) }
        else if b >= kb { format!("{:.2} KB", b / kb) }
        else { format!("{} B", bytes) }
    }
}