"""Run a bounded set of security mutations against an isolated git HEAD snapshot.

The default gate requires every versioned target to be caught by its exact Rust
unit test. Missing/stale targets, failed baselines, zero executed tests, compile
errors, timeouts and surviving mutations all fail. Cargo is locked and offline;
the selected tests use synthetic state and do not dispatch network requests.
This is a targeted regression gate, not a workspace mutation score.
"""

from __future__ import annotations

import argparse
import contextlib
import hashlib
import json
import os
import re
import shutil
import signal
import subprocess
import sys
import tarfile
import tempfile
import threading
import time
from dataclasses import dataclass
from pathlib import Path, PurePosixPath
from typing import TYPE_CHECKING, NoReturn

if TYPE_CHECKING:
    from collections.abc import Callable

ROOT = Path(__file__).resolve().parents[1]
MANIFEST = ROOT / "conformance/mcp/security-mutations-v1.json"
REQUIRED_CATEGORIES = frozenset(
    {
        "capability",
        "confirmation_scope",
        "confirmation_expiry",
        "replay",
        "ssrf",
        "guardrail",
        "error_classification",
        "redaction",
    },
)
MAX_OUTPUT_BYTES = 8 * 1024 * 1024
MAX_SNAPSHOT_BYTES = 128 * 1024 * 1024


class HarnessError(ValueError):
    """A fail-closed manifest, snapshot or execution precondition failure."""


def fail(message: str) -> NoReturn:
    """Stop before an incomplete or invalid run can be accepted."""
    raise HarnessError(message)


@dataclass(frozen=True)
class Mutation:
    """A single exact source edit and the existing test that must detect it."""

    id: str
    category: str
    path: str
    before: str
    after: str
    package: str
    test: str
    expected_assertion_message: str | None = None


@dataclass(frozen=True)
class RunResult:
    """Bounded process output and execution facts, before classification."""

    returncode: int | None
    output: str
    elapsed_seconds: float
    timed_out: bool = False
    output_limit_exceeded: bool = False


def unique_object(pairs: list[tuple[str, object]]) -> dict[str, object]:
    """Reject duplicate JSON keys instead of accepting the final occurrence."""
    value: dict[str, object] = {}
    for key, item in pairs:
        if key in value:
            fail("duplicate manifest JSON key")
        value[key] = item
    return value


def parse_manifest(value: object) -> list[Mutation]:
    """Validate the complete target catalogue without interpreting shell text."""
    if not isinstance(value, dict) or set(value) != {"schema_version", "mutations"}:
        fail("invalid mutation manifest envelope")
    if type(value["schema_version"]) is not int or value["schema_version"] != 1:
        fail("unsupported mutation manifest version")
    rows = value["mutations"]
    if not isinstance(rows, list) or not 1 <= len(rows) <= 32:
        fail("mutation catalogue must contain 1..32 targets")
    targets = [_parse_target(row) for row in rows]
    if len({target.id for target in targets}) != len(targets):
        fail("duplicate mutation identifier")
    if {target.category for target in targets} != REQUIRED_CATEGORIES:
        fail("mutation catalogue omits a required security boundary")
    return targets


def _parse_target(row: object) -> Mutation:
    fields = set(Mutation.__dataclass_fields__)
    required = fields - {"expected_assertion_message"}
    if not isinstance(row, dict) or not required <= set(row) <= fields:
        fail("invalid mutation target fields")
    if any(not isinstance(item, str) or not item for item in row.values()):
        fail("mutation fields must be nonempty strings")
    if not re.fullmatch(r"[a-z][a-z0-9-]{2,79}", row["id"]):
        fail("invalid mutation identifier")
    if row["category"] not in REQUIRED_CATEGORIES or row["package"] not in {"fyi-core", "fyi-mcp"}:
        fail("unknown mutation category or package")
    source = PurePosixPath(row["path"])
    if (
        source.is_absolute()
        or ".." in source.parts
        or "\\" in row["path"]
        or row["path"] != source.as_posix()
        or source.suffix != ".rs"
        or source.parts[:3] not in {("crates", "fyi-core", "src"), ("crates", "fyi-mcp", "src")}
    ):
        fail("mutation path is outside the supported Rust source tree")
    if not re.fullmatch(r"[a-zA-Z_][a-zA-Z_0-9]*(?:::[a-zA-Z_][a-zA-Z_0-9]*)+", row["test"]):
        fail("mutation requires an exact Rust test selector")
    if row["before"] == row["after"] or max(len(row["before"]), len(row["after"])) > 8192:
        fail("mutation is a no-op or exceeds the source-edit bound")
    message = row.get("expected_assertion_message")
    if message is not None and (
        len(message) > 256 or not message.isprintable() or message != message.strip()
    ):
        fail("custom assertion message must be a bounded, unpadded, printable line")
    return Mutation(**row)


def validate_sources(root: Path, targets: list[Mutation]) -> None:
    """Refuse target drift and ambiguous edits before executing any tests."""
    for target in targets:
        path = root / target.path
        if (
            path.is_symlink()
            or not path.is_file()
            or not path.resolve().is_relative_to(root.resolve())
        ):
            fail(f"missing or unsafe mutation source: {target.id}")
        if path.read_text(encoding="utf-8").count(target.before) != 1:
            fail(f"mutation source drift or ambiguity: {target.id}")


def test_command(cargo: str, package: str, selector: str) -> list[str]:
    """Use one exact offline test; fyi-mcp is a binary crate, not a library."""
    target = ["--bin", "fyi-mcp"] if package == "fyi-mcp" else ["--lib"]
    return [
        cargo,
        "test",
        "--offline",
        "--locked",
        "-p",
        package,
        *target,
        selector,
        "--",
        "--exact",
        "--test-threads=1",
        "--color=never",
    ]


def classify_run(
    run: RunResult,
    selector: str,
    expected_assertion_message: str | None = None,
) -> str:
    """Count only the intended test's executed assertion/expectation failure."""
    if run.timed_out:
        return "timeout"
    if run.output_limit_exceeded:
        return "unviable"
    output = run.output.replace("\r\n", "\n")
    tests = re.findall(r"^test ([\w:]+) \.\.\. (ok|FAILED|ignored)$", output, re.MULTILINE)
    summaries = re.findall(
        r"^test result: (ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored; "
        r"(\d+) measured; \d+ filtered out; finished in",
        output,
        re.MULTILINE,
    )
    if len(summaries) != 1 or len(tests) != 1 or tests[0][0] != selector:
        return "unviable"
    if "could not compile" in output:
        return "unviable"
    if run.returncode == 0 and tests[0][1] == "ok" and summaries == [("ok", "1", "0", "0", "0")]:
        return "passed"
    if (
        run.returncode != 101
        or tests[0][1] != "FAILED"
        or summaries != [("FAILED", "0", "1", "0", "0")]
    ):
        return "unviable"
    messages = re.findall(
        rf"^thread '{re.escape(selector)}'(?: \(\d+\))? panicked at [^\n]*:\n([^\n]*)",
        output,
        re.MULTILINE,
    )
    if len(messages) != 1:
        return "unviable"
    if expected_assertion_message is not None:
        # An explicit message also prevents a different earlier assertion in
        # the same test from satisfying this target's named security boundary.
        return "caught" if messages[0] == expected_assertion_message else "unviable"
    if re.match(
        r"assertion (?:failed|`[^\n]+` failed)|called `(?:Result|Option)::(?:unwrap|expect)[^\n]*`",
        messages[0],
    ):
        return "caught"
    return "unviable"


def _stop_process_tree(process: subprocess.Popen) -> None:
    if os.name == "posix":
        with contextlib.suppress(ProcessLookupError):
            os.killpg(process.pid, signal.SIGKILL)
    else:
        # Windows taskkill terminates Cargo's compiler/test descendants too.
        taskkill = shutil.which("taskkill")
        if taskkill:
            subprocess.run(  # noqa: S603 -- fixed arguments, no shell
                [taskkill, "/PID", str(process.pid), "/T", "/F"],
                check=False,
                capture_output=True,
                timeout=10,
            )
        process.kill()
    process.wait(timeout=10)


def run_process(
    command: list[str],
    *,
    cwd: Path,
    timeout: float,
    env: dict[str, str] | None = None,
) -> RunResult:
    """Cancel immediately on timeout/output overflow and reap Cargo's children."""
    started = time.monotonic()
    deadline = started + timeout
    timed_out = False
    output = bytearray()
    output_limit = threading.Event()
    with subprocess.Popen(  # noqa: S603 -- argument vectors only, no shell
        command,
        cwd=cwd,
        env=env,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        start_new_session=os.name == "posix",
    ) as process:
        stream = process.stdout
        if stream is None:
            fail("subprocess output pipe is unavailable")

        def read_output() -> None:
            while chunk := stream.read1(64 * 1024):
                available = MAX_OUTPUT_BYTES - len(output)
                output.extend(chunk[:available])
                if len(chunk) > available:
                    output_limit.set()
                    return

        reader = threading.Thread(target=read_output, daemon=True)
        reader.start()
        try:
            while process.poll() is None and not output_limit.is_set():
                remaining = deadline - time.monotonic()
                if remaining <= 0:
                    timed_out = True
                    break
                with contextlib.suppress(subprocess.TimeoutExpired):
                    process.wait(timeout=min(0.05, remaining))
            while reader.is_alive() and not output_limit.is_set() and not timed_out:
                remaining = deadline - time.monotonic()
                if remaining <= 0:
                    timed_out = True
                    break
                reader.join(timeout=min(0.05, remaining))
        finally:
            if timed_out or output_limit.is_set() or process.poll() is None or reader.is_alive():
                _stop_process_tree(process)
            reader.join(timeout=10)
    return RunResult(
        process.returncode,
        output.decode("utf-8", errors="replace"),
        time.monotonic() - started,
        timed_out,
        output_limit.is_set(),
    )


def _execution_record(
    run: RunResult,
    outcome: str,
    command: list[str],
    log_dir: Path | None,
    log_name: str,
) -> dict:
    contents = run.output.encode()
    record = {
        "outcome": outcome,
        "command": command,
        "returncode": run.returncode,
        "elapsed_seconds": round(run.elapsed_seconds, 3),
        "output_sha256": hashlib.sha256(contents).hexdigest(),
        "timed_out": run.timed_out,
        "output_limit_exceeded": run.output_limit_exceeded,
    }
    if log_dir is not None:
        log_dir.mkdir(parents=True, exist_ok=True)
        (log_dir / log_name).write_bytes(contents)
        record["log_path"] = (Path(log_dir.name) / log_name).as_posix()
    return record


def execute_suite(
    snapshot: Path,
    targets: list[Mutation],
    *,
    cargo: str,
    target_dir: Path,
    timeout_seconds: int,
    total_seconds: int,
    log_dir: Path | None = None,
    runner: Callable[..., RunResult] = run_process,
) -> dict:
    """Require all baselines first, then restore source after every mutation."""
    validate_sources(snapshot, targets)
    environment = dict(os.environ)
    environment.update(
        {
            "CARGO_NET_OFFLINE": "true",
            "CARGO_TARGET_DIR": str(target_dir),
            "CARGO_BUILD_JOBS": "1",
            "CARGO_TERM_COLOR": "never",
            "RUST_BACKTRACE": "0",
        },
    )
    deadline = time.monotonic() + total_seconds
    report = {
        "status": "running",
        "baselines": [],
        "mutations": [],
        "counts": {"caught": 0, "survived": 0, "unviable": 0, "timeout": 0},
    }

    def invoke(package: str, test: str) -> tuple[RunResult, list[str]]:
        command = test_command(cargo, package, test)
        remaining = deadline - time.monotonic()
        if remaining <= 0:
            return RunResult(None, "overall execution budget exhausted", 0, timed_out=True), command
        return runner(
            command,
            cwd=snapshot,
            timeout=min(timeout_seconds, remaining),
            env=environment,
        ), command

    baselines = dict.fromkeys((target.package, target.test) for target in targets)
    for index, (package, test) in enumerate(baselines, start=1):
        run, command = invoke(package, test)
        outcome = classify_run(run, test)
        report["baselines"].append(
            _execution_record(
                run,
                outcome,
                command,
                log_dir,
                f"baseline-{index:02d}-{package}.log",
            ),
        )
        if outcome != "passed":
            report["status"] = "baseline_failed"
            return report

    for target in targets:
        path = snapshot / target.path
        original = path.read_text(encoding="utf-8")
        try:
            path.write_text(original.replace(target.before, target.after, 1), encoding="utf-8")
            run, command = invoke(target.package, target.test)
        finally:
            path.write_text(original, encoding="utf-8")
        outcome = classify_run(run, target.test, target.expected_assertion_message)
        if outcome == "passed":
            outcome = "survived"
        report["counts"][outcome] += 1
        report["mutations"].append(
            {
                "id": target.id,
                "category": target.category,
                "path": target.path,
                "expected_assertion_message": target.expected_assertion_message,
                "source_sha256": hashlib.sha256(original.encode()).hexdigest(),
                **_execution_record(run, outcome, command, log_dir, f"mutation-{target.id}.log"),
            },
        )
    report["status"] = "passed" if report["counts"]["caught"] == len(targets) else "failed"
    return report


def _git(root: Path, *arguments: str) -> str:
    result = run_process(["git", "-C", str(root), *arguments], cwd=root, timeout=60)
    if result.returncode or result.timed_out or result.output_limit_exceeded:
        fail("git snapshot operation failed")
    return result.output.strip()


def snapshot_head(root: Path, destination: Path) -> dict[str, str]:
    """Export exactly HEAD without hooks, untracked files or worktree mutation."""
    head = _git(root, "rev-parse", "HEAD")
    tree = _git(root, "rev-parse", f"{head}^{{tree}}")
    archive_path = destination.parent / "snapshot.tar"
    _git(root, "archive", "--format=tar", f"--output={archive_path}", head)
    if archive_path.stat().st_size > MAX_SNAPSHOT_BYTES * 2:
        fail("source archive exceeds its bounded size")
    with archive_path.open("rb") as archive_file:
        archive_hash = hashlib.file_digest(archive_file, "sha256").hexdigest()
    total_bytes = 0
    with tarfile.open(archive_path, "r:") as archive:
        for member in archive:
            path = PurePosixPath(member.name)
            total_bytes += member.size
            if (
                path.is_absolute()
                or ".." in path.parts
                or "\\" in member.name
                or not (member.isfile() or member.isdir())
                or total_bytes > MAX_SNAPSHOT_BYTES
            ):
                fail("unsafe or oversized source archive")
            target = destination.joinpath(*path.parts)
            if member.isdir():
                target.mkdir(parents=True, exist_ok=True)
            else:
                target.parent.mkdir(parents=True, exist_ok=True)
                extracted = archive.extractfile(member)
                if extracted is None:
                    fail("missing source archive entry")
                with extracted, target.open("wb") as output:
                    shutil.copyfileobj(extracted, output)
                target.chmod(member.mode & 0o777)
    archive_path.unlink()
    return {"source_commit": head, "source_tree": tree, "source_archive_sha256": archive_hash}


def manifest_provenance(root: Path, head: str, manifest: Path, contents: bytes) -> dict[str, str]:
    """Require manifest evidence to belong to the exact committed source snapshot."""
    if not manifest.resolve().is_relative_to(root.resolve()):
        fail("mutation manifest must be committed inside the selected repository")
    relative = manifest.resolve().relative_to(root.resolve()).as_posix()
    blob = run_process(
        ["git", "-C", str(root), "show", f"{head}:{relative}"],
        cwd=root,
        timeout=30,
    )
    if (
        blob.returncode
        or blob.timed_out
        or blob.output_limit_exceeded
        or blob.output.encode() != contents
    ):
        fail("mutation manifest differs from the exact committed source snapshot")
    return {
        "kind": "committed_blob",
        "commit": head,
        "path": relative,
        "blob_sha": _git(root, "rev-parse", f"{head}:{relative}"),
    }


def _arguments() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=ROOT)
    parser.add_argument("--manifest", type=Path, default=MANIFEST)
    parser.add_argument("--cargo", default="cargo")
    parser.add_argument(
        "--report",
        type=Path,
        default=ROOT / "target/mcp-security-mutations/report.json",
    )
    parser.add_argument(
        "--target-dir",
        type=Path,
        default=ROOT / "target/mcp-security-mutations/build",
    )
    parser.add_argument("--timeout-seconds", type=int, default=300)
    parser.add_argument("--total-seconds", type=int, default=1800)
    parser.add_argument(
        "--only",
        action="append",
        default=[],
        help="diagnostic subset; never full acceptance",
    )
    parser.add_argument(
        "--list",
        action="store_true",
        help="validate/list targets without executing them",
    )
    return parser.parse_args()


def main() -> int:
    """Emit explicit, reproducible evidence and a failing status on any gap."""
    args = _arguments()
    report: dict = {
        "schema_version": 1,
        "scope": "targeted_security_mutations",
        "status": "blocked",
    }
    try:
        if not 1 <= args.timeout_seconds <= 600 or not 1 <= args.total_seconds <= 7200:
            fail("execution budgets must be bounded positive seconds")
        manifest_bytes = args.manifest.read_bytes()
        targets = parse_manifest(json.loads(manifest_bytes, object_pairs_hook=unique_object))
        report["manifest_sha256"] = hashlib.sha256(manifest_bytes).hexdigest()
        report["complete_manifest"] = not args.only
        if args.only:
            if not set(args.only).issubset({target.id for target in targets}):
                fail("unknown mutation requested")
            targets = [target for target in targets if target.id in args.only]
        if args.list:
            validate_sources(args.root, targets)
            sys.stdout.write(
                json.dumps(
                    {"validated_targets": [target.id for target in targets], "executed": False},
                    indent=2,
                )
                + "\n",
            )
            return 0
        cargo = shutil.which(args.cargo)
        if cargo is None:
            fail("Cargo is unavailable")
        with tempfile.TemporaryDirectory(prefix="fyi-mcp-mutations-") as temporary:
            snapshot = Path(temporary) / "source"
            snapshot.mkdir()
            report.update(snapshot_head(args.root.resolve(), snapshot))
            report["manifest_provenance"] = manifest_provenance(
                args.root.resolve(),
                report["source_commit"],
                args.manifest,
                manifest_bytes,
            )
            version = run_process([cargo, "--version"], cwd=snapshot, timeout=30)
            rustc = shutil.which(
                "rustc",
                path=str(Path(cargo).parent) + os.pathsep + os.environ.get("PATH", ""),
            )
            if rustc is None:
                fail("rustc is unavailable")
            compiler = run_process([rustc, "--version", "--verbose"], cwd=snapshot, timeout=30)
            if version.returncode or compiler.returncode or version.timed_out or compiler.timed_out:
                fail("cannot record the Rust toolchain")
            report.update(
                {
                    "cargo_version": version.output.strip(),
                    "rustc_version": compiler.output.strip(),
                    "timeout_seconds": args.timeout_seconds,
                    "total_seconds": args.total_seconds,
                    "parallel_jobs": 1,
                    "cargo_offline": True,
                    "cargo_locked": True,
                },
            )
            args.report.parent.mkdir(parents=True, exist_ok=True)
            log_dir = Path(
                tempfile.mkdtemp(prefix="mcp-security-mutation-logs-", dir=args.report.parent),
            )
            report["log_directory"] = log_dir.name
            report.update(
                execute_suite(
                    snapshot,
                    targets,
                    cargo=cargo,
                    target_dir=args.target_dir.resolve(),
                    timeout_seconds=args.timeout_seconds,
                    total_seconds=args.total_seconds,
                    log_dir=log_dir,
                ),
            )
        if args.only and report["status"] == "passed":
            report["status"] = "selected_passed"
    except (OSError, HarnessError, json.JSONDecodeError, tarfile.TarError) as error:
        report["status"] = "blocked"
        report["error"] = str(error) if isinstance(error, HarnessError) else type(error).__name__
    args.report.parent.mkdir(parents=True, exist_ok=True)
    args.report.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    sys.stdout.write(json.dumps(report, indent=2) + "\n")
    return 0 if report["status"] in {"passed", "selected_passed"} else 1


if __name__ == "__main__":
    raise SystemExit(main())
