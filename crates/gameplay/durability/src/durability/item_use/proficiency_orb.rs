//! P08 Lunar Ascension Orb: `ORB_RANK` burns one orb through the item-use writer in the
//! modification's own transaction (PROFICIENCY-1C §5). The burn's TransactionId is the
//! modification's occurrence; migration 0575 pairs the burn with its single ORB_RANK line at
//! commit. English TibiaWiki `Lunar_Ascension_Orb`; tibiamonk perk shaping (read 2026-10-30).
use super::*;

pub const ORB_CAUSE: u32 = 495;

/// The one orb `ORB_RANK` burns.
pub const LUNAR_ASCENSION_ORB: &str = "oteryn:item.tibia.i53695";

/// Burn one orb in the caller's transaction, after its gameplay fence locked the holder's root.
pub(in crate::durability) async fn burn_orb(
    connection: &mut PgConnection,
    fence: &CurrentCharacterItemFence,
    request: &ItemUseRequest,
    tuple: SelectedType2Tuple,
) -> Result<ItemUseOutcome> {
    request.validate()?;
    if request.kind != ItemUseKind::ProficiencyOrb
        || request.source_definition.production_key != LUNAR_ASCENSION_ORB
    {
        return Err(ItemUseError::InvalidInput);
    }
    write(connection, fence, request, tuple).await
}
