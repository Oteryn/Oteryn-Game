"""Source regressions for the reviewed row command-owner boundaries.

These checks deliberately do not claim to execute a native player journey.
"""
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[3]
PHASES = ROOT / "apps/game-server/src/qualification/client_phases"
USE_COUNTS = {
    "client_row_i09": 2, "client_row_i11": 3, "client_row_i12": 1,
    "client_row_m06": 1, "client_row_m07": 1, "client_row_m08": 1,
    "client_row_m09": 1, "client_row_w19a": 1,
    "full_row_g08": 1, "full_row_g09": 1, "full_row_g36": 1,
    "full_row_i10": 1, "full_row_i11": 2, "full_row_i14": 1,
    "full_row_m06": 1, "full_row_m08": 2, "full_row_m09": 1,
    "full_row_w07": 1, "full_row_w09": 1, "full_row_w19a": 1,
    "full_row_w19b": 1, "full_row_w19d": 1, "full_row_w19e": 1,
}


class RowCommandRoutes(unittest.TestCase):
    def test_all_28_reviewed_use_calls_use_the_core_session_owner(self):
        self.assertEqual(sum(USE_COUNTS.values()), 28)
        shared_calls = {"full_row_i10": 2, "full_row_i11": 4}
        for name, count in USE_COUNTS.items():
            with self.subTest(row=name):
                source = (PHASES / f"{name}.rs").read_text()
                if name in shared_calls:
                    helper_calls = source.count("use_ingredient(")
                    helper_definitions = source.count("fn use_ingredient(")
                    self.assertEqual(helper_calls - helper_definitions, shared_calls[name])
                    self.assertEqual(source.count(".world_context_use("), helper_definitions)
                else:
                    self.assertEqual(source.count(".world_context_use("), count)
                self.assertNotRegex(source, r"gated_command\(\s*(?:\w+::)*CAPABILITY_ITEM_USE_V1")

    def test_both_bed_calls_use_capability_11(self):
        for name in ("client_row_s07", "full_row_s07"):
            source = (PHASES / f"{name}.rs").read_text()
            self.assertRegex(source, r"gated_command\(\s*(?:\w+::)*CAPABILITY_WORLD_INTERACTION_V1,\s*COMMAND_TYPE_BED_SLEEP_INTENT")
            self.assertNotIn("CAPABILITY_ITEM_USE_V1", source)

    def test_core_achievements_are_not_sent_as_capability_zero(self):
        for name in ("client_row_p11", "full_row_p11"):
            with self.subTest(row=name):
                source = (PHASES / f"{name}.rs").read_text()
                self.assertNotRegex(source, r"gated_command\(\s*0,")
                self.assertIn(".account_achievements(", source)
                self.assertIn("decode_account_achievements_query(&[0x08])", source)
                self.assertIn("next_command_id()", source)
                self.assertIn("last_server_sequence()", source)

    def test_countdown_negative_probe_is_local_and_cannot_abort_observation(self):
        for name in ("client_row_w05a", "full_row_w05a"):
            with self.subTest(row=name):
                source = (PHASES / f"{name}.rs").read_text()
                self.assertNotRegex(source, r"gated_command\([^)]*CAPABILITY_SERVER_SAVE_COUNTDOWN_V1[^)]*\)\s*\.await\?")
                self.assertIn("SessionError::CapabilityNotSelected", source)
                self.assertIn("next_command_id()", source)
                self.assertIn("last_server_sequence()", source)
                self.assertIn("Duration::from_secs(7)", source)
                if name == "full_row_w05a":
                    self.assertIn("countdown_poll(client.take_events(), close)", source)

    def test_p11_capture_does_not_present_local_refusal_as_a_wire_status(self):
        capture = (PHASES / "client_row_p11.rs").read_text()
        replay = (ROOT / "crates/session/src/tests/e2e_p11.rs").read_text()
        self.assertNotIn("refused_status=Rejected", capture)
        self.assertIn("malformed_query=local-codec-refusal", capture)
        self.assertIn("malformed_wire_submission=false", capture)
        self.assertIn('value(e, "malformed_query")', replay)
        self.assertIn('value(e, "malformed_wire_submission")', replay)

class ClientShardProvenance(unittest.TestCase):
    def test_client_shards_own_their_prerequisite_journeys(self):
        for name, owner in (
            ("c14", "full_e2ef18::client_progression"),
            ("c19a", "full_e2ef6::client_starter"),
            ("c20b", "full_e2ef6::client_item_loss"),
            ("c20c", "full_e2ef6::client_blessing_death"),
        ):
            with self.subTest(row=name):
                source = (PHASES / f"client_row_{name}.rs").read_text()
                dispatch = (PHASES / "world.rs").read_text()
                if name == "c14":
                    self.assertIn(owner, source)
                else:
                    self.assertRegex(dispatch, rf"client_row_{name}::run_packet\([^;]+{owner}")
                    self.assertIn("prerequisite", source)
                self.assertNotIn("completed_prerequisite", source)

    def test_twist_capture_uses_the_death_just_proved(self):
        full = (PHASES / "full_e2ef6.rs").read_text()
        death = full[full.index("async fn blessing_death("):full.index("async fn blessing_services(")]
        self.assertIn("*twist_death = Some((before, after))", death)
        self.assertIn("capture_twist(clients, twist_death).await", full)
        self.assertIn("client_row_c20c::capture_receipt(&mut capture, expected_occurrence)", full)

    def test_exiva_cleanup_has_its_own_release_budget(self):
        source = (PHASES / "client_row_c05.rs").read_text()
        self.assertNotIn("indexed_timeout(Duration::from_secs(100)", source)
        self.assertIn('"exiva-restoration"', source)
        self.assertIn("restore_after(changed", source)

    def test_wrap_reconciliation_precedes_house_exit(self):
        source = (PHASES / "client_row_i10.rs").read_text()
        recovery = source.split("async fn recover_on_fresh_session(")[1].split("async fn journey(")[0]
        self.assertLess(recovery.index("wrap_result("), recovery.index("transfer(&mut client, house, true)"))
        self.assertIn("recover_on_fresh_session(clients", source.split("let proofs =")[1])

    def test_shield_denominator_comes_from_control_hit(self):
        source = (PHASES / "client_row_i15.rs").read_text()
        self.assertIn("observe_drowning(&mut client).await", source)
        self.assertIn("paid, incoming_damage", source)
        self.assertNotIn("const DROWNING_TICK", source)

    def test_wrap_prepares_fixture_before_requiring_the_oven(self):
        source = (PHASES / "client_row_i10.rs").read_text()
        self.assertIn("prepare_fixture(&mut db, clients, world", source)
        self.assertIn("assign_house_scope", source)
        self.assertIn("owned_house_fixture.sql", source)


class QualificationReReview(unittest.TestCase):
    def source(self, name):
        return (PHASES / f"{name}.rs").read_text()

    def test_progression_capacity_is_a_full_row_requirement(self):
        source = self.source("full_e2ef18")
        _, journey = source.split("async fn journey(")
        self.assertIn("carry_capacity(&mut client, baseline.stats.level).await?", journey)
        self.assertIn("carry_capacity_grew(Some(capacity), Some(grown))", journey)
        self.assertIn("persisted_capacity != capacity", journey)

    def test_client_progression_does_not_require_full_capacity_evidence(self):
        source = (PHASES / "full_e2ef18.rs").read_text()
        client = source.split("async fn client_progression", 1)[1].split("async fn stats", 1)[0]
        self.assertIn('action != "level-and-maxima"', client)
        self.assertIn("row.passed.contains(action)", client)

    def test_amulet_moves_use_the_equipment_protocol(self):
        item_loss = self.source("full_e2ef6").split("async fn item_loss(")[1].split("const BLESSING_DEATH")[0]
        self.assertNotIn(".move_item(", item_loss)
        self.assertEqual(item_loss.count("inventory::send("), 2)

    def test_starter_budget_starts_after_the_prerequisite(self):
        capture = self.source("client_row_c19a")
        self.assertNotIn("async fn journey(clients: &SeamClients<'_>, prerequisite", capture)
        self.assertIn("prerequisite.await?;", capture.split("pub(super) async fn run_packet")[1])

    def test_i15_uses_equipment_codec_and_restores_after_failed_sample(self):
        source = self.source("client_row_i15")
        self.assertIn("inventory::send(", source)
        self.assertNotIn("move_ring(&mut client, handle, false).await?;", source)

    def test_i10_rechecks_assignment_before_entry(self):
        source = self.source("client_row_i10")
        journey = source.split("async fn journey(")[1]
        self.assertIn("ensure_assignment(", journey)
        self.assertLess(journey.index("ensure_assignment("), journey.index("transfer("))

    def test_i10_prefers_the_existing_housing_slot(self):
        source = self.source("client_row_i10")
        self.assertIn("game_account_housing_slots", source)
        self.assertIn("if owned_house.is_none()", source)

    def test_i10_releases_the_stream_before_ambiguous_recovery(self):
        source = self.source("client_row_i10")
        self.assertIn("recover_on_fresh_session", source)
        self.assertIn("drop(client);", source.split("let proofs =")[1])

    def test_capture_errors_are_retained_separately_from_full_result(self):
        source = self.source("full_e2ef6")
        self.assertNotIn("capture_inventory(&mut client)?", source)
        self.assertNotIn("capture_receipt(&mut client, expected_occurrence).await?", source)
        self.assertIn("capture_result", source)


    def test_twist_capture_uses_post_cleanup_session_and_budget(self):
        source = self.source("full_e2ef6")
        death = source.split("async fn blessing_death(")[1].split("async fn blessing_services(")[0]
        self.assertNotIn("capture_receipt(", death)
        self.assertNotIn("durable_death_occurrence(", death)
        capture = source.split("async fn capture_twist(")[1].split("async fn blessing_death(")[0]
        self.assertIn("indexed_timeout(", capture)
        self.assertIn("connect_world(", capture)

    def test_progression_requires_an_observed_unlock(self):
        journey = self.source("full_e2ef18").split("async fn journey(")[1]
        self.assertNotIn("no-unlock-at-level", journey)
        self.assertIn("automatic_unlocks_observed(&expected_unlocks, &newly_learned)", journey)
        self.assertIn("newly_learned(&spells_before, &spells, unlock_level, &spell_book)", journey)
        self.assertIn("reached.stats.level != unlock_level", journey)

    def test_ring_recovery_outlives_the_sample_deadline(self):
        source = self.source("client_row_i15")
        packet = source.split("async fn run_packet(")[1].split("fn capture(")[0]
        self.assertNotIn("indexed_timeout(", packet)
        sample, recovery = source.split("if recover_ring {", 1)
        self.assertIn("timeout_at(sample_deadline, async {", sample)
        self.assertIn("restore_after_reconnect(", recovery)

    def test_house_restore_timeout_waits_for_durable_release(self):
        source = self.source("client_row_i10")
        recovery = source.split("async fn recover_on_fresh_session(")[1].split("async fn journey(")[0]
        self.assertIn("drop(client);", recovery)
        self.assertIn("wait_character_released(clients, 1).await", recovery)
        self.assertNotIn("restore_wrapped(&mut client, db, request)).await?", recovery)

    def test_house_assignment_writes_share_the_channel_generation_lock(self):
        source = self.source("client_row_i10")
        assignment = source.split("async fn ensure_assignment(")[1].split("type PlacedOven")[0]
        self.assertIn("FOR SHARE", assignment)
        self.assertIn("clients.scope_generation", assignment)
        self.assertIn("assign_house_scope_in_transaction(&mut tx", assignment)
        self.assertIn("tx.commit().await?", assignment)


if __name__ == "__main__":
    unittest.main()
