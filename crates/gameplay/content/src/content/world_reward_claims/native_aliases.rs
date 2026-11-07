//! Explicit aliases for source startup UIDs absent from the native map.
use super::*;
use serde::Deserialize;
use std::collections::BTreeSet;
use std::sync::OnceLock;

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct NativePlacementAlias {
    pub claim: String,
    pub expected_revision: String,
    pub expected_source_uid: u16,
    pub project_position: AliasPosition,
    pub appearance: u16,
    pub ordinal: u8,
    pub sources: Vec<String>,
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct AliasPosition {
    x: u16,
    y: u16,
    z: u8,
}

impl AliasPosition {
    fn cell(self) -> Option<BoundCell> {
        let floor = i8::try_from(self.z).ok()?.checked_neg()?;
        (-15..=0).contains(&floor).then_some(BoundCell {
            x: self.x,
            y: self.y,
            floor,
        })
    }
    fn matches(self, value: &Value) -> bool {
        value.get("x").and_then(Value::as_u64) == Some(u64::from(self.x))
            && value.get("y").and_then(Value::as_u64) == Some(u64::from(self.y))
            && value.get("z").and_then(Value::as_u64) == Some(u64::from(self.z))
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Document {
    schema: u32,
    aliases: Vec<NativePlacementAlias>,
}

pub(super) fn load() -> Result<&'static [NativePlacementAlias], ClaimsError> {
    static DATA: OnceLock<Result<Vec<NativePlacementAlias>, ClaimsError>> = OnceLock::new();
    DATA.get_or_init(|| {
        parse(include_str!(
            "../../../../../../content/quests/native-reward-placement-aliases.json"
        ))
    })
    .as_ref()
    .map(Vec::as_slice)
    .map_err(Clone::clone)
}

fn parse(text: &str) -> Result<Vec<NativePlacementAlias>, ClaimsError> {
    let document: Document =
        serde_json::from_str(text).map_err(|_| ClaimsError("invalid native placement aliases"))?;
    if document.schema != 1 || document.aliases.len() > 64 {
        return Err(ClaimsError("native alias bounds"));
    }
    let mut sources = BTreeSet::new();
    let mut targets = BTreeSet::new();
    for alias in &document.aliases {
        let p = alias.project_position;
        if !alias.claim.starts_with("oteryn:reward-claim.")
            || ProductionKey::new(&alias.claim).is_err()
            || DefinitionRevisionRef::new(&alias.expected_revision).is_err()
            || alias.expected_source_uid == 0
            || alias.appearance == 0
            || alias.ordinal > 63
            || p.z > 15
            || alias.sources.is_empty()
            || alias.sources.len() > 8
            || alias.sources.iter().any(|s| s.is_empty() || s.len() > 1024)
            || !sources.insert((
                alias.claim.clone(),
                alias.expected_source_uid,
                p.x,
                p.y,
                p.z,
            ))
            || !targets.insert((p.x, p.y, p.z, alias.ordinal))
        {
            return Err(ClaimsError("invalid or ambiguous native placement alias"));
        }
    }
    Ok(document.aliases)
}

impl NativePlacementAlias {
    /// The supplement cannot invent a source owner or a reward plan.
    pub(super) fn validates(&self, definition: &serde_json::Map<String, Value>) -> bool {
        if definition
            .get("identity")
            .and_then(|v| v.get("revision"))
            .and_then(Value::as_str)
            != Some(self.expected_revision.as_str())
        {
            return false;
        }
        let Some(placements) = definition.get("placements").and_then(Value::as_array) else {
            return false;
        };
        placements
            .iter()
            .filter(|placement| {
                let binding = &placement["source_binding"];
                self.project_position.matches(&binding["project_position"])
                    && placement["appearance_tibia_id"].as_u64() == Some(u64::from(self.appearance))
                    && binding.get("native_ordinal").is_none()
                    && binding["legacy_unique_ids"].as_array().is_some_and(|ids| {
                        ids.iter().any(|id| {
                            id["server"] == "crystalserver"
                                && id["unique_id"].as_u64()
                                    == Some(u64::from(self.expected_source_uid))
                        })
                    })
            })
            .count()
            == 1
    }

    pub(super) fn applies(&self, key: &str, placement: &CandidatePlacement) -> bool {
        self.claim == key
            && placement.native_ordinal.is_none()
            && placement.crystal_unique_id == Some(u64::from(self.expected_source_uid))
            && placement.appearance == u64::from(self.appearance)
            && placement.cell == self.project_position.cell()
    }

    pub(super) fn bind(
        &self,
        base: &impl RewardClaimBase,
        placement: &CandidatePlacement,
    ) -> Result<(BoundCell, u64), UnboundReason> {
        let cell = placement.cell.ok_or(UnboundReason::CellOutOfBounds)?;
        let tile = base
            .tile(cell.x, cell.y, cell.floor)
            .ok_or(UnboundReason::NoEntry)?;
        let top = tile.depths().iter().filter(|depth| **depth == 0).count();
        // An alias is only for a wholly untagged native tile, never a conflicting source identity.
        for ordinal in 0..top {
            let ordinal = u8::try_from(ordinal).map_err(|_| UnboundReason::NoEntry)?;
            if tile.unique(ordinal).is_some() {
                return Err(UnboundReason::NoEntry);
            }
        }
        if usize::from(self.ordinal) >= top {
            return Err(UnboundReason::NoEntry);
        }
        if tile.appearance(self.ordinal) != Some(self.appearance) {
            return Err(UnboundReason::AppearanceMismatch);
        }
        let key = bundle::placement_key(cell.floor, cell.x, cell.y, self.ordinal)
            .ok_or(UnboundReason::NoEntry)?;
        Ok((cell, key))
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Copy)]
    struct Tile {
        uid: Option<u16>,
        ground_uid: Option<u16>,
        appearance: u16,
    }
    impl RewardClaimTile for Tile {
        fn depths(&self) -> &[u8] {
            &[0, 0]
        }
        fn unique(&self, ordinal: u8) -> Option<UniqueEntry> {
            let uid = match ordinal {
                0 => self.ground_uid,
                1 => self.uid,
                _ => None,
            }?;
            Some(UniqueEntry {
                unique: uid,
                appearance: self.appearance(ordinal),
            })
        }
        fn appearance(&self, ordinal: u8) -> Option<u16> {
            match ordinal {
                0 => Some(351),
                1 => Some(self.appearance),
                _ => None,
            }
        }
    }
    struct Base(Tile);
    impl RewardClaimBase for Base {
        type Tile<'a> = Tile;
        fn tile(&self, x: u16, y: u16, floor: i8) -> Option<Tile> {
            ((x, y, floor) == (32591, 32097, -14)).then_some(self.0)
        }
    }
    fn base() -> Base {
        Base(Tile {
            uid: None,
            ground_uid: None,
            appearance: 2472,
        })
    }
    fn fixture() -> Result<(String, serde_json::Value), ClaimsError> {
        let alias: Value = serde_json::from_str(include_str!(
            "../../../../../../content/quests/native-reward-placement-aliases.json"
        ))
        .map_err(|_| ClaimsError("alias fixture"))?;
        let alias = alias["aliases"]
            .as_array()
            .ok_or(ClaimsError("alias rows"))?
            .iter()
            .find(|a| a["claim"] == "oteryn:reward-claim.kv.mintwallincyclops")
            .ok_or(ClaimsError("mint alias"))?
            .clone();
        for shard in REWARD_CLAIM_SHARDS {
            let shard: Value =
                serde_json::from_str(shard).map_err(|_| ClaimsError("source fixture"))?;
            for record in shard["records"]
                .as_array()
                .ok_or(ClaimsError("source rows"))?
            {
                if record["definition"]["identity"]["key"] == alias["claim"] {
                    return Ok((
                        serde_json::json!({"records":[record]}).to_string(),
                        serde_json::json!({"schema":1,"aliases":[alias]}),
                    ));
                }
            }
        }
        Err(ClaimsError("mint source"))
    }
    fn bind(
        base: &Base,
        shard: &str,
        aliases: &[NativePlacementAlias],
        admitted: bool,
    ) -> Result<BindingReport, ClaimsError> {
        super::super::bind_reward_claims_with_aliases(base, &[shard], &|_| admitted, aliases)
    }

    #[test]
    fn native_alias_keeps_source_ownership_and_ordinary_binder_strict() -> Result<(), ClaimsError> {
        let (shard, document) = fixture()?;
        let aliases = parse(&document.to_string())?;
        assert!(
            super::super::bind_reward_claims(&base(), &[&shard], &|_| true)?
                .served
                .is_empty()
        );
        let report = bind(&base(), &shard, &aliases, true)?;
        assert_eq!(report.served.len(), 1);
        assert_eq!(
            report.served[0].key.as_str(),
            "oteryn:reward-claim.kv.mintwallincyclops"
        );
        assert_eq!(report.served[0].placements[0].placement.items[0].count, 1);
        assert_eq!(
            report.served[0].placements[0].placement.items[0]
                .item
                .key()
                .as_str(),
            "oteryn:item.tibia.i3028"
        );
        assert!(bind(&base(), &shard, &aliases, false)?.served.is_empty());
        for (uid, ground_uid, appearance) in [
            (Some(6139), None, 2472),
            (None, Some(999), 2472),
            (None, None, 2473),
        ] {
            assert!(
                bind(
                    &Base(Tile {
                        uid,
                        ground_uid,
                        appearance
                    }),
                    &shard,
                    &aliases,
                    true
                )?
                .served
                .is_empty()
            );
        }
        let legacy = Base(Tile {
            uid: Some(6138),
            ground_uid: None,
            appearance: 2472,
        });
        assert_eq!(bind(&legacy, &shard, &aliases, true)?.served.len(), 1);
        Ok(())
    }

    #[test]
    fn native_alias_cannot_change_source_revision_uid_position_or_reward() -> Result<(), ClaimsError>
    {
        let (shard, document) = fixture()?;
        for case in 0..5 {
            let mut changed = document.clone();
            match case {
                0 => changed["aliases"][0]["expected_revision"] = "reward-claim-r2".into(),
                1 => changed["aliases"][0]["expected_source_uid"] = 6139.into(),
                2 => changed["aliases"][0]["project_position"]["x"] = 32590.into(),
                3 => changed["aliases"][0]["appearance"] = 2473.into(),
                _ => {
                    changed["aliases"][0]["claim"] = "oteryn:reward-claim.kv.not_the_source".into()
                }
            }
            let aliases = parse(&changed.to_string())?;
            assert!(
                bind(&base(), &shard, &aliases, true).is_err(),
                "declaration {case}"
            );
        }
        let mut changed = document.clone();
        changed["aliases"][0]["give"] =
            serde_json::json!({"item":"oteryn:item.tibia.i3031","count":100});
        assert!(parse(&changed.to_string()).is_err());
        let mut changed = document.clone();
        let alias = changed["aliases"][0].clone();
        changed["aliases"]
            .as_array_mut()
            .ok_or(ClaimsError("aliases"))?
            .push(alias);
        assert!(parse(&changed.to_string()).is_err());
        let mut changed = document;
        changed["aliases"][0]["ordinal"] = 2.into();
        assert!(
            bind(&base(), &shard, &parse(&changed.to_string())?, true)?
                .served
                .is_empty()
        );
        Ok(())
    }
}
