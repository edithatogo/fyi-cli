#!/usr/bin/env python3
"""Export live fyi-mcp tools/resources/prompts into an MCPB-style manifest.json."""

from __future__ import annotations

import json
import os
import subprocess
import sys
import time
from pathlib import Path


def rpc(method: str, params: dict | None = None, req_id: int = 1) -> dict:
    msg: dict = {"jsonrpc": "2.0", "id": req_id, "method": method}
    if params is not None:
        msg["params"] = params
    return msg


def main() -> int:
    root = Path(__file__).resolve().parents[1]
    exe = root / "target" / "release" / "fyi-mcp.exe"
    if not exe.exists():
        exe = root / "target" / "x86_64-pc-windows-gnu" / "release" / "fyi-mcp.exe"
    if not exe.exists():
        print(f"fyi-mcp binary not found at {exe}", file=sys.stderr)
        return 1

    env = os.environ.copy()
    env["FYI_MCP_EPHEMERAL"] = "1"
    env["DATABASE_URL"] = "sqlite::memory:"

    proc = subprocess.Popen(
        [str(exe)],
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        env=env,
        bufsize=1,
    )
    assert proc.stdin and proc.stdout

    def call(method: str, params: dict | None = None, req_id: int = 1) -> dict:
        line = json.dumps(rpc(method, params, req_id))
        proc.stdin.write(line + "\n")
        proc.stdin.flush()
        # Read until we get a response with matching id
        deadline = time.time() + 15
        while time.time() < deadline:
            out = proc.stdout.readline()
            if not out:
                break
            out = out.strip()
            if not out:
                continue
            try:
                data = json.loads(out)
            except json.JSONDecodeError:
                continue
            if data.get("id") == req_id:
                return data
        raise RuntimeError(f"No response for {method}")

    try:
        init = call("initialize", {"protocolVersion": "2024-11-05", "capabilities": {}, "clientInfo": {"name": "export", "version": "0"}}, 1)
        tools = call("tools/list", None, 2)
        resources = call("resources/list", None, 3)
        prompts = call("prompts/list", None, 4)
    finally:
        proc.terminate()
        try:
            proc.wait(timeout=3)
        except subprocess.TimeoutExpired:
            proc.kill()

    init_result = init.get("result") or {}
    tools_list = (tools.get("result") or {}).get("tools") or []
    resources_list = (resources.get("result") or {}).get("resources") or []
    prompts_list = (prompts.get("result") or {}).get("prompts") or []

    manifest = {
        "manifest_version": "0.4",
        "name": "fyi-mcp",
        "display_name": "FYI MCP",
        "version": "0.1.2",
        "description": (
            "Local-first MCP server for Freedom of Information / Official Information "
            "request tracking against Alaveteli platforms (FYI.org.nz and multi-jurisdiction catalog)."
        ),
        "long_description": (
            "Native Rust MCP server for managing FOI/OIA requests locally: request lifecycle, "
            "authority import, correspondence, offline sync monitoring, conflict resolution, "
            "statutory deadlines, hybrid search demo, and corpus resources (fyi://)."
        ),
        "author": {"name": "edithatogo"},
        "repository": {"type": "git", "url": "https://github.com/edithatogo/fyi-cli"},
        "homepage": "https://github.com/edithatogo/fyi-cli",
        "license": "MIT",
        "keywords": ["fyi", "mcp", "alaveteli", "foi", "oia", "privacy", "sync"],
        "server": {
            "type": "binary",
            "entry_point": "server/fyi-mcp.exe",
            "mcp_config": {
                "command": "server/fyi-mcp.exe",
                "args": [],
                "env": {},
            },
        },
        "tools": tools_list,
        "resources": resources_list,
        "prompts": prompts_list,
        "instructions": init_result.get("instructions"),
        "serverInfo": init_result.get("serverInfo"),
        "capabilities": init_result.get("capabilities"),
        "tools_generated": True,
        "compatibility": {"platforms": ["win32"]},
    }

    out_path = root / "packaging" / "mcpb" / "fyi-mcp" / "manifest.json"
    out_path.parent.mkdir(parents=True, exist_ok=True)
    out_path.write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")
    print(f"Wrote {out_path}")
    print(f"tools={len(tools_list)} resources={len(resources_list)} prompts={len(prompts_list)}")
    print(f"instructions={'yes' if manifest.get('instructions') else 'no'}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
