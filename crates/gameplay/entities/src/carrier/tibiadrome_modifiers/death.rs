//! ARENA1 A4: the actual last hitter is the centre, including a familiar.
use super::*;
impl DromeModifierState {
    pub fn death(
        &self,
        corpse: MovementLocalPosition,
        last_hitter: Option<ExactActorRef>,
        participants: &[DromeParticipant],
    ) -> Vec<DromeModifierEffect> {
        let hitter = participants
            .iter()
            .find(|player| Some(player.actor) == last_hitter)
            .copied();
        self.death_at(corpse, hitter, participants)
    }
    pub fn death_at(
        &self,
        corpse: MovementLocalPosition,
        last_hitter: Option<DromeParticipant>,
        participants: &[DromeParticipant],
    ) -> Vec<DromeModifierEffect> {
        let mut effects = Vec::new();
        for modifier in self.selected {
            let (center, percent) = match modifier {
                DromeModifier::GoingDownWithMe => {
                    let Some(hitter) = last_hitter else {
                        continue;
                    };
                    if !participants
                        .iter()
                        .any(|player| player.actor == hitter.actor)
                    {
                        effects.push(DromeModifierEffect::CurrentHealthDamage {
                            target: hitter.actor,
                            percent: 40,
                        });
                    }
                    (hitter.position, 40)
                }
                DromeModifier::ExplodingCorpses => (corpse, 25),
                _ => continue,
            };
            effects.extend(
                participants
                    .iter()
                    .filter(|player| adjacent(center, player.position))
                    .map(|player| DromeModifierEffect::CurrentHealthDamage {
                        target: player.actor,
                        percent,
                    }),
            );
        }
        effects
    }
}
