"""Verify the bounded, versioned synthetic Alaveteli fixture corpus offline.

This verifies fixture identity and its declared syntax/schema outcome. It does
not certify a live platform or establish that the production feed adapter rejects
malformed XML. Executable source coverage is recorded separately by the tests.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import logging
import re
import xml.etree.ElementTree as ET
from pathlib import Path, PurePosixPath
from urllib.parse import urlsplit

from jsonschema import Draft202012Validator
from jsonschema.exceptions import SchemaError
from referencing.exceptions import Unresolvable

ROOT = Path(__file__).resolve().parents[1]
FIXTURE_ROOT = ROOT / "tests/fixtures/api_contracts"
MAX_FILE_BYTES = 128 * 1024
MANIFEST_PATH = "v1/manifest.json"
PROVENANCE = "synthetic; not live-platform certification"
REQUIRED_CASES = frozenset(
    {
        "requests-wrapped",
        "requests-missing-title",
        "create-response",
        "http-401",
        "http-403",
        "http-404",
        "http-429",
        "http-503",
        "rss-links",
        "atom-links",
        "requests-array",
        "requests-results",
        "authorities-array",
        "authorities-wrapped",
        "truncated-json",
        "truncated-rss",
        "truncated-atom",
    },
)
JSON_CONTRACTS = frozenset(
    {
        "requests-wrapped",
        "requests-array",
        "requests-results",
        "create-response",
        "authorities-array",
        "authorities-wrapped",
        "http-error",
        "rate-limit",
    },
)
LOGGER = logging.getLogger(__name__)


class CorpusError(ValueError):
    """A private, actionable corpus-validation failure."""


def _fail(message: str) -> None:
    raise CorpusError(message)


def read_fixture(root: Path, relative: str) -> bytes:
    """Read a bounded regular file beneath the corpus, refusing symlink traversal."""
    if not isinstance(relative, str) or "\\" in relative or ":" in relative:
        _fail("fixture path is not a canonical relative path")
    parts = PurePosixPath(relative)
    if parts.is_absolute() or relative != parts.as_posix() or ".." in parts.parts:
        _fail("fixture path is not a canonical relative path")
    path = root
    for component in parts.parts:
        path = path / component
        if path.is_symlink():
            _fail("fixture path traverses a symlink")
    try:
        if not path.is_file():
            _fail("fixture file is missing or is not a regular file")
        with path.open("rb") as stream:
            data = stream.read(MAX_FILE_BYTES + 1)
    except OSError:
        _fail("fixture file cannot be read")
    if len(data) > MAX_FILE_BYTES:
        _fail("fixture file exceeds the corpus byte limit")
    return data


def _unique_object(pairs: list[tuple[str, object]]) -> dict[str, object]:
    result: dict[str, object] = {}
    for key, value in pairs:
        if key in result:
            _fail("fixture contains a duplicate JSON key")
        result[key] = value
    return result


def _reject_constant(_value: str) -> None:
    _fail("fixture contains a non-finite JSON number")


def _json(data: bytes) -> object:
    try:
        return json.loads(
            data.decode("utf-8"),
            object_pairs_hook=_unique_object,
            parse_constant=_reject_constant,
        )
    except (UnicodeError, ValueError, RecursionError):
        _fail("fixture is not strict UTF-8 JSON")


def _safe_schema(value: object) -> dict:
    if not isinstance(value, dict):
        _fail("corpus schema must be an object")
    pending = [value]
    while pending:
        node = pending.pop()
        if isinstance(node, dict):
            for key, item in node.items():
                if key in {"$id", "$dynamicRef", "$recursiveRef"}:
                    _fail("corpus schema contains an unsupported reference keyword")
                if key == "$ref" and (
                    not isinstance(item, str) or re.fullmatch(r"#/(?:\$defs)/[a-z-]+", item) is None
                ):
                    _fail("corpus schemas may reference only local definitions")
                pending.append(item)
        elif isinstance(node, list):
            pending.extend(node)
    try:
        Draft202012Validator.check_schema(value)
    except (SchemaError, RecursionError):
        _fail("corpus schema is invalid")
    return value


def _valid(value: object, schema: dict) -> bool:
    try:
        return Draft202012Validator(schema).is_valid(value)
    except (SchemaError, Unresolvable, RecursionError):
        _fail("corpus schema cannot be resolved locally")


def _feed_ids(root: ET.Element, contract: str) -> list[int]:
    if contract == "rss":
        if root.tag != "rss" or root.get("version") != "2.0":
            _fail("RSS fixture has an unexpected root or version")
        links = [element.text or "" for element in root.findall("./channel/item/link")]
    else:
        namespace = "{http://www.w3.org/2005/Atom}"
        if root.tag != f"{namespace}feed":
            _fail("Atom fixture has an unexpected root or namespace")
        links = [
            element.text or "" for element in root.findall(f"./{namespace}entry/{namespace}id")
        ]
        links.extend(
            element.get("href", "")
            for element in root.findall(f"./{namespace}entry/{namespace}link")
        )
    ids = set()
    for link in links:
        try:
            path = urlsplit(link).path
        except ValueError:
            _fail("feed fixture contains a malformed request URL")
        match = re.fullmatch(r"/requests?/([0-9]+)(?:/.*)?", path)
        if match is not None:
            ids.add(int(match[1]))
    return sorted(ids)


def _check_xml(case: dict, data: bytes) -> None:
    # Fixture input is byte-bounded and DTD/entity declarations are rejected
    # before ElementTree sees it; no external entity or document is fetched.
    try:
        text = data.decode("utf-8")
    except UnicodeError:
        _fail("feed fixture is not UTF-8 XML")
    if "<!DOCTYPE" in text.upper() or "<!ENTITY" in text.upper():
        _fail("XML corpus fixtures must not contain DTD or entity declarations")
    try:
        root = ET.fromstring(text)  # noqa: S314 - bounded synthetic input; DTD/entities denied above.
    except ET.ParseError:
        if case["expected"]["outcome"] == "invalid_xml":
            return
        _fail("feed fixture unexpectedly contains malformed XML")
    if case["expected"]["outcome"] != "accepted":
        _fail("malformed feed fixture unexpectedly parses successfully")
    if _feed_ids(root, case["contract"]) != case["expected"].get("ids"):
        _fail("feed fixture request IDs disagree with the versioned expectation")


def _record_ids(contract: str, value: object) -> list:
    if contract == "create-response":
        return [value["id"]]
    if contract.startswith("requests"):
        records = value if isinstance(value, list) else value.get("results", value.get("requests"))
        return [record["id"] for record in records]
    records = value if isinstance(value, list) else value["authorities"]
    return [record.get("id") for record in records]


def _check_json(case: dict, data: bytes, schemas: dict) -> None:
    expected = case["expected"]
    try:
        value = _json(data)
    except CorpusError:
        if expected["outcome"] == "invalid_json":
            return
        raise
    schema = {"$ref": f"#/$defs/{case['contract']}", "$defs": schemas["$defs"]}
    valid = _valid(value, schema)
    if expected["outcome"] == "invalid_schema":
        if valid:
            _fail("invalid-schema fixture unexpectedly satisfies its contract")
        return
    if not valid or expected["outcome"] == "invalid_json":
        _fail("JSON fixture disagrees with its declared schema outcome")
    if expected["outcome"] == "http_error":
        if value["status"] != expected["status"]:
            _fail("HTTP fixture status disagrees with its versioned expectation")
        return
    if _record_ids(case["contract"], value) != expected.get("ids"):
        _fail("JSON fixture IDs disagree with the versioned expectation")


def load_corpus(root: Path = FIXTURE_ROOT) -> dict:
    """Return a fully verified manifest, without executing any source operation."""
    manifest_schema = _safe_schema(_json(read_fixture(root, "v1/manifest.schema.json")))
    payload_schema = _safe_schema(_json(read_fixture(root, "v1/payloads.schema.json")))
    manifest = _json(read_fixture(root, MANIFEST_PATH))
    if not isinstance(manifest, dict) or type(manifest.get("schema_version")) is not int:
        _fail("corpus schema_version must be an exact integer")
    if not _valid(manifest, manifest_schema):
        _fail("corpus manifest violates its versioned schema")
    cases = manifest["cases"]
    if len(cases) != len(REQUIRED_CASES) or {case["id"] for case in cases} != REQUIRED_CASES:
        _fail("corpus case inventory is incomplete, duplicated or unsupported")
    if len({case["path"] for case in cases}) != len(cases):
        _fail("corpus fixture paths must be unique")
    if not JSON_CONTRACTS.issubset(payload_schema.get("$defs", {})):
        _fail("corpus payload schema definitions are incomplete")
    for case in cases:
        data = read_fixture(root, case["path"])
        if hashlib.sha256(data).hexdigest() != case["sha256"]:
            _fail("fixture SHA-256 differs from the reviewed versioned manifest")
        if case["contract"] in {"rss", "atom"}:
            _check_xml(case, data)
        else:
            _check_json(case, data, payload_schema)
    return manifest


def validate_corpus(root: Path = FIXTURE_ROOT) -> list[str]:
    """Return safe diagnostic messages; an empty list means the corpus is valid."""
    try:
        load_corpus(root)
    except (CorpusError, OSError, UnicodeError, RecursionError) as error:
        return [str(error) if isinstance(error, CorpusError) else "corpus could not be validated"]
    return []


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=FIXTURE_ROOT, help="fixture corpus directory")
    args = parser.parse_args()
    errors = validate_corpus(args.root)
    for error in errors:
        LOGGER.error("%s", error)
    if errors:
        return 1
    LOGGER.info(
        "Verified %d synthetic source fixtures; no live platform was contacted",
        len(REQUIRED_CASES),
    )
    return 0


if __name__ == "__main__":
    logging.basicConfig(level=logging.INFO, format="%(message)s")
    raise SystemExit(main())
