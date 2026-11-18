"""Check the live harness uses its evidence and recovery paths in the required order."""

from pathlib import Path
import unittest


ROOT = Path(__file__).resolve().parents[3]
GAME = ROOT / "apps/game-server/src"
PHASES = GAME / "qualification/client_phases"


class QualificationWiringTests(unittest.TestCase):
    def test_private_channel_fixture_precedes_both_create_commands(self):
        journey = (PHASES / "full_e2ef19.rs").read_text().split("async fn journey", 1)[1]
        first, second, following = journey.split("prepare_channel_owner(&mut a).await?")
        self.assertNotIn("PrivateChannelIntent::Create", first)
        self.assertEqual(second.count("PrivateChannelIntent::Create"), 1)
        self.assertEqual(following.count("PrivateChannelIntent::Create"), 1)

    def test_stored_teardown_is_sampled_between_logout_and_readmission(self):
        journey = (PHASES / "full_e2ef19.rs").read_text()
        before, after = journey.split("logout(&mut a).await?;", 1)
        self.assertIn("durable_channel_rows(&database", before)
        read, admission = after.split("a = connect_character", 1)
        self.assertIn("durable_channel_rows(&database", read)
        self.assertNotIn("channel_list(", read)
        self.assertIn("relog-transient-list", admission)

    def test_c14_consumes_refreshed_maxima_and_independently_checks_award(self):
        source = (PHASES / "full_e2ef18.rs").read_text()
        self.assertIn("max_mana == expected_mana", source)
        self.assertIn("after.vitals = await_level_publication", source)
        self.assertIn("catalog_deadline", source)
        self.assertNotIn("hunting_experience::hunting_experience", source)
        self.assertIn("let expected = expected_rat_award", source)

    def test_spawn_selection_advances_beyond_an_empty_lane(self):
        source = (PHASES / "world.rs").read_text().split("async fn approach_spawn_target", 1)[1]
        self.assertIn("remaining_lanes.remove", source)
        self.assertIn("point.respawn_ms", source)
        self.assertNotIn("unreachable.insert", (PHASES / "full_gold.rs").read_text())

    def test_moving_target_must_pass_lane_budget_before_each_step(self):
        search = (PHASES / "world.rs").read_text().split("async fn approach_spawn_target", 1)[1]
        budget = search.find("if chase_steps == 32")
        step = search.find("walk_until(client, world, &[direction]")
        self.assertGreaterEqual(budget, 0)
        self.assertLess(budget, step)
        self.assertTrue("".join(search[budget:step].split()).startswith(
            "ifchase_steps==32&&!remaining_lanes.is_empty()"
            "||tokio::time::Instant::now()>=deadline{"
        ))
        self.assertIn("search=true;continue;", "".join(search[budget:step].split()))
        self.assertIn("chase_steps+=1;", "".join(search[budget:step].split()))

    def test_gold_scans_stocked_species_before_waiting_for_respawn(self):
        earn = (PHASES / "full_gold.rs").read_text().split("async fn earn", 1)[1]
        first_pass = earn.split("let (candidate, target)", 1)[0]
        self.assertIn("SpawnRespawn::Skip", first_pass)
        self.assertNotIn("SpawnRespawn::Wait", earn)
        self.assertIn("Some(probe_deadline)", first_pass)

    def test_m04_resolves_action_before_cleanup_and_checks_collision_first(self):
        source = (PHASES / "full_e2ef1.rs").read_text()
        m04 = source.split("async fn m04", 1)[1].split("async fn player_push", 1)[0]
        self.assertIn('["collision", "push-player", "push-creature", "relogin"]', source)
        self.assertIn("player.and(creature).and(left)", m04)
        self.assertNotIn("left?;", m04)

    def test_ordinary_build_profiles_use_replayable_occurrences(self):
        control = (GAME / "node/serve/qualification_fixture.rs").read_text()
        setup = control.split("let combat_progression", 1)[1].split("if !combat_progression", 1)[0]
        compact = "".join(setup.split())
        self.assertIn(
            "let(xp_occurrence,build_occurrence)="
            "qualification_build::fixture_occurrences(character,vocation,level,combat_progression);",
            compact,
        )
        self.assertIn(
            "QualificationBuildSpec{vocation,level,context,xp_occurrence,build_occurrence,}",
            compact,
        )
        fixture = (GAME / "qualification_spell_character.rs").read_text()
        self.assertNotIn("let resumed = if before_level_up", fixture)

    def test_pursuit_step_uses_cooperative_lane_deadline(self):
        source = (PHASES / "world.rs").read_text()
        compact = "".join(source.split())
        self.assertTrue("walk_until(client,world,&[direction],Some(deadline),true)" in compact)

    def test_gold_probes_with_a_species_deadline(self):
        earn = (PHASES / "full_gold.rs").read_text()
        self.assertTrue("Some(probe_deadline)" in earn)

    def test_gold_reprobes_all_species_when_initial_scan_is_dry(self):
        earn = (PHASES / "full_gold.rs").read_text()
        self.assertTrue("if picked.is_none()" in earn and 'progress("gold-respawn")' in earn)

    def test_item_replay_continues_for_same_and_replacement_sessions(self):
        control = (GAME / "node/serve/qualification_fixture.rs").read_text()
        self.assertTrue("RewardClaimMintOutcome::AlreadyCommitted" in control)
        self.assertTrue("RewardClaimRefusal::AlreadyClaimed" in control)
        items = control.split("async fn prepare_items", 1)[1]
        claim_lookup = items.find("qualification_fixture_item_is_claimed(")
        self.assertGreaterEqual(claim_lookup, 0)
        self.assertLess(claim_lookup, items.index("freeze_reward_claim_mint("))

    def test_player_setup_marks_its_stage_before_collision(self):
        player = (PHASES / "full_e2ef1.rs").read_text().split("async fn player_push", 1)[1]
        self.assertTrue(player.find('checks.active = Some("push-player")') >= 0)
        self.assertLess(player.index('checks.active = Some("push-player")'),
                        player.index('checks.active = Some("collision")'))

    def test_live_push_validates_wire_refusals_with_the_classifier(self):
        source = (PHASES / "full_e2ef1.rs").read_text()
        push = source.split("async fn push", 1)[1].split("async fn m04", 1)[0]
        self.assertTrue("expect_push(result.status, &result.payload, expected)" in push)

    def test_blocked_spawn_route_replans_other_lanes(self):
        source = (PHASES / "world.rs").read_text().split("async fn approach_spawn_target", 1)[1]
        self.assertTrue("remaining_lanes.retain" in source)

    def test_partial_gold_approach_retains_species_priority(self):
        source = (PHASES / "full_gold.rs").read_text()
        self.assertTrue("partial_species = Some" in source)

    def test_spawn_exchange_has_a_hard_cutoff(self):
        source = (PHASES / "world.rs").read_text()
        self.assertTrue("timeout_at(deadline, client.step(direction))" in source)
        walk = source.split("async fn walk_until", 1)[1].split("pub(super) async fn logout", 1)[0]
        self.assertEqual(walk.count("probe_step(client, direction, deadline)"), 2)

    def test_species_liveness_has_a_hard_cutoff(self):
        source = (PHASES / "world.rs").read_text()
        self.assertTrue("async fn probe_liveness" in source)
        self.assertTrue("probe_liveness(client, Duration::from_millis(100), probe_deadline)" in source)

    def test_gold_cancellation_invalidates_the_session(self):
        source = (PHASES / "full_gold.rs").read_text()
        self.assertEqual(source.count("invalidate_after_cancellation()"), 5)

    def test_action_failure_is_preserved_before_cleanup(self):
        source = (PHASES / "full_e2ef1.rs").read_text()
        rows = source.split("async fn rows", 1)[1]
        compact = "".join(rows.split())
        self.assertLess(
            compact.index("verdicts.action_failure=Some"),
            compact.index("inventory_signature(&client)?"),
        )
        self.assertIn("action.or(self.aborted.take())", "".join(source.split()))
        self.assertIn("Some(checks.action_failure(required,error.as_ref()))", compact)
        self.assertIn(
            "elseifletSome((root,cause))=action_failure{cause.suffix(&root)}",
            compact,
        )

    def test_combat_setup_reconciles_both_retained_writes(self):
        source = (GAME / "qualification_spell_character.rs").read_text()
        self.assertIn("reconcile_character_experience(authority, xp_occurrence)", source)
        self.assertIn("reconcile_character_build(authority, build_occurrence)", source)
        self.assertIn("!resumed && experience.get() > 0", source)


if __name__ == "__main__":
    unittest.main()
