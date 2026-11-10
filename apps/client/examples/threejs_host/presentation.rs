//! Bounded presentation records, not application-protocol messages.
use super::Result;
use oteryn_client::scene::Scene;
use oteryn_renderer::{AtlasImage, MAX_BATCH_QUADS, QuadInstance};
use std::io::{Write, stdout};
use tokio::sync::mpsc::{Sender, channel};

pub struct Frame {
    atlas: AtlasImage,
    width: u32,
    height: u32,
    quads: Vec<QuadInstance>,
}

impl Frame {
    pub fn from_scene(scene: &Scene) -> Self {
        let (width, height) = scene.view().viewport_px();
        Self {
            atlas: scene.atlas().clone(),
            width,
            height,
            quads: scene.sprites().instances().to_vec(),
        }
    }
}

/// One queued frame; a blocked stdout cannot block native session/liveness processing.
pub fn writer() -> Result<Sender<Frame>> {
    let (sender, mut receiver) = channel::<Frame>(1);
    let _writer = std::thread::Builder::new()
        .name("threejs-output".into())
        .spawn(move || -> Result<()> {
            let mut output = stdout().lock();
            let mut encoder = Encoder::default();
            while let Some(frame) = receiver.blocking_recv() {
                encoder.write(&mut output, frame)?;
            }
            Ok(())
        })?;
    Ok(sender)
}

#[derive(Default)]
struct Encoder {
    atlas: Option<AtlasImage>,
    atlas_id: u32,
    frame_id: u32,
}

impl Encoder {
    fn write(&mut self, output: &mut impl Write, frame: Frame) -> Result<()> {
        if frame.quads.len() > MAX_BATCH_QUADS {
            return Err("presentation quad cap".into());
        }
        if self.atlas.as_ref() != Some(&frame.atlas) {
            self.atlas_id = self.atlas_id.checked_add(1).ok_or("atlas counter exhausted")?;
            let mut header = Vec::from(*b"OTAT");
            for value in [self.atlas_id, frame.atlas.width(), frame.atlas.height()] {
                header.extend_from_slice(&value.to_le_bytes());
            }
            let length = u32::try_from(header.len() + frame.atlas.rgba().len())?;
            output.write_all(&length.to_le_bytes())?;
            output.write_all(&header)?;
            output.write_all(frame.atlas.rgba())?;
            self.atlas = Some(frame.atlas);
        }
        self.frame_id = self.frame_id.checked_add(1).ok_or("frame counter exhausted")?;
        let mut body = Vec::with_capacity(24 + frame.quads.len() * 32);
        body.extend_from_slice(b"OTFR");
        for value in [self.frame_id, self.atlas_id, frame.width, frame.height, u32::try_from(frame.quads.len())?] {
            body.extend_from_slice(&value.to_le_bytes());
        }
        for quad in frame.quads {
            for value in quad.position.into_iter().chain(quad.size).chain(quad.uv) {
                body.extend_from_slice(&value.to_le_bytes());
            }
        }
        output.write_all(&u32::try_from(body.len())?.to_le_bytes())?;
        output.write_all(&body)?;
        output.flush()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame() -> Result<Frame> {
        Ok(Frame {
            atlas: AtlasImage::new(1, 1, 1, vec![255; 4])?,
            width: 32,
            height: 32,
            quads: vec![QuadInstance { position: [0.0; 2], size: [32.0; 2], uv: [0.0, 0.0, 1.0, 1.0] }],
        })
    }

    #[test]
    fn first_frame_has_atlas_and_later_frames_reuse_its_generation() -> Result<()> {
        let mut encoder = Encoder::default();
        let mut bytes = Vec::new();
        encoder.write(&mut bytes, frame()?)?;
        assert_eq!(&bytes[0..4], &20_u32.to_le_bytes());
        assert_eq!(&bytes[4..8], b"OTAT");
        assert_eq!(&bytes[24..28], &56_u32.to_le_bytes());
        assert_eq!(&bytes[28..32], b"OTFR");
        assert_eq!(bytes.len(), 84);
        bytes.clear();
        encoder.write(&mut bytes, frame()?)?;
        assert_eq!(&bytes[4..8], b"OTFR");
        assert_eq!(&bytes[8..12], &2_u32.to_le_bytes());
        assert_eq!(&bytes[12..16], &1_u32.to_le_bytes());
        assert_eq!(bytes.len(), 60);
        Ok(())
    }
}
