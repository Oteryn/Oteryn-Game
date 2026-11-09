//! Opt-in pre-event frames. Sampling, age, count and byte bounds keep readback cost bounded.
use crate::{
    play::ScreenshotPreferences,
    screenshots::{CaptureController, Frame, Rect},
};
use std::{
    collections::VecDeque,
    io,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};
const MAX_BYTES: usize = 64 * 1024 * 1024;
const MAX_FRAMES: usize = 10;
struct StoredFrame {
    at: Instant,
    width: u32,
    height: u32,
    bgra: Vec<u8>,
    game: Rect,
}
impl StoredFrame {
    fn frame(&self) -> Frame<'_> {
        Frame {
            width: self.width,
            height: self.height,
            bgra: &self.bgra,
            game: self.game,
        }
    }
}
#[derive(Default)]
pub struct CaptureBacklog {
    frames: VecDeque<StoredFrame>,
    bytes: usize,
    requested: Option<Instant>,
}
impl CaptureBacklog {
    fn remove_oldest(&mut self) {
        if let Some(frame) = self.frames.pop_front() {
            self.bytes -= frame.bgra.len();
        }
    }
    fn trim(&mut self, now: Instant, seconds: u8) {
        while self.frames.front().is_some_and(|frame| {
            seconds == 0
                || now.saturating_duration_since(frame.at)
                    > Duration::from_secs(u64::from(seconds.min(10)))
        }) {
            self.remove_oldest();
        }
    }
    /// No background readback unless opted in; at most one request per second.
    pub fn sample_due(&mut self, now: Instant, seconds: u8) -> bool {
        self.trim(now, seconds);
        if seconds == 0 {
            self.requested = None;
            return false;
        }
        if self
            .requested
            .is_some_and(|last| now.saturating_duration_since(last) < Duration::from_secs(1))
        {
            return false;
        }
        self.requested = Some(now);
        true
    }
    fn remember(&mut self, frame: Frame<'_>, now: Instant, seconds: u8) {
        let size = u64::from(frame.width) * u64::from(frame.height);
        if seconds == 0
            || self.frames.back().is_some_and(|previous| {
                now.saturating_duration_since(previous.at) < Duration::from_secs(1)
            })
            || frame.width == 0
            || frame.height == 0
            || size > (MAX_BYTES / 4) as u64
            || size * 4 != frame.bgra.len() as u64
        {
            return;
        }
        while self.bytes + frame.bgra.len() > MAX_BYTES || self.frames.len() >= MAX_FRAMES {
            self.remove_oldest();
        }
        self.bytes += frame.bgra.len();
        self.frames.push_back(StoredFrame {
            at: now,
            width: frame.width,
            height: frame.height,
            bgra: frame.bgra.to_vec(),
            game: frame.game,
        });
    }
    /// The shell calls this for real renderer readbacks. Only a pending trigger writes files.
    pub fn process(
        &mut self,
        capture: &mut CaptureController,
        preferences: &ScreenshotPreferences,
        fallback: &Path,
        frame: Frame<'_>,
        now: Instant,
    ) -> io::Result<Vec<PathBuf>> {
        self.trim(now, preferences.backlog_seconds);
        let mut paths = Vec::new();
        if capture.pending() {
            let saved = (|| {
                for previous in &self.frames {
                    paths.push(preferences.save_frame(capture, fallback, previous.frame())?);
                }
                paths.push(preferences.save_frame(capture, fallback, frame)?);
                Ok::<(), io::Error>(())
            })();
            if let Err(error) = saved {
                capture.manual();
                return Err(error);
            }
        }
        self.remember(frame, now, preferences.backlog_seconds);
        Ok(paths)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn backlog_evicts_oldest_frames_before_exceeding_memory_budget()
    -> Result<(), Box<dyn std::error::Error>> {
        let pixels = vec![0; 8 * 1024 * 1024];
        let preferences = ScreenshotPreferences {
            backlog_seconds: 10,
            ..Default::default()
        };
        let mut backlog = CaptureBacklog::default();
        let mut capture = CaptureController::default();
        let now = Instant::now();
        for second in 0..10 {
            backlog.process(
                &mut capture,
                &preferences,
                Path::new("unused"),
                Frame {
                    width: 2048,
                    height: 1024,
                    bgra: &pixels,
                    game: Rect {
                        x: 0,
                        y: 0,
                        width: 2048,
                        height: 1024,
                    },
                },
                now + Duration::from_secs(second),
            )?;
            assert!(backlog.bytes <= MAX_BYTES);
        }
        backlog.process(
            &mut capture,
            &preferences,
            Path::new("unused"),
            Frame {
                width: 2048,
                height: 1024,
                bgra: &pixels,
                game: Rect {
                    x: 0,
                    y: 0,
                    width: 2048,
                    height: 1024,
                },
            },
            now + Duration::from_millis(9500),
        )?;
        assert_eq!(backlog.bytes, MAX_BYTES);
        assert_eq!(backlog.frames.len(), 8);
        assert_eq!(
            backlog.frames.front().map(|frame| frame.at),
            Some(now + Duration::from_secs(2))
        );
        assert!(!backlog.sample_due(now + Duration::from_secs(11), 0));
        assert_eq!(backlog.bytes, 0);
        assert!(backlog.frames.is_empty());
        Ok(())
    }
}
