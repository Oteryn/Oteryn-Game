//! Public encounter facade for the shared source health policy.
pub use oteryn_gameplay_domain::domain::soul_war_mirror::*;

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn srx3_mirror_image_direct_hits_preserve_exact_source_vocation_distribution()
    -> Result<(), String> {
        for own in Apparition::ALL {
            let mut counts = [0; 5];
            for draw in 0..200 {
                let Some(DirectPlayerReaction::ReplaceAtFullHealth(target)) =
                    direct_player_reaction(Some(own), draw)
                else {
                    return Err("valid player's direct hit must replace the real image".into());
                };
                counts[Apparition::ALL
                    .iter()
                    .position(|v| *v == target)
                    .ok_or("known apparition")?] += 1;
            }
            for (target, count) in Apparition::ALL.into_iter().zip(counts) {
                assert_eq!(count, if target == own { 140 } else { 15 });
            }
        }
        assert_eq!(direct_player_reaction(Some(Apparition::Knight), 200), None);
        Ok(())
    }
    #[test]
    fn srx3_mirror_image_vocationless_source_failure_still_removes_image() -> Result<(), String> {
        let mut disappeared = 0;
        let mut counts = [0; 5];
        for draw in 0..200 {
            match direct_player_reaction(None, draw).ok_or("valid source draw")? {
                DirectPlayerReaction::Disappear => disappeared += 1,
                DirectPlayerReaction::ReplaceAtFullHealth(target) => {
                    counts[Apparition::ALL
                        .iter()
                        .position(|v| *v == target)
                        .ok_or("known")?] += 1
                }
            }
        }
        assert_eq!(disappeared, 140);
        assert_eq!(counts, [12; 5]);
        Ok(())
    }
    #[test]
    fn srx3_mirror_image_non_player_damage_keeps_accepted_one_hp_floor() {
        assert_eq!(non_player_damage(100, 23), 23);
        assert_eq!(non_player_damage(100, 100), 99);
        assert_eq!(non_player_damage(1, u32::MAX), 0);
        assert_eq!(non_player_damage(0, 5), 0);
    }
}
