-- The payment survives publication and is bound to the staged Count's death identity.
CREATE TABLE game_pending_vampire_coffin_payments (
    character_id UUID PRIMARY KEY REFERENCES game_character_roots(character_id),
    death_id UUID UNIQUE
);

REVOKE ALL ON game_pending_vampire_coffin_payments FROM PUBLIC;
GRANT SELECT, INSERT, DELETE ON game_pending_vampire_coffin_payments TO oteryn_game_runtime;
GRANT UPDATE (death_id) ON game_pending_vampire_coffin_payments TO oteryn_game_runtime;

CREATE FUNCTION game_finish_vampire_coffin_payment() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    DELETE FROM game_pending_vampire_coffin_payments WHERE death_id = NEW.death_id;
    RETURN NEW;
END
$$;

-- Settlement and payment cleanup roll back together if the death cannot commit.
CREATE CONSTRAINT TRIGGER game_vampire_coffin_death_committed
    AFTER INSERT ON game_boss_death_settlements
    DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_finish_vampire_coffin_payment();
