-- One independently expiring replica mechanism of each kind per World.
CREATE TABLE game_forgotten_knowledge_replica_windows (
    world_id uuid NOT NULL,
    replica_kind smallint NOT NULL CHECK (replica_kind IN (1, 2)),
    expires_at timestamptz NOT NULL,
    kills integer NOT NULL DEFAULT 0 CHECK (kills BETWEEN 0 AND 5),
    credited_deaths bytea[] NOT NULL DEFAULT ARRAY[]::bytea[] CHECK (cardinality(credited_deaths) <= 5),
    PRIMARY KEY (world_id, replica_kind)
);
REVOKE ALL ON game_forgotten_knowledge_replica_windows FROM PUBLIC;
GRANT SELECT, INSERT, UPDATE ON game_forgotten_knowledge_replica_windows TO oteryn_game_runtime;
GRANT SELECT ON game_forgotten_knowledge_replica_windows TO oteryn_game_control;

-- Only one selected-phial activation can own the tree per World.
CREATE TABLE game_forgotten_knowledge_tree_windows (
    world_id uuid PRIMARY KEY,
    expires_at timestamptz NOT NULL
);
REVOKE ALL ON game_forgotten_knowledge_tree_windows FROM PUBLIC;
GRANT SELECT, INSERT, UPDATE ON game_forgotten_knowledge_tree_windows TO oteryn_game_runtime;
GRANT SELECT ON game_forgotten_knowledge_tree_windows TO oteryn_game_control;

-- Crystal's Lore Keeper leaves the lever's original room deadline in force after victory.
CREATE OR REPLACE FUNCTION game_boss_room_runtime_guard() RETURNS trigger
LANGUAGE plpgsql SET search_path = pg_catalog, public, pg_temp AS $$ BEGIN
    IF TG_OP = 'INSERT' THEN
        PERFORM 1 FROM game_activity_handoff_groups g JOIN game_activity_runtime_entries e USING(handoff_id)
            WHERE g.handoff_id=NEW.admission AND g.state=2 AND g.world_id=NEW.world_id AND g.instance_id=NEW.instance_id
              AND g.activity_key=NEW.boss_key AND g.content_revision=NEW.content_revision
              AND e.deadline=NEW.deadline AND g.prepared_at=NEW.opened_at;
        IF NOT FOUND OR NEW.state<>1 OR NEW.boss_creature_key IS NULL OR NEW.defeat_id IS NOT NULL THEN
            RAISE EXCEPTION 'boss room requires its complete committed activity' USING ERRCODE='23514';
        END IF;
    ELSIF TG_OP='UPDATE' THEN
        IF (to_jsonb(NEW)-'state'-'terminal_at'-'exit_deadline'-'defeat_id') IS DISTINCT FROM
           (to_jsonb(OLD)-'state'-'terminal_at'-'exit_deadline'-'defeat_id') THEN
            RAISE EXCEPTION 'boss admission identity and policy are immutable' USING ERRCODE='23514';
        END IF;
        IF OLD.state=1 AND NEW.state=2 AND NEW.defeat_id IS NOT NULL
           AND NEW.terminal_at BETWEEN OLD.opened_at AND OLD.deadline
           AND (CASE
             WHEN OLD.boss_creature_key='oteryn:creature.the_last_lore_keeper'
               THEN NEW.exit_deadline=OLD.deadline
             WHEN OLD.after_kill_ms=0 THEN NEW.exit_deadline IS NULL
             ELSE NEW.exit_deadline=NEW.terminal_at+OLD.after_kill_ms
           END) THEN RETURN NEW; END IF;
        IF OLD.state=1 AND NEW.state=3 AND NEW.defeat_id IS NULL
           AND NEW.terminal_at>=OLD.deadline AND NEW.exit_deadline=NEW.terminal_at THEN RETURN NEW; END IF;
        IF OLD.state IN(1,2,3) AND NEW.state=4 AND NOT EXISTS(
            SELECT 1 FROM game_activity_handoff_participants p WHERE p.handoff_id=OLD.admission
              AND NOT EXISTS(SELECT 1 FROM game_activity_runtime_returns r WHERE (r.handoff_id,r.character_id)=(p.handoff_id,p.character_id)))
           AND (OLD.defeat_id IS NOT DISTINCT FROM NEW.defeat_id) THEN RETURN NEW; END IF;
        RAISE EXCEPTION 'boss room transition is not current or complete' USING ERRCODE='23514';
    ELSE
        RAISE EXCEPTION 'boss room history is retained' USING ERRCODE='23514';
    END IF;
    RETURN NEW;
END; $$;
