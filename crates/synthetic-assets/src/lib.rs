//! Project-owned bounded synthetic asset fixtures.

use oteryn_foundation::{BoundedText, ProcessGeneration};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::fmt::{self, Display, Formatter};

pub const MAX_DIMENSION: u32 = 4096;
pub const MAX_RGBA_BYTES: usize = 16 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyntheticAssetError {
    InvalidManifest,
    UnsupportedSchema,
    DimensionOutOfRange,
    SizeOverflow,
    ByteLengthMismatch,
    StaleGeneration,
}

impl Display for SyntheticAssetError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::InvalidManifest => "synthetic asset manifest is invalid",
            Self::UnsupportedSchema => "synthetic asset schema is unsupported",
            Self::DimensionOutOfRange => "synthetic asset dimensions are out of range",
            Self::SizeOverflow => "synthetic asset size overflowed",
            Self::ByteLengthMismatch => "synthetic RGBA byte length does not match dimensions",
            Self::StaleGeneration => "synthetic asset generation is stale",
        })
    }
}

impl std::error::Error for SyntheticAssetError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyntheticImage {
    pub asset_id: BoundedText,
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

impl SyntheticImage {
    pub fn from_manifest_and_rgba(
        manifest: &[u8],
        rgba: Vec<u8>,
    ) -> Result<Self, SyntheticAssetError> {
        let value: Value = serde_json::from_slice(manifest)
            .map_err(|_error| SyntheticAssetError::InvalidManifest)?;
        let object = value
            .as_object()
            .ok_or(SyntheticAssetError::InvalidManifest)?;
        if object.get("schema").and_then(Value::as_str) != Some("synthetic-v1") {
            return Err(SyntheticAssetError::UnsupportedSchema);
        }
        let width = u32::try_from(
            object
                .get("width")
                .and_then(Value::as_u64)
                .ok_or(SyntheticAssetError::InvalidManifest)?,
        )
        .map_err(|_error| SyntheticAssetError::DimensionOutOfRange)?;
        let height = u32::try_from(
            object
                .get("height")
                .and_then(Value::as_u64)
                .ok_or(SyntheticAssetError::InvalidManifest)?,
        )
        .map_err(|_error| SyntheticAssetError::DimensionOutOfRange)?;
        if width == 0 || height == 0 || width > MAX_DIMENSION || height > MAX_DIMENSION {
            return Err(SyntheticAssetError::DimensionOutOfRange);
        }
        let expected = usize::try_from(width)
            .ok()
            .and_then(|width| {
                usize::try_from(height)
                    .ok()
                    .and_then(|height| width.checked_mul(height))
            })
            .and_then(|pixels| pixels.checked_mul(4))
            .ok_or(SyntheticAssetError::SizeOverflow)?;
        if expected > MAX_RGBA_BYTES {
            return Err(SyntheticAssetError::SizeOverflow);
        }
        if rgba.len() != expected {
            return Err(SyntheticAssetError::ByteLengthMismatch);
        }
        let asset_id = BoundedText::new(
            object
                .get("asset_id")
                .and_then(Value::as_str)
                .ok_or(SyntheticAssetError::InvalidManifest)?,
            128,
        )
        .map_err(|_error| SyntheticAssetError::InvalidManifest)?;
        Ok(Self {
            asset_id,
            width,
            height,
            rgba,
        })
    }

    #[must_use]
    pub fn sha256_hex(&self) -> String {
        let digest = Sha256::digest(&self.rgba);
        let mut output = String::with_capacity(digest.len() * 2);
        for byte in digest {
            use std::fmt::Write as _;
            let _result = write!(&mut output, "{byte:02x}");
        }
        output
    }
}

#[derive(Debug, Clone)]
pub struct SyntheticAssetRuntime {
    generation: ProcessGeneration,
    image: Option<SyntheticImage>,
}

impl SyntheticAssetRuntime {
    #[must_use]
    pub const fn new(generation: ProcessGeneration) -> Self {
        Self {
            generation,
            image: None,
        }
    }

    pub fn replace(
        &mut self,
        generation: ProcessGeneration,
        image: SyntheticImage,
    ) -> Result<(), SyntheticAssetError> {
        if generation != self.generation {
            return Err(SyntheticAssetError::StaleGeneration);
        }
        self.image = Some(image);
        Ok(())
    }

    #[must_use]
    pub const fn image(&self) -> Option<&SyntheticImage> {
        self.image.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixture_is_bounded_and_deterministic() -> Result<(), SyntheticAssetError> {
        let image = SyntheticImage::from_manifest_and_rgba(
            br#"{"schema":"synthetic-v1","asset_id":"checker","width":2,"height":2}"#,
            vec![0_u8; 16],
        )?;
        assert_eq!(image.width, 2);
        assert_eq!(image.sha256_hex().len(), 64);
        Ok(())
    }
}
