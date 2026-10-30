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
    let title = |raw| CharacterTitle::from_id(raw).ok_or(DecodeError);
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

fn unix_now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .ok()
        .and_then(|duration| i64::try_from(duration.as_millis()).ok())
        .unwrap_or(0)
}

impl ComposedFreshAdmission<'_, '_, '_> {
    /// PAR-P12a automatic unlock of `session`'s Character from its native facts. Best effort:
    /// a failed refresh keeps every stored title and is repeated at the next trigger.
    pub(super) async fn refresh_session_titles(&self, session: crate::foundation::GameSessionId) {
        let Ok(Some(fence)) = self.current_quest_fence(session).await else {
            return;
        };
        let item_fence = crate::durability::item_transfer::CurrentCharacterItemFence {
            character_id: fence.character_id,
            game_session_id: fence.game_session_id,
            connection_generation: fence.connection_generation,
            character_lease_generation: fence.character_lease_generation,
            runtime_scope: fence.runtime_scope,
            scope_ownership_generation: fence.scope_ownership_generation,
        };
        let _ = self
            .root
            .refresh_character_titles(self.character, self.holder, item_fence, unix_now_ms())
            .await;
    }

    /// The actor-cadence leg of [`Self::refresh_session_titles`] for a Character marked by a
    /// kill settlement (level, tracked creature or boss).
    pub(super) async fn refresh_marked_titles(
        &self,
        actor: crate::foundation::ExactActorRef,
        session: crate::foundation::GameSessionId,
    ) {
        let character = {
            let runtime = self.runtime.lock().await;
            let Ok(lease) = runtime.bound_attacker_lease(actor, session) else {
                return;
            };
            let Ok(character) = crate::domain::CharacterId::from_bytes(*lease.character_id().as_bytes())
            else {
                return;
            };
            character
        };
        if !self.states.title_refresh.is_marked(character) {
            return;
        }
        self.states.title_refresh.unmark(character);
        self.refresh_session_titles(session).await;
    }

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
        let now = unix_now_ms();
        match operation {
            CharacterInfoOperation::Unlock(title) => {
                // Qualification is deliberately sourced from the native owners. Until an
                // observation proves a catalogue predicate, a client cannot mint an earned fact.
                let facts = match self
                    .root
                    .character_title_facts(self.character, character, now)
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
                    .unlock_character_title(self.character, self.holder, fence, title, now, expiry)
                    .await
                {
                    Ok(_) => CharacterInfoReply::Payload(vec![2, title.id()]),
                    Err(_) => CharacterInfoReply::Rejected,
                }
            }
            CharacterInfoOperation::Select(title) => match self
                .root
                .select_character_title(self.character, self.holder, fence, title)
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
                self.character_info_read(fence, character, page, now).await
            }
            CharacterInfoOperation::QueryVisible {
                character: target,
                page,
            } => {
                let Ok(target) = crate::domain::CharacterId::from_bytes(target) else {
                    return CharacterInfoReply::Rejected;
                };
                self.character_info_read(fence, target, page, now).await
            }
            CharacterInfoOperation::LookVisible { character: target } => {
                let Ok(target) = crate::domain::CharacterId::from_bytes(target) else {
                    return CharacterInfoReply::Rejected;
                };
                self.character_info_read(
                    fence,
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
        fence: crate::durability::item_transfer::CurrentCharacterItemFence,
        target: crate::domain::CharacterId,
        page: PageRequest,
        now: i64,
    ) -> CharacterInfoReply {
        // The self view runs the automatic unlock first, so it never shows a stale catalogue.
        if target == fence.character_id
            && self
                .root
                .refresh_character_titles(self.character, self.holder, fence, now)
                .await
                .is_err()
        {
            return CharacterInfoReply::Rejected;
        }
        let mut bytes = match self
            .root
            .read_character_info(self.character, fence.character_id, target, page, now)
            .await
        {
            Ok(bytes) => bytes,
            Err(_) => return CharacterInfoReply::Rejected,
        };
        let facts = match self
            .root
            .character_title_facts(self.character, target, now)
            .await
        {
            Ok(facts) => facts,
            Err(_) => return CharacterInfoReply::Rejected,
        };
        let titles = match self
            .root
            .read_and_requalify_character_titles(
                self.character,
                self.holder,
                fence,
                target,
                facts,
                now,
            )
            .await
        {
            Ok(titles) => titles,
            Err(_) => return CharacterInfoReply::Rejected,
        };
        bytes.push(u8::try_from(titles.earned.len()).unwrap_or(u8::MAX));
        bytes.extend(titles.earned.into_iter().map(|(title, _)| title.id()));
        bytes.push(titles.selected.map_or(0, CharacterTitle::id));
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
    fn every_operation_fits_registered_limit() -> Result<(), Box<dyn std::error::Error>> {
        let registry: serde_json::Value = serde_json::from_str(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../docs/contracts/protocol-oteryn/v1/registry/command_types/0113-character-info.json"
        )))?;
        let maximum = registry["max_payload_bytes"]
            .as_u64()
            .ok_or("registered payload limit")? as usize;
        let mut visible = vec![1];
        visible.extend_from_slice(&[1; 16]);
        visible.extend_from_slice(&[0, 0, 50]);
        let mut look = vec![4];
        look.extend_from_slice(&[1; 16]);
        for payload in [
            vec![0, 0, 0, 50],
            visible,
            vec![2, 1],
            vec![3, 0],
            vec![3, 4],
            look,
        ] {
            let operation = decode(&payload).map_err(|_| "invalid operation fixture")?;
            let encoded = oteryn_protocol_oteryn::encode_client_command(
                1,
                &oteryn_protocol_oteryn::ClientCommandValue {
                    command_id: 1,
                    command_type: oteryn_protocol_oteryn::character_info::COMMAND_TYPE_CHARACTER_INFO,
                    payload: &payload,
                },
            )?;
            let envelope = oteryn_protocol_oteryn::decode_wire_envelope(&encoded)?;
            let command = envelope.client_command(1)?;
            assert_eq!(
                u64::from(command.command_type),
                registry["id"].as_u64().ok_or("command id")?
            );
            assert!(command.payload.len() <= maximum, "{payload:?}");
            assert_eq!(decode(command.payload), Ok(operation));
        }
        assert_eq!(maximum, 20);
        Ok(())
    }
    #[test]
    fn bounds_pages() {
        assert!(decode(&[0, 0, 0, 0]).is_err());
        assert!(decode(&[0, 0, 0, 51]).is_err());
    }
    #[test]
    fn selection_clear() {
        assert_eq!(decode(&[3, 0]), Ok(CharacterInfoOperation::Select(None)));
    }
    /// PAR-P12: the Global-named titles keep their registered wire ids.
    #[test]
    fn p12_global_titles_keep_wire_ids() {
        for (raw, title) in [
            (1, CharacterTitle::AdeptArmorer),
            (2, CharacterTitle::AdeptEnhancer),
            (3, CharacterTitle::Globetrotter),
            (4, CharacterTitle::ReigningDromeChampion),
            // Monk pilgrimage reward: Crystal data-global/npc/enpa_deia_pema.lua.
            (5, CharacterTitle::PensiveWanderer),
        ] {
            assert_eq!(decode(&[2, raw]), Ok(CharacterInfoOperation::Unlock(title)));
            assert_eq!(
                decode(&[3, raw]),
                Ok(CharacterInfoOperation::Select(Some(title)))
            );
        }
        assert_eq!(decode(&[2, 0]), Err(DecodeError));
        assert_eq!(decode(&[2, 44]), Err(DecodeError));
        assert_eq!(decode(&[3, 44]), Err(DecodeError));
    }
}
