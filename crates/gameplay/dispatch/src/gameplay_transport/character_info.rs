//! CHARACTER-INFO-1 transport seam. Native owners resolve identity and facts.
use oteryn_protocol_oteryn::character_info::{CharacterInfoOperation, CharacterTitle, PageRequest};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CharacterInfoReply {
    Payload(Vec<u8>),
    Rejected,
    Terminal(oteryn_protocol_oteryn::FoundationProtocolError),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DecodeError;

pub fn decode(payload: &[u8]) -> Result<CharacterInfoOperation, DecodeError> {
    let (&tag, rest) = payload.split_first().ok_or(DecodeError)?;
    let page = |bytes: &[u8]| {
        let [a, b, limit] = bytes else {
            return Err(DecodeError);
        };
        PageRequest::new(u16::from_le_bytes([*a, *b]), *limit).ok_or(DecodeError)
    };
    let title = |raw| match raw {
        1 => Ok(CharacterTitle::AdeptArmorer),
        2 => Ok(CharacterTitle::AdeptEnhancer),
        3 => Ok(CharacterTitle::Cartographer),
        4 => Ok(CharacterTitle::RotationChampion),
        _ => Err(DecodeError),
    };
    match (tag, rest) {
        (0, bytes) => Ok(CharacterInfoOperation::QuerySelf(page(bytes)?)),
        (1, bytes) if bytes.len() == 19 => {
            let mut character = [0; 16];
            character.copy_from_slice(&bytes[..16]);
            Ok(CharacterInfoOperation::QueryVisible {
                character,
                page: page(&bytes[16..])?,
            })
        }
        (2, [value]) => Ok(CharacterInfoOperation::Unlock(title(*value)?)),
        (3, [0]) => Ok(CharacterInfoOperation::Select(None)),
        (3, [value]) => Ok(CharacterInfoOperation::Select(Some(title(*value)?))),
        (4, id) if id.len() == 16 => {
            let mut character = [0; 16];
            character.copy_from_slice(id);
            Ok(CharacterInfoOperation::LookVisible { character })
        }
        _ => Err(DecodeError),
    }
}

macro_rules! authority_methods {
    () => {
        fn character_info(
            &self,
            _admitted: $crate::gameplay_transport::AdmittedSession,
            _operation: oteryn_protocol_oteryn::character_info::CharacterInfoOperation,
        ) -> impl std::future::Future<
            Output = $crate::gameplay_transport::character_info::CharacterInfoReply,
        > {
            async { $crate::gameplay_transport::character_info::CharacterInfoReply::Rejected }
        }
    };
}
pub(super) use authority_methods;

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bounds_pages() {
        assert!(decode(&[0, 0, 0, 0]).is_err());
        assert!(decode(&[0, 0, 0, 51]).is_err());
    }
    #[test]
    fn selection_clear() {
        assert_eq!(decode(&[3, 0]), Ok(CharacterInfoOperation::Select(None)));
    }
}
