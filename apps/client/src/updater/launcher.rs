use oteryn_platform_client::updates::UpdateError;
use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom};
use std::path::Path;

#[path = "../../build_support/launcher_version.rs"]
mod version;
pub(super) use version::PE_IMAGE_VERSION;

pub(super) const LAUNCHER_EXECUTABLE: &str = "oteryn-launcher.exe";

pub(super) fn require_slot_support(launcher: &Path) -> Result<(), UpdateError> {
    if File::open(launcher)
        .and_then(|mut file| read_image_version(&mut file))
        .is_ok_and(|found| found == Some(PE_IMAGE_VERSION))
    {
        return Ok(());
    }
    Err(UpdateError::Unavailable)
}

/// Read only the PE headers. Starting an older launcher to probe it can start the active client.
fn read_image_version(file: &mut File) -> io::Result<Option<(u16, u16)>> {
    let mut dos = [0_u8; 64];
    file.read_exact(&mut dos)?;
    let pe_offset = u32::from_le_bytes([dos[60], dos[61], dos[62], dos[63]]);
    if &dos[..2] != b"MZ" || pe_offset < 64 {
        return Ok(None);
    }

    file.seek(SeekFrom::Start(u64::from(pe_offset)))?;
    let mut coff = [0_u8; 24];
    file.read_exact(&mut coff)?;
    // The optional header follows the PE signature and the 20-byte COFF header.
    let mut optional = [0_u8; 48];
    let optional_size = usize::from(u16::from_le_bytes([coff[20], coff[21]]));
    if &coff[..4] != b"PE\0\0" || optional_size < optional.len() {
        return Ok(None);
    }
    file.read_exact(&mut optional)?;
    // PE32 and PE32+ share these offsets for MajorImageVersion and MinorImageVersion.
    if !matches!(
        u16::from_le_bytes([optional[0], optional[1]]),
        0x10b | 0x20b
    ) {
        return Ok(None);
    }
    Ok(Some((
        u16::from_le_bytes([optional[44], optional[45]]),
        u16::from_le_bytes([optional[46], optional[47]]),
    )))
}
