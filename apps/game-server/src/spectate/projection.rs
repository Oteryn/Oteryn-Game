//! GRANT/SESSION egress accepts existing public-world payload shapes only. WIRE-1 owns the
//! observer envelope and remaining map/entity/presentation projection integration. No opaque
//! target packet, inventory, private room or command outcome has a forwarding API here.
use super::Refusal;
use oteryn_protocol_oteryn::{
    chat::{ChatLine, encode_chat_line},
    world_spatial::{WorldSpatialObservation, encode_world_spatial},
};

pub(crate) enum Projection<'a> {
    Spatial(&'a WorldSpatialObservation),
    Chat(&'a ChatLine),
}

pub(crate) enum WorldFrame {
    Spatial(Vec<u8>),
    PublicSpeech(Vec<u8>),
}
impl WorldFrame {
    pub(crate) fn bytes(&self) -> &[u8] {
        match self {
            Self::Spatial(bytes) | Self::PublicSpeech(bytes) => bytes,
        }
    }
    pub(super) fn len(&self) -> usize {
        self.bytes().len()
    }
}
impl Projection<'_> {
    pub(super) fn encode(self) -> Result<Option<WorldFrame>, Refusal> {
        match self {
            Self::Spatial(fact) => Ok(Some(WorldFrame::Spatial(encode_world_spatial(fact)))),
            Self::Chat(line @ ChatLine::Local { .. }) => encode_chat_line(line)
                .map(|bytes| Some(WorldFrame::PublicSpeech(bytes)))
                .map_err(|_| Refusal::NotAvailable),
            Self::Chat(ChatLine::Private { .. } | ChatLine::Room { .. } | ChatLine::Dropped) => {
                Ok(None)
            }
        }
    }
}
