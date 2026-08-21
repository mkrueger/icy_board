use std::{
    collections::{HashMap, HashSet},
    time::{Duration, Instant},
};

pub const GFX_BACKEND_NONE: i32 = -1;
pub const GFX_BACKEND_AUTO: i32 = 0;
// 1 is reserved for a future character based backend.
pub const GFX_BACKEND_SIXEL: i32 = 2;
pub const GFX_BACKEND_JXL: i32 = 3;

const MAX_DIMENSION: usize = 2048;
const MAX_RESIDENT_BYTES: usize = 64 * 1024 * 1024;
const MAX_SURFACES: usize = 256;

fn color_component(value: i32) -> u32 {
    value.clamp(0, 255) as u32
}

pub fn rgba_value(red: i32, green: i32, blue: i32, alpha: i32) -> u32 {
    (color_component(red) << 24) | (color_component(green) << 16) | (color_component(blue) << 8) | color_component(alpha)
}

/// Queries the terminal answers before a backend is chosen. `syncterm_extensions.md`
/// asks for JPEG XL to be probed rather than inferred, and the cell size is what turns
/// a text coordinate into the pixel destination the image APC wants.
pub const DEVICE_ATTRIBUTES_QUERY: &[u8] = b"\x1b[c";
pub const CTERM_ATTRIBUTES_QUERY: &[u8] = b"\x1b[<0c";
pub const CELL_SIZE_QUERY: &[u8] = b"\x1b[16t";
pub const PIXEL_SIZE_QUERY: &[u8] = b"\x1b[14t";
pub const JXL_QUERY: &[u8] = b"\x1b_SyncTERM:Q;JXL\x1b\\";
pub const CACHE_LIST_QUERY: &[u8] = b"\x1b_SyncTERM:C;L;*\x1b\\";

/// Everything below this directory in the caller's per board cache belongs to PPL graphics.
pub const CACHE_PREFIX: &str = "gfx/";

/// Where the sound a caller has already been sent lives in the same cache.
pub const SOUND_CACHE_PREFIX: &str = "snd/";

/// `CTerm` revision that introduced the inline `*Blob` verbs, which draw a changing
/// frame without writing it to the caller's disk cache first.
pub const CTERM_INLINE_BLOB_REVISION: u32 = 1329;

const MAX_REPLY_BYTES: usize = 64 * 1024;

/// What the caller's terminal turned out to support.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GfxCapabilities {
    pub sixel: bool,
    pub jxl: bool,
    pub physical_keys: bool,
    pub cterm_revision: Option<u32>,
    pub cell_width: i32,
    pub cell_height: i32,
    pub screen_width: i32,
    pub screen_height: i32,
}

impl Default for GfxCapabilities {
    fn default() -> Self {
        Self {
            sixel: false,
            jxl: false,
            physical_keys: false,
            cterm_revision: None,
            cell_width: 8,
            cell_height: 16,
            screen_width: 0,
            screen_height: 0,
        }
    }
}

impl GfxCapabilities {
    /// Inline blobs are only safe once the terminal has named a revision that has them.
    pub fn inline_blobs(&self) -> bool {
        self.cterm_revision.is_some_and(|revision| revision >= CTERM_INLINE_BLOB_REVISION)
    }

    /// The backend a `GFXINIT` request resolves to, or `GFX_BACKEND_NONE` when the
    /// caller cannot be served what was asked for.
    pub fn resolve_backend(&self, requested: i32) -> i32 {
        match requested {
            GFX_BACKEND_AUTO => {
                if self.jxl {
                    GFX_BACKEND_JXL
                } else if self.sixel {
                    GFX_BACKEND_SIXEL
                } else {
                    GFX_BACKEND_NONE
                }
            }
            GFX_BACKEND_SIXEL => GFX_BACKEND_SIXEL,
            // Never send JPEG XL to a terminal that did not answer the query for it.
            GFX_BACKEND_JXL => {
                if self.jxl {
                    GFX_BACKEND_JXL
                } else {
                    GFX_BACKEND_NONE
                }
            }
            _ => GFX_BACKEND_NONE,
        }
    }
}

/// Collects the capability answers out of the caller's input while a probe is running.
///
/// Replies arrive interleaved with ordinary typing, so anything that is not one of the
/// answers being waited for is handed back and reaches the keyboard unchanged.
#[derive(Default)]
pub struct GfxProbe {
    active: bool,
    pending: Vec<u8>,
    capabilities: GfxCapabilities,
    jxl_answered: bool,
    cache_listing: Option<HashSet<String>>,
}

impl GfxProbe {
    pub fn start(&mut self) {
        self.active = true;
        self.pending.clear();
        self.capabilities = GfxCapabilities::default();
        self.jxl_answered = false;
        self.cache_listing = None;
    }

    pub fn capabilities(&self) -> GfxCapabilities {
        self.capabilities
    }

    pub fn jxl_answered(&self) -> bool {
        self.jxl_answered
    }

    pub fn cache_listed(&self) -> bool {
        self.cache_listing.is_some()
    }

    /// Ends the probe and reports what was learned, along with any half finished
    /// sequence that turned out not to be an answer.
    pub fn finish(&mut self) -> (GfxCapabilities, Option<HashSet<String>>, Vec<u8>) {
        self.active = false;
        let leftover = std::mem::take(&mut self.pending);
        (self.capabilities, self.cache_listing.take(), leftover)
    }

    pub fn feed(&mut self, byte: u8) -> Vec<u8> {
        if !self.active {
            return vec![byte];
        }
        if self.pending.is_empty() {
            if byte == 0x1B {
                self.pending.push(byte);
                return Vec::new();
            }
            return vec![byte];
        }

        self.pending.push(byte);
        if self.pending.len() == 2 {
            if byte != b'[' && byte != b'_' {
                return std::mem::take(&mut self.pending);
            }
            return Vec::new();
        }
        if self.pending.len() > MAX_REPLY_BYTES {
            // A cache listing can be long; replaying one as keystrokes helps nobody.
            log::warn!("Discarding an overlong terminal reply while probing graphics support");
            self.pending.clear();
            return Vec::new();
        }
        if self.pending[1] == b'[' {
            if !(0x40..=0x7E).contains(&byte) {
                return Vec::new();
            }
            let reply = std::mem::take(&mut self.pending);
            if self.parse_csi(&reply) { Vec::new() } else { reply }
        } else {
            if !self.pending.ends_with(b"\x1b\\") {
                return Vec::new();
            }
            let reply = std::mem::take(&mut self.pending);
            if self.parse_apc(&reply) { Vec::new() } else { reply }
        }
    }

    fn parse_csi(&mut self, reply: &[u8]) -> bool {
        let Ok(text) = std::str::from_utf8(reply) else {
            return false;
        };
        let Some(body) = text.strip_prefix("\x1b[") else {
            return false;
        };
        let (body, final_byte) = body.split_at(body.len() - 1);
        match final_byte {
            // CSI = 1 ; <0-or-1> - n answers the JPEG XL query.
            "n" => {
                let Some(values) = body.strip_prefix('=') else {
                    return false;
                };
                let mut parts = values.trim_end_matches('-').split(';');
                if parts.next() != Some("1") {
                    return false;
                }
                let Some(state) = parts.next() else {
                    return false;
                };
                self.capabilities.jxl = state == "1";
                self.jxl_answered = true;
                true
            }
            // CSI = 67;84;101;114;109;MAJOR;MINOR c spells "CTerm" and its revision.
            "c" => {
                if let Some(values) = body.strip_prefix('<') {
                    let features = values.split(';').filter_map(|value| value.parse::<i32>().ok()).collect::<Vec<_>>();
                    self.capabilities.sixel = features.contains(&4);
                    self.capabilities.physical_keys = features.contains(&8);
                    return true;
                }
                let Some(values) = body.strip_prefix('=') else {
                    return false;
                };
                let numbers: Vec<&str> = values.split(';').collect();
                // Icy Term names itself instead of CTerm. It carries the inline blob
                // verbs from 0.8.4, which is what the revision stands in for here.
                if numbers.len() >= 10 && numbers[..7] == ["73", "99", "121", "84", "101", "114", "109"] {
                    let (Ok(major), Ok(minor), Ok(patch)) = (numbers[7].parse::<u32>(), numbers[8].parse::<u32>(), numbers[9].parse::<u32>()) else {
                        return false;
                    };
                    if (major, minor, patch) >= (0, 8, 4) {
                        self.capabilities.cterm_revision = Some(CTERM_INLINE_BLOB_REVISION);
                    }
                    return true;
                }
                if numbers.len() < 7 || numbers[..5] != ["67", "84", "101", "114", "109"] {
                    return false;
                }
                let (Ok(major), Ok(minor)) = (numbers[5].parse::<u32>(), numbers[6].parse::<u32>()) else {
                    return false;
                };
                self.capabilities.cterm_revision = Some(major * 1000 + minor);
                true
            }
            // CSI 6 ; height ; width t answers the cell size request.
            "t" => {
                let mut parts = body.split(';');
                let Some(report) = parts.next() else {
                    return false;
                };
                let (Some(Ok(height)), Some(Ok(width))) = (parts.next().map(str::parse::<i32>), parts.next().map(str::parse::<i32>)) else {
                    return false;
                };
                if height <= 0 || width <= 0 {
                    return false;
                }
                match report {
                    "4" => {
                        self.capabilities.screen_height = height;
                        self.capabilities.screen_width = width;
                        true
                    }
                    "6" => {
                        self.capabilities.cell_height = height;
                        self.capabilities.cell_width = width;
                        true
                    }
                    _ => false,
                }
            }
            _ => false,
        }
    }

    fn parse_apc(&mut self, reply: &[u8]) -> bool {
        let Ok(text) = std::str::from_utf8(reply) else {
            return false;
        };
        let Some(payload) = text.strip_prefix("\x1b_").and_then(|rest| rest.strip_suffix("\x1b\\")) else {
            return false;
        };
        let Some(body) = payload.strip_prefix("SyncTERM:C;L") else {
            return false;
        };
        // The header line carries the command back; every line after it is name TAB md5.
        let entries = body.split_once('\n').map_or("", |(_, rest)| rest);
        self.cache_listing = Some(
            entries
                .lines()
                .filter_map(|line| line.split('\t').next())
                .map(str::trim)
                .filter(|name| !name.is_empty())
                .map(str::to_string)
                .collect(),
        );
        true
    }
}

#[derive(Clone)]
pub struct GfxSurface {
    pub width: usize,
    pub height: usize,
    pub pixels: Vec<u8>,
    /// True while the pixels are still exactly what `GFXLOAD` read. Only such a surface
    /// is worth keeping in the caller's cache, where it survives into the next session.
    pub cacheable: bool,
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
            cacheable: false,
        })
    }

    pub fn from_rgba(width: usize, height: usize, pixels: Vec<u8>) -> Option<Self> {
        let expected = width.checked_mul(height)?.checked_mul(4)?;
        (width > 0 && height > 0 && width <= MAX_DIMENSION && height <= MAX_DIMENSION && expected <= MAX_RESIDENT_BYTES && pixels.len() == expected).then_some(
            Self {
                width,
                height,
                pixels,
                cacheable: false,
            },
        )
    }

    pub fn clear(&mut self, color: u32) {
        self.cacheable = false;
        let rgba = color.to_be_bytes();
        for pixel in self.pixels.chunks_exact_mut(4) {
            pixel.copy_from_slice(&rgba);
        }
    }

    pub fn set_pixel(&mut self, x: i32, y: i32, color: u32) {
        let (Ok(x), Ok(y)) = (usize::try_from(x), usize::try_from(y)) else {
            return;
        };
        if x >= self.width || y >= self.height {
            return;
        }
        self.cacheable = false;
        let start = (y * self.width + x) * 4;
        self.pixels[start..start + 4].copy_from_slice(&color.to_be_bytes());
    }

    pub fn fill_rect(&mut self, x: i32, y: i32, width: i32, height: i32, color: u32) {
        self.cacheable = false;
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
        self.cacheable = false;
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

    /// The requested rectangle as a surface of its own.
    ///
    /// The image APC places a partial update by pixel destination, so unlike the sixel
    /// path this does not have to carry the untouched pixels left of the region along.
    /// Answers the clipped origin as well, since clipping can move it.
    pub fn region(&self, x: i32, y: i32, width: i32, height: i32) -> Option<(Self, i32, i32)> {
        let (x, y, width, height) = clipped_rect(x, y, width, height, self.width, self.height)?;
        let mut pixels = Vec::with_capacity(width.checked_mul(height)?.checked_mul(4)?);
        for row in y..y + height {
            let start = (row * self.width + x) * 4;
            pixels.extend_from_slice(&self.pixels[start..start + width * 4]);
        }
        let region = Self::from_rgba(width, height, pixels)?;
        Some((region, x as i32, y as i32))
    }

    pub fn region_at(&self, source: (i32, i32, i32, i32), destination: (i32, i32)) -> Option<Self> {
        let (region, _, _) = self.region(source.0, source.1, source.2, source.3)?;
        let destination_x = destination.0.max(0) as usize;
        let destination_y = destination.1.max(0) as usize;
        let output_width = destination_x.checked_add(region.width)?;
        let output_height = destination_y.checked_add(region.height)?;
        let mut output = Self::new(output_width, output_height)?;
        output.blit(
            &region,
            (0, 0, region.width as i32, region.height as i32),
            (destination_x as i32, destination_y as i32),
        );
        Some(output)
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
    pub fullscreen: bool,
    pub capabilities: GfxCapabilities,
    pub surfaces: HashMap<i32, GfxSurface>,
    pub pinned: HashMap<i32, u8>,
    pub pacing: bool,
    resident_bytes: usize,
    next_handle: i32,
    frame_rate: u32,
    next_frame: Option<Instant>,
}

impl PplGraphicsState {
    /// `backend` is the already resolved one, so an unsupported request never gets this far.
    pub fn new(backend: i32, fullscreen: bool, capabilities: GfxCapabilities) -> Option<Self> {
        matches!(backend, GFX_BACKEND_SIXEL | GFX_BACKEND_JXL).then(|| Self {
            backend,
            fullscreen,
            capabilities,
            surfaces: HashMap::new(),
            pinned: HashMap::new(),
            pacing: false,
            resident_bytes: 0,
            next_handle: 0,
            frame_rate: 0,
            next_frame: None,
        })
    }

    /// A name only the engine hands out, so two callers can never pick the same one.
    pub fn allocate_handle(&mut self) -> i32 {
        self.next_handle -= 1;
        self.next_handle
    }

    pub fn insert_surface(&mut self, slot: i32, surface: GfxSurface) -> bool {
        let replaced_bytes = self.surfaces.get(&slot).map_or(0, |existing| existing.pixels.len());
        if replaced_bytes == 0 && self.surfaces.len() >= MAX_SURFACES {
            return false;
        }
        let resident_bytes = self.resident_bytes.saturating_sub(replaced_bytes).saturating_add(surface.pixels.len());
        if resident_bytes > MAX_RESIDENT_BYTES {
            return false;
        }
        self.pinned.remove(&slot);
        self.resident_bytes = resident_bytes;
        self.surfaces.insert(slot, surface);
        true
    }

    pub fn remove_surface(&mut self, slot: i32) -> bool {
        self.pinned.remove(&slot);
        let Some(surface) = self.surfaces.remove(&slot) else {
            return false;
        };
        self.resident_bytes = self.resident_bytes.saturating_sub(surface.pixels.len());
        true
    }

    pub fn pin(&mut self, slot: i32) -> Option<u8> {
        if let Some(buffer) = self.pinned.get(&slot) {
            return Some(*buffer);
        }
        let buffer = (0..=1).find(|buffer| !self.pinned.values().any(|used| used == buffer))?;
        self.pinned.insert(slot, buffer);
        Some(buffer)
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
    fn a_region_carries_only_its_own_pixels() {
        let mut surface = GfxSurface::new(4, 4).unwrap();
        surface.clear(0x1020_30FF);

        let (region, x, y) = surface.region(2, 1, 2, 2).unwrap();

        assert_eq!((region.width, region.height), (2, 2));
        assert_eq!((x, y), (2, 1));
        assert_eq!(&region.pixels[0..4], &[0x10, 0x20, 0x30, 0xFF]);
    }

    #[test]
    fn drawing_on_a_loaded_surface_makes_it_unfit_for_the_cache() {
        let mut surface = GfxSurface::new(2, 2).unwrap();
        surface.cacheable = true;

        surface.fill_rect(0, 0, 1, 1, 0xFF00_00FF);

        assert!(!surface.cacheable);
    }

    #[test]
    fn collects_the_capability_answers_and_passes_typing_through() {
        let mut probe = GfxProbe::default();
        probe.start();
        let mut typed = Vec::new();
        for byte in b"\x1b[=67;84;101;114;109;1;332c\x1b[6;20;10tA\x1b[=1;1-n" {
            typed.extend(probe.feed(*byte));
        }

        assert!(probe.jxl_answered());
        let (capabilities, _, leftover) = probe.finish();
        assert_eq!(typed, b"A");
        assert!(leftover.is_empty());
        assert!(capabilities.jxl);
        assert_eq!(capabilities.cterm_revision, Some(1332));
        assert!(capabilities.inline_blobs());
        assert_eq!((capabilities.cell_width, capabilities.cell_height), (10, 20));
    }

    #[test]
    fn a_denied_answer_keeps_jpeg_xl_off() {
        let mut probe = GfxProbe::default();
        probe.start();
        for byte in b"\x1b[=1;0-n" {
            assert!(probe.feed(*byte).is_empty());
        }

        assert!(probe.jxl_answered());
        assert_eq!(probe.capabilities().resolve_backend(GFX_BACKEND_AUTO), GFX_BACKEND_NONE);
        assert_eq!(probe.capabilities().resolve_backend(GFX_BACKEND_JXL), GFX_BACKEND_NONE);
    }

    /// What Icy Term answers: it names itself rather than a `CTerm` revision, and from
    /// 0.8.4 it carries the inline blob verbs that keep frames out of the disk cache.
    #[test]
    fn icy_term_is_served_jpeg_xl_and_inline_blobs_from_0_8_4() {
        let probe_icy_term = |identity: &[u8]| {
            let mut probe = GfxProbe::default();
            probe.start();
            let mut typed = Vec::new();
            for byte in identity {
                typed.extend(probe.feed(*byte));
            }
            for byte in b"\x1b[<1;2;3;4;5;6;7c\x1b[6;16;8t\x1b[4;400;640t\x1b[=1;1-n" {
                typed.extend(probe.feed(*byte));
            }
            let (capabilities, _, _) = probe.finish();
            (capabilities, typed)
        };

        let (capabilities, typed) = probe_icy_term(b"\x1b[=73;99;121;84;101;114;109;0;8;4c");
        assert_eq!(capabilities.resolve_backend(GFX_BACKEND_AUTO), GFX_BACKEND_JXL);
        assert!(capabilities.sixel);
        assert!(capabilities.inline_blobs());
        assert_eq!((capabilities.cell_width, capabilities.cell_height), (8, 16));
        // The identity is an answer now, so it no longer reaches the keyboard.
        assert!(typed.is_empty());

        let (older, typed) = probe_icy_term(b"\x1b[=73;99;121;84;101;114;109;0;8;3c");
        assert_eq!(older.resolve_backend(GFX_BACKEND_AUTO), GFX_BACKEND_JXL);
        assert!(!older.inline_blobs());
        assert!(typed.is_empty());
    }

    #[test]
    fn a_cache_listing_names_what_the_caller_already_holds() {
        let mut probe = GfxProbe::default();
        probe.start();
        for byte in b"\x1b_SyncTERM:C;L\ngfx/abc.jxl\td41d8c\ngfx/def.jxl\t0cc175\n\x1b\\" {
            assert!(probe.feed(*byte).is_empty());
        }

        assert!(probe.cache_listed());
        let (_, listing, _) = probe.finish();
        let listing = listing.unwrap();
        assert!(listing.contains("gfx/abc.jxl"));
        assert!(listing.contains("gfx/def.jxl"));
    }

    #[test]
    fn an_unrelated_escape_sequence_reaches_the_keyboard() {
        let mut probe = GfxProbe::default();
        probe.start();
        let mut typed = Vec::new();
        for byte in b"\x1b[D" {
            typed.extend(probe.feed(*byte));
        }

        assert_eq!(typed, b"\x1b[D");
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
