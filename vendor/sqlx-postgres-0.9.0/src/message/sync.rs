use crate::message::{FrontendMessage, FrontendMessageFormat};
use sqlx_core::Error;
use std::num::Saturating;

#[derive(Debug)]
pub struct Sync;

impl FrontendMessage for Sync {
    const FORMAT: FrontendMessageFormat = FrontendMessageFormat::Sync;

    #[inline(always)]
    fn body_size_bound(&self) -> Saturating<usize> {
        Saturating(0)
    }

    #[inline(always)]
    fn encode_body(&self, _buf: &mut Vec<u8>) -> Result<(), Error> {
        Ok(())
    }
}
