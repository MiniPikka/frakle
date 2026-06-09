//! File-based debug logger for Farkle
//!
//! Writes log messages to `\frakle_debug.log` on the EFI System Partition.
//! Reference: UHIA implementation (uefi 0.28)

use spin::Mutex;
use uefi::prelude::*;
use uefi::proto::media::file::{File, FileAttribute, FileMode, RegularFile};
use uefi::proto::media::fs::SimpleFileSystem;

/// Logger state protected by spinlock
struct LoggerState {
    count: u32,
}

impl LoggerState {
    const fn new() -> Self {
        Self { count: 0 }
    }

    fn next_count(&mut self) -> u32 {
        self.count += 1;
        self.count
    }
}

// Use spin::Mutex for thread-safe interior mutability
static LOGGER: Mutex<LoggerState> = Mutex::new(LoggerState::new());

// ── Low-level file I/O ───────────────────────────────────────────────────────

/// Write raw bytes to the log file (appends)
fn raw_write(image: Handle, bs: &BootServices, data: &[u8]) -> Result<(), ()> {
    let mut fs: uefi::table::boot::ScopedProtocol<SimpleFileSystem> =
        bs.get_image_file_system(image).map_err(|_| ())?;
    let mut root = fs.open_volume().map_err(|_| ())?;

    let file_handle = root
        .open(cstr16!("\\frakle_debug.log"), FileMode::ReadWrite, FileAttribute::empty())
        .or_else(|_| {
            root.open(cstr16!("\\frakle_debug.log"), FileMode::CreateReadWrite, FileAttribute::empty())
        })
        .map_err(|_| ())?;

    let mut file: RegularFile = file_handle.into_regular_file().ok_or(())?;
    file.set_position(RegularFile::END_OF_FILE).map_err(|_| ())?;
    file.write(data).map_err(|_| ())?;
    file.flush().ok();
    Ok(())
}

/// Log a message
pub fn log(image: Handle, bs: &BootServices, msg: &str) {
    let count = LOGGER.lock().next_count();

    // Build log line
    let line = alloc::format!("[{count:04}] {msg}\r\n");

    // Try to write to file
    let _ = raw_write(image, bs, line.as_bytes());
}

/// Initialize logger - create/reset log file
pub fn init(image: Handle, bs: &BootServices) -> Result<(), &'static str> {
    // Try to create/reset log file
    reset_log_file(image, bs)?;

    // Log startup message
    log(image, bs, "=== Farkle Debug Log Started ===");

    Ok(())
}

/// Reset log file (for real hardware testing)
fn reset_log_file(image: Handle, bs: &BootServices) -> Result<(), &'static str> {
    let mut fs: uefi::table::boot::ScopedProtocol<SimpleFileSystem> =
        bs.get_image_file_system(image).map_err(|_| "Failed to get filesystem")?;
    let mut root = fs.open_volume().map_err(|_| "Failed to open volume")?;

    // Delete existing file if present
    if let Ok(file_handle) = root.open(
        cstr16!("\\frakle_debug.log"),
        FileMode::ReadWrite,
        FileAttribute::empty(),
    ) {
        let _ = file_handle.delete();
    }

    // Create new file
    let handle = root
        .open(
            cstr16!("\\frakle_debug.log"),
            FileMode::CreateReadWrite,
            FileAttribute::empty(),
        )
        .map_err(|_| "Failed to create log file")?;

    let mut file: RegularFile = handle.into_regular_file().ok_or("Failed to get regular file")?;
    file.write(b"=== Farkle Debug Log ===\r\n").map_err(|_| "Failed to write header")?;
    file.flush().ok();

    Ok(())
}

/// Log game state to file
#[allow(clippy::too_many_arguments)]
pub fn log_game_state(
    image: Handle,
    bs: &BootServices,
    phase: &str,
    player: usize,
    score: u32,
    turn_score: u32,
    dice: &[u8; 6],
    held: &[bool; 6],
) {
    let msg = alloc::format!(
        "State: phase={} player={} score={} turn={} dice={:?} held={:?}",
        phase, player, score, turn_score, dice, held,
    );
    log(image, bs, &msg);
}

/// Log memory usage
pub fn log_memory_usage(image: Handle, bs: &BootServices, game_size: usize) {
    let msg = alloc::format!("Memory: Game struct {} bytes", game_size);
    log(image, bs, &msg);
}

/// Debug log macro (no-op in release; use logger::log() for actual logging)
#[macro_export]
macro_rules! debug_log {
    ($($arg:tt)*) => {};
}