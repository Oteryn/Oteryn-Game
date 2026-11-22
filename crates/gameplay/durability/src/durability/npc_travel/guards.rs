//! Source-specific guards for the paid travel transaction.
use super::{TravelRefusal, TravelRoute};
use crate::durability::character_progression::{
    CurrentCharacterGameplayFence, FencedCharacterRoot,
};
use crate::durability::item_mint_audit::Type2Transaction;
use crate::durability::item_use::{ItemUseError, read_use_inventory};
use crate::durability::{DurabilityError, npc_marriage};

pub(super) async fn quote(
    tx: &mut Type2Transaction<'_>,
    fence: CurrentCharacterGameplayFence,
    route: &TravelRoute,
) -> Result<Option<TravelRefusal>, DurabilityError> {
    if route.service == "oteryn:service.travel.ferryman_kamil"
        && route.route.key == "meluna"
        && !npc_marriage::eligible_for_honeymoon(tx, fence).await?
    {
        return Ok(Some(TravelRefusal::HoneymoonIneligible));
    }
    Ok(None)
}

pub(super) async fn confirmation(
    tx: &mut Type2Transaction<'_>,
    fence: CurrentCharacterGameplayFence,
    root: FencedCharacterRoot,
    route: &TravelRoute,
) -> Result<Option<TravelRefusal>, DurabilityError> {
    // Crystal 201189f8 captain_sinbeard.lua and petros.lua both run hasCheese on "yes".
    let refusal = match (route.service.as_str(), route.route.key.as_str()) {
        (
            "oteryn:service.travel.captain_sinbeard",
            "darashia" | "edron" | "liberty_bay" | "port_hope" | "venore" | "yalahar",
        ) => TravelRefusal::SinbeardScarabCheese,
        (
            "oteryn:service.travel.petros",
            "ankrahmun" | "gray_island" | "issavi" | "krailos" | "liberty_bay" | "port_hope"
            | "venore" | "yalahar",
        ) => TravelRefusal::PetrosScarabCheese,
        _ => return Ok(None),
    };
    // The gameplay fence holds the Character root lock through payment and arrival.
    // The inventory owner resolves current custody, including equipment and closed bags.
    let inventory = read_use_inventory(
        tx,
        &fence.item_fence(),
        root.content_revision,
        root.ruleset_revision,
    )
    .await
    .map_err(|error| match error {
        ItemUseError::Unavailable(error) => error,
        _ => DurabilityError::InvalidStoredState,
    })?;
    let has_cheese = inventory
        .equipment
        .iter()
        .chain(
            inventory
                .backpack
                .iter()
                .flat_map(|bag| &bag.entries)
                .map(|entry| &entry.item),
        )
        .chain(inventory.nested.iter().map(|(_, entry)| &entry.item))
        .any(|item| item.definition.production_key == "oteryn:item.tibia.i169");
    Ok(has_cheese.then_some(refusal))
}
