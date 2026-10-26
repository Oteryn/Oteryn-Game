//! CHARACTER-INFO-1 transport seam. Native owners resolve identity and facts.
use oteryn_protocol_oteryn::character_info::{CharacterInfoOperation, CharacterTitle, PageRequest};

use super::{ComposedFreshAdmission, capabilities::OfferedCapability, connection::AdmittedSession};

pub(super) fn offered(base: &[OfferedCapability], available: bool) -> Vec<OfferedCapability> {
    let mut result: Vec<_> = base
        .iter()
        .copied()
        .filter(|entry| {
            entry.id != oteryn_protocol_oteryn::character_info::CAPABILITY_CHARACTER_INFO_V1
        })
        .collect();
    if available {
        let at = result.partition_point(|entry| {
            entry.id < oteryn_protocol_oteryn::character_info::CAPABILITY_CHARACTER_INFO_V1
        });
        result.insert(
            at,
            OfferedCapability {
                id: 97,
                requires: &[],
            },
        );
    }
    result
}

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
        fn character_info_available(&self) -> bool {
            false
        }
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

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn character_info_command(
        &self,
        admitted: AdmittedSession,
        operation: CharacterInfoOperation,
    ) -> CharacterInfoReply {
        use oteryn_protocol_oteryn::character_info::CAPABILITY_CHARACTER_INFO_V1;
        if !admitted
            .continuity
            .selected_capabilities
            .contains(CAPABILITY_CHARACTER_INFO_V1)
        {
            return CharacterInfoReply::Rejected;
        }
        let Some(fence) = admitted.item_fence else {
            return CharacterInfoReply::Rejected;
        };
        if fence.game_session_id != admitted.game_session_id
            || fence.connection_generation.get() != admitted.continuity.connection_generation
        {
            return CharacterInfoReply::Rejected;
        }
        let character = fence.character_id;
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .ok()
            .and_then(|duration| i64::try_from(duration.as_millis()).ok())
            .unwrap_or(0);
        match operation {
            CharacterInfoOperation::Unlock(title) => {
                // Qualification is deliberately sourced from the native owners. Until an
                // observation proves a catalogue predicate, a client cannot mint an earned fact.
                let facts = match self
                    .root
                    .character_title_facts(self.character, character)
                    .await
                {
                    Ok(facts) => facts,
                    Err(_) => return CharacterInfoReply::Rejected,
                };
                let qualification = oteryn_protocol_oteryn::character_info::qualify(title, facts);
                if !oteryn_protocol_oteryn::character_info::valid_at(qualification, now) {
                    return CharacterInfoReply::Rejected;
                }
                let expiry = match qualification.lifetime {
                    oteryn_protocol_oteryn::character_info::TitleLifetime::Until(value) => {
                        Some(value)
                    }
                    _ => None,
                };
                match self
                    .root
                    .unlock_character_title(self.character, character, title, now, expiry)
                    .await
                {
                    Ok(_) => CharacterInfoReply::Payload(vec![2, title as u8]),
                    Err(_) => CharacterInfoReply::Rejected,
                }
            }
            CharacterInfoOperation::Select(title) => match self
                .root
                .select_character_title(self.character, character, title)
                .await
            {
                Ok(revision) => {
                    let mut bytes = vec![3];
                    bytes.extend_from_slice(&revision.to_le_bytes());
                    CharacterInfoReply::Payload(bytes)
                }
                Err(_) => CharacterInfoReply::Rejected,
            },
            CharacterInfoOperation::QuerySelf(page) => {
                self.character_info_read(character, character, page, now)
                    .await
            }
            CharacterInfoOperation::QueryVisible {
                character: target,
                page,
            } => {
                let Ok(target) = crate::domain::CharacterId::from_bytes(target) else {
                    return CharacterInfoReply::Rejected;
                };
                self.character_info_read(character, target, page, now).await
            }
            CharacterInfoOperation::LookVisible { character: target } => {
                let Ok(target) = crate::domain::CharacterId::from_bytes(target) else {
                    return CharacterInfoReply::Rejected;
                };
                self.character_info_read(
                    character,
                    target,
                    PageRequest {
                        offset: 0,
                        limit: 1,
                    },
                    now,
                )
                .await
            }
        }
    }

    async fn character_info_read(
        &self,
        viewer: crate::domain::CharacterId,
        target: crate::domain::CharacterId,
        page: PageRequest,
        now: i64,
    ) -> CharacterInfoReply {
        let mut bytes = match self
            .root
            .read_character_info(self.character, viewer, target, page, now)
            .await
        {
            Ok(bytes) => bytes,
            Err(_) => return CharacterInfoReply::Rejected,
        };
        let facts = match self
            .root
            .character_title_facts(self.character, target)
            .await
        {
            Ok(facts) => facts,
            Err(_) => return CharacterInfoReply::Rejected,
        };
        let titles = match self
            .root
            .read_and_requalify_character_titles(self.character, target, facts, now)
            .await
        {
            Ok(titles) => titles,
            Err(_) => return CharacterInfoReply::Rejected,
        };
        bytes.push(u8::try_from(titles.earned.len()).unwrap_or(u8::MAX));
        bytes.extend(titles.earned.into_iter().map(|(title, _)| title as u8));
        bytes.push(titles.selected.map_or(0, |title| title as u8));
        CharacterInfoReply::Payload(bytes)
    }
}

macro_rules! composed_methods {
    () => {
        fn character_info_available(&self) -> bool {
            true
        }
        async fn character_info(
            &self,
            admitted: connection::AdmittedSession,
            operation: oteryn_protocol_oteryn::character_info::CharacterInfoOperation,
        ) -> character_info::CharacterInfoReply {
            self.character_info_command(admitted, operation).await
        }
    };
}
pub(super) use composed_methods;

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
