import json

import pytest

from fyi_system.process_events import (
    export_process_events,
    validate_process_event_file,
    validate_public_event,
)


def _write(root, request_id, events):
    path = root / "Agency" / str(request_id)
    path.mkdir(parents=True, exist_ok=True)
    path.joinpath("request.json").write_text(
        json.dumps({"id": request_id, "title": "private", "info_request_events": events}),
        encoding="utf-8",
    )


def test_export_preserves_source_order_and_excludes_content(tmp_path):
    derived = tmp_path / "derived"
    _write(
        derived,
        2,
        [
            {"event_type": "closed", "created_at": "2026-01-02T00:00:00Z"},
            {"event_type": "opened", "created_at": "2026-01-01T00:00:00Z", "body": "secret"},
        ],
    )
    output = tmp_path / "events.ndjson"
    result = export_process_events(
        derived_dir=derived,
        output=output,
        captured_at="2026-02-01T00:00:00Z",
    )
    rows = [json.loads(line) for line in output.read_text().splitlines()]
    assert result["event_count"] == 2
    assert [row["activity"] for row in rows] == ["closed", "opened"]
    assert [row["source_order"]["event_sequence"] for row in rows] == [0, 1]
    assert all("body" not in row and "title" not in row for row in rows)
    for row in rows:
        validate_public_event(row)


def test_checkpoint_makes_repeated_export_empty(tmp_path):
    derived = tmp_path / "derived"
    _write(derived, 1, [{"event_type": "opened"}])
    checkpoint = tmp_path / "checkpoint.json"
    first = export_process_events(
        derived_dir=derived,
        output=tmp_path / "one.ndjson",
        captured_at="2026-02-01T00:00:00Z",
        checkpoint=checkpoint,
    )
    second = export_process_events(
        derived_dir=derived,
        output=tmp_path / "two.ndjson",
        captured_at="2026-02-01T00:00:00Z",
        checkpoint=checkpoint,
    )
    assert first["event_count"] == 1
    assert second["event_count"] == 0
    assert second["total_event_count"] == 1


def test_changed_activity_keeps_logical_event_id_and_increments_revision(tmp_path):
    derived = tmp_path / "derived"
    _write(derived, 1, [{"id": "evt-1", "event_type": "opened"}])
    checkpoint = tmp_path / "checkpoint.json"
    first_output = tmp_path / "one.ndjson"
    export_process_events(
        derived_dir=derived,
        output=first_output,
        captured_at="2026-02-01T00:00:00Z",
        checkpoint=checkpoint,
    )
    _write(derived, 1, [{"id": "evt-1", "event_type": "closed"}])
    second_output = tmp_path / "two.ndjson"
    export_process_events(
        derived_dir=derived,
        output=second_output,
        captured_at="2026-02-02T00:00:00Z",
        checkpoint=checkpoint,
    )
    first = json.loads(first_output.read_text().splitlines()[0])
    second = json.loads(second_output.read_text().splitlines()[0])
    assert second["event_id"] == first["event_id"]
    assert second["revision"] == 2
    assert second["activity"] == "closed"


def test_deleted_event_is_tombstoned_once(tmp_path):
    derived = tmp_path / "derived"
    _write(derived, 1, [{"id": "evt-1", "event_type": "opened"}])
    checkpoint = tmp_path / "checkpoint.json"
    export_process_events(
        derived_dir=derived,
        output=tmp_path / "one.ndjson",
        captured_at="2026-02-01T00:00:00Z",
        checkpoint=checkpoint,
    )
    (derived / "Agency" / "1" / "request.json").write_text(json.dumps({"id": 1}), encoding="utf-8")
    deleted_output = tmp_path / "deleted.ndjson"
    export_process_events(
        derived_dir=derived,
        output=deleted_output,
        captured_at="2026-02-02T00:00:00Z",
        checkpoint=checkpoint,
    )
    repeat_output = tmp_path / "repeat.ndjson"
    export_process_events(
        derived_dir=derived,
        output=repeat_output,
        captured_at="2026-02-03T00:00:00Z",
        checkpoint=checkpoint,
    )
    tombstone = json.loads(deleted_output.read_text().splitlines()[0])
    assert tombstone["operation"] == "delete"
    assert repeat_output.read_text() == ""


def test_attachment_projection_contains_metadata_only(tmp_path):
    derived = tmp_path / "derived"
    path = derived / "Agency" / "1"
    path.mkdir(parents=True)
    path.joinpath("request.json").write_text(
        json.dumps(
            {
                "id": 1,
                "files": [
                    {
                        "name": "private-name.pdf",
                        "url": "https://fyi.example/attach/1",
                        "content_type": "application/pdf",
                        "size": 12,
                    },
                ],
            },
        ),
        encoding="utf-8",
    )
    attachment_output = tmp_path / "attachments.ndjson"
    export_process_events(
        derived_dir=derived,
        output=tmp_path / "events.ndjson",
        attachments_output=attachment_output,
        captured_at="2026-02-01T00:00:00Z",
    )
    row = json.loads(attachment_output.read_text().splitlines()[0])
    assert row["byte_size"] == 12
    assert "name" not in row
    assert "private-name.pdf" not in attachment_output.read_text()


def test_public_event_validation_is_recursive():
    with pytest.raises(ValueError, match="excluded fields"):
        validate_public_event({"schema_version": "1.0.0", "provenance": {"body": "secret"}})


def test_missing_timestamp_is_explicit_and_state_is_preserved(tmp_path):
    derived = tmp_path / "derived"
    _write(
        derived,
        1,
        [{"id": "evt-1", "event_type": "opened", "state": "open", "message_id": "m-1"}],
    )
    output = tmp_path / "events.ndjson"
    export_process_events(
        derived_dir=derived,
        output=output,
        captured_at="2026-02-01T00:00:00Z",
    )
    row = json.loads(output.read_text().splitlines()[0])
    assert row["timestamp"] is None
    assert row["timestamp_status"] == "missing"
    assert row["state"] == "open"
    assert row["provenance"]["message_reference_id"] == "m-1"


def test_exported_file_validator_counts_rows_and_rejects_wrong_version(tmp_path):
    path = tmp_path / "events.ndjson"
    path.write_text(
        json.dumps(
            {
                "schema_version": "1.0.0",
                "event_id": "e1",
                "logical_request_id": "r1",
                "activity": "opened",
                "timestamp": "2026-01-01T00:00:00Z",
                "source_order": {},
                "provenance": {},
            },
        )
        + "\n",
        encoding="utf-8",
    )
    assert validate_process_event_file(path) == {"row_count": 1}
    path.write_text('{"schema_version":"0.0.0"}\n', encoding="utf-8")
    with pytest.raises(ValueError, match="unsupported schema version"):
        validate_process_event_file(path)
