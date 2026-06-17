// Non-blocking PC speaker sound effects.
//
// The PC speaker is driven by PIT channel 2 + the 0x61 gate. Once programmed,
// the PIT generates the square wave autonomously — the CPU does nothing to
// sustain it. This lets us produce real audio *without* stalling the game loop.
//
// Timing is frame-driven: `tick()` is called once per frame (~16ms, matching
// `FRAME_DELAY_US` in main.rs) and only touches the hardware at note
// boundaries — reprogramming the PIT divisor and toggling the gate. No
// `bs.stall()` is ever called, so sound never steals from the frame budget.

use core::arch::asm;

// Predefined sound effect sequences: (frequency_hz, duration_us)
//
// Design notes:
// - All effects <150ms total — snappy, never block the game loop
// - Pitch direction = semantic: rising = positive, falling = negative
// - Frequencies stay >250Hz (PC speaker drops off below ~200Hz)
// - Dice roll uses rapid alternation to simulate physical rattle

// Quick rattling burst — alternating high freqs mimic dice bounce (~75ms)
pub const SND_ROLL: &[(u32, u64)] = &[
    (1600, 12000), (2000, 12000), (1400, 12000),
    (1800, 12000), (2200, 12000), (1200, 15000),
];

// Rising arpeggio — "ka-ching" satisfaction (~55ms)
pub const SND_BANK: &[(u32, u64)] = &[
    (800, 18000), (1200, 18000), (1600, 20000),
];

// Descending boo — quick disappointed tone (~60ms)
pub const SND_FARKLE: &[(u32, u64)] = &[
    (600, 20000), (400, 20000), (300, 20000),
];

// Triumphant fanfare — C-E-G-C' major arpeggio (~125ms)
pub const SND_VICTORY: &[(u32, u64)] = &[
    (523, 30000), (659, 30000), (784, 30000), (1047, 35000),
];

// PIT base clock: 1.193182 MHz
const PIT_CLOCK: u32 = 1_193_182;
// One game frame, in microseconds — must match FRAME_DELAY_US in main.rs.
const FRAME_US: u64 = 16_000;
// PIT chip I/O ports.
const PIT_CMD: u16 = 0x43;
const PIT_CH2: u16 = 0x42;
const SPKR_CTL: u16 = 0x61;

#[inline]
unsafe fn outb(port: u16, val: u8) {
    asm!("out dx, al", in("dx") port, in("al") val, options(nomem, nostack, preserves_flags));
}

#[inline]
unsafe fn inb(port: u16) -> u8 {
    let val: u8;
    asm!("in al, dx", in("dx") port, out("al") val, options(nomem, nostack, preserves_flags));
    val
}

/// Convert a microsecond duration into the number of frames it spans.
/// Rounds up so the minimum note length is always one frame.
const fn us_to_frames(us: u64) -> u32 {
    (us.div_ceil(FRAME_US)) as u32
}

/// Non-blocking sound queue. Plays a sequence of notes cooperatively:
/// the PIT keeps beeping between frames, and `tick()` advances to the
/// next note only when the current note's frame budget is spent.
pub struct SoundQueue {
    notes: &'static [(u32, u64)],
    note_idx: usize,
    frames_left: u32,
    playing: bool,
}

impl Default for SoundQueue {
    fn default() -> Self { Self::new() }
}

impl SoundQueue {
    pub fn new() -> Self {
        Self { notes: &[], note_idx: 0, frames_left: 0, playing: false }
    }

    pub fn play(&mut self, notes: &'static [(u32, u64)]) {
        // Cut off whatever is currently playing and start the new effect.
        if self.playing {
            unsafe { speaker_off(); }
        }
        self.notes = notes;
        self.note_idx = 0;
        self.frames_left = if notes.is_empty() { 0 } else { us_to_frames(notes[0].1) };
        self.playing = !notes.is_empty();
        if self.playing {
            set_freq(notes[0].0);
            unsafe { speaker_on(); }
        }
    }

    /// Returns `true` while a sound is still playing.
    pub fn tick(&mut self) -> bool {
        if !self.playing {
            return false;
        }
        if self.frames_left > 1 {
            self.frames_left -= 1;
            return true;
        }
        // Current note finished — advance to the next one.
        self.note_idx += 1;
        if self.note_idx >= self.notes.len() {
            unsafe { speaker_off(); }
            self.playing = false;
            return false;
        }
        let (freq, dur) = self.notes[self.note_idx];
        self.frames_left = us_to_frames(dur);
        set_freq(freq);
        // Gate stays on across notes for a continuous tone; only the
        // divisor changes, so we don't need to re-toggle the speaker.
        true
    }

    /// Silence immediately (e.g. on quit). Safe to call any time.
    pub fn stop(&mut self) {
        if self.playing {
            unsafe { speaker_off(); }
            self.playing = false;
        }
    }
}

impl Drop for SoundQueue {
    fn drop(&mut self) {
        // Never leave the speaker squealing.
        if self.playing {
            unsafe { speaker_off(); }
        }
    }
}

fn set_freq(freq: u32) {
    // Clamp: too low a frequency overflows the 16-bit divisor; 0 is invalid.
    let f = freq.max(19); // 19 Hz → divisor 62799, near the 16-bit ceiling
    let divisor = PIT_CLOCK / f;
    unsafe {
        outb(PIT_CMD, 0xB6); // channel 2, lo-byte/hi-byte, mode 3 (square wave)
        outb(PIT_CH2, (divisor & 0xFF) as u8);
        outb(PIT_CH2, ((divisor >> 8) & 0xFF) as u8);
    }
}

unsafe fn speaker_on() {
    // Set bits 0 and 1: gate PIT ch2 to the speaker and enable ch2 access.
    outb(SPKR_CTL, inb(SPKR_CTL) | 0x03);
}

unsafe fn speaker_off() {
    // Clear bits 0 and 1 without disturbing the upper bits.
    outb(SPKR_CTL, inb(SPKR_CTL) & !0x03);
}
