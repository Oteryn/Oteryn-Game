-- QUICKLOOT-1: Character-owned manual quick-loot routing.
-- Container ids are references to the existing item-instance truth; this table owns only the
-- player's routing preference.  A NULL target is the main container.
CREATE TABLE game_character_quick_loot_settings (
    character_id UUID PRIMARY KEY REFERENCES game_character_roots(character_id),
    unassigned_container_id UUID,
    unassigned_gold_pouch BOOLEAN NOT NULL DEFAULT FALSE,
    fallback_to_main BOOLEAN NOT NULL DEFAULT TRUE,
    committed_character_revision NUMERIC(20,0) NOT NULL CHECK(committed_character_revision>=1),
    CHECK(NOT unassigned_gold_pouch OR unassigned_container_id IS NOT NULL)
);

CREATE TABLE game_character_quick_loot_categories (
    character_id UUID NOT NULL REFERENCES game_character_quick_loot_settings(character_id)
        ON DELETE CASCADE,
    category SMALLINT NOT NULL CHECK(category BETWEEN 0 AND 4),
    container_id UUID,
    gold_pouch BOOLEAN NOT NULL DEFAULT FALSE,
    PRIMARY KEY(character_id,category),
    CHECK(NOT gold_pouch OR container_id IS NOT NULL)
);

REVOKE ALL ON game_character_quick_loot_settings,game_character_quick_loot_categories FROM PUBLIC;
GRANT SELECT,INSERT,UPDATE ON game_character_quick_loot_settings,game_character_quick_loot_categories
    TO oteryn_game_runtime;
