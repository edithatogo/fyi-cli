"""Verify declared Python/Rust CLI command-name parity against the source parsers.

This is a deliberately syntactic inventory, not a functional migration claim.
"""
from __future__ import annotations

import argparse
import json
from pathlib import Path
import re


def commands(repo: Path) -> dict[str, object]:
    python = (repo / "src/fyi_system/cli.py").read_text(encoding="utf-8")
    rust = (repo / "crates/fyi-cli/src/main.rs").read_text(encoding="utf-8")
    start = rust.index("pub enum Commands {")
    stop = rust.find("\n#[derive(", start + 1)
    top_level = rust[start:stop if stop >= 0 else None]
    py = sorted(set(re.findall(r"""\.add_parser\(\s*["']([^"']+)""", python)))
    variant_names = re.findall(r"^    ([A-Z][A-Za-z0-9]+)(?:\s*\{|,)", top_level, re.M)
    def kebab(name: str) -> str:
        return re.sub(r"([a-z0-9])([A-Z])", r"\1-\2", name).lower()
    rs = sorted(set(map(kebab, variant_names)))
    return {
        "schema_version": "1.0.0",
        "python_entrypoints": ["fyi", "fyi-cli", "fyi-system"],
        "python_commands": py,
        "rust_commands": rs,
        "shared_commands": sorted(set(py) & set(rs)),
        "python_only": sorted(set(py) - set(rs)),
        "rust_only": sorted(set(rs) - set(py)),
        "statement": "Command names only: behavioral equivalence not established; many Rust variants are placeholders.",
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", type=Path, default=Path(__file__).resolve().parents[1])
    args = parser.parse_args()
    expected = json.loads(
        (args.root / "conformance/cli/command-name-parity.json").read_text(encoding="utf-8")
    )
    observed = commands(args.root)
    if observed != expected:
        print(json.dumps({"error": "CLI parity manifest drift", "observed": observed}, indent=2))
        return 1
    print(
        "CLI command-name parity: OK; "
        f"Python {len(observed['python_commands'])}, "
        f"Rust {len(observed['rust_commands'])}, "
        f"shared {len(observed['shared_commands'])}. Behavioural parity not asserted."
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
