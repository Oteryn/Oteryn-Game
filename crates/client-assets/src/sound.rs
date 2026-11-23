//! Routing information from the pinned official sounds protobuf.
use prost::Message;
use std::collections::BTreeMap;

#[derive(Message)]
struct Sounds {
    #[prost(message, repeated, tag = "1")]
    files: Vec<Sound>,
    #[prost(message, repeated, tag = "2")]
    effects: Vec<Effect>,
    #[prost(message, repeated, tag = "5")]
    music: Vec<Music>,
}
#[derive(Message)]
struct Sound {
    #[prost(uint32, optional, tag = "1")]
    id: Option<u32>,
    #[prost(string, optional, tag = "2")]
    filename: Option<String>,
}
#[derive(Message)]
struct Effect {
    #[prost(uint32, optional, tag = "1")]
    id: Option<u32>,
    #[prost(uint32, optional, tag = "2")]
    kind: Option<u32>,
}
#[derive(Message)]
struct Music {
    #[prost(uint32, optional, tag = "2")]
    sound: Option<u32>,
    #[prost(uint32, optional, tag = "3")]
    kind: Option<u32>,
}
#[derive(Default)]
pub struct SoundRouting {
    pub effects: BTreeMap<u32, u32>,
    pub title: Option<String>,
}
impl SoundRouting {
    pub fn decode(bytes: &[u8]) -> Option<Self> {
        let sounds = Sounds::decode(bytes).ok()?;
        let title_id = sounds
            .music
            .iter()
            .find(|m| m.kind == Some(3))
            .and_then(|m| m.sound);
        let title = title_id.and_then(|id| {
            sounds
                .files
                .into_iter()
                .find(|s| s.id == Some(id))?
                .filename
        });
        let effects = sounds
            .effects
            .into_iter()
            .filter_map(|e| Some((e.id?, e.kind?)))
            .collect();
        Some(Self { effects, title })
    }
}
