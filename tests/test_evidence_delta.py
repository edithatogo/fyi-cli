"""Tests for the experimental fyi-process EvidenceDelta emitter."""

import json

from fyi_system.cli import build_parser
from fyi_system.evidence_delta import emit_evidence_deltas


def write_request(root, request_id, **extra):
    path = root / "Agency" / str(request_id)
    path.mkdir(parents=True, exist_ok=True)
    path.joinpath("request.json").write_text(
        json.dumps({"id": request_id, "url_title": f"request-{request_id}", **extra}),
        encoding="utf-8",
    )


def test_emitter_requires_explicit_experimental_flag():
    args = build_parser().parse_args(
        [
            "emit-evidence-delta",
            "--output",
            "out.ndjson",
            "--captured-at",
            "2026-07-16T00:00:00Z",
        ],
    )
    assert args.experimental is False


def test_emitter_produces_revision_aware_deltas_and_skips_unchanged(tmp_path):
    derived = tmp_path / "derived"
    output = tmp_path / "out.ndjson"
    write_request(derived, 1, title="changed", content_sha256="a" * 64)
    write_request(derived, 3, title="new", content_sha256="b" * 64)
    previous = tmp_path / "previous.json"
    previous.write_text(
        json.dumps(
            {
                "requests": [
                    {"request_id": 1, "content_sha256": "c" * 64, "revision": 2},
                    {"request_id": 2, "content_sha256": "d" * 64, "revision": 1},
                ],
            },
        ),
        encoding="utf-8",
    )

    deltas = emit_evidence_deltas(
        derived_dir=derived,
        output=output,
        captured_at="2026-07-16T00:00:00Z",
        previous_manifest=previous,
    )

    assert [row["operation"] for row in deltas] == ["upsert", "upsert", "delete"]
    assert [row["revision"] for row in deltas] == [3, 1, 2]
    assert [row["position"]["sequence"] for row in deltas] == [1, 2, 3]
    assert all(row["evidence"]["privacy"]["disposition"] == "needs_review" for row in deltas[:2])
    assert len(output.read_text(encoding="utf-8").splitlines()) == 3
