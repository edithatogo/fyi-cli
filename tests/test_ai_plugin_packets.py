from pathlib import Path

from scripts.validate_ai_plugin_packets import validate, validate_packet


ROOT = Path(__file__).parents[1]


def test_ai_plugin_packets_are_valid():
    assert validate(ROOT) == []


def test_packets_cannot_enable_remote_writes():
    packet = {
        "schema_version": 1,
        "target": "openai-codex-plugins",
        "product": "fyi-mcp",
        "repository": "https://github.com/edithatogo/fyi-cli",
        "release_source": "https://github.com/edithatogo/fyi-cli/releases",
        "submission_route": "https://help.openai.com/en/articles/20001256-plugins-in-codex",
        "capabilities": {"remote_request_submission": False, "remote_authority_writes": True},
        "review_checklist": ["a", "b", "c", "d"],
        "rollback": "remove listing",
    }
    errors = validate_packet(packet, "openai-codex-plugins", Path("codex/submission.json"))
    assert any("remote_authority_writes must remain false" in error for error in errors)
