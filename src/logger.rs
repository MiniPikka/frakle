//! File-based debug logger for Farkle

use spin::Mutex;
use uefi::prelude::*;
use uefi::proto::media::file::{File, FileAttribute, FileMode, RegularFile};
use uefi::proto::media::fs::SimpleFileSystem;

static COUNT: Mutex<u32> = Mutex::new(0);

fn raw_write(image: Handle, bs: &BootServices, data: &[u8]) -> bool {
    let Ok(mut fs): Result<uefi::table::boot::ScopedProtocol<SimpleFileSystem>, _> =
        bs.get_image_file_system(image) else { return false; };
    let Ok(mut root) = fs.open_volume() else { return false; };

    let Ok(file_handle) = root
        .open(cstr16!("\\frakle_debug.log"), FileMode::ReadWrite, FileAttribute::empty())
        .or_else(|_| root.open(cstr16!("\\frakle_debug.log"), FileMode::CreateReadWrite, FileAttribute::empty()))
    else { return false; };

    let Some(mut file) = file_handle.into_regular_file() else { return false; };
    let _ = file.set_position(RegularFile::END_OF_FILE);
    file.write(data).is_ok()
}

pub fn log(image: Handle, bs: &BootServices, msg: &str) -> bool {
    let count = {
        let mut c = COUNT.lock();
        *c += 1;
        *c
    };
    let line = alloc::format!("[{count:04}] {msg}\r\n");
    raw_write(image, bs, line.as_bytes())
}

pub fn init(image: Handle, bs: &BootServices) -> Result<(), &'static str> {
    {
        let mut fs: uefi::table::boot::ScopedProtocol<SimpleFileSystem> =
            bs.get_image_file_system(image).map_err(|_| "No FS")?;
        let mut root = fs.open_volume().map_err(|_| "No vol")?;

        // Delete old log if present
        if let Ok(h) = root.open(cstr16!("\\frakle_debug.log"), FileMode::ReadWrite, FileAttribute::empty()) {
            let _ = h.delete();
        }

        // Create new
        let h = root.open(
            cstr16!("\\frakle_debug.log"), FileMode::CreateReadWrite, FileAttribute::empty()
        ).map_err(|_| "Create fail")?;
        let mut f: RegularFile = h.into_regular_file().ok_or("Not file")?;
        f.write(b"=== Farkle Debug Log ===\r\n").map_err(|_| "Write fail")?;
        let _ = f.flush();
    }
    // fs/root/f dropped here — file closed

    log(image, bs, "=== Farkle Debug Log Started ===");
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub fn log_game_state(
    image: Handle, bs: &BootServices,
    phase: &str, player: usize, score: u32, turn_score: u32,
    dice: &[u8; 6], held: &[bool; 6],
) -> bool {
    let msg = alloc::format!(
        "State: phase={} player={} score={} turn={} dice={:?} held={:?}",
        phase, player, score, turn_score, dice, held,
    );
    log(image, bs, &msg)
}

pub fn log_memory_usage(image: Handle, bs: &BootServices, game_size: usize) -> bool {
    let msg = alloc::format!("Memory: Game struct {} bytes", game_size);
    log(image, bs, &msg)
}
