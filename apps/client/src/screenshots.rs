//! Bounded BGRA screenshot export and explicit automatic-capture policy.
use std::{
    io::{self, Write},
    path::Path,
};
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    Manual,
    Level,
    Death,
    Achievement,
}
#[derive(Debug, Default)]
pub struct CapturePolicy {
    pub level: bool,
    pub death: bool,
    pub achievement: bool,
    last_sequence: Option<u64>,
}
impl CapturePolicy {
    /// Caller supplies the session's increasing event sequence, preventing repeated captures.
    pub fn capture(&mut self, event: Event, sequence: u64) -> bool {
        let enabled = match event {
            Event::Manual => true,
            Event::Level => self.level,
            Event::Death => self.death,
            Event::Achievement => self.achievement,
        };
        if !enabled || self.last_sequence.is_some_and(|last| sequence <= last) {
            return false;
        }
        self.last_sequence = Some(sequence);
        true
    }
}
/// Save either the entire supplied frame or a selected game-view rectangle as a top-down BMP.
/// Frame acquisition remains with the renderer; this function never captures other applications.
pub fn save_bmp(path: &Path, width: u32, height: u32, bgra: &[u8], crop: Rect) -> io::Result<()> {
    let invalid = || {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            tr!("Invalid screenshot dimensions, frame or crop"),
        )
    };
    let pixels = u64::from(width) * u64::from(height);
    if width == 0
        || height == 0
        || width > 16384
        || height > 16384
        || pixels > 32 * 1024 * 1024
        || u64::try_from(bgra.len()).map_err(|_| invalid())? != pixels * 4
        || crop.width == 0
        || crop.height == 0
        || crop.x.checked_add(crop.width).is_none_or(|end| end > width)
        || crop
            .y
            .checked_add(crop.height)
            .is_none_or(|end| end > height)
    {
        return Err(invalid());
    }
    let size = crop.width * crop.height * 4;
    let mut header = [0u8; 54];
    header[..2].copy_from_slice(b"BM");
    header[2..6].copy_from_slice(&(54 + size).to_le_bytes());
    header[10..14].copy_from_slice(&54u32.to_le_bytes());
    header[14..18].copy_from_slice(&40u32.to_le_bytes());
    header[18..22].copy_from_slice(&(crop.width as i32).to_le_bytes());
    header[22..26].copy_from_slice(&(-(crop.height as i32)).to_le_bytes());
    header[26..28].copy_from_slice(&1u16.to_le_bytes());
    header[28..30].copy_from_slice(&32u16.to_le_bytes());
    header[34..38].copy_from_slice(&size.to_le_bytes());
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(path)?;
    file.write_all(&header)?;
    for y in crop.y..crop.y + crop.height {
        let offset = ((u64::from(y) * u64::from(width) + u64::from(crop.x)) * 4) as usize;
        file.write_all(&bgra[offset..offset + crop.width as usize * 4])?;
    }
    file.sync_all()
}
