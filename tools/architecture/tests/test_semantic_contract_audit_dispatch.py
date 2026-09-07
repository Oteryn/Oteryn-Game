from __future__ import annotations

import re
import sys
import unittest
from pathlib import Path


REPOSITORY_ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(REPOSITORY_ROOT / "tools" / "architecture"))

import semantic_contract_audit as audit  # noqa: E402


class SemanticContractAuditDispatchTests(unittest.TestCase):
    def test_selects_profile_for_one_relevant_file(self) -> None:
        self.assertEqual(
            audit.select_profiles(
                {"docs/agents/tasks/archive/OTV2-20260815-alpha-client-architecture.md"}
            ),
            ["ALPHA_CLIENT_01"],
        )

    def test_unrelated_file_does_not_suppress_profile(self) -> None:
        self.assertEqual(
            audit.select_profiles(audit.F_PATHS | {"README.md"}),
            ["ANL_02_ANL_03"],
        )

    def test_selects_every_affected_profile(self) -> None:
        self.assertEqual(
            audit.select_profiles(
                {
                    "docs/architecture/ALPHA-CLIENT-01_NATIVE_CLIENT_ARCHITECTURE_ANALYSIS.md",
                    "apps/game-server/src/foundation/fnd04_verifier.rs",
                }
            ),
            ["ALPHA_CLIENT_01", "FOUNDATION_RECONNECT_DURABILITY_V1"],
        )

    def test_unrelated_only_is_not_applicable(self) -> None:
        self.assertEqual(audit.select_profiles({"README.md"}), [])

    def test_profile_order_is_stable_for_unordered_input(self) -> None:
        self.assertEqual(
            audit.select_profiles(audit.R_PATHS | audit.F_PATHS | audit.E_PATHS),
            [
                "ALPHA_CLIENT_01",
                "ANL_02_ANL_03",
                "FOUNDATION_RECONNECT_DURABILITY_V1",
            ],
        )

    def test_current_repository_profiles_execute_against_real_inputs(self) -> None:
        for profile_check in (audit.alpha, audit.analytics, audit.foundation_reconnect):
            with self.subTest(profile=profile_check.__name__):
                self.assertTrue(profile_check())

    def test_failure_does_not_suppress_later_selected_profile(self) -> None:
        calls = []

        def fails() -> list[str]:
            calls.append("first")
            raise SystemExit("first failed")

        def passes() -> list[str]:
            calls.append("second")
            return ["second check"]

        profiles, failures = audit.run_profiles(
            ["FIRST", "SECOND"], {"FIRST": fails, "SECOND": passes}
        )

        self.assertEqual(calls, ["first", "second"])
        self.assertEqual([profile["verdict"] for profile in profiles], ["FAIL", "PASS"])
        self.assertEqual(failures, ["FIRST: first failed"])

    def test_decode_error_does_not_suppress_later_selected_profile(self) -> None:
        calls = []

        def decode_fails() -> list[str]:
            calls.append("decode")
            raise UnicodeDecodeError("utf-8", b"\xff", 0, 1, "invalid start byte")

        def passes() -> list[str]:
            calls.append("second")
            return ["second check"]

        profiles, failures = audit.run_profiles(
            ["DECODE", "SECOND"], {"DECODE": decode_fails, "SECOND": passes}
        )

        self.assertEqual(calls, ["decode", "second"])
        self.assertEqual([profile["verdict"] for profile in profiles], ["FAIL", "PASS"])
        self.assertEqual(profiles[0]["error_type"], "UnicodeDecodeError")
        self.assertEqual(len(failures), 1)
        self.assertIn("invalid start byte", failures[0])

    def test_foundation_oracle_rejects_each_relaxed_authority_term(self) -> None:
        task, implementation, verifier = self._foundation_documents()
        authority_matcher = audit.rust_braced_block(
            implementation,
            "fn current_authority_matches_record(",
            "complete current authority matcher",
        )
        for term in audit.CURRENT_AUTHORITY_TERMS:
            if term.startswith("authenticated_evidence_observed_by"):
                relaxed = "!" + term
            elif "==" in term:
                relaxed = term.replace("==", "!=", 1)
            elif "candidate.is_live_at" in term:
                term = "candidate.is_live_at(current.observed_at)"
                relaxed = "!candidate.is_live_at(current.observed_at)"
            else:
                self.assertEqual(term, "!current.current_controller_present")
                relaxed = term.removeprefix("!")
            with self.subTest(term=term):
                mutated_matcher = self._replace_compact_fragment(
                    authority_matcher, term, relaxed
                )
                mutated = implementation.replace(authority_matcher, mutated_matcher, 1)
                with self.assertRaisesRegex(
                    SystemExit, "current authority matcher"
                ):
                    audit.foundation_reconnect_documents(task, mutated, verifier)

    def test_foundation_oracle_rejects_relaxed_authority_connectors(self) -> None:
        task, implementation, verifier = self._foundation_documents()
        authority_matcher = audit.rust_braced_block(
            implementation,
            "fn current_authority_matches_record(",
            "complete current authority matcher",
        )
        for occurrence in range(len(audit.CURRENT_AUTHORITY_TERMS) - 1):
            with self.subTest(connector=occurrence):
                mutated_matcher = self._replace_nth(
                    authority_matcher, "&&", "||", occurrence
                )
                mutated = implementation.replace(authority_matcher, mutated_matcher, 1)
                with self.assertRaisesRegex(SystemExit, "current authority"):
                    audit.foundation_reconnect_documents(task, mutated, verifier)

    def test_foundation_oracle_rejects_early_authority_success(self) -> None:
        task, implementation, verifier = self._foundation_documents()
        authority_matcher = audit.rust_braced_block(
            implementation,
            "fn current_authority_matches_record(",
            "complete current authority matcher",
        )
        opening = authority_matcher.index("{") + 1
        mutated_matcher = (
            authority_matcher[:opening]
            + " if current.current_controller_present { return Ok(true); } "
            + authority_matcher[opening:]
        )
        mutated = implementation.replace(authority_matcher, mutated_matcher, 1)
        with self.assertRaisesRegex(SystemExit, "exclusive current authority"):
            audit.foundation_reconnect_documents(task, mutated, verifier)

    def test_foundation_oracle_rejects_relaxed_evidence_logic(self) -> None:
        task, implementation, verifier = self._foundation_documents()
        evidence = audit.rust_braced_block(
            implementation,
            "fn authenticated_evidence_observed_by(",
            "authenticated evidence observation matcher",
        )
        for comparison in audit.AUTHENTICATED_EVIDENCE_COMPARISONS:
            with self.subTest(comparison=comparison):
                mutated_evidence = self._replace_compact_fragment(
                    evidence, comparison, comparison.replace(">=", "<", 1)
                )
                mutated = implementation.replace(evidence, mutated_evidence, 1)
                with self.assertRaisesRegex(SystemExit, "authenticated evidence"):
                    audit.foundation_reconnect_documents(task, mutated, verifier)
        mutated_evidence = self._replace_nth(evidence, "&&", "||", 0)
        mutated = implementation.replace(evidence, mutated_evidence, 1)
        with self.assertRaisesRegex(SystemExit, "authenticated evidence"):
            audit.foundation_reconnect_documents(task, mutated, verifier)

    def test_foundation_oracle_rejects_early_evidence_success(self) -> None:
        task, implementation, verifier = self._foundation_documents()
        evidence = audit.rust_braced_block(
            implementation,
            "fn authenticated_evidence_observed_by(",
            "authenticated evidence observation matcher",
        )
        opening = evidence.index("{") + 1
        mutated_evidence = (
            evidence[:opening]
            + " if observed_at >= 0 { return true; } "
            + evidence[opening:]
        )
        mutated = implementation.replace(evidence, mutated_evidence, 1)
        with self.assertRaisesRegex(SystemExit, "exclusive authenticated evidence"):
            audit.foundation_reconnect_documents(task, mutated, verifier)

    def test_foundation_oracle_allows_nonsemantic_helper_comments(self) -> None:
        task, implementation, verifier = self._foundation_documents()
        for marker in (
            "fn current_authority_matches_record(",
            "fn authenticated_evidence_observed_by(",
        ):
            helper = audit.rust_braced_block(implementation, marker, marker)
            opening = helper.index("{") + 1
            commented = helper[:opening] + " /* formatting note */ " + helper[opening:]
            mutated = implementation.replace(helper, commented, 1)
            with self.subTest(marker=marker):
                self.assertTrue(
                    audit.foundation_reconnect_documents(task, mutated, verifier)
                )

    def test_rust_scanner_ignores_comment_and_literal_tokens(self) -> None:
        source = r'''
fn target() {
    let normal = "} fn target() { // not a comment";
    let raw = br###"{ /* fn target() {} */ }"###;
    let byte = b'}';
    let unicode = '\u{7d}';
    // unmatched closing brace: } fn target() {
    /* unmatched opening braces: { { /* nested } */
       fn target() { */
}
'''
        block = audit.rust_named_function(source, "target", "synthetic target")
        self.assertTrue(block.rstrip().endswith("}"))
        self.assertIn('"} fn target() { // not a comment"', block)
        self.assertIn('br###"{ /* fn target() {} */ }"###', block)

    def test_foundation_oracle_allows_unmatched_comment_braces(self) -> None:
        task, implementation, verifier = self._foundation_documents()
        targets = (
            audit.rust_named_function(
                implementation,
                "current_authority_matches_record",
                "authority helper",
            ),
            audit.rust_impl_method(
                implementation,
                "impl ReconnectDurabilityFlowV1 {",
                "pub fn authorize_commit(",
                "V1 authorize_commit",
            ),
        )
        comment = (
            " // unmatched closing brace and fake declaration: } fn authorize_commit() {\n"
            " /* unmatched opening braces: { { /* nested } */ */ "
        )
        for target in targets:
            opening = target.index("{") + 1
            commented = target[:opening] + comment + target[opening:]
            mutated = implementation.replace(target, commented, 1)
            with self.subTest(target=target.split("(", 1)[0]):
                self.assertTrue(
                    audit.foundation_reconnect_documents(task, mutated, verifier)
                )

    def test_foundation_oracle_finds_whitespace_and_comment_spaced_declarations(
        self,
    ) -> None:
        task, implementation, verifier = self._foundation_documents()
        mutated = implementation.replace(
            "impl ReconnectDurabilityFlowV1 {",
            "impl /* scanner spacing */\n ReconnectDurabilityFlowV1\n {",
            1,
        ).replace(
            "pub fn authorize_commit(",
            "pub\n /* scanner spacing */ fn\n authorize_commit(",
            1,
        )
        self.assertTrue(audit.foundation_reconnect_documents(task, mutated, verifier))

    def test_foundation_oracle_normalizes_raw_audited_identifiers(self) -> None:
        task, implementation, verifier = self._foundation_documents()
        mutated = implementation.replace(
            "impl ReconnectDurabilityFlowV1 {",
            "impl r#ReconnectDurabilityFlowV1 {",
            1,
        ).replace(
            "pub fn authorize_commit(",
            "pub fn r#authorize_commit(",
            1,
        ).replace(
            "fn current_authority_matches_record(",
            "fn r#current_authority_matches_record(",
            1,
        )
        self.assertTrue(audit.foundation_reconnect_documents(task, mutated, verifier))

    def test_foundation_oracle_rejects_raw_cfg_shadowed_declarations(self) -> None:
        task, implementation, verifier = self._foundation_documents()
        helper = audit.rust_named_function(
            implementation,
            "current_authority_matches_record",
            "authority helper",
        )
        active_helper = helper.replace(
            "fn current_authority_matches_record(",
            "fn r#current_authority_matches_record(",
            1,
        )
        helper_opening = active_helper.index("{") + 1
        active_helper = (
            active_helper[:helper_opening]
            + " if current.current_controller_present { return Ok(true); } "
            + active_helper[helper_opening:]
        )
        helper_shadow = (
            "#[r#cfg(any())]\n" + helper + "\n" + active_helper
        )

        method = audit.rust_impl_method(
            implementation,
            "impl ReconnectDurabilityFlowV1 {",
            "pub fn authorize_commit(",
            "V1 authorize_commit",
        )
        raw_method = method.replace(
            "fn authorize_commit(", "fn r#authorize_commit(", 1
        )
        method_shadow = (
            "#[r#cfg(any())]\npub " + method + "\npub " + raw_method
        )

        cases = (
            implementation.replace(helper, helper_shadow, 1),
            implementation.replace("pub " + method, method_shadow, 1),
            implementation.replace(
                "impl ReconnectDurabilityFlowV1 {",
                "#[r#cfg(any())]\nimpl r#ReconnectDurabilityFlowV1 {",
                1,
            ),
        )
        for mutated in cases:
            with self.subTest(case=cases.index(mutated)):
                with self.assertRaises(SystemExit):
                    audit.foundation_reconnect_documents(task, mutated, verifier)

    def test_foundation_oracle_rejects_cfg_shadowed_audited_methods(self) -> None:
        task, implementation, verifier = self._foundation_documents()
        for version, method_name in (
            ("V1", "authorize_commit"),
            ("V1", "accept_reconciliation"),
            ("V2", "authorize_commit"),
            ("V2", "accept_reconciliation"),
        ):
            method = audit.rust_impl_method(
                implementation,
                f"impl ReconnectDurabilityFlow{version} {{",
                f"pub fn {method_name}(",
                f"{version} {method_name}",
            )
            declaration = "pub " + method
            shadowed = "#[cfg(any())]\n" + declaration + "\npub\n" + method
            mutated = implementation.replace(declaration, shadowed, 1)
            with self.subTest(version=version, method=method_name):
                with self.assertRaisesRegex(SystemExit, "exactly one method"):
                    audit.foundation_reconnect_documents(task, mutated, verifier)

    def test_foundation_oracle_rejects_cfg_controlled_audited_declarations(
        self,
    ) -> None:
        task, implementation, verifier = self._foundation_documents()
        method = audit.rust_impl_method(
            implementation,
            "impl ReconnectDurabilityFlowV1 {",
            "pub fn authorize_commit(",
            "V1 authorize_commit",
        )
        helper = audit.rust_named_function(
            implementation,
            "current_authority_matches_record",
            "authority helper",
        )
        cfg_then_braced_doc = '#[cfg(any())]\n#[doc = concat! { "dormant" }]\n'
        cfg_attr_then_braced_doc = (
            '#[cfg_attr(any(), cfg(any()))]\n#[doc = concat! { "dormant" }]\n'
        )
        cases = (
            implementation.replace(
                "pub " + method, "#[cfg(any())]\npub " + method, 1
            ),
            implementation.replace(
                "pub " + method, cfg_attr_then_braced_doc + "pub " + method, 1
            ),
            implementation.replace(
                "impl ReconnectDurabilityFlowV1 {",
                cfg_then_braced_doc + "impl ReconnectDurabilityFlowV1 {",
                1,
            ),
            implementation.replace(
                helper, cfg_then_braced_doc + helper, 1
            ),
        )
        for mutated in cases:
            with self.subTest(case=cases.index(mutated)):
                with self.assertRaisesRegex(SystemExit, "conditional audited"):
                    audit.foundation_reconnect_documents(task, mutated, verifier)

    def test_foundation_oracle_rejects_cfg_shadowed_helper(self) -> None:
        task, implementation, verifier = self._foundation_documents()
        helper = audit.rust_named_function(
            implementation,
            "current_authority_matches_record",
            "authority helper",
        )
        shadowed = "#[cfg(any())]\n" + helper + "\n" + helper
        mutated = implementation.replace(helper, shadowed, 1)
        with self.assertRaisesRegex(SystemExit, "exactly one function"):
            audit.foundation_reconnect_documents(task, mutated, verifier)

    def test_foundation_oracle_rejects_nested_audited_items(self) -> None:
        task, implementation, verifier = self._foundation_documents()
        helper = audit.rust_named_function(
            implementation,
            "current_authority_matches_record",
            "authority helper",
        )
        flow_impl = audit.rust_braced_block(
            implementation,
            "impl ReconnectDurabilityFlowV1 {",
            "V1 impl",
        )
        cases = (
            (
                implementation.replace(
                    helper,
                    "#[cfg(any())]\nmod dormant_helper {\n" + helper + "\n}",
                    1,
                ),
                "top-level item",
            ),
            (
                implementation.replace(
                    flow_impl,
                    "#[cfg(any())]\nmod dormant_impl {\n" + flow_impl + "\n}",
                    1,
                ),
                "top-level item",
            ),
        )
        for mutated, expected in cases:
            with self.subTest(expected=expected):
                with self.assertRaisesRegex(SystemExit, expected):
                    audit.foundation_reconnect_documents(task, mutated, verifier)

    def test_foundation_oracle_rejects_method_in_nested_impl(self) -> None:
        task, implementation, verifier = self._foundation_documents()
        method = audit.rust_impl_method(
            implementation,
            "impl ReconnectDurabilityFlowV1 {",
            "pub fn authorize_commit(",
            "V1 authorize_commit",
        )
        declaration = "pub " + method
        mutated = implementation.replace(declaration, "", 1)
        mutated += (
            "\nfn scanner_container() {\n"
            "impl ReconnectDurabilityFlowV1 {\npub "
            + method
            + "\n}\n}\n"
        )
        with self.assertRaisesRegex(SystemExit, "top-level item"):
            audit.foundation_reconnect_documents(task, mutated, verifier)

    def test_foundation_oracle_allows_unconditional_and_unrelated_attributes(
        self,
    ) -> None:
        task, implementation, verifier = self._foundation_documents()
        helper = audit.rust_named_function(
            implementation,
            "current_authority_matches_record",
            "authority helper",
        )
        mutated = implementation.replace(helper, "#[inline]\n" + helper, 1)
        mutated += r'''
#[cfg(any())]
impl UnrelatedScannerProbe {
    pub fn authorize_commit(&self) { let marker = "fn authorize_commit() {"; }
}
'''
        self.assertTrue(audit.foundation_reconnect_documents(task, mutated, verifier))

    def test_foundation_oracle_rejects_relaxed_authorize_guards_per_version(self) -> None:
        task, implementation, verifier = self._foundation_documents()
        mutations = (
            (
                "self.phase != ReconnectDurabilityPhaseV1::AwaitFinalRevalidation",
                "self.phase == ReconnectDurabilityPhaseV1::AwaitFinalRevalidation",
            ),
            (
                "if !current_authority_matches_record",
                "if current_authority_matches_record",
            ),
            (
                ")? || !authenticated_evidence_observed_by",
                ")? && !authenticated_evidence_observed_by",
            ),
            (
                "|| !authenticated_evidence_observed_by",
                "|| authenticated_evidence_observed_by",
            ),
            (
                "ReconnectDurabilityErrorV1::StaleAuthority",
                "ReconnectDurabilityErrorV1::InvalidPhase",
            ),
            (
                "let deadline = self.record.authorization_deadline()?;",
                "let deadline = self.record.authorization_deadline()? + 1;",
            ),
            ("if now > deadline", "if now <= deadline"),
            ("deadline || current.observed_at", "deadline && current.observed_at"),
            ("current.observed_at > deadline", "current.observed_at <= deadline"),
            (
                "ReconnectDurabilityErrorV1::DeadlineExpired",
                "ReconnectDurabilityErrorV1::StaleAuthority",
            ),
        )
        for version in ("V1", "V2"):
            method = audit.rust_impl_method(
                implementation,
                f"impl ReconnectDurabilityFlow{version} {{",
                "pub fn authorize_commit(",
                f"{version} authorize_commit",
            )
            for original, relaxed in mutations:
                with self.subTest(version=version, mutation=original):
                    mutated_method = self._replace_compact_fragment(
                        method, original, relaxed
                    )
                    mutated = implementation.replace(method, mutated_method, 1)
                    with self.assertRaisesRegex(
                        SystemExit, f"Flow{version} authorize_commit"
                    ):
                        audit.foundation_reconnect_documents(task, mutated, verifier)

    def test_foundation_oracle_rejects_extra_early_commit_request_per_version(self) -> None:
        task, implementation, verifier = self._foundation_documents()
        for version in ("V1", "V2"):
            method = audit.rust_impl_method(
                implementation,
                f"impl ReconnectDurabilityFlow{version} {{",
                "pub fn authorize_commit(",
                f"{version} authorize_commit",
            )
            request = self._braced_expression(
                method, "let request = ReconnectCommitRequestV1 {"
            )
            request += ";"
            early_request = request.replace(
                "let request =", "let _early_request =", 1
            ).replace(
                "authorization_deadline: deadline",
                "authorization_deadline: self.record.authorization_deadline()?",
                1,
            )
            opening = method.index("{") + 1
            mutated_method = method[:opening] + early_request + method[opening:]
            mutated = implementation.replace(method, mutated_method, 1)
            with self.subTest(version=version):
                with self.assertRaisesRegex(
                    SystemExit, f"Flow{version} authorize_commit"
                ):
                    audit.foundation_reconnect_documents(task, mutated, verifier)

    def test_foundation_oracle_rejects_cached_request_before_guards(self) -> None:
        task, implementation, verifier = self._foundation_documents()
        method = audit.rust_impl_method(
            implementation,
            "impl ReconnectDurabilityFlowV1 {",
            "pub fn authorize_commit(",
            "V1 authorize_commit",
        )
        opening = method.index("{") + 1
        early = (
            " if self.commit_request.is_some() { "
            "return Ok(self.commit_request.clone().unwrap()); } "
        )
        mutated_method = method[:opening] + early + method[opening:]
        mutated = implementation.replace(method, mutated_method, 1)
        with self.assertRaisesRegex(SystemExit, "FlowV1 authorize_commit"):
            audit.foundation_reconnect_documents(task, mutated, verifier)

    def test_foundation_oracle_rejects_relaxed_reconciliation_guards_per_version(self) -> None:
        task, implementation, verifier = self._foundation_documents()
        versions = (
            (
                "V1",
                "snapshot.current_generation != Some(self.record.connection().candidate())",
                "snapshot.current_transport_ref != Some(self.record.connection().transport_ref())",
                "generation: self.record.connection().candidate()",
                "generation: self.record.connection().predecessor()",
                "transport_ref: self.record.connection().transport_ref()",
                "transport_ref: snapshot.current_transport_ref.expect(\"committed transport\")",
            ),
            (
                "V2",
                "current_generation != self.record.connection().candidate()",
                "current_transport_ref != self.record.connection().transport_ref()",
                "generation: current_generation",
                "generation: self.record.connection().predecessor()",
                "transport_ref: current_transport_ref",
                "transport_ref: self.record.connection().transport_ref()",
            ),
        )
        for (
            version,
            generation,
            transport,
            output_generation,
            relaxed_output_generation,
            output_transport,
            relaxed_output_transport,
        ) in versions:
            method = audit.rust_impl_method(
                implementation,
                f"impl ReconnectDurabilityFlow{version} {{",
                "pub fn accept_reconciliation(",
                f"{version} accept_reconciliation",
            )
            mutations = (
                (
                    "self.phase != ReconnectDurabilityPhaseV1::ReconciliationRequired",
                    "self.phase == ReconnectDurabilityPhaseV1::ReconciliationRequired",
                ),
                (
                    "snapshot.record != self.record",
                    "snapshot.record == self.record",
                ),
                (generation, generation.replace("!=", "==", 1)),
                (transport, transport.replace("!=", "==", 1)),
                (
                    "current.observed_at > self.record.authorization_deadline()?",
                    "current.observed_at <= self.record.authorization_deadline()?",
                ),
                (
                    "|| !current_authority_matches_record",
                    "|| current_authority_matches_record",
                ),
                (
                    "self.phase = ReconnectDurabilityPhaseV1::Completed;",
                    "self.phase = ReconnectDurabilityPhaseV1::Terminal;",
                ),
                (output_generation, relaxed_output_generation),
                (output_transport, relaxed_output_transport),
            )
            for original, relaxed in mutations:
                with self.subTest(version=version, mutation=original):
                    mutated_method = self._replace_compact_fragment(
                        method, original, relaxed
                    )
                    mutated = implementation.replace(method, mutated_method, 1)
                    with self.assertRaisesRegex(
                        SystemExit, f"Flow{version} accept_reconciliation"
                    ):
                        audit.foundation_reconnect_documents(task, mutated, verifier)

    def test_foundation_oracle_rejects_relaxed_reconciliation_connectors(self) -> None:
        task, implementation, verifier = self._foundation_documents()
        versions = (
            ("V1", "if snapshot.current_generation != Some("),
            ("V2", "if current_generation"),
        )
        for version, guard_marker in versions:
            method = audit.rust_impl_method(
                implementation,
                f"impl ReconnectDurabilityFlow{version} {{",
                "pub fn accept_reconciliation(",
                f"{version} accept_reconciliation",
            )
            guard = audit.rust_braced_block(
                method, guard_marker, f"{version} committed guard"
            )
            for occurrence in range(3):
                mutated_guard = self._replace_nth(guard, "||", "&&", occurrence)
                mutated_method = method.replace(guard, mutated_guard, 1)
                mutated = implementation.replace(method, mutated_method, 1)
                with self.subTest(version=version, connector=occurrence):
                    with self.assertRaisesRegex(
                        SystemExit, f"Flow{version} accept_reconciliation"
                    ):
                        audit.foundation_reconnect_documents(task, mutated, verifier)

    def test_foundation_oracle_rejects_extra_early_controller_install(self) -> None:
        task, implementation, verifier = self._foundation_documents()
        for version in ("V1", "V2"):
            method = audit.rust_impl_method(
                implementation,
                f"impl ReconnectDurabilityFlow{version} {{",
                "pub fn accept_reconciliation(",
                f"{version} accept_reconciliation",
            )
            controller = self._braced_expression(
                method,
                f"Ok(ReconnectProjectionDecision{version}::InstallController {{",
            ) + ")"
            guard_marker = (
                "if snapshot.current_generation != Some("
                if version == "V1"
                else "if current_generation"
            )
            insertion = method.index(guard_marker)
            early = (
                "if current.observed_at == current.observed_at { "
                f"return {controller}; }} "
            )
            mutated_method = method[:insertion] + early + method[insertion:]
            mutated = implementation.replace(method, mutated_method, 1)
            with self.subTest(version=version):
                with self.assertRaisesRegex(
                    SystemExit, f"Flow{version} accept_reconciliation"
                ):
                    audit.foundation_reconnect_documents(task, mutated, verifier)

    def test_foundation_oracle_rejects_aliased_controller_before_guards(self) -> None:
        task, implementation, verifier = self._foundation_documents()
        for version in ("V1", "V2"):
            method = audit.rust_impl_method(
                implementation,
                f"impl ReconnectDurabilityFlow{version} {{",
                "pub fn accept_reconciliation(",
                f"{version} accept_reconciliation",
            )
            opening = method.index("{") + 1
            early = (
                f" use ReconnectProjectionDecision{version}::InstallController as IC; "
                "if current.current_controller_present { "
                "return Ok(IC { "
                "generation: self.record.connection().candidate(), "
                "transport_ref: self.record.connection().transport_ref() }); } "
            )
            mutated_method = method[:opening] + early + method[opening:]
            mutated = implementation.replace(method, mutated_method, 1)
            with self.subTest(version=version):
                with self.assertRaisesRegex(
                    SystemExit, f"Flow{version} accept_reconciliation"
                ):
                    audit.foundation_reconnect_documents(task, mutated, verifier)

    def test_foundation_oracle_rejects_missing_v1_record_equality(self) -> None:
        task, implementation, verifier = self._foundation_documents()
        v1 = audit.rust_impl_method(
            implementation,
            "impl ReconnectDurabilityFlowV1 {",
            "pub fn accept_reconciliation(",
            "V1 reconciliation",
        )
        v2 = audit.rust_impl_method(
            implementation,
            "impl ReconnectDurabilityFlowV2 {",
            "pub fn accept_reconciliation(",
            "V2 reconciliation",
        )
        mutated_v1 = self._replace_compact_fragment(
            v1,
            "snapshot.record != self.record",
            "snapshot.record == self.record",
        )
        mutated = implementation.replace(v1, mutated_v1, 1)
        self.assertIn("if snapshot.record != self.record", audit.compact(v2))
        with self.assertRaisesRegex(SystemExit, "FlowV1 accept_reconciliation"):
            audit.foundation_reconnect_documents(task, mutated, verifier)

    @staticmethod
    def _foundation_documents() -> tuple[str, str, str]:
        return (
            audit.text(
                "docs/agents/tasks/archive/OTV2-20260826-impl-foundation-reconnect-durability.md"
            ),
            audit.text("apps/game-server/src/foundation/admission_recovery_inner.rs"),
            audit.text("apps/game-server/src/foundation/fnd04_verifier.rs"),
        )

    @staticmethod
    def _replace_compact_fragment(
        block: str, fragment: str, replacement: str
    ) -> str:
        pattern = re.sub(r"\\ ", r"\\s+", re.escape(fragment))
        mutated, replacements = re.subn(pattern, replacement, block, count=1)
        if replacements != 1:
            raise AssertionError(f"expected exactly one match for {fragment!r}")
        return mutated

    @staticmethod
    def _replace_nth(block: str, original: str, replacement: str, index: int) -> str:
        starts = [match.start() for match in re.finditer(re.escape(original), block)]
        if index >= len(starts):
            raise AssertionError(f"missing occurrence {index} of {original!r}")
        start = starts[index]
        return block[:start] + replacement + block[start + len(original) :]

    @staticmethod
    def _braced_expression(block: str, marker: str) -> str:
        return audit.rust_braced_block(block, marker, marker)


if __name__ == "__main__":
    unittest.main()
