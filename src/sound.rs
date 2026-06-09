// Sound effect stubs. No audio hardware access (no unsafe code).
// Kept as hooks for future PC Speaker / audio driver implementation.

// Predefined sound effect sequences for reference: (frequency_hz, duration_us)
pub const SND_ROLL: &[(u32, u64)] = &[
    (800, 20000), (1000, 20000), (800, 20000), (1000, 20000),
    (800, 20000), (1000, 20000), (1200, 30000),
];
pub const SND_BANK: &[(u32, u64)] = &[(600, 80000), (900, 80000), (1200, 100000)];
pub const SND_FARKLE: &[(u32, u64)] = &[(200, 100000), (150, 200000)];
pub const SND_VICTORY: &[(u32, u64)] = &[
    (523, 120000), (659, 120000), (784, 120000), (1047, 200000),
];

/// Minimal sound queue stub — no audio is produced.
pub struct SoundQueue;

impl Default for SoundQueue {
    fn default() -> Self { Self }
}

impl SoundQueue {
    pub fn new() -> Self { Self }
    pub fn play(&mut self, _notes: &'static [(u32, u64)]) {}
    pub fn tick(&mut self) {}
}
