//! Exact Crystal Galthen BossLever closure; no generic unknown-hook admission.
use super::*;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BossCallbackSourceClosure {
    pub monster_path: String,
    pub monster_sha256: String,
}

/// Crystal boss_lever.lua:337-338 installs SimpleTeleport with no condition.
/// teleport.lua:5-27 accepts only Players and emits effect 11 at the landing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QualifiedBossExitTeleporter {
    pub(super) trigger: LogicalCell,
    pub(super) destination: LogicalCell,
}
impl QualifiedBossExitTeleporter {
    pub fn trigger(self) -> LogicalCell {
        self.trigger
    }
    pub fn destination(self) -> LogicalCell {
        self.destination
    }
}

pub(super) fn exact_closure(candidate: &BossLeverCandidate) -> bool {
    let Some(proof) = &candidate.source_callback_closure else {
        return false;
    };
    let (suffix, lever, monster, profile, trigger) = match candidate.creature_key.as_deref() {
        Some("oteryn:creature.ahau") => (
            "ahau",
            "637e8b3bac91884d5fc64be387847de05c5ad5fdaa2283134fd6f5777777911d",
            "7cadc3ac4adc02a216df390e566704317470fc359c895de735041c296368e8c6",
            "bcbcb86ca93364c98266cadf211cc5db18492ead42df59a04b57fe412e24f2c2",
            Some([34002, 31706, 10]),
        ),
        Some("oteryn:creature.mitmah_vanguard") => (
            "mitmah_vanguard",
            "47896b61cb0b2d1a4f04c72c9330cce5a4e3a885a1069117eeab5cf5150c170a",
            "bb631589f2d316ad907ed3a2fa0bdfacbae29d0fe9ab1909805e39e6c0e8855e",
            "d9ecdc5e3241798184e8fd5fbd1269b7f4ebe2ba771dabc798e942fb9277a568",
            None,
        ),
        _ => return false,
    };
    candidate.path
        == format!("data-global/scripts/quests/adventures_of_galthen/actions_{suffix}.lua")
        && candidate.sha256 == lever
        && proof.monster_path == format!("data-global/monster/bosses/{suffix}.lua")
        && proof.monster_sha256 == monster
        && candidate.native_profile_sha256.as_deref() == Some(profile)
        && candidate.source_exit_teleporter == trigger
        && candidate.required_callbacks == ["BossLeverOnDeath"]
        && candidate.callbacks_known
        && candidate.blockers.is_empty()
}
