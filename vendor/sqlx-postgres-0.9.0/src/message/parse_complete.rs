use crate::message::{BackendMessage, BackendMessageFormat};
use sqlx_core::net::OwnedBytes as Bytes;
use sqlx_core::Error;

pub struct ParseComplete;

impl BackendMessage for ParseComplete {
    const FORMAT: BackendMessageFormat = BackendMessageFormat::ParseComplete;

    fn decode_body(bytes: Bytes) -> Result<Self, Error> {
        if !bytes.is_empty() {
            return Err(Error::Io(std::io::ErrorKind::InvalidData.into()));
        }
        Ok(ParseComplete)
    }
}
