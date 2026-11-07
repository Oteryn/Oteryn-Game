pub use oteryn_session::{ActorPosition as Position, ChatSpeechMode as SpeechMode};
// Crystal protocolgame.cpp:7626,7846,8043: only public speech, effects and map pixels.
use crate::cast::{DisplayTile, Message, PublicMedia, Scene};
use std::{
    collections::{BTreeMap, VecDeque},
    time::{Duration, Instant},
};
#[derive(Debug, Clone, Default)]
pub struct Picture {
    pub epoch: u64,
    pub chat_enabled: bool,
    pub floor: Option<i16>,
    pub tiles: BTreeMap<(i16, u16, u16), Vec<u16>>,
    pub actors: Vec<(u32, u16, u16, u16, u8)>,
    pub effects: Vec<(Instant, oteryn_session::PresentationEvent)>,
    pub speech: VecDeque<String>,
    pub chat: VecDeque<([u8; 16], String)>,
    pub delay: u16,
    pub viewers: u8,
    pub ended: bool,
    pub slate: bool,
    pending_tiles: Option<BTreeMap<(i16, u16, u16), Vec<u16>>>,
    pending_scene: Option<Scene>,
    pending_floor: Option<i16>,
}
impl Picture {
    pub fn clear(&mut self) {
        self.floor = None;
        self.tiles.clear();
        self.actors.clear();
        self.effects.clear();
        self.speech.clear();
        self.chat.clear();
        self.pending_tiles = None;
        self.pending_scene = None;
        self.pending_floor = None;
    }
    pub fn expire(&mut self, now: Instant) {
        self.effects
            .retain(|(at, _)| now.saturating_duration_since(*at) < Duration::from_millis(800));
    }
    pub fn apply(&mut self, message: Message, now: Instant) -> Result<(), &'static str> {
        message.encode().map_err(|_| "Invalid cast payload")?;
        if self.ended && !matches!(message, Message::End) {
            return Err("Cast ended");
        }
        let epoch = match &message {
            Message::Scene { epoch, .. }
            | Message::Slate { epoch, .. }
            | Message::Media { epoch, .. }
            | Message::State { epoch, .. }
            | Message::ChatLine { epoch, .. } => Some(*epoch),
            _ => None,
        };
        if epoch.is_some_and(|epoch| epoch != self.epoch) {
            return Err("Cast epoch mismatch");
        }
        if matches!(&message,Message::Clear{epoch} if *epoch<self.epoch) {
            return Err("Cast epoch rollback");
        }

        self.expire(now);
        match message {
            Message::Ready { epoch, chat, delay } => {
                self.clear();
                self.epoch = epoch;
                self.chat_enabled = chat;
                self.delay = delay;
                self.slate = false;
            }
            Message::Clear { epoch } => {
                self.clear();
                self.epoch = epoch;
                self.slate = false;
            }
            Message::Slate { .. } => {
                self.clear();
                self.slate = true;
            }
            Message::End => {
                self.clear();
                self.ended = true;
            }
            Message::State { delay, count, .. } => {
                self.delay = delay;
                self.viewers = count;
            }
            Message::Scene { scene, .. } => self.pending_scene = Some(scene),
            Message::Media {
                media: PublicMedia::Camera { floor },
                ..
            } => self.pending_floor = Some(floor),
            Message::Media {
                media: PublicMedia::Map { reset, last, tiles },
                ..
            } => {
                if reset {
                    self.pending_tiles = Some(BTreeMap::new());
                }
                let pending = self
                    .pending_tiles
                    .as_mut()
                    .ok_or("Map chunk without reset")?;
                for DisplayTile {
                    x,
                    y,
                    floor,
                    appearances,
                } in tiles
                {
                    if x > 64
                        || y > 64
                        || pending.insert((floor, y, x), appearances).is_some()
                        || pending.len() > 2016
                    {
                        return Err("Invalid cast map");
                    }
                }
                if last {
                    let scene = self.pending_scene.take().ok_or("Cast map without scene")?;
                    self.floor = Some(self.pending_floor.take().ok_or("Cast map without camera")?);
                    self.tiles = self.pending_tiles.take().ok_or("Cast map without reset")?;
                    self.actors = scene.actors;
                    self.effects.clear();
                    self.slate = false;
                }
            }
            Message::Media {
                media: PublicMedia::Effects(events),
                ..
            } => {
                if self.floor.is_some() {
                    self.effects
                        .extend(events.into_iter().map(|event| (now, event)));
                }
                let excess = self.effects.len().saturating_sub(128);
                self.effects.drain(..excess);
            }
            Message::Media {
                media: PublicMedia::Speech { text, .. },
                ..
            } => {
                if self.floor.is_some() {
                    self.speech.push_back(text);
                    if self.speech.len() > 32 {
                        self.speech.pop_front();
                    }
                }
            }
            Message::ChatLine { handle, text, .. } => {
                self.chat.push_back((handle, text));
                if self.chat.len() > 128 {
                    self.chat.pop_front();
                }
            }
            _ => return Err("Unexpected viewer message"),
        }
        Ok(())
    }
    pub fn status(&self) -> String {
        let state = if self.ended {
            "Cast ended"
        } else if self.slate {
            "Cast hidden: bystander consent required"
        } else if self.floor.is_none() {
            "Waiting for cast picture / delay warm-up"
        } else {
            "Watching"
        };
        format!(
            "{state} · {} viewers · delay {} s",
            self.viewers, self.delay
        )
    }
}
