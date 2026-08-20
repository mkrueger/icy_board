use std::{
    collections::HashMap,
    time::{Duration, Instant},
};

pub const GFX_BACKEND_NONE: i32 = -1;
pub const GFX_BACKEND_AUTO: i32 = 0;
pub const GFX_BACKEND_SIXEL: i32 = 2;

const MAX_DIMENSION: usize = 2048;
const MAX_RESIDENT_BYTES: usize = 64 * 1024 * 1024;

#[derive(Clone)]
pub struct GfxSurface {
    pub width: usize,
    pub height: usize,
    pub pixels: Vec<u8>,
}

impl GfxSurface {
    pub fn new(width: usize, height: usize) -> Option<Self> {
        if width == 0 || height == 0 || width > MAX_DIMENSION || height > MAX_DIMENSION {
            return None;
        }
        let len = width.checked_mul(height)?.checked_mul(4)?;
        (len <= MAX_RESIDENT_BYTES).then(|| Self {
            width,
            height,
            pixels: vec![0; len],
        })
    }

    pub fn from_rgba(width: usize, height: usize, pixels: Vec<u8>) -> Option<Self> {
        let expected = width.checked_mul(height)?.checked_mul(4)?;
        (width > 0 && height > 0 && width <= MAX_DIMENSION && height <= MAX_DIMENSION && expected <= MAX_RESIDENT_BYTES && pixels.len() == expected)
            .then_some(Self { width, height, pixels })
    }

    pub fn clear(&mut self, color: u32) {
        let rgba = color.to_be_bytes();
        for pixel in self.pixels.chunks_exact_mut(4) {
            pixel.copy_from_slice(&rgba);
        }
    }

    pub fn fill_rect(&mut self, x: i32, y: i32, width: i32, height: i32, color: u32) {
        let Some((x, y, width, height)) = clipped_rect(x, y, width, height, self.width, self.height) else {
            return;
        };
        let rgba = color.to_be_bytes();
        for row in y..y + height {
            let start = (row * self.width + x) * 4;
            let end = start + width * 4;
            for pixel in self.pixels[start..end].chunks_exact_mut(4) {
                pixel.copy_from_slice(&rgba);
            }
        }
    }

    pub fn rect(&mut self, x: i32, y: i32, width: i32, height: i32, color: u32) {
        if width <= 0 || height <= 0 {
            return;
        }
        self.fill_rect(x, y, width, 1, color);
        if height > 1 {
            self.fill_rect(x, y.saturating_add(height - 1), width, 1, color);
        }
        if height > 2 {
            self.fill_rect(x, y.saturating_add(1), 1, height - 2, color);
            if width > 1 {
                self.fill_rect(x.saturating_add(width - 1), y.saturating_add(1), 1, height - 2, color);
            }
        }
    }

    pub fn blit(&mut self, source: &Self, source_rect: (i32, i32, i32, i32), destination: (i32, i32)) {
        let (source_x, source_y, width, height) = source_rect;
        let (destination_x, destination_y) = destination;
        for offset_y in 0..height.max(0) {
            let sy = source_y + offset_y;
            let dy = destination_y + offset_y;
            if sy < 0 || dy < 0 || sy >= source.height as i32 || dy >= self.height as i32 {
                continue;
            }
            for offset_x in 0..width.max(0) {
                let sx = source_x + offset_x;
                let dx = destination_x + offset_x;
                if sx < 0 || dx < 0 || sx >= source.width as i32 || dx >= self.width as i32 {
                    continue;
                }
                let source_index = (sy as usize * source.width + sx as usize) * 4;
                let destination_index = (dy as usize * self.width + dx as usize) * 4;
                blend_pixel(
                    &mut self.pixels[destination_index..destination_index + 4],
                    &source.pixels[source_index..source_index + 4],
                );
            }
        }
    }

    pub fn region_from_origin(&self, x: i32, y: i32, width: i32, height: i32) -> Option<Self> {
        let (x, y, width, height) = clipped_rect(x, y, width, height, self.width, self.height)?;
        let output_width = x + width;
        let output_height = y + height;
        let mut pixels = vec![0; output_width.checked_mul(output_height)?.checked_mul(4)?];
        for row in y..output_height {
            let source_start = (row * self.width + x) * 4;
            let destination_start = (row * output_width + x) * 4;
            let len = width * 4;
            pixels[destination_start..destination_start + len].copy_from_slice(&self.pixels[source_start..source_start + len]);
        }
        Self::from_rgba(output_width, output_height, pixels)
    }
}

pub struct PplGraphicsState {
    pub backend: i32,
    pub surfaces: HashMap<i32, GfxSurface>,
    frame_rate: u32,
    next_frame: Option<Instant>,
}

impl PplGraphicsState {
    pub fn new(requested_backend: i32) -> Option<Self> {
        let backend = match requested_backend {
            GFX_BACKEND_AUTO | GFX_BACKEND_SIXEL => GFX_BACKEND_SIXEL,
            _ => GFX_BACKEND_NONE,
        };
        (backend != GFX_BACKEND_NONE).then(|| Self {
            backend,
            surfaces: HashMap::new(),
            frame_rate: 0,
            next_frame: None,
        })
    }

    pub fn insert_surface(&mut self, slot: i32, surface: GfxSurface) -> bool {
        let resident = self
            .surfaces
            .iter()
            .filter(|(existing_slot, _)| **existing_slot != slot)
            .map(|(_, existing)| existing.pixels.len())
            .sum::<usize>();
        if resident.saturating_add(surface.pixels.len()) > MAX_RESIDENT_BYTES {
            return false;
        }
        self.surfaces.insert(slot, surface);
        true
    }

    pub fn next_frame_deadline(&mut self, requested_rate: i32) -> Option<Instant> {
        if requested_rate <= 0 {
            self.frame_rate = 0;
            self.next_frame = None;
            return None;
        }
        let frame_rate = requested_rate.clamp(1, 240) as u32;
        let interval = Duration::from_secs_f64(1.0 / f64::from(frame_rate));
        let now = Instant::now();
        let deadline = if self.frame_rate == frame_rate {
            self.next_frame.unwrap_or(now + interval)
        } else {
            now + interval
        };
        let following = deadline + interval;
        self.frame_rate = frame_rate;
        self.next_frame = Some(if following <= now { now + interval } else { following });
        Some(deadline)
    }
}

fn clipped_rect(x: i32, y: i32, width: i32, height: i32, target_width: usize, target_height: usize) -> Option<(usize, usize, usize, usize)> {
    if width <= 0 || height <= 0 {
        return None;
    }
    let left = x.max(0) as usize;
    let top = y.max(0) as usize;
    let right = x.saturating_add(width).clamp(0, target_width as i32) as usize;
    let bottom = y.saturating_add(height).clamp(0, target_height as i32) as usize;
    (left < right && top < bottom).then_some((left, top, right - left, bottom - top))
}

fn blend_pixel(destination: &mut [u8], source: &[u8]) {
    let alpha = u16::from(source[3]);
    if alpha == 255 {
        destination.copy_from_slice(source);
        return;
    }
    if alpha == 0 {
        return;
    }
    let inverse = 255 - alpha;
    for channel in 0..3 {
        destination[channel] = ((u16::from(source[channel]) * alpha + u16::from(destination[channel]) * inverse + 127) / 255) as u8;
    }
    destination[3] = (alpha + (u16::from(destination[3]) * inverse + 127) / 255).min(255) as u8;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn surfaces_can_be_composed_offscreen() {
        let mut target = GfxSurface::new(2, 2).unwrap();
        target.clear(0x0000_00FF);
        let mut source = GfxSurface::new(2, 1).unwrap();
        source.clear(0xFF00_00FF);

        target.blit(&source, (0, 0, 2, 1), (1, 1));

        assert_eq!(&target.pixels[12..16], &[255, 0, 0, 255]);
        assert_eq!(&target.pixels[8..12], &[0, 0, 0, 255]);
    }

    #[test]
    fn partial_region_keeps_native_pixel_origin() {
        let mut surface = GfxSurface::new(4, 4).unwrap();
        surface.clear(0x1020_30FF);

        let region = surface.region_from_origin(2, 1, 2, 2).unwrap();

        assert_eq!((region.width, region.height), (4, 3));
        assert_eq!(&region.pixels[0..4], &[0, 0, 0, 0]);
        assert_eq!(&region.pixels[(4 + 2) * 4..(4 + 3) * 4], &[0x10, 0x20, 0x30, 0xFF]);
    }

    #[test]
    fn rectangle_draws_only_its_outline() {
        let mut surface = GfxSurface::new(4, 4).unwrap();
        surface.clear(0x0000_00FF);

        surface.rect(0, 0, 3, 3, 0xFF00_00FF);

        let pixel = |x: usize, y: usize| &surface.pixels[(y * 4 + x) * 4..(y * 4 + x + 1) * 4];
        assert_eq!(pixel(0, 0), &[255, 0, 0, 255]);
        assert_eq!(pixel(2, 1), &[255, 0, 0, 255]);
        assert_eq!(pixel(1, 1), &[0, 0, 0, 255]);
        assert_eq!(pixel(3, 3), &[0, 0, 0, 255]);
    }
}
