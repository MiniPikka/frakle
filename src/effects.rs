// Visual effects: particles, screen shake, victory celebration.

use embedded_graphics::{
    geometry::Point,
    pixelcolor::Rgb888,
    prelude::*,
    primitives::{PrimitiveStyle, Rectangle, StyledDrawable},
};

use crate::framebuffer::{Framebuffer, COLOR_FARKLE, COLOR_TITLE, COLOR_TURN_SCORE, COLOR_SELECTED};

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
}

impl Effects {
    pub fn new(w: i32, _h: i32) -> Self {
        Self {
            particles: [PARTICLE_EMPTY; 64],
            particle_count: 0,
            shake_x: 0, shake_y: 0, shake_frames: 0,
            screen_w: w,
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
                      Rgb888::new(0xFF, 0x66, 0x66), Rgb888::new(0x66, 0xFF, 0x66),
                      Rgb888::new(0x66, 0x66, 0xFF)];
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

    pub fn tick(&mut self) {
        let mut j = 0;
        for i in 0..self.particle_count {
            let p = &mut self.particles[i];
            p.x += p.vx;
            p.y += p.vy;
            p.vy += 0.08;
            p.life -= 1.0;
            if p.life > 0.0 {
                if j != i {
                    self.particles[j] = *p;
                }
                j += 1;
            }
        }
        self.particle_count = j;

        if self.shake_frames > 0 {
            let intensity = (self.shake_frames as i32).min(8);
            self.shake_x = ((self.shake_frames as i32).wrapping_mul(13) & 0xF) % (intensity * 2 + 1) - intensity;
            self.shake_y = ((self.shake_frames as i32).wrapping_mul(7) & 0xF) % (intensity * 2 + 1) - intensity;
            self.shake_frames -= 1;
        } else {
            self.shake_x = 0;
            self.shake_y = 0;
        }
    }

    pub fn render(&self, fb: &mut Framebuffer) {
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
            if px >= 0 && py >= 0 && px < fb.width() as i32 && py < fb.height() as i32 {
                let size = if p.life > p.max_life * 0.6 { 2 } else { 1 };
                let _ = Rectangle::new(
                    Point::new(px, py), Size::new(size, size),
                ).draw_styled(&PrimitiveStyle::with_fill(color), fb);
            }
        }
    }
}

const PARTICLE_EMPTY: Particle = Particle {
    x: 0.0, y: 0.0, vx: 0.0, vy: 0.0,
    life: 0.0, max_life: 1.0,
    color: Rgb888::new(0, 0, 0),
};
