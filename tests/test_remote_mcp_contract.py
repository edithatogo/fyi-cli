"""Offline contract and security invariants for the remote MCP surface."""

from scripts.verify_remote_mcp_contract import validate_contract


def test_remote_mcp_v1_contract_is_valid() -> None:
    assert validate_contract() == []
