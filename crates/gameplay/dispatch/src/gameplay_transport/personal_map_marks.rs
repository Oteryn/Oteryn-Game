//! M16 display projection. Native Character/Quest receipts remain the sole writer.
use super::{ComposedFreshAdmission, capabilities::OfferedCapability, connection::AdmittedSession};
use oteryn_protocol_oteryn::personal_map_marks::{
    CAPABILITY_PERSONAL_MAP_MARKS_V1, PersonalMapMark, PersonalMapMarks,
};
pub(super) fn offered(base: &[OfferedCapability], available: bool) -> Vec<OfferedCapability> {
    base.iter()
        .copied()
        .filter(|entry| available || entry.id != CAPABILITY_PERSONAL_MAP_MARKS_V1)
        .collect()
}
macro_rules! authority_methods {
    () => {
        fn personal_map_marks_available(&self) -> bool {
            false
        }
        fn personal_map_marks_query(
            &self,
            _admitted: $crate::gameplay_transport::AdmittedSession,
        ) -> impl std::future::Future<Output = Option<Vec<u8>>> {
            async { None }
        }
    };
}
pub(super) use authority_methods;
impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn personal_map_marks_page(
        &self,
        admitted: AdmittedSession,
    ) -> Option<Vec<u8>> {
        if !admitted
            .continuity
            .selected_capabilities
            .contains(CAPABILITY_PERSONAL_MAP_MARKS_V1)
        {
            return None;
        }
        let _permit = self.spell_lane_permit().await?;
        {
            let runtime = self.runtime.lock().await;
            if !super::battle_instinct_status::actor_is_current(&runtime, admitted) {
                return None;
            }
        }
        let fence = self
            .current_quest_fence(admitted.game_session_id)
            .await
            .ok()??;
        let admitted_fence = admitted.item_fence?;
        if fence.connection_generation.get() != admitted.continuity.connection_generation
            || fence.item_fence() != admitted_fence
        {
            return None;
        }
        let snapshot = self
            .root
            .read_character_quest_map_marks(self.character, self.holder, fence)
            .await
            .ok()?;
        {
            let runtime = self.runtime.lock().await;
            if !super::battle_instinct_status::actor_is_current(&runtime, admitted)
                || runtime.binding().runtime_scope() != fence.runtime_scope
                || runtime.binding().scope_generation().get()
                    != fence.scope_ownership_generation.get()
            {
                return None;
            }
        }
        encode_snapshot(snapshot)
    }
}
macro_rules! composed_methods {
    () => {
        fn personal_map_marks_available(&self) -> bool {
            self.active_generation
                .and_then(|active| active.native_gameplay())
                .is_some()
                && oteryn_gameplay_quests::quest::reward_callbacks::load()
                    .is_ok_and(|rows| rows.iter().any(|row| row.map_mark.is_some()))
        }
        async fn personal_map_marks_query(
            &self,
            admitted: connection::AdmittedSession,
        ) -> Option<Vec<u8>> {
            self.personal_map_marks_page(admitted).await
        }
    };
}
pub(super) use composed_methods;

fn encode_snapshot(
    snapshot: crate::durability::quest_state::QuestMapMarksSnapshot,
) -> Option<Vec<u8>> {
    let mut marks = snapshot
        .marks
        .into_iter()
        .map(|fact| {
            // Adding a new durable kind must explicitly decide its negotiated wire representation.
            let oteryn_gameplay_quests::quest::map_marks::MapMarkKind::Cross = fact.marker.kind;
            PersonalMapMark {
                key: fact.key,
                x: fact.marker.position.x,
                y: fact.marker.position.y,
                floor: fact.marker.position.floor,
                text: fact.marker.text,
            }
        })
        .collect::<Vec<_>>();
    // PostgreSQL's database collation is not the protocol's byte ordering.
    marks.sort_by(|left, right| left.key.cmp(&right.key));
    PersonalMapMarks(marks).encode()
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::durability::quest_state::{PersonalQuestMapMark, QuestMapMarksSnapshot};
    use oteryn_gameplay_quests::quest::{
        map_marks::{MapMarkKind, QuestMapMark},
        world_callbacks::Position,
    };
    #[test]
    fn personal_map_marks_selection_requires_the_world_map_dependency_closure()
    -> Result<(), &'static str> {
        use crate::gameplay_transport::capabilities::{
            BUNDLE_WORLD_OFFERED_CAPABILITIES, SelectedCapabilities,
        };
        let incomplete =
            SelectedCapabilities::select(BUNDLE_WORLD_OFFERED_CAPABILITIES, &[3, 6, 18, 450])
                .ok_or("bounded selection")?;
        assert!(!incomplete.contains(18));
        assert!(!incomplete.contains(450));
        let complete =
            SelectedCapabilities::select(BUNDLE_WORLD_OFFERED_CAPABILITIES, &[3, 4, 6, 18, 450])
                .ok_or("bounded selection")?;
        assert!(complete.contains(18));
        assert!(complete.contains(450));
        Ok(())
    }
    #[test]
    fn personal_map_marks_projection_sorts_independently_of_database_collation()
    -> Result<(), &'static str> {
        let fact = |key: &str| PersonalQuestMapMark {
            key: key.into(),
            quest: "oteryn:quest.targuna_quest".into(),
            marker: QuestMapMark {
                position: Position {
                    x: 31948,
                    y: 31924,
                    floor: -7,
                },
                kind: MapMarkKind::Cross,
                text: "Treasure".into(),
            },
        };
        let bytes = encode_snapshot(QuestMapMarksSnapshot {
            character_revision: 123,
            marks: vec![fact("z"), fact("a-a"), fact("a_a")],
        })
        .ok_or("encode")?;
        let wire = PersonalMapMarks::decode(&bytes).ok_or("decode")?;
        assert_eq!(
            wire.0
                .iter()
                .map(|mark| mark.key.as_str())
                .collect::<Vec<_>>(),
            vec!["a-a", "a_a", "z"]
        );
        assert!(
            wire.0
                .iter()
                .all(|mark| (mark.x, mark.y, mark.floor, mark.text.as_str())
                    == (31948, 31924, -7, "Treasure"))
        );
        assert!(
            encode_snapshot(QuestMapMarksSnapshot {
                character_revision: 1,
                marks: vec![fact("duplicate"), fact("duplicate")]
            })
            .is_none()
        );
        Ok(())
    }
}
