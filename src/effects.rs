// Visual effects: particles, screen shake, victory celebration.

use embedded_graphics::pixelcolor::Rgb888;
use embedded_graphics::prelude::RgbColor;

use crate::framebuffer::{Framebuffer, COLOR_FARKLE, COLOR_TITLE, COLOR_TURN_SCORE, COLOR_SELECTED, COLOR_BUTTON_ROLL};

const DIR16: [(f32, f32); 16] = [
    (1.000, 0.000), (0.924, 0.383), (0.707, 0.707), (0.383, 0.924),
    (0.000, 1.000), (-0.383, 0.924), (-0.707, 0.707), (-0.924, 0.383),
    (-1.000, 0.000), (-0.924, -0.383), (-0.707, -0.707), (-0.383, -0.924),
    (0.000, -1.000), (0.383, -0.924), (0.707, -0.707), (0.924, -0.383),
];

const DIR24: [(f32, f32); 24] = [
    (1.000, 0.000), (0.966, 0.259), (0.866, 0.500), (0.707, 0.707),
    (0.500, 0.866), (0.259, 0.966), (0.000, 1.000), (-0.259, 0.966),
    (-0.500, 0.866), (-0.707, 0.707), (-0.866, 0.500), (-0.966, 0.259),
    (-1.000, 0.000), (-0.966, -0.259), (-0.866, -0.500), (-0.707, -0.707),
    (-0.500, -0.866), (-0.259, -0.966), (0.000, -1.000), (0.259, -0.966),
    (0.500, -0.866), (0.707, -0.707), (0.866, -0.500), (0.966, -0.259),
];

#[derive(Clone, Copy)]
pub struct Particle {
    pub x: f32, pub y: f32,
    pub vx: f32, pub vy: f32,
    pub life: f32, pub max_life: f32,
    pub color: Rgb888,
}

pub struct Effects {
    pub particles: [Particle; 64],
    pub particle_count: usize,
    pub shake_x: i32,
    pub shake_y: i32,
    pub shake_frames: u32,
    screen_w: i32,
    screen_h: i32,

    /// Full-screen color flash overlay (fades over `flash_frames` ticks).
    flash_color: Rgb888,
    flash_frames: u32,
    flash_max_frames: u32,

    /// Animated score display: smoothly counts from `anim_score` → actual total.
    pub anim_scores: [u32; 2],  // per-player animated score
    pub anim_speed: u32,        // points per frame during animation

    /// Global tick counter — drives title breathing, etc.
    pub global_tick: u32,
}

impl Effects {
    pub fn new(w: i32, h: i32) -> Self {
        Self {
            particles: [PARTICLE_EMPTY; 64],
            particle_count: 0,
            shake_x: 0, shake_y: 0, shake_frames: 0,
            screen_w: w,
            screen_h: h,
            flash_color: Rgb888::new(0, 0, 0),
            flash_frames: 0,
            flash_max_frames: 1,
            anim_scores: [0, 0],
            anim_speed: 30,  // 30 points per frame ≈ 0.5s for 1000pts
            global_tick: 0,
        }
    }

    pub fn spawn_score_pop(&mut self, x: i32, y: i32, _score: u32) {
        for (i, &(dx, dy)) in DIR16.iter().enumerate() {
            if self.particle_count >= 64 { break; }
            let speed = 0.8 + (i as f32) * 0.15;
            self.particles[self.particle_count] = Particle {
                x: x as f32, y: y as f32,
                vx: dx * speed, vy: dy * speed - 1.5,
                life: 25.0, max_life: 25.0,
                color: if i % 3 == 0 { COLOR_TITLE } else if i % 3 == 1 { COLOR_TURN_SCORE } else { COLOR_SELECTED },
            };
            self.particle_count += 1;
        }
    }

    pub fn spawn_farkle(&mut self, cx: i32, cy: i32) {
        self.shake_frames = 20;
        for (i, &(dx, dy)) in DIR24.iter().enumerate() {
            if self.particle_count >= 64 { break; }
            let speed = 1.0 + (i % 4) as f32 * 0.5;
            self.particles[self.particle_count] = Particle {
                x: cx as f32, y: cy as f32,
                vx: dx * speed, vy: dy * speed - 1.0,
                life: 18.0, max_life: 18.0,
                color: COLOR_FARKLE,
            };
            self.particle_count += 1;
        }
    }

    pub fn spawn_victory(&mut self) {
        let w = self.screen_w;
        let colors = [COLOR_TITLE, COLOR_TURN_SCORE, COLOR_SELECTED,
                      COLOR_FARKLE, COLOR_BUTTON_ROLL,
                      Rgb888::new(0x80, 0xD0, 0xFF)];
        for i in 0..48 {
            if self.particle_count >= 64 { break; }
            let x = ((i * 137 + 53) % w as usize) as f32;
            let (dx, _dy) = DIR24[i % 24];
            let speed = 1.5 + ((i * 7) % 5) as f32 * 0.5;
            self.particles[self.particle_count] = Particle {
                x, y: -10.0,
                vx: dx * speed * 0.3, vy: 1.5 + (i as f32) * 0.15,
                life: 150.0 + (i as f32) * 2.0, max_life: 150.0 + (i as f32) * 2.0,
                color: colors[i % colors.len()],
            };
            self.particle_count += 1;
        }
    }

    pub fn center_x(&self) -> i32 { self.screen_w / 2 }
    pub fn center_y(&self) -> i32 { self.screen_h / 2 }

    /// Trigger a full-screen color flash overlay.
    pub fn flash(&mut self, color: Rgb888, frames: u32) {
        self.flash_color = color;
        self.flash_frames = frames;
        self.flash_max_frames = frames;
    }

    /// Update animated scores toward actual totals. Call each frame.
    pub fn update_anim_scores(&mut self, actual: &[u32; 2]) {
        for (i, &target) in actual.iter().enumerate() {
            if self.anim_scores[i] < target {
                let diff = target - self.anim_scores[i];
                let step = diff.min(self.anim_speed);
                self.anim_scores[i] += step;
                if self.anim_scores[i] + 2 >= target {
                    self.anim_scores[i] = target;
                }
            } else if self.anim_scores[i] > target {
                self.anim_scores[i] = target;
            }
        }
    }

    /// Title text breathing value: returns 0.6..1.0 brightness multiplier.
    /// Creates a smooth pulsing glow on the title text.
    pub fn title_breathe(&self) -> f32 {
        // Integer sin-approximation: triangle wave with period ~120 frames (2s)
        let t = self.global_tick % 120;
        let phase = if t < 60 { t } else { 120 - t } as f32;
        0.6 + 0.4 * (phase / 60.0)  // 0.6..1.0
    }

    pub fn tick(&mut self) {
        self.global_tick = self.global_tick.wrapping_add(1);

        // Advance particles
        let mut j = 0;
        for i in 0..self.particle_count {
            let p = &mut self.particles[i];
            p.x += p.vx;
            p.y += p.vy;
            p.vy += 0.08;
            p.life -= 1.0;
            if p.life > 0.0 {
                if j != i { self.particles[j] = *p; }
                j += 1;
            }
        }
        self.particle_count = j;

        // Advance screen shake
        if self.shake_frames > 0 {
            let intensity = (self.shake_frames as i32).min(8);
            self.shake_x = ((self.shake_frames as i32).wrapping_mul(13) & 0xF) % (intensity * 2 + 1) - intensity;
            self.shake_y = ((self.shake_frames as i32).wrapping_mul(7) & 0xF) % (intensity * 2 + 1) - intensity;
            self.shake_frames -= 1;
        } else {
            self.shake_x = 0;
            self.shake_y = 0;
        }

        // Advance flash overlay
        if self.flash_frames > 0 {
            self.flash_frames -= 1;
        }
    }

    /// Render all live particles + flash overlay.
    pub fn render(&self, fb: &mut Framebuffer) {
        // Particles
        for i in 0..self.particle_count {
            let p = &self.particles[i];
            let alpha = (p.life / p.max_life).clamp(0.0, 1.0);
            if alpha <= 0.0 { continue; }
            let r = (p.color.r() as f32 * alpha) as u8;
            let g = (p.color.g() as f32 * alpha) as u8;
            let b = (p.color.b() as f32 * alpha) as u8;
            let color = Rgb888::new(r, g, b);
            let px = p.x as i32 + self.shake_x;
            let py = p.y as i32 + self.shake_y;
            if p.life > p.max_life * 0.6 {
                fb.set_pixel(px, py, color);
                fb.set_pixel(px + 1, py, color);
                fb.set_pixel(px, py + 1, color);
                fb.set_pixel(px + 1, py + 1, color);
            } else {
                fb.set_pixel(px, py, color);
            }
        }

        // Full-screen flash overlay (additive blend, fading out)
        if self.flash_frames > 0 {
            let fade = self.flash_frames;
            let max = self.flash_max_frames.max(1);
            let intensity = (fade * 16 / max).min(16);  // 0..16
            if intensity > 0 {
                let fr = self.flash_color.r() as u32 * intensity / 16;
                let fg = self.flash_color.g() as u32 * intensity / 16;
                let fb_c = self.flash_color.b() as u32 * intensity / 16;
                let w = fb.width();
                let h = fb.height();
                let buf = fb.buffer_direct();
                for y in 0..h {
                    let row_start = y * w;
                    for x in 0..w {
                        let px = buf[row_start + x];
                        let b = ((px & 0xFF) + fb_c).min(255);
                        let g = (((px >> 8) & 0xFF) + fg).min(255);
                        let r = (((px >> 16) & 0xFF) + fr).min(255);
                        buf[row_start + x] = b | (g << 8) | (r << 16);
                    }
                }
                fb.mark_dirty();
            }
        }
    }
}

const PARTICLE_EMPTY: Particle = Particle {
    x: 0.0, y: 0.0, vx: 0.0, vy: 0.0,
    life: 0.0, max_life: 1.0,
    color: Rgb888::new(0, 0, 0),
};
