// Non-blocking PC speaker sound effects.
// Notes are played cooperatively: one short segment per frame tick.
// This allows visual effects and sound to run simultaneously.

use core::arch::asm;

const PIT_CMD: u16 = 0x43;
const PIT_CH2: u16 = 0x42;
const SPKR_CTL: u16 = 0x61;
const TICK_US: u64 = 16_000; // one frame at ~60fps

unsafe fn outb(port: u16, val: u8) {
    asm!("out dx, al", in("dx") port, in("al") val, options(nomem, nostack, preserves_flags));
}
unsafe fn inb(port: u16) -> u8 {
    let val: u8;
    asm!("in al, dx", in("dx") port, out("al") val, options(nomem, nostack, preserves_flags));
    val
}

// Predefined sound effect sequences: (frequency_hz, duration_us)
pub const SND_ROLL: &[(u32, u64)] = &[
    (800, 20000), (1000, 20000), (800, 20000), (1000, 20000),
    (800, 20000), (1000, 20000), (1200, 30000),
];
pub const SND_BANK: &[(u32, u64)] = &[(600, 80000), (900, 80000), (1200, 100000)];
pub const SND_FARKLE: &[(u32, u64)] = &[(200, 100000), (150, 200000)];
pub const SND_VICTORY: &[(u32, u64)] = &[
    (523, 120000), (659, 120000), (784, 120000), (1047, 200000),
];

pub struct SoundQueue {
    notes: &'static [(u32, u64)],
    note_idx: usize,
    elapsed_us: u64,
    note_started: bool,
}

impl SoundQueue {
    pub fn new() -> Self {
        Self { notes: &[], note_idx: 0, elapsed_us: 0, note_started: false }
    }

    pub fn play(&mut self, notes: &'static [(u32, u64)]) {
        self.notes = notes;
        self.note_idx = 0;
        self.elapsed_us = 0;
        self.note_started = false;
    }

    /// Call once per frame. Plays a short segment of the current note.
    /// Returns true while sound is playing.
    pub fn tick(&mut self) -> bool {
        if self.note_idx >= self.notes.len() {
            // All notes played, ensure speaker is off
            if self.note_started {
                unsafe { speaker_off(); }
                self.note_started = false;
            }
            return false;
        }

        let (freq, total_dur) = self.notes[self.note_idx];

        if !self.note_started {
            set_freq(freq);
            unsafe { speaker_on(); }
            self.note_started = true;
            self.elapsed_us = 0;
        }

        // Play for one frame
        let play_us = TICK_US.min(total_dur - self.elapsed_us);
        uefi::boot::stall(core::time::Duration::from_micros(play_us));
        self.elapsed_us += play_us;

        if self.elapsed_us >= total_dur {
            unsafe { speaker_off(); }
            self.note_started = false;
            self.note_idx += 1;
            self.elapsed_us = 0;
            // Small gap between notes
            uefi::boot::stall(core::time::Duration::from_micros(5000));
        }

        true
    }
}

fn set_freq(freq: u32) {
    let divisor = 1193180u32 / freq;
    unsafe {
        outb(PIT_CMD, 0xB6);
        outb(PIT_CH2, (divisor & 0xFF) as u8);
        outb(PIT_CH2, ((divisor >> 8) & 0xFF) as u8);
    }
}

unsafe fn speaker_on() { outb(SPKR_CTL, inb(SPKR_CTL) | 3); }
unsafe fn speaker_off() { outb(SPKR_CTL, inb(SPKR_CTL) & !3); }
