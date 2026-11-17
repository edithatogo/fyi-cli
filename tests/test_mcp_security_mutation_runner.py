"""Negative controls for the bounded, offline MCP security mutation runner."""

from __future__ import annotations

import copy
import hashlib
import json
import os
import sys
import tempfile
import unittest
from dataclasses import replace
from pathlib import Path
from unittest.mock import Mock, patch

import pytest

from scripts import run_mcp_security_mutations as mutations


def result(test: str, outcome: str, *, returncode: int | None = None) -> mutations.RunResult:
    """Return the relevant stable libtest output, including its exact selector."""
    failed = outcome == "FAILED"
    output = f"running 1 test\ntest {test} ... {outcome}\n"
    if failed:
        output += f"thread '{test}' panicked at src/example.rs:10:9:\nassertion failed: denied\n"
    output += (
        f"test result: {'FAILED' if failed else 'ok'}. {0 if failed else 1} passed; "
        f"{1 if failed else 0} failed; 0 ignored; 0 measured; 10 filtered out; "
        "finished in 0.01s\n"
    )
    return mutations.RunResult(
        returncode=(101 if failed else 0) if returncode is None else returncode,
        output=output,
        elapsed_seconds=0.01,
    )


class MutationManifestTests(unittest.TestCase):
    """The committed target list is versioned, bounded and fail closed."""

    def setUp(self) -> None:
        self.raw = json.loads(mutations.MANIFEST.read_text(encoding="utf-8"))

    def test_manifest_covers_every_required_boundary_and_current_source(self) -> None:
        targets = mutations.parse_manifest(self.raw)
        assert {target.category for target in targets} == mutations.REQUIRED_CATEGORIES
        assert len(targets) >= 10
        mutations.validate_sources(mutations.ROOT, targets)

    def test_manifest_rejects_unknown_ambiguous_or_missing_version(self) -> None:
        for version in (None, True, "1", 0, 2):
            with self.subTest(version=version):
                value = copy.deepcopy(self.raw)
                value["schema_version"] = version
                with pytest.raises(mutations.HarnessError):
                    mutations.parse_manifest(value)

    def test_manifest_rejects_incomplete_duplicate_and_empty_targets(self) -> None:
        for targets in ([], self.raw["mutations"][:1], self.raw["mutations"] * 2):
            with self.subTest(targets=len(targets)):
                value = copy.deepcopy(self.raw)
                value["mutations"] = targets
                with pytest.raises(mutations.HarnessError):
                    mutations.parse_manifest(value)

    def test_manifest_rejects_paths_commands_and_unknown_fields(self) -> None:
        for field, value in (
            ("path", "../private.rs"),
            ("path", "/outside-source/private.rs"),
            ("path", "crates/fyi-core/src/../../../private.rs"),
            ("path", "crates\\fyi-core\\src\\sync.rs"),
            ("package", "fyi-core;curl.example"),
            ("test", "--ignored"),
            ("test", "sync::tests::example --skip"),
            ("category", "invented"),
            ("shell", "echo should-never-execute"),
            ("after", ""),
        ):
            with self.subTest(field=field, value=value):
                changed = copy.deepcopy(self.raw)
                changed["mutations"][0][field] = value
                with pytest.raises(mutations.HarnessError):
                    mutations.parse_manifest(changed)

    def test_manifest_rejects_noop_and_duplicate_json_keys(self) -> None:
        value = copy.deepcopy(self.raw)
        value["mutations"][0]["after"] = value["mutations"][0]["before"]
        with pytest.raises(mutations.HarnessError):
            mutations.parse_manifest(value)
        with pytest.raises(mutations.HarnessError):
            mutations.unique_object([("schema_version", 1), ("schema_version", 1)])

    def test_optional_custom_assertion_message_is_bounded_and_unambiguous(self) -> None:
        for message in (None, True, "", " ", " message", "message\ncopy", "x" * 257):
            with self.subTest(message=message):
                value = copy.deepcopy(self.raw)
                value["mutations"][0]["expected_assertion_message"] = message
                with pytest.raises(mutations.HarnessError):
                    mutations.parse_manifest(value)


class LibtestResultTests(unittest.TestCase):
    """Only the intended executed test can kill a mutation."""

    test = "policy::tests::read_enablement_does_not_enable_write"

    def test_pass_and_assertion_failure_are_distinct(self) -> None:
        assert mutations.classify_run(result(self.test, "ok"), self.test) == "passed"
        assert mutations.classify_run(result(self.test, "FAILED"), self.test) == "caught"

    def test_compile_failure_missing_selector_and_ignored_test_are_not_caught(self) -> None:
        for output in (
            "error[E0308]: mismatched types\nerror: could not compile fyi-mcp\n",
            "test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 20 filtered out; "
            "finished in 0.00s\n",
            result("different::test", "FAILED").output,
            result(self.test, "FAILED").output.replace("... FAILED", "... ignored"),
            result(self.test, "FAILED").output.replace("assertion failed: denied", "abort"),
            result(self.test, "FAILED").output + result(self.test, "ok").output,
        ):
            with self.subTest(output=output):
                run = mutations.RunResult(101, output, 0.01)
                assert mutations.classify_run(run, self.test) == "unviable"

    def test_result_counts_and_process_status_must_agree(self) -> None:
        run = result(self.test, "FAILED", returncode=0)
        assert mutations.classify_run(run, self.test) == "unviable"
        run = result(self.test, "ok", returncode=101)
        assert mutations.classify_run(run, self.test) == "unviable"

    def test_timeout_never_counts_as_a_kill(self) -> None:
        run = mutations.RunResult(None, result(self.test, "FAILED").output, 5, timed_out=True)
        assert mutations.classify_run(run, self.test) == "timeout"


CUSTOM_ASSERTION = "uncertain writes must not be retried automatically"
CUSTOM_SELECTOR = (
    "observability_tests::remote_error_classification_maps_status_and_redacts_upstream_secrets"
)


def custom_assertion_result() -> mutations.RunResult:
    """Reproduce the panic frame observed in the retained 40bfe36 hosted log."""
    output = result(CUSTOM_SELECTOR, "FAILED").output.replace(
        f"thread '{CUSTOM_SELECTOR}' panicked at src/example.rs:10:9:\nassertion failed: denied",
        f"thread '{CUSTOM_SELECTOR}' (13642) panicked at "
        f"crates/fyi-mcp/src/observability_tests.rs:450:13:\n{CUSTOM_ASSERTION}",
    )
    return mutations.RunResult(101, output, 0.01)


def test_declared_custom_assertion_matches_only_its_exact_panic_frame() -> None:
    run = custom_assertion_result()
    assert mutations.classify_run(run, CUSTOM_SELECTOR) == "unviable"
    assert mutations.classify_run(run, CUSTOM_SELECTOR, CUSTOM_ASSERTION) == "caught"


@pytest.mark.parametrize(
    "message",
    [
        "",
        "unrelated panic",
        "assertion failed: unrelated earlier check",
        f"prefix {CUSTOM_ASSERTION}",
        f"{CUSTOM_ASSERTION} suffix",
    ],
)
def test_custom_message_copies_elsewhere_do_not_authorize_an_unrelated_panic(message: str) -> None:
    output = custom_assertion_result().output.replace(CUSTOM_ASSERTION, message)
    output = f"compiler log: {CUSTOM_ASSERTION}\n{CUSTOM_ASSERTION}\n{output}\n{CUSTOM_ASSERTION}\n"
    run = mutations.RunResult(101, output, 0.01)
    assert mutations.classify_run(run, CUSTOM_SELECTOR, CUSTOM_ASSERTION) == "unviable"


def test_custom_message_requires_exact_test_frame_failure_and_process_status() -> None:
    output = custom_assertion_result().output
    for invalid in (
        output.replace(f"thread '{CUSTOM_SELECTOR}'", "thread 'unrelated::test'"),
        output.replace("panicked at", "logged at"),
        output + "error: could not compile fyi-mcp\n",
        output.replace("0 passed; 1 failed", "0 passed; 2 failed"),
    ):
        run = mutations.RunResult(101, invalid, 0.01)
        assert mutations.classify_run(run, CUSTOM_SELECTOR, CUSTOM_ASSERTION) == "unviable"
    run = mutations.RunResult(0, output, 0.01)
    assert mutations.classify_run(run, CUSTOM_SELECTOR, CUSTOM_ASSERTION) == "unviable"
    run = mutations.RunResult(None, output, 1, timed_out=True)
    assert mutations.classify_run(run, CUSTOM_SELECTOR, CUSTOM_ASSERTION) == "timeout"


class MutationExecutionTests(unittest.TestCase):
    """Exercise restoration, baseline refusal and real command construction."""

    def setUp(self) -> None:
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        self.target = mutations.Mutation(
            id="example-guard",
            category="capability",
            path="crates/fyi-mcp/src/policy.rs",
            before="if !enabled {",
            after="if enabled {",
            package="fyi-mcp",
            test="policy::tests::read_enablement_does_not_enable_write",
        )
        self.path = self.root / self.target.path
        self.path.parent.mkdir(parents=True)
        self.path.write_text("fn check() { if !enabled { reject(); } }\n", encoding="utf-8")
        self.original = self.path.read_bytes()

    def execute(self, runner: Mock) -> dict:
        return mutations.execute_suite(
            self.root,
            [self.target],
            cargo="cargo",
            target_dir=self.root / "build-cache",
            timeout_seconds=30,
            total_seconds=60,
            log_dir=self.root / "logs",
            runner=runner,
        )

    def test_baseline_then_single_mutation_with_exact_offline_selector(self) -> None:
        def run(command: list[str], **_kwargs: object) -> mutations.RunResult:
            assert "--offline" in command
            assert "--locked" in command
            assert "--exact" in command
            assert "--test-threads=1" in command
            assert self.target.test in command
            assert command[command.index("--bin") + 1] == "fyi-mcp"
            if self.target.after in self.path.read_text(encoding="utf-8"):
                return result(self.target.test, "FAILED")
            return result(self.target.test, "ok")

        runner = Mock(side_effect=run)
        report = self.execute(runner)
        assert report["status"] == "passed"
        assert report["counts"] == {"caught": 1, "survived": 0, "unviable": 0, "timeout": 0}
        assert runner.call_count == 2
        assert self.path.read_bytes() == self.original
        assert len(report["mutations"][0]["output_sha256"]) == 64
        for entry in [*report["baselines"], *report["mutations"]]:
            contents = (self.root / entry["log_path"]).read_bytes()
            assert hashlib.sha256(contents).hexdigest() == entry["output_sha256"]
        assert "assertion failed" in (self.root / report["mutations"][0]["log_path"]).read_text()

    def test_failed_baseline_never_runs_a_mutant(self) -> None:
        runner = Mock(return_value=result(self.target.test, "FAILED"))
        report = self.execute(runner)
        assert report["status"] == "baseline_failed"
        assert report["mutations"] == []
        assert runner.call_count == 1
        assert self.path.read_bytes() == self.original

    def test_runner_requires_the_declared_custom_assertion(self) -> None:
        self.target = replace(
            self.target,
            test=CUSTOM_SELECTOR,
            expected_assertion_message=CUSTOM_ASSERTION,
        )
        for failure, expected in (
            (custom_assertion_result(), "caught"),
            (result(CUSTOM_SELECTOR, "FAILED"), "unviable"),
        ):
            with self.subTest(expected=expected):
                runner = Mock(side_effect=[result(CUSTOM_SELECTOR, "ok"), failure])
                report = self.execute(runner)
                assert report["mutations"][0]["outcome"] == expected
                assert report["mutations"][0]["expected_assertion_message"] == CUSTOM_ASSERTION
                assert self.path.read_bytes() == self.original

    def test_survivors_compile_failures_and_timeouts_fail_the_gate_and_restore_source(self) -> None:
        scenarios = (
            (result(self.target.test, "ok"), "survived"),
            (mutations.RunResult(101, "error: could not compile", 0.01), "unviable"),
            (mutations.RunResult(None, "", 30, timed_out=True), "timeout"),
        )
        for run, outcome in scenarios:
            with self.subTest(outcome=outcome):
                runner = Mock(side_effect=[result(self.target.test, "ok"), run])
                report = self.execute(runner)
                assert report["status"] == "failed"
                assert report["counts"][outcome] == 1
                assert self.path.read_bytes() == self.original

    def test_missing_or_ambiguous_source_prevents_all_execution(self) -> None:
        for content in ("changed source", self.original.decode() * 2):
            with self.subTest(content=content):
                self.path.write_text(content, encoding="utf-8")
                runner = Mock()
                with pytest.raises(mutations.HarnessError):
                    self.execute(runner)
                runner.assert_not_called()

    def test_source_restored_after_runner_error(self) -> None:
        runner = Mock(
            side_effect=[result(self.target.test, "ok"), OSError("synthetic spawn error")],
        )
        with pytest.raises(OSError, match="synthetic spawn error"):
            self.execute(runner)
        assert self.path.read_bytes() == self.original

    def test_lib_target_is_selected_only_for_core(self) -> None:
        command = mutations.test_command("cargo", "fyi-core", "sync::tests::example")
        assert "--lib" in command
        assert "--bin" not in command

    def test_overall_budget_exhaustion_refuses_to_start_a_process(self) -> None:
        runner = Mock()
        with patch.object(mutations.time, "monotonic", side_effect=[0, 61]):
            report = self.execute(runner)
        assert report["status"] == "baseline_failed"
        assert report["baselines"][0]["outcome"] == "timeout"
        runner.assert_not_called()
        assert self.path.read_bytes() == self.original


def test_process_timeout_and_output_bound(tmp_path: Path) -> None:
    timed_out = mutations.run_process(
        [sys.executable, "-c", "import time; time.sleep(30)"],
        cwd=tmp_path,
        timeout=0.05,
    )
    assert timed_out.timed_out
    assert timed_out.elapsed_seconds < 5
    with patch.object(mutations, "MAX_OUTPUT_BYTES", 64):
        oversized = mutations.run_process(
            [sys.executable, "-c", "print('x' * 1000)"],
            cwd=tmp_path,
            timeout=5,
        )
    assert oversized.output_limit_exceeded
    assert len(oversized.output) == 64
    assert mutations.classify_run(oversized, "example::test") == "unviable"


def test_output_overflow_stops_continuous_producer_before_timeout(tmp_path: Path) -> None:
    producer = (
        "import os\n"
        "from pathlib import Path\n"
        "Path('producer.pid').write_text(str(os.getpid()))\n"
        "while True:\n"
        "    os.write(1, b'x' * 65536)\n"
    )
    with patch.object(mutations, "MAX_OUTPUT_BYTES", 4096):
        output = mutations.run_process([sys.executable, "-c", producer], cwd=tmp_path, timeout=10)
    assert output.output_limit_exceeded
    assert not output.timed_out
    assert len(output.output) == 4096
    assert output.elapsed_seconds < 5
    if os.name == "posix":
        pid = int((tmp_path / "producer.pid").read_text())
        with pytest.raises(ProcessLookupError):
            os.kill(pid, 0)


def test_snapshot_uses_committed_head_and_leaves_original_worktree_untouched(
    tmp_path: Path,
) -> None:
    repository = tmp_path / "repository"
    repository.mkdir()
    for arguments in (
        ["init", "--quiet"],
        ["config", "user.email", "synthetic@example.invalid"],
        ["config", "user.name", "Synthetic Fixture"],
    ):
        process = mutations.run_process(["git", *arguments], cwd=repository, timeout=10)
        assert process.returncode == 0
    source = repository / "source.rs"
    source.write_text("committed synthetic source\n", encoding="utf-8")
    for arguments in (["add", "source.rs"], ["commit", "--quiet", "-m", "synthetic baseline"]):
        process = mutations.run_process(["git", *arguments], cwd=repository, timeout=10)
        assert process.returncode == 0
    head = mutations.run_process(
        ["git", "rev-parse", "HEAD"],
        cwd=repository,
        timeout=10,
    ).output.strip()
    provenance = mutations.manifest_provenance(repository, head, source, source.read_bytes())
    assert provenance["commit"] == head
    assert provenance["path"] == "source.rs"
    assert len(provenance["blob_sha"]) == 40
    source.write_text("uncommitted synthetic change\n", encoding="utf-8")
    with pytest.raises(mutations.HarnessError, match="differs from the exact committed"):
        mutations.manifest_provenance(repository, head, source, source.read_bytes())
    (repository / "untracked.txt").write_text("exclude this fixture\n", encoding="utf-8")
    with pytest.raises(mutations.HarnessError, match="differs from the exact committed"):
        mutations.manifest_provenance(repository, head, repository / "untracked.txt", b"untracked")
    with pytest.raises(mutations.HarnessError, match="must be committed inside"):
        mutations.manifest_provenance(repository, head, tmp_path / "external.json", b"external")
    snapshot = tmp_path / "snapshot"
    snapshot.mkdir()
    identity = mutations.snapshot_head(repository, snapshot)
    assert (snapshot / "source.rs").read_text(encoding="utf-8") == "committed synthetic source\n"
    assert not (snapshot / "untracked.txt").exists()
    assert not (snapshot / ".git").exists()
    assert source.read_text(encoding="utf-8") == "uncommitted synthetic change\n"
    assert len(identity["source_commit"]) == 40
    assert len(identity["source_tree"]) == 40
    assert len(identity["source_archive_sha256"]) == 64


if __name__ == "__main__":
    unittest.main()
