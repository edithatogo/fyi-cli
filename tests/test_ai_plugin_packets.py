from pathlib import Path

from scripts.validate_ai_plugin_packets import (
    readme_status_line,
    validate,
    validate_packet,
    validate_readme,
)


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


def test_readme_status_must_match_ledger():
    errors = validate_readme(
        "Current repo-side status: `planned`.\n",
        "assets-ready",
        Path("packaging/ai-plugins/codex/README.md"),
    )
    assert any("missing status line" in error for error in errors)
    assert any("legacy planned-status wording must be removed" in error for error in errors)


def test_readme_status_accepts_matching_ledger_state():
    assert (
        validate_readme(
            f"{readme_status_line('assets-ready')}\n",
            "assets-ready",
            Path("packaging/ai-plugins/codex/README.md"),
        )
        == []
    )
