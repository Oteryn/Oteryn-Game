-- PREM-2b+PREM-5 (D1106/D1176, ACCEPT_BATCH §3, PREMIUM_ACTIVATION §4.2/§4.4, DEATH_COMPLETE0
-- §4.7): the durable one-time promotion and the Premium blessing purchase, each one fenced
-- CharacterRevision successor paid by a native fee in the same physical Type2 transaction.
--
-- Scope of this migration only:
--   * `game_character_promotion_receipts`: one immutable receipt per promotion purchase and
--     at most one per Character, keyed by its `PromotionPurchase` occurrence (UUIDv7). XP and
--     level are unchanged; `premium_admission` records the Premium admission (1 = Current,
--     2 = the accepted NotActivated pre-delivery exception);
--   * `game_character_promotions`: the permanent promotion fact, one row per Character, bound
--     to its receipt. It is never rewritten, deleted or revoked by a Premium lapse;
--   * `game_character_blessing_purchase_receipts`: one immutable receipt per blessing purchase
--     with the server-selected price band, the optional bank part and the held set before and
--     after; `game_character_blessings` gains its purchase INSERT (DEATH-0 reserved it for this
--     receipt kind);
--   * fee causes 2 (`PromotionPurchase`, coins only, exactly 20,000 gold, never the bank) and
--     3 (`BlessingPurchase`, coins then bank, with its `blessing_key`) on `game_item_fee_burns`;
--     `charm_key` is set exactly for cause 1;
--   * the shared progression guard (0093 body re-issued unchanged) gains both receipt kinds in
--     its revision-one, chain and transition arms;
--   * same-transaction proofs in both directions: receipt <-> fee record <-> promotion or
--     blessing row. A blessing row without its purchase receipt is admitted only for a role that
--     already holds TRUNCATE on the relation (the schema owner's own fixtures), never for the
--     runtime role.
--
-- Rollback: applied migrations are immutable, so rollback is a new migration. Before any
-- receipt exists it re-issues the 0093 guard body, drops the new triggers, function and tables,
-- the fee CHECKs and `blessing_key`, restores `charm_key NOT NULL` and `cause_kind = 1`, and
-- revokes the blessing INSERT. After receipts exist they, their fee records and the promotion and
-- blessing rows are retained evidence of paid value: a rollback keeps them and may only stop new
-- writes by revoking the INSERT grants.

-- Fee causes: 1 = CharmUnassign (0023), 2 = PromotionPurchase, 3 = BlessingPurchase.
ALTER TABLE game_item_fee_burns
    ADD COLUMN blessing_key TEXT NULL
        CONSTRAINT game_item_fee_burns_blessing_key_check
        CHECK (blessing_key ~ '^[A-Za-z0-9][A-Za-z0-9._:-]{0,127}$'),
    ALTER COLUMN charm_key DROP NOT NULL,
    DROP CONSTRAINT game_item_fee_burns_cause_kind_check,
    ADD CONSTRAINT game_item_fee_burns_cause_kind_check CHECK (cause_kind IN (1, 2, 3)),
    ADD CONSTRAINT game_item_fee_burns_cause_source_key
        CHECK ((cause_kind = 1) = (charm_key IS NOT NULL)
               AND (cause_kind = 3) = (blessing_key IS NOT NULL)),
    -- ACCEPT_BATCH §3: the promotion is paid in coins only, never from the bank.
    ADD CONSTRAINT game_item_fee_burns_promotion_coins_only
        CHECK (cause_kind <> 2 OR (fee_gold_units = 20000 AND bank_debit_gold_units = 0));

CREATE TABLE game_character_promotion_receipts (
    purchase_occurrence_id UUID PRIMARY KEY
        CHECK (game_character_is_uuid_v7(purchase_occurrence_id)),
    command_binding BYTEA NOT NULL CHECK (octet_length(command_binding) = 32),
    -- One-time: at most one promotion receipt per Character.
    character_id UUID NOT NULL UNIQUE REFERENCES game_character_roots (character_id),
    original_character_revision NUMERIC(20,0) NOT NULL
        CHECK (original_character_revision BETWEEN 1 AND 18446744073709551614),
    committed_character_revision NUMERIC(20,0) NOT NULL
        CHECK (committed_character_revision = original_character_revision + 1),
    fee_transaction_id UUID NOT NULL UNIQUE REFERENCES game_item_fee_burns (transaction_id),
    fee_event_id UUID NOT NULL UNIQUE CHECK (game_character_is_uuid_v7(fee_event_id)),
    npc_definition BYTEA NOT NULL CHECK (octet_length(npc_definition) BETWEEN 1 AND 4096),
    service_definition BYTEA NOT NULL CHECK (octet_length(service_definition) BETWEEN 1 AND 4096),
    npc_actor_id UUID NOT NULL,
    npc_actor_generation NUMERIC(20,0) NOT NULL
        CHECK (npc_actor_generation BETWEEN 1 AND 18446744073709551615),
    world_id UUID NOT NULL CHECK (game_character_is_uuid_v7(world_id)),
    account_id UUID NOT NULL,
    premium_admission SMALLINT NOT NULL CHECK (premium_admission IN (1, 2)),
    -- PROMOTION_MINIMUM_LEVEL is 20.
    level_before BIGINT NOT NULL CHECK (level_before BETWEEN 20 AND 4294967295),
    level_after BIGINT NOT NULL CHECK (level_after = level_before),
    experience_before BIGINT NOT NULL CHECK (experience_before >= 0),
    experience_after BIGINT NOT NULL CHECK (experience_after = experience_before),
    profile_revision TEXT NOT NULL CHECK (octet_length(profile_revision) BETWEEN 1 AND 128),
    ruleset_revision TEXT NOT NULL CHECK (octet_length(ruleset_revision) BETWEEN 1 AND 128),
    content_revision TEXT NOT NULL CHECK (octet_length(content_revision) BETWEEN 1 AND 128),
    simulation_revision TEXT NOT NULL CHECK (octet_length(simulation_revision) BETWEEN 1 AND 128),
    evidence_revision TEXT NOT NULL CHECK (octet_length(evidence_revision) BETWEEN 1 AND 128),
    declaration_revision TEXT NOT NULL CHECK (octet_length(declaration_revision) BETWEEN 1 AND 128),
    policy_revision TEXT NOT NULL CHECK (octet_length(policy_revision) BETWEEN 1 AND 128),
    reward_revision TEXT NOT NULL CHECK (octet_length(reward_revision) BETWEEN 1 AND 128),
    committed_at BIGINT NOT NULL CHECK (committed_at >= 0),
    -- Stamped by the insert trigger below; never caller supplied.
    created_xact_id xid8 NOT NULL DEFAULT pg_current_xact_id(),
    UNIQUE (character_id, original_character_revision),
    UNIQUE (character_id, committed_character_revision),
    UNIQUE (purchase_occurrence_id, character_id)
);

CREATE TABLE game_character_promotions (
    character_id UUID PRIMARY KEY REFERENCES game_character_roots (character_id),
    promoted BOOLEAN NOT NULL CHECK (promoted),
    purchase_occurrence_id UUID NOT NULL UNIQUE,
    created_xact_id xid8 NOT NULL DEFAULT pg_current_xact_id(),
    FOREIGN KEY (purchase_occurrence_id, character_id)
        REFERENCES game_character_promotion_receipts (purchase_occurrence_id, character_id)
);

CREATE TABLE game_character_blessing_purchase_receipts (
    purchase_occurrence_id UUID PRIMARY KEY
        CHECK (game_character_is_uuid_v7(purchase_occurrence_id)),
    command_binding BYTEA NOT NULL
        CHECK (octet_length(command_binding) = 33 AND get_byte(command_binding, 0) = 1),
    character_id UUID NOT NULL REFERENCES game_character_roots (character_id),
    original_character_revision NUMERIC(20,0) NOT NULL
        CHECK (original_character_revision BETWEEN 1 AND 18446744073709551614),
    committed_character_revision NUMERIC(20,0) NOT NULL
        CHECK (committed_character_revision = original_character_revision + 1),
    fee_transaction_id UUID NOT NULL UNIQUE REFERENCES game_item_fee_burns (transaction_id),
    fee_event_id UUID NOT NULL UNIQUE CHECK (game_character_is_uuid_v7(fee_event_id)),
    npc_definition BYTEA NOT NULL CHECK (octet_length(npc_definition) BETWEEN 1 AND 4096),
    service_definition BYTEA NOT NULL CHECK (octet_length(service_definition) BETWEEN 1 AND 4096),
    npc_actor_id UUID NOT NULL,
    npc_actor_generation NUMERIC(20,0) NOT NULL
        CHECK (npc_actor_generation BETWEEN 1 AND 18446744073709551615),
    world_id UUID NOT NULL CHECK (game_character_is_uuid_v7(world_id)),
    account_id UUID NOT NULL,
    blessing_key TEXT NOT NULL CHECK (blessing_key ~ '^[A-Za-z0-9][A-Za-z0-9._:-]{0,127}$'),
    -- 1 = Wisdom of Solitude, 2 = Blood of the Mountain, 3 = Heart of the Mountain.
    blessing_service SMALLINT NOT NULL CHECK (blessing_service IN (1, 2, 3)),
    price_minimum_level BIGINT NOT NULL CHECK (price_minimum_level BETWEEN 1 AND 4294967295),
    price_maximum_level BIGINT NOT NULL
        CHECK (price_maximum_level BETWEEN price_minimum_level AND 4294967295),
    -- The fee record's bound (0072).
    price_gold_units BIGINT NOT NULL CHECK (price_gold_units BETWEEN 1 AND 1000019999999),
    bank_entry_id UUID NULL UNIQUE REFERENCES game_account_bank_entries (entry_id),
    bank_debit_gold_units BIGINT NOT NULL
        CHECK (bank_debit_gold_units BETWEEN 0 AND price_gold_units),
    bank_balance_before BIGINT NULL CHECK (bank_balance_before BETWEEN 0 AND 999999999999),
    bank_balance_after BIGINT NULL CHECK (bank_balance_after BETWEEN 0 AND 999999999999),
    blessings_before TEXT[] NOT NULL CHECK (game_character_is_blessing_set(blessings_before)),
    blessings_after TEXT[] NOT NULL CHECK (game_character_is_blessing_set(blessings_after)),
    level_before BIGINT NOT NULL CHECK (level_before BETWEEN 1 AND 4294967295),
    level_after BIGINT NOT NULL CHECK (level_after = level_before),
    experience_before BIGINT NOT NULL CHECK (experience_before >= 0),
    experience_after BIGINT NOT NULL CHECK (experience_after = experience_before),
    profile_revision TEXT NOT NULL CHECK (octet_length(profile_revision) BETWEEN 1 AND 128),
    ruleset_revision TEXT NOT NULL CHECK (octet_length(ruleset_revision) BETWEEN 1 AND 128),
    content_revision TEXT NOT NULL CHECK (octet_length(content_revision) BETWEEN 1 AND 128),
    simulation_revision TEXT NOT NULL CHECK (octet_length(simulation_revision) BETWEEN 1 AND 128),
    evidence_revision TEXT NOT NULL CHECK (octet_length(evidence_revision) BETWEEN 1 AND 128),
    declaration_revision TEXT NOT NULL CHECK (octet_length(declaration_revision) BETWEEN 1 AND 128),
    policy_revision TEXT NOT NULL CHECK (octet_length(policy_revision) BETWEEN 1 AND 128),
    reward_revision TEXT NOT NULL CHECK (octet_length(reward_revision) BETWEEN 1 AND 128),
    committed_at BIGINT NOT NULL CHECK (committed_at >= 0),
    created_xact_id xid8 NOT NULL DEFAULT pg_current_xact_id(),
    -- The price band was selected at the locked level.
    CHECK (level_before BETWEEN price_minimum_level AND price_maximum_level),
    -- A bank part has its ledger entry and balances, and nothing else does.
    CHECK ((bank_entry_id IS NULL) = (bank_debit_gold_units = 0)
           AND (bank_entry_id IS NULL) = (bank_balance_before IS NULL)
           AND (bank_entry_id IS NULL) = (bank_balance_after IS NULL)),
    CHECK (bank_balance_after = bank_balance_before - bank_debit_gold_units),
    -- A purchase adds exactly its own, not yet held blessing.
    CHECK (blessings_after @> blessings_before
           AND cardinality(blessings_after) = cardinality(blessings_before) + 1
           AND blessing_key = ANY (blessings_after)
           AND NOT (blessing_key = ANY (blessings_before))),
    UNIQUE (character_id, original_character_revision),
    UNIQUE (character_id, committed_character_revision)
);

-- D495 and the 0073 guard union: re-issues the 0093 body unchanged with the promotion and
-- blessing purchase receipt kinds added to its revision-one, chain and transition arms.
-- Existing triggers bind the function by name.
CREATE OR REPLACE FUNCTION game_character_progression_consistency_guard() RETURNS trigger
LANGUAGE plpgsql AS $$
DECLARE
    v_character UUID;
    v_root_revision NUMERIC(20,0);
    v_state game_character_progression_state%ROWTYPE;
BEGIN
    -- The stance, progress and build row triggers also fire on DELETE (rejected
    -- before they run).
    IF TG_OP = 'DELETE' THEN
        v_character := OLD.character_id;
    ELSE
        v_character := NEW.character_id;
    END IF;
    SELECT character_revision INTO v_root_revision
      FROM game_character_roots WHERE character_id = v_character;
    IF NOT FOUND THEN
        RAISE EXCEPTION 'Character progression lost its root' USING ERRCODE = '23514';
    END IF;
    SELECT * INTO v_state FROM game_character_progression_state
     WHERE character_id = v_character;

    IF v_root_revision = 1 THEN
        IF EXISTS (SELECT 1 FROM game_character_xp_receipts WHERE character_id = v_character)
           OR EXISTS (SELECT 1 FROM game_character_death_receipts WHERE character_id = v_character)
           OR EXISTS (SELECT 1 FROM game_character_stance_receipts WHERE character_id = v_character)
           OR EXISTS (SELECT 1 FROM game_character_stance WHERE character_id = v_character)
           OR EXISTS (SELECT 1 FROM game_character_appearance_receipts WHERE character_id = v_character)
           OR EXISTS (SELECT 1 FROM game_character_appearance WHERE character_id = v_character)
           OR EXISTS (SELECT 1 FROM game_character_bestiary_kill_receipts WHERE character_id = v_character)
           OR EXISTS (SELECT 1 FROM game_character_bestiary_progress WHERE character_id = v_character)
           OR EXISTS (SELECT 1 FROM game_character_charm_receipts WHERE character_id = v_character)
           OR EXISTS (SELECT 1 FROM game_character_monk_state_receipts WHERE character_id = v_character)
           -- CHAR-BUILD-1
           OR EXISTS (SELECT 1 FROM game_character_build_receipts WHERE character_id = v_character)
           OR EXISTS (SELECT 1 FROM game_character_build_state WHERE character_id = v_character)
           -- PROF-1: revision one has no proficiency relation.
           OR EXISTS (SELECT 1 FROM game_character_proficiency_receipts WHERE character_id = v_character)
           OR EXISTS (SELECT 1 FROM game_character_proficiency_receipt_lines WHERE character_id = v_character)
           OR EXISTS (SELECT 1 FROM game_character_proficiency WHERE character_id = v_character)
           OR EXISTS (SELECT 1 FROM game_character_familiar_receipts WHERE character_id = v_character)
           OR EXISTS (SELECT 1 FROM game_character_familiar_state WHERE character_id = v_character)
           -- QUEST-STATE-1: revision one has no quest receipt, track or state (an obligation
           -- is outside the revision chain).
           OR EXISTS (SELECT 1 FROM game_character_quest_receipts WHERE character_id = v_character)
           OR EXISTS (SELECT 1 FROM game_character_quest_tracks WHERE character_id = v_character)
           OR EXISTS (SELECT 1 FROM game_character_quest_states WHERE character_id = v_character)
           -- WHEEL-W1: revision one has no Wheel receipt, state or slot.
           OR EXISTS (SELECT 1 FROM game_character_wheel_receipts WHERE character_id = v_character)
           OR EXISTS (SELECT 1 FROM game_character_wheel_state WHERE character_id = v_character)
           OR EXISTS (SELECT 1 FROM game_character_wheel_slots WHERE character_id = v_character)
           -- PREM-2b+PREM-5 (0137): revision one has no NPC purchase receipt or promotion.
           OR EXISTS (SELECT 1 FROM game_character_promotion_receipts WHERE character_id = v_character)
           OR EXISTS (SELECT 1 FROM game_character_promotions WHERE character_id = v_character)
           OR EXISTS (SELECT 1 FROM game_character_blessing_purchase_receipts WHERE character_id = v_character)
           OR (v_state.character_id IS NOT NULL AND v_state.character_revision <> 1)
           -- H-1: only a monk state receipt sets Harmony or a forced Serene time.
           OR (v_state.character_id IS NOT NULL
               AND (v_state.harmony <> 0 OR v_state.serene_forced_remaining_micros <> 0)) THEN
            RAISE EXCEPTION 'initial Character progression is inconsistent' USING ERRCODE = '23514';
        END IF;
        RETURN NULL;
    END IF;

    IF v_state.character_id IS NULL
       OR v_state.character_revision <> v_root_revision
       OR EXISTS (
            WITH chain AS (
                SELECT x.original_character_revision, x.committed_character_revision,
                       x.level_before, x.level_after, x.experience_before, x.experience_after,
                       x.profile_revision, x.ruleset_revision, x.content_revision,
                       x.simulation_revision, x.evidence_revision, x.declaration_revision,
                       x.policy_revision, x.reward_revision
                  FROM game_character_xp_receipts x WHERE x.character_id = v_character
                UNION ALL
                SELECT d.original_character_revision, d.committed_character_revision,
                       d.level_before, d.level_after, d.experience_before, d.experience_after,
                       d.profile_revision, d.ruleset_revision, d.content_revision,
                       d.simulation_revision, d.evidence_revision, d.declaration_revision,
                       d.policy_revision, d.reward_revision
                  FROM game_character_death_receipts d WHERE d.character_id = v_character
                UNION ALL
                SELECT s.original_character_revision, s.committed_character_revision,
                       s.level_before, s.level_after, s.experience_before, s.experience_after,
                       s.profile_revision, s.ruleset_revision, s.content_revision,
                       s.simulation_revision, s.evidence_revision, s.declaration_revision,
                       s.policy_revision, s.reward_revision
                  FROM game_character_stance_receipts s WHERE s.character_id = v_character
                UNION ALL
                SELECT a.original_character_revision, a.committed_character_revision,
                       a.level_before, a.level_after, a.experience_before, a.experience_after,
                       a.profile_revision, a.ruleset_revision, a.content_revision,
                       a.simulation_revision, a.evidence_revision, a.declaration_revision,
                       a.policy_revision, a.reward_revision
                  FROM game_character_appearance_receipts a WHERE a.character_id = v_character
                UNION ALL
                -- CHARM-2
                SELECT b.original_character_revision, b.committed_character_revision,
                       b.level_before, b.level_after, b.experience_before, b.experience_after,
                       b.profile_revision, b.ruleset_revision, b.content_revision,
                       b.simulation_revision, b.evidence_revision, b.declaration_revision,
                       b.policy_revision, b.reward_revision
                  FROM game_character_bestiary_kill_receipts b WHERE b.character_id = v_character
                UNION ALL
                -- CHARM-3
                SELECT c.original_character_revision, c.committed_character_revision,
                       c.level_before, c.level_after, c.experience_before, c.experience_after,
                       c.profile_revision, c.ruleset_revision, c.content_revision,
                       c.simulation_revision, c.evidence_revision, c.declaration_revision,
                       c.policy_revision, c.reward_revision
                  FROM game_character_charm_receipts c WHERE c.character_id = v_character
                UNION ALL
                -- H-1
                SELECT m.original_character_revision, m.committed_character_revision,
                       m.level_before, m.level_after, m.experience_before, m.experience_after,
                       m.profile_revision, m.ruleset_revision, m.content_revision,
                       m.simulation_revision, m.evidence_revision, m.declaration_revision,
                       m.policy_revision, m.reward_revision
                  FROM game_character_monk_state_receipts m WHERE m.character_id = v_character
                UNION ALL
                -- CHAR-BUILD-1
                SELECT u.original_character_revision, u.committed_character_revision,
                       u.level_before, u.level_after, u.experience_before, u.experience_after,
                       u.profile_revision, u.ruleset_revision, u.content_revision,
                       u.simulation_revision, u.evidence_revision, u.declaration_revision,
                       u.policy_revision, u.reward_revision
                  FROM game_character_build_receipts u WHERE u.character_id = v_character
                UNION ALL
                -- PROF-1: eighth shared CharacterRevision receipt kind.
                SELECT f.original_character_revision, f.committed_character_revision,
                       f.level_before, f.level_after, f.experience_before, f.experience_after,
                       f.profile_revision, f.ruleset_revision, f.content_revision,
                       f.simulation_revision, f.evidence_revision, f.declaration_revision,
                       f.policy_revision, f.reward_revision
                  FROM game_character_proficiency_receipts f WHERE f.character_id = v_character
                UNION ALL
                -- FAMILIAR-1: changed snapshot, XP/level unchanged.
                SELECT f.original_character_revision, f.committed_character_revision,
                       f.level_before, f.level_after, f.experience_before, f.experience_after,
                       f.profile_revision, f.ruleset_revision, f.content_revision,
                       f.simulation_revision, f.evidence_revision, f.declaration_revision,
                       f.policy_revision, f.reward_revision
                  FROM game_character_familiar_receipts f WHERE f.character_id = v_character
                UNION ALL
                -- QUEST-STATE-1: ninth shared CharacterRevision receipt kind.
                SELECT q.original_character_revision, q.committed_character_revision,
                       q.level_before, q.level_after, q.experience_before, q.experience_after,
                       q.profile_revision, q.ruleset_revision, q.content_revision,
                       q.simulation_revision, q.evidence_revision, q.declaration_revision,
                       q.policy_revision, q.reward_revision
                  FROM game_character_quest_receipts q WHERE q.character_id = v_character
                UNION ALL
                -- WHEEL-W1: tenth shared CharacterRevision receipt kind.
                SELECT w.original_character_revision, w.committed_character_revision,
                       w.level_before, w.level_after, w.experience_before, w.experience_after,
                       w.profile_revision, w.ruleset_revision, w.content_revision,
                       w.simulation_revision, w.evidence_revision, w.declaration_revision,
                       w.policy_revision, w.reward_revision
                  FROM game_character_wheel_receipts w WHERE w.character_id = v_character
                UNION ALL
                -- PREM-2b+PREM-5 (0137): one-time promotion purchase, XP/level unchanged.
                SELECT pr.original_character_revision, pr.committed_character_revision,
                       pr.level_before, pr.level_after, pr.experience_before, pr.experience_after,
                       pr.profile_revision, pr.ruleset_revision, pr.content_revision,
                       pr.simulation_revision, pr.evidence_revision, pr.declaration_revision,
                       pr.policy_revision, pr.reward_revision
                  FROM game_character_promotion_receipts pr WHERE pr.character_id = v_character
                UNION ALL
                -- PREM-5 (0137): Premium blessing purchase, XP/level unchanged.
                SELECT bp.original_character_revision, bp.committed_character_revision,
                       bp.level_before, bp.level_after, bp.experience_before, bp.experience_after,
                       bp.profile_revision, bp.ruleset_revision, bp.content_revision,
                       bp.simulation_revision, bp.evidence_revision, bp.declaration_revision,
                       bp.policy_revision, bp.reward_revision
                  FROM game_character_blessing_purchase_receipts bp WHERE bp.character_id = v_character
            )
            SELECT 1 FROM chain
            HAVING count(*)::numeric <> v_root_revision - 1
                OR count(DISTINCT committed_character_revision)::numeric <> v_root_revision - 1
            UNION ALL
            SELECT 1 WHERE NOT EXISTS (
                SELECT 1 FROM chain c
                 WHERE c.committed_character_revision = v_root_revision
                   AND c.level_after = v_state.level
                   AND c.experience_after = v_state.total_experience
                   AND c.profile_revision = v_state.profile_revision
                   AND c.ruleset_revision = v_state.ruleset_revision
                   AND c.content_revision = v_state.content_revision
                   AND c.simulation_revision = v_state.simulation_revision
                   AND c.evidence_revision = v_state.evidence_revision
                   AND c.declaration_revision = v_state.declaration_revision
                   AND c.policy_revision = v_state.policy_revision
                   AND c.reward_revision = v_state.reward_revision)
            UNION ALL
            SELECT 1 FROM chain x
             LEFT JOIN chain p ON p.committed_character_revision = x.original_character_revision
             WHERE x.committed_character_revision > v_root_revision
                OR (x.original_character_revision <> 1 AND p.committed_character_revision IS NULL)
                OR (p.committed_character_revision IS NOT NULL AND
                    (p.experience_after <> x.experience_before
                     OR p.level_after <> x.level_before))) THEN
        RAISE EXCEPTION 'Character progression revision/receipt chain is inconsistent'
            USING ERRCODE = '23514';
    END IF;

    IF EXISTS (
            WITH stance_chain AS (
                SELECT s.stance_occurrence_id AS occurrence_id, s.committed_character_revision,
                       s.stance_before, s.stance_after
                  FROM game_character_stance_receipts s WHERE s.character_id = v_character
                UNION ALL
                -- CHAR-BUILD-1 (A13 §4.2 "Stance fields"): a vocation change that prunes the
                -- stance is a stance transition.
                SELECT u.build_occurrence_id, u.committed_character_revision,
                       u.stance_before, u.stance_after
                  FROM game_character_build_receipts u
                 WHERE u.character_id = v_character
                   AND u.stance_before IS DISTINCT FROM u.stance_after
            ), ordered AS (
                SELECT c.*,
                       row_number() OVER w AS position,
                       lag(c.stance_after) OVER w AS previous_after,
                       count(*) OVER () AS transitions
                  FROM stance_chain c
                WINDOW w AS (ORDER BY c.committed_character_revision)
            )
            SELECT 1 FROM ordered o
             WHERE o.stance_before IS DISTINCT FROM o.previous_after
                OR (o.position = o.transitions AND NOT EXISTS (
                    SELECT 1 FROM game_character_stance r
                     WHERE r.character_id = v_character
                       AND r.stance_key IS NOT DISTINCT FROM o.stance_after
                       AND r.committed_character_revision = o.committed_character_revision
                       AND r.last_stance_occurrence_id = o.occurrence_id))
            UNION ALL
            SELECT 1 FROM game_character_stance r
             WHERE r.character_id = v_character
               AND NOT EXISTS (SELECT 1 FROM stance_chain)) THEN
        RAISE EXCEPTION 'Character stance slot is inconsistent with its stance receipts'
            USING ERRCODE = '23514';
    END IF;

    -- FAMILIAR-1 projection must remain coherent even when another Character writer fires
    -- this shared guard; snapshots do not replace the standard stance or build chains.
    IF EXISTS (
        WITH ordered AS (
            SELECT f.*, lag(f.state_after,1,
                '{"selected_look":0,"granted_looks":[],"saved_expiry_unix":0,"last_logout_unix":0,"lifecycle_epoch":0,"familiar_definition":null,"familiar_revision":null,"profile_revision":"none"}'::jsonb)
                OVER (ORDER BY f.committed_character_revision) AS previous,
                row_number() OVER (ORDER BY f.committed_character_revision DESC) AS latest
            FROM game_character_familiar_receipts f WHERE f.character_id=v_character
        ) SELECT 1 FROM ordered f
          WHERE f.state_before <> f.previous
             OR (f.latest=1 AND NOT EXISTS (SELECT 1 FROM game_character_familiar_state s
                 WHERE s.character_id=v_character AND s.state=f.state_after
                   AND s.committed_character_revision=f.committed_character_revision
                   AND s.last_familiar_occurrence_id=f.familiar_occurrence_id))
        UNION ALL SELECT 1 FROM game_character_familiar_state s
         WHERE s.character_id=v_character AND NOT EXISTS(SELECT 1 FROM ordered)
    ) THEN
        RAISE EXCEPTION 'Character familiar snapshot differs from its receipt chain' USING ERRCODE='23514';
    END IF;

    -- CHARM-2: the per-race kill chain and the progress rows.
    IF EXISTS (
            WITH ordered AS (
                SELECT b.bestiary_occurrence_id, b.race_key, b.committed_character_revision,
                       b.kill_count_before, b.kill_count_after,
                       coalesce(lag(b.kill_count_after) OVER w, 0) AS previous_after,
                       row_number() OVER w AS position,
                       count(*) OVER (PARTITION BY b.race_key) AS kills
                  FROM game_character_bestiary_kill_receipts b
                 WHERE b.character_id = v_character
                WINDOW w AS (PARTITION BY b.race_key ORDER BY b.committed_character_revision)
            )
            SELECT 1 FROM ordered o
             WHERE o.kill_count_before <> o.previous_after
                OR (o.position = o.kills AND NOT EXISTS (
                    SELECT 1 FROM game_character_bestiary_progress r
                     WHERE r.character_id = v_character
                       AND r.race_key = o.race_key
                       AND r.kill_count = o.kill_count_after
                       AND r.committed_character_revision = o.committed_character_revision
                       AND r.last_bestiary_occurrence_id = o.bestiary_occurrence_id))
            UNION ALL
            SELECT 1 FROM game_character_bestiary_progress r
             WHERE r.character_id = v_character
               AND NOT EXISTS (
                    SELECT 1 FROM game_character_bestiary_kill_receipts b
                     WHERE b.character_id = v_character AND b.race_key = r.race_key)) THEN
        RAISE EXCEPTION 'Character Bestiary progress is inconsistent with its kill receipts'
            USING ERRCODE = '23514';
    END IF;

    -- CHAR-BUILD-1 (A13 §4.2 "Build chain" and "Row guard", SKILLS-0 §3.2): build receipts and
    -- death receipts with build fields, ordered by revision. Each `before` equals the previous
    -- `after`, or the seed for the first; the build row equals the latest one (values, revision,
    -- occurrence) and is absent when there is none. So a row-only write, a receipt without its
    -- row update, and any other kind that changes the row all fail.
    IF EXISTS (
            WITH build_chain AS (
                SELECT u.build_occurrence_id AS occurrence_id, u.committed_character_revision,
                       u.vocation_before, u.magic_level_before, u.mana_spent_before,
                       u.fist_level_before, u.fist_tries_before, u.club_level_before,
                       u.club_tries_before, u.sword_level_before, u.sword_tries_before,
                       u.axe_level_before, u.axe_tries_before, u.distance_level_before,
                       u.distance_tries_before, u.shielding_level_before,
                       u.shielding_tries_before, u.fishing_level_before, u.fishing_tries_before,
                       u.vocation_after, u.magic_level_after, u.mana_spent_after,
                       u.fist_level_after, u.fist_tries_after, u.club_level_after,
                       u.club_tries_after, u.sword_level_after, u.sword_tries_after,
                       u.axe_level_after, u.axe_tries_after, u.distance_level_after,
                       u.distance_tries_after, u.shielding_level_after, u.shielding_tries_after,
                       u.fishing_level_after, u.fishing_tries_after
                  FROM game_character_build_receipts u WHERE u.character_id = v_character
                UNION ALL
                SELECT d.death_occurrence_id, d.committed_character_revision,
                       d.vocation, d.magic_level_before, d.mana_spent_before,
                       d.fist_level_before, d.fist_tries_before, d.club_level_before,
                       d.club_tries_before, d.sword_level_before, d.sword_tries_before,
                       d.axe_level_before, d.axe_tries_before, d.distance_level_before,
                       d.distance_tries_before, d.shielding_level_before,
                       d.shielding_tries_before, d.fishing_level_before, d.fishing_tries_before,
                       d.vocation, d.magic_level_after, d.mana_spent_after,
                       d.fist_level_after, d.fist_tries_after, d.club_level_after,
                       d.club_tries_after, d.sword_level_after, d.sword_tries_after,
                       d.axe_level_after, d.axe_tries_after, d.distance_level_after,
                       d.distance_tries_after, d.shielding_level_after, d.shielding_tries_after,
                       d.fishing_level_after, d.fishing_tries_after
                  FROM game_character_death_receipts d
                 WHERE d.character_id = v_character AND d.vocation IS NOT NULL
            ), ordered AS (
                SELECT c.*,
                       row_number() OVER (ORDER BY c.committed_character_revision) AS position,
                       count(*) OVER () AS transitions
                  FROM build_chain c
            )
            SELECT 1 FROM ordered o
              LEFT JOIN ordered p ON p.position = o.position - 1
             WHERE (p.position IS NULL
                    AND (o.vocation_before, o.magic_level_before, o.mana_spent_before,
                         o.fist_level_before, o.fist_tries_before, o.club_level_before,
                         o.club_tries_before, o.sword_level_before, o.sword_tries_before,
                         o.axe_level_before, o.axe_tries_before, o.distance_level_before,
                         o.distance_tries_before, o.shielding_level_before,
                         o.shielding_tries_before, o.fishing_level_before,
                         o.fishing_tries_before)
                        IS DISTINCT FROM ('none', 0, 0, 10, 0, 10, 0, 10, 0, 10, 0, 10, 0, 10, 0,
                                          10, 0))
                OR (p.position IS NOT NULL
                    AND (o.vocation_before, o.magic_level_before, o.mana_spent_before,
                         o.fist_level_before, o.fist_tries_before, o.club_level_before,
                         o.club_tries_before, o.sword_level_before, o.sword_tries_before,
                         o.axe_level_before, o.axe_tries_before, o.distance_level_before,
                         o.distance_tries_before, o.shielding_level_before,
                         o.shielding_tries_before, o.fishing_level_before,
                         o.fishing_tries_before)
                        IS DISTINCT FROM (p.vocation_after, p.magic_level_after,
                         p.mana_spent_after, p.fist_level_after, p.fist_tries_after,
                         p.club_level_after, p.club_tries_after, p.sword_level_after,
                         p.sword_tries_after, p.axe_level_after, p.axe_tries_after,
                         p.distance_level_after, p.distance_tries_after,
                         p.shielding_level_after, p.shielding_tries_after,
                         p.fishing_level_after, p.fishing_tries_after))
                OR (o.position = o.transitions AND NOT EXISTS (
                    SELECT 1 FROM game_character_build_state r
                     WHERE r.character_id = v_character
                       AND (r.vocation, r.magic_level, r.mana_spent, r.fist_level, r.fist_tries,
                            r.club_level, r.club_tries, r.sword_level, r.sword_tries,
                            r.axe_level, r.axe_tries, r.distance_level, r.distance_tries,
                            r.shielding_level, r.shielding_tries, r.fishing_level,
                            r.fishing_tries)
                         = (o.vocation_after, o.magic_level_after, o.mana_spent_after,
                            o.fist_level_after, o.fist_tries_after, o.club_level_after,
                            o.club_tries_after, o.sword_level_after, o.sword_tries_after,
                            o.axe_level_after, o.axe_tries_after, o.distance_level_after,
                            o.distance_tries_after, o.shielding_level_after,
                            o.shielding_tries_after, o.fishing_level_after,
                            o.fishing_tries_after)
                       AND r.committed_character_revision = o.committed_character_revision
                       AND r.last_build_occurrence_id = o.occurrence_id))
            UNION ALL
            SELECT 1 FROM game_character_build_state r
             WHERE r.character_id = v_character
               AND NOT EXISTS (SELECT 1 FROM build_chain)) THEN
        RAISE EXCEPTION 'Character build state is inconsistent with its build receipts'
            USING ERRCODE = '23514';
    END IF;

    -- WHEEL-W1 (WHEEL-0 §4 "Guards"): the Wheel full-chain, ruleset-chain, capacity and
    -- row-tip arm.
    PERFORM game_character_wheel_consistency(v_character);

    IF TG_TABLE_NAME = 'game_character_progression_state' AND TG_OP = 'UPDATE' THEN
        IF NOT EXISTS (
                SELECT 1 FROM game_character_xp_receipts x
                 WHERE x.character_id = v_character
                   AND x.original_character_revision = OLD.character_revision
                   AND x.committed_character_revision = NEW.character_revision
                   AND x.level_before = OLD.level AND x.level_after = NEW.level
                   AND x.experience_before = OLD.total_experience
                   AND x.experience_after = NEW.total_experience
                UNION ALL
                SELECT 1 FROM game_character_death_receipts d
                 WHERE d.character_id = v_character
                   AND d.original_character_revision = OLD.character_revision
                   AND d.committed_character_revision = NEW.character_revision
                   AND d.level_before = OLD.level AND d.level_after = NEW.level
                   AND d.experience_before = OLD.total_experience
                   AND d.experience_after = NEW.total_experience
                UNION ALL
                SELECT 1 FROM game_character_stance_receipts s
                 WHERE s.character_id = v_character
                   AND s.original_character_revision = OLD.character_revision
                   AND s.committed_character_revision = NEW.character_revision
                   AND s.level_before = OLD.level AND s.level_after = NEW.level
                   AND s.experience_before = OLD.total_experience
                   AND s.experience_after = NEW.total_experience
                UNION ALL
                SELECT 1 FROM game_character_appearance_receipts a
                 WHERE a.character_id = v_character
                   AND a.original_character_revision = OLD.character_revision
                   AND a.committed_character_revision = NEW.character_revision
                   AND a.level_before = OLD.level AND a.level_after = NEW.level
                   AND a.experience_before = OLD.total_experience
                   AND a.experience_after = NEW.total_experience
                UNION ALL
                -- CHARM-2
                SELECT 1 FROM game_character_bestiary_kill_receipts b
                 WHERE b.character_id = v_character
                   AND b.original_character_revision = OLD.character_revision
                   AND b.committed_character_revision = NEW.character_revision
                   AND b.level_before = OLD.level AND b.level_after = NEW.level
                   AND b.experience_before = OLD.total_experience
                   AND b.experience_after = NEW.total_experience
                UNION ALL
                -- CHARM-3
                SELECT 1 FROM game_character_charm_receipts c
                 WHERE c.character_id = v_character
                   AND c.original_character_revision = OLD.character_revision
                   AND c.committed_character_revision = NEW.character_revision
                   AND c.level_before = OLD.level AND c.level_after = NEW.level
                   AND c.experience_before = OLD.total_experience
                   AND c.experience_after = NEW.total_experience
                UNION ALL
                -- H-1
                SELECT 1 FROM game_character_monk_state_receipts m
                 WHERE m.character_id = v_character
                   AND m.original_character_revision = OLD.character_revision
                   AND m.committed_character_revision = NEW.character_revision
                   AND m.level_before = OLD.level AND m.level_after = NEW.level
                   AND m.experience_before = OLD.total_experience
                   AND m.experience_after = NEW.total_experience
                   AND m.harmony_before = OLD.harmony AND m.harmony_after = NEW.harmony
                   AND m.serene_forced_remaining_micros_before = OLD.serene_forced_remaining_micros
                   AND m.serene_forced_remaining_micros_after = NEW.serene_forced_remaining_micros
                UNION ALL
                -- PROF-1: a proficiency successor explains unchanged Character XP/level.
                SELECT 1 FROM game_character_proficiency_receipts f
                 WHERE f.character_id = v_character
                   AND f.original_character_revision = OLD.character_revision
                   AND f.committed_character_revision = NEW.character_revision
                   AND f.level_before = OLD.level AND f.level_after = NEW.level
                   AND f.experience_before = OLD.total_experience
                   AND f.experience_after = NEW.total_experience
                UNION ALL
                -- CHAR-BUILD-1 (#1271 F3)
                SELECT 1 FROM game_character_build_receipts u
                 WHERE u.character_id = v_character
                   AND u.original_character_revision = OLD.character_revision
                   AND u.committed_character_revision = NEW.character_revision
                   AND u.level_before = OLD.level AND u.level_after = NEW.level
                   AND u.experience_before = OLD.total_experience
                   AND u.experience_after = NEW.total_experience
                UNION ALL
                SELECT 1 FROM game_character_familiar_receipts f
                 WHERE f.character_id = v_character
                   AND f.original_character_revision = OLD.character_revision
                   AND f.committed_character_revision = NEW.character_revision
                   AND f.level_before = OLD.level AND f.level_after = NEW.level
                   AND f.experience_before = OLD.total_experience
                   AND f.experience_after = NEW.total_experience
                UNION ALL
                -- QUEST-STATE-1: a quest successor explains unchanged Character XP/level.
                SELECT 1 FROM game_character_quest_receipts q
                 WHERE q.character_id = v_character
                   AND q.original_character_revision = OLD.character_revision
                   AND q.committed_character_revision = NEW.character_revision
                   AND q.level_before = OLD.level AND q.level_after = NEW.level
                   AND q.experience_before = OLD.total_experience
                   AND q.experience_after = NEW.total_experience
                UNION ALL
                -- WHEEL-W1: a Wheel successor explains unchanged Character XP/level.
                SELECT 1 FROM game_character_wheel_receipts w
                 WHERE w.character_id = v_character
                   AND w.original_character_revision = OLD.character_revision
                   AND w.committed_character_revision = NEW.character_revision
                   AND w.level_before = OLD.level AND w.level_after = NEW.level
                   AND w.experience_before = OLD.total_experience
                   AND w.experience_after = NEW.total_experience
                UNION ALL
                -- PREM-2b+PREM-5 (0137): a promotion successor explains unchanged XP/level.
                SELECT 1 FROM game_character_promotion_receipts pr
                 WHERE pr.character_id = v_character
                   AND pr.original_character_revision = OLD.character_revision
                   AND pr.committed_character_revision = NEW.character_revision
                   AND pr.level_before = OLD.level AND pr.level_after = NEW.level
                   AND pr.experience_before = OLD.total_experience
                   AND pr.experience_after = NEW.total_experience
                UNION ALL
                -- PREM-5 (0137): a blessing purchase successor explains unchanged XP/level.
                SELECT 1 FROM game_character_blessing_purchase_receipts bp
                 WHERE bp.character_id = v_character
                   AND bp.original_character_revision = OLD.character_revision
                   AND bp.committed_character_revision = NEW.character_revision
                   AND bp.level_before = OLD.level AND bp.level_after = NEW.level
                   AND bp.experience_before = OLD.total_experience
                   AND bp.experience_after = NEW.total_experience)
                THEN
            RAISE EXCEPTION 'Character progression transition has no matching receipt'
                USING ERRCODE = '23514';
        END IF;
        -- H-1: a death transition empties Harmony and the forced Serene time; any other
        -- transition but a monk state receipt keeps both unchanged.
        IF EXISTS (
                SELECT 1 FROM game_character_death_receipts d
                 WHERE d.character_id = v_character
                   AND d.original_character_revision = OLD.character_revision
                   AND d.committed_character_revision = NEW.character_revision) THEN
            IF NEW.harmony <> 0 OR NEW.serene_forced_remaining_micros <> 0 THEN
                RAISE EXCEPTION 'a Character death must empty Harmony and the forced Serene time'
                    USING ERRCODE = '23514';
            END IF;
        ELSIF (NEW.harmony <> OLD.harmony
               OR NEW.serene_forced_remaining_micros <> OLD.serene_forced_remaining_micros)
              AND NOT EXISTS (
                SELECT 1 FROM game_character_monk_state_receipts m
                 WHERE m.character_id = v_character
                   AND m.original_character_revision = OLD.character_revision
                   AND m.committed_character_revision = NEW.character_revision) THEN
            RAISE EXCEPTION 'Harmony and the forced Serene time change only with a monk state receipt'
                USING ERRCODE = '23514';
        END IF;
    END IF;
    RETURN NULL;
END;
$$;

-- Commit-time same-transaction proofs of one NPC purchase. SECURITY INVOKER: the runtime role
-- reads every relation here already.
CREATE FUNCTION game_character_npc_purchase_consistency_guard() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF TG_TABLE_NAME = 'game_character_promotion_receipts' THEN
        IF NOT EXISTS (
               SELECT 1 FROM game_character_roots r
                WHERE r.character_id = NEW.character_id
                  AND r.world_id = NEW.world_id
                  AND r.account_id = NEW.account_id)
           OR NOT EXISTS (
               SELECT 1 FROM game_item_fee_burns f
                WHERE f.transaction_id = NEW.fee_transaction_id
                  AND f.event_id = NEW.fee_event_id
                  AND f.cause_kind = 2
                  AND f.cause_occurrence_id = NEW.purchase_occurrence_id
                  AND f.character_id = NEW.character_id
                  AND f.world_id = NEW.world_id
                  AND f.committed_character_revision = NEW.committed_character_revision
                  AND f.created_xact_id = pg_current_xact_id())
           OR NOT EXISTS (
               SELECT 1 FROM game_character_promotions p
                WHERE p.character_id = NEW.character_id
                  AND p.purchase_occurrence_id = NEW.purchase_occurrence_id
                  AND p.promoted
                  AND p.created_xact_id = pg_current_xact_id()) THEN
            RAISE EXCEPTION 'a promotion receipt must commit with its coins-only fee and promotion'
                USING ERRCODE = '23514';
        END IF;
    ELSIF TG_TABLE_NAME = 'game_character_blessing_purchase_receipts' THEN
        IF NOT EXISTS (
               SELECT 1 FROM game_character_roots r
                WHERE r.character_id = NEW.character_id
                  AND r.world_id = NEW.world_id
                  AND r.account_id = NEW.account_id)
           OR NOT EXISTS (
               SELECT 1 FROM game_item_fee_burns f
                WHERE f.transaction_id = NEW.fee_transaction_id
                  AND f.event_id = NEW.fee_event_id
                  AND f.cause_kind = 3
                  AND f.cause_occurrence_id = NEW.purchase_occurrence_id
                  AND f.blessing_key = NEW.blessing_key
                  AND f.character_id = NEW.character_id
                  AND f.world_id = NEW.world_id
                  AND f.committed_character_revision = NEW.committed_character_revision
                  AND f.fee_gold_units = NEW.price_gold_units
                  AND f.bank_debit_gold_units = NEW.bank_debit_gold_units
                  AND f.created_xact_id = pg_current_xact_id())
           OR (NEW.bank_entry_id IS NOT NULL AND NOT EXISTS (
               SELECT 1 FROM game_account_bank_entries e
                WHERE e.entry_id = NEW.bank_entry_id
                  AND e.fee_transaction_id = NEW.fee_transaction_id
                  AND e.kind = 5
                  AND e.account_id = NEW.account_id
                  AND e.world_id = NEW.world_id
                  AND e.amount = NEW.bank_debit_gold_units
                  AND e.balance_before = NEW.bank_balance_before
                  AND e.balance_after = NEW.bank_balance_after))
           OR NOT EXISTS (
               SELECT 1 FROM game_character_blessings b
                WHERE b.character_id = NEW.character_id
                  AND b.blessing_key = NEW.blessing_key
                  AND b.provenance = 'purchase:' || encode(uuid_send(NEW.purchase_occurrence_id), 'hex')
                  AND b.xmin = pg_current_xact_id()::xid)
           OR NEW.blessings_after IS DISTINCT FROM (
               SELECT coalesce(array_agg(b.blessing_key COLLATE "C"
                                         ORDER BY b.blessing_key COLLATE "C"), ARRAY[]::TEXT[])
                 FROM game_character_blessings b
                WHERE b.character_id = NEW.character_id) THEN
            RAISE EXCEPTION 'a blessing purchase receipt must commit with its fee, bank part and held blessing'
                USING ERRCODE = '23514';
        END IF;
    ELSIF TG_TABLE_NAME = 'game_item_fee_burns' THEN
        IF (NEW.cause_kind = 2 AND NOT EXISTS (
               SELECT 1 FROM game_character_promotion_receipts r
                WHERE r.fee_transaction_id = NEW.transaction_id
                  AND r.purchase_occurrence_id = NEW.cause_occurrence_id
                  AND r.created_xact_id = pg_current_xact_id()))
           OR (NEW.cause_kind = 3 AND NOT EXISTS (
               SELECT 1 FROM game_character_blessing_purchase_receipts r
                WHERE r.fee_transaction_id = NEW.transaction_id
                  AND r.purchase_occurrence_id = NEW.cause_occurrence_id
                  AND r.created_xact_id = pg_current_xact_id())) THEN
            RAISE EXCEPTION 'a promotion or blessing fee must commit with its purchase receipt'
                USING ERRCODE = '23514';
        END IF;
    ELSIF TG_TABLE_NAME = 'game_character_blessings' THEN
        -- DEATH-0 reserved blessing INSERT for this receipt kind. A role that may already
        -- TRUNCATE the relation (its owner) is not constrained further by this proof.
        IF NOT EXISTS (
               SELECT 1 FROM game_character_blessing_purchase_receipts r
                WHERE r.character_id = NEW.character_id
                  AND r.blessing_key = NEW.blessing_key
                  AND NEW.provenance = 'purchase:' || encode(uuid_send(r.purchase_occurrence_id), 'hex')
                  AND r.created_xact_id = pg_current_xact_id())
           AND NOT has_table_privilege(current_user, 'game_character_blessings', 'TRUNCATE') THEN
            RAISE EXCEPTION 'a held blessing is inserted only with its purchase receipt'
                USING ERRCODE = '23514';
        END IF;
    END IF;
    RETURN NULL;
END;
$$;

DO $$
DECLARE
    v_function TEXT;
BEGIN
    FOREACH v_function IN ARRAY ARRAY[
        'game_character_progression_consistency_guard()',
        'game_character_npc_purchase_consistency_guard()'
    ] LOOP
        -- CREATE OR REPLACE clears function-level settings; restore the search_path pin.
        EXECUTE format('ALTER FUNCTION %s SET search_path = %I, pg_temp', v_function, current_schema());
        EXECUTE format('REVOKE ALL ON FUNCTION %s FROM PUBLIC', v_function);
    END LOOP;
END $$;

CREATE TRIGGER game_character_promotion_receipts_stamp_xact BEFORE INSERT
    ON game_character_promotion_receipts FOR EACH ROW
    EXECUTE FUNCTION game_item_stamp_created_xact_id();
CREATE TRIGGER game_character_promotions_stamp_xact BEFORE INSERT
    ON game_character_promotions FOR EACH ROW
    EXECUTE FUNCTION game_item_stamp_created_xact_id();
CREATE TRIGGER game_character_blessing_purchase_receipts_stamp_xact BEFORE INSERT
    ON game_character_blessing_purchase_receipts FOR EACH ROW
    EXECUTE FUNCTION game_item_stamp_created_xact_id();

CREATE TRIGGER game_character_promotion_receipt_immutable BEFORE UPDATE OR DELETE
    ON game_character_promotion_receipts FOR EACH ROW EXECUTE FUNCTION game_character_immutable();
CREATE TRIGGER game_character_promotion_immutable BEFORE UPDATE OR DELETE
    ON game_character_promotions FOR EACH ROW EXECUTE FUNCTION game_character_immutable();
CREATE TRIGGER game_character_blessing_purchase_receipt_immutable BEFORE UPDATE OR DELETE
    ON game_character_blessing_purchase_receipts FOR EACH ROW
    EXECUTE FUNCTION game_character_immutable();
CREATE TRIGGER game_character_promotion_receipts_no_truncate BEFORE TRUNCATE
    ON game_character_promotion_receipts EXECUTE FUNCTION game_character_reject_truncate();
CREATE TRIGGER game_character_promotions_no_truncate BEFORE TRUNCATE
    ON game_character_promotions EXECUTE FUNCTION game_character_reject_truncate();
CREATE TRIGGER game_character_blessing_purchase_receipts_no_truncate BEFORE TRUNCATE
    ON game_character_blessing_purchase_receipts EXECUTE FUNCTION game_character_reject_truncate();

CREATE CONSTRAINT TRIGGER game_character_promotion_receipt_progression_consistent
    AFTER INSERT ON game_character_promotion_receipts DEFERRABLE INITIALLY DEFERRED
    FOR EACH ROW EXECUTE FUNCTION game_character_progression_consistency_guard();
CREATE CONSTRAINT TRIGGER game_character_promotion_progression_consistent
    AFTER INSERT ON game_character_promotions DEFERRABLE INITIALLY DEFERRED
    FOR EACH ROW EXECUTE FUNCTION game_character_progression_consistency_guard();
CREATE CONSTRAINT TRIGGER game_character_blessing_purchase_receipt_progression_consistent
    AFTER INSERT ON game_character_blessing_purchase_receipts DEFERRABLE INITIALLY DEFERRED
    FOR EACH ROW EXECUTE FUNCTION game_character_progression_consistency_guard();

CREATE CONSTRAINT TRIGGER game_character_promotion_receipt_purchase_consistent
    AFTER INSERT ON game_character_promotion_receipts DEFERRABLE INITIALLY DEFERRED
    FOR EACH ROW EXECUTE FUNCTION game_character_npc_purchase_consistency_guard();
CREATE CONSTRAINT TRIGGER game_character_blessing_purchase_receipt_purchase_consistent
    AFTER INSERT ON game_character_blessing_purchase_receipts DEFERRABLE INITIALLY DEFERRED
    FOR EACH ROW EXECUTE FUNCTION game_character_npc_purchase_consistency_guard();
CREATE CONSTRAINT TRIGGER game_item_fee_burn_purchase_source_proven
    AFTER INSERT ON game_item_fee_burns DEFERRABLE INITIALLY DEFERRED
    FOR EACH ROW EXECUTE FUNCTION game_character_npc_purchase_consistency_guard();
CREATE CONSTRAINT TRIGGER game_character_blessing_purchase_proven
    AFTER INSERT ON game_character_blessings DEFERRABLE INITIALLY DEFERRED
    FOR EACH ROW EXECUTE FUNCTION game_character_npc_purchase_consistency_guard();

REVOKE ALL ON game_character_promotion_receipts, game_character_promotions,
    game_character_blessing_purchase_receipts FROM PUBLIC;
GRANT SELECT, INSERT ON game_character_promotion_receipts, game_character_promotions,
    game_character_blessing_purchase_receipts TO oteryn_game_runtime;
-- DEATH-0 (0016) reserved this grant for the blessing purchase receipt kind.
GRANT INSERT ON game_character_blessings TO oteryn_game_runtime;
GRANT SELECT ON game_character_promotion_receipts, game_character_promotions,
    game_character_blessing_purchase_receipts TO oteryn_game_control;
