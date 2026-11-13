//! Small framebuffer UI. The glyphs are a tiny in-tree 5x7 bitmap font.

use crate::Framebuffer;
use core::{ptr, sync::atomic::{AtomicU8, AtomicUsize, Ordering}};

#[derive(Clone, Copy)]
struct Color(u8, u8, u8);

struct Canvas<'a> { fb: &'a Framebuffer }
static INPUT: [AtomicU8; 80] = [const { AtomicU8::new(b' ') }; 80];
static INPUT_LEN: AtomicUsize = AtomicUsize::new(0);
impl Canvas<'_> {
    fn color(&self, c: Color) -> u32 {
        let r = (c.0 as u32) & ((1u32 << self.fb.red_mask_size) - 1);
        let g = (c.1 as u32) & ((1u32 << self.fb.green_mask_size) - 1);
        let b = (c.2 as u32) & ((1u32 << self.fb.blue_mask_size) - 1);
        (r << self.fb.red_mask_shift) | (g << self.fb.green_mask_shift) | (b << self.fb.blue_mask_shift)
    }
    fn rect(&self, x: u64, y: u64, w: u64, h: u64, color: Color) {
        if self.fb.bpp != 32 || x >= self.fb.width || y >= self.fb.height { return; }
        let x_end = x.saturating_add(w).min(self.fb.width);
        let y_end = y.saturating_add(h).min(self.fb.height);
        let value = self.color(color);
        for py in y..y_end {
            for px in x..x_end {
                let at = unsafe { self.fb.address.add((py * self.fb.pitch + px * 4) as usize) as *mut u32 };
                unsafe { ptr::write_volatile(at, value); }
            }
        }
    }
    fn text(&self, x: u64, y: u64, text: &str, scale: u64, color: Color) {
        let mut cursor = x;
        for ch in text.bytes() {
            let glyph = glyph(ch as char);
            for (row, bits) in glyph.iter().enumerate() {
                for col in 0..5 {
                    if bits & (1 << (4 - col)) != 0 {
                        self.rect(cursor + col * scale, y + row as u64 * scale, scale, scale, color);
                    }
                }
            }
            cursor += 6 * scale;
        }
    }
}

pub fn draw_desktop(fb: &Framebuffer) {
    let c = Canvas { fb };
    if fb.bpp != 32 || fb.width < 200 || fb.height < 140 { return; }
    c.rect(0, 0, fb.width, fb.height, Color(17, 27, 47));
    c.rect(0, 0, fb.width, 30, Color(11, 17, 31));
    c.text(14, 10, "OS 28", 1, Color(238, 242, 250));
    c.text(fb.width.saturating_sub(110), 10, "RUST DESKTOP", 1, Color(170, 188, 213));

    // Main app window: crisp flat surfaces, thin borders, no transparency or glass effect.
    let ww = (fb.width * 3 / 5).clamp(260, 720).min(fb.width - 32);
    let wh = (fb.height * 3 / 5).clamp(180, 460).min(fb.height - 112);
    let wx = (fb.width - ww) / 2;
    let wy = (fb.height - wh) / 2;
    c.rect(wx, wy, ww, wh, Color(29, 41, 63));
    c.rect(wx, wy, ww, 32, Color(39, 53, 78));
    c.rect(wx, wy, ww, 1, Color(91, 111, 144));
    c.rect(wx, wy + wh - 1, ww, 1, Color(91, 111, 144));
    c.rect(wx, wy, 1, wh, Color(91, 111, 144));
    c.rect(wx + ww - 1, wy, 1, wh, Color(91, 111, 144));
    c.rect(wx + 12, wy + 12, 8, 8, Color(238, 99, 91));
    c.rect(wx + 26, wy + 12, 8, 8, Color(232, 185, 78));
    c.rect(wx + 40, wy + 12, 8, 8, Color(80, 190, 117));
    c.text(wx + 62, wy + 12, "WELCOME", 1, Color(238, 242, 250));
    c.text(wx + 28, wy + 68, "RUST DESKTOP", 2, Color(239, 244, 252));
    c.text(wx + 30, wy + 100, "KERNEL READY", 1, Color(163, 184, 214));
    c.text(wx + 30, wy + 126, "PREEMPTIVE TASKS: 2", 1, Color(163, 184, 214));
    c.text(wx + 30, wy + 148, "INPUT: PS/2 KEYBOARD", 1, Color(163, 184, 214));

    let dock_w = 220.min(fb.width - 32);
    let dock_x = (fb.width - dock_w) / 2;
    let dock_y = fb.height - 56;
    c.rect(dock_x, dock_y, dock_w, 42, Color(31, 44, 66));
    for i in 0..5 {
        let x = dock_x + 14 + i * 40;
        let tint = [Color(96, 154, 230), Color(104, 190, 156), Color(224, 164, 83), Color(162, 132, 220), Color(106, 178, 198)][i as usize];
        c.rect(x, dock_y + 8, 26, 26, tint);
        c.rect(x + 5, dock_y + 5, 16, 2, Color(235, 241, 250));
    }
}

pub fn draw_key(fb: &Framebuffer, byte: u8) {
    if fb.bpp != 32 || fb.width < 200 || fb.height < 140 { return; }
    let len = INPUT_LEN.load(Ordering::Relaxed);
    if byte == 8 {
        if len > 0 { INPUT_LEN.store(len - 1, Ordering::Relaxed); }
    } else if byte != b'\n' && byte != b'\t' {
        let ch = byte.to_ascii_uppercase();
        if (32..=126).contains(&ch) && len < INPUT.len() {
            INPUT[len].store(ch, Ordering::Relaxed);
            INPUT_LEN.store(len + 1, Ordering::Relaxed);
        }
    }
    let c = Canvas { fb };
    let ww = (fb.width * 3 / 5).clamp(260, 720).min(fb.width - 32);
    let wh = (fb.height * 3 / 5).clamp(180, 460).min(fb.height - 112);
    let wx = (fb.width - ww) / 2;
    let wy = (fb.height - wh) / 2;
    let y = wy + wh - 28;
    c.rect(wx + 20, y - 4, ww - 40, 18, Color(20, 30, 48));
    c.text(wx + 26, y, "INPUT", 1, Color(130, 164, 207));
    let mut text = [b' '; 80];
    let len = INPUT_LEN.load(Ordering::Relaxed);
    for (i, slot) in text.iter_mut().enumerate().take(len) { *slot = INPUT[i].load(Ordering::Relaxed); }
    let rendered = unsafe { core::str::from_utf8_unchecked(&text[..len]) };
    c.text(wx + 72, y, rendered, 1, Color(239, 244, 252));
}

fn glyph(ch: char) -> [u8; 7] {
    match ch {
        'A' => [14,17,17,31,17,17,17], 'B' => [30,17,17,30,17,17,30],
        'C' => [14,17,16,16,16,17,14], 'D' => [30,17,17,17,17,17,30],
        'E' => [31,16,16,30,16,16,31], 'F' => [31,16,16,30,16,16,16],
        'G' => [14,17,16,23,17,17,15], 'H' => [17,17,17,31,17,17,17],
        'I' => [31,4,4,4,4,4,31], 'J' => [7,2,2,2,18,18,12],
        'K' => [17,18,20,24,20,18,17], 'L' => [16,16,16,16,16,16,31],
        'M' => [17,27,21,21,17,17,17], 'N' => [17,25,25,21,19,19,17],
        'O' => [14,17,17,17,17,17,14], 'P' => [30,17,17,30,16,16,16],
        'Q' => [14,17,17,17,21,18,13], 'R' => [30,17,17,30,20,18,17],
        'S' => [15,16,16,14,1,1,30], 'T' => [31,4,4,4,4,4,4],
        'U' => [17,17,17,17,17,17,14], 'V' => [17,17,17,17,17,10,4],
        'W' => [17,17,17,21,21,21,10], 'X' => [17,17,10,4,10,17,17],
        'Y' => [17,17,10,4,4,4,4], 'Z' => [31,1,2,4,8,16,31],
        '0' => [14,17,19,21,25,17,14], '1' => [4,12,4,4,4,4,14],
        '2' => [14,17,1,2,4,8,31], '3' => [30,1,1,14,1,1,30],
        '4' => [2,6,10,18,31,2,2], '5' => [31,16,16,30,1,1,30],
        '6' => [14,16,16,30,17,17,14], '7' => [31,1,2,4,8,8,8],
        '8' => [14,17,17,14,17,17,14], '9' => [14,17,17,15,1,1,14],
        ' ' => [0;7], ':' => [0,4,4,0,4,4,0], '-' => [0,0,0,31,0,0,0],
        _ => [0;7],
    }
}
