"""Synchronize fyi-cli GitHub Project items into the RIOPA umbrella project.

GitHub Projects do not support nested projects or native project-to-project
mirroring. This script keeps the practical mirror aligned by ensuring every
issue or pull request in the fyi-cli roadmap also exists in the Rare Insights
on Open Policy from Aotearoa project and by copying the fields that both
projects can represent.
"""

from __future__ import annotations

import argparse
import json
import subprocess
import sys
from dataclasses import dataclass
from typing import Any


DEFAULT_OWNER = "edithatogo"
DEFAULT_SOURCE_PROJECT = 6
DEFAULT_TARGET_PROJECT = 4
DEFAULT_REPOSITORY = "edithatogo/fyi-cli"


@dataclass(frozen=True)
class Field:
    id: str
    name: str
    type: str
    options: dict[str, str]


@dataclass(frozen=True)
class Project:
    id: str
    number: int
    fields: dict[str, Field]


@dataclass(frozen=True)
class SyncAction:
    kind: str
    message: str
    command: list[str] | None = None


def run_gh(args: list[str], *, dry_run: bool = False) -> Any:
    if dry_run:
        print("+ gh " + " ".join(args))
        return {}

    completed = subprocess.run(
        ["gh", *args],
        check=True,
        encoding="utf-8",
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    output = completed.stdout.strip()
    if not output:
        return {}
    return json.loads(output)


def run_gh_raw(args: list[str], *, dry_run: bool = False) -> str:
    if dry_run:
        print("+ gh " + " ".join(args))
        return ""

    completed = subprocess.run(
        ["gh", *args],
        check=True,
        encoding="utf-8",
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    return completed.stdout.strip()


def load_project(owner: str, number: int) -> Project:
    project_payload = run_gh(
        ["project", "view", str(number), "--owner", owner, "--format", "json"]
    )
    fields_payload = run_gh(
        ["project", "field-list", str(number), "--owner", owner, "--format", "json"]
    )

    fields: dict[str, Field] = {}
    for raw_field in fields_payload["fields"]:
        options = {
            option["name"]: option["id"] for option in raw_field.get("options", [])
        }
        fields[raw_field["name"]] = Field(
            id=raw_field["id"],
            name=raw_field["name"],
            type=raw_field["type"],
            options=options,
        )

    return Project(id=project_payload["id"], number=number, fields=fields)


def load_items(owner: str, project_number: int, limit: int) -> list[dict[str, Any]]:
    payload = run_gh(
        [
            "project",
            "item-list",
            str(project_number),
            "--owner",
            owner,
            "--limit",
            str(limit),
            "--format",
            "json",
        ]
    )
    return payload["items"]


def content_url(item: dict[str, Any]) -> str | None:
    content = item.get("content")
    if not isinstance(content, dict):
        return None
    return content.get("url")


def content_repository(item: dict[str, Any]) -> str | None:
    content = item.get("content")
    if not isinstance(content, dict):
        return None
    return content.get("repository")


def source_items_for_repository(
    source_items: list[dict[str, Any]], repository: str
) -> list[dict[str, Any]]:
    return [
        item
        for item in source_items
        if content_url(item) and content_repository(item) == repository
    ]


def target_items_for_repository(
    target_items: list[dict[str, Any]], repository: str
) -> dict[str, dict[str, Any]]:
    by_url: dict[str, dict[str, Any]] = {}
    for item in target_items:
        if content_repository(item) != repository:
            continue
        url = content_url(item)
        if url:
            by_url[url] = item
    return by_url


def select_option_id(field: Field | None, value: str | None) -> str | None:
    if not field or not value:
        return None
    return field.options.get(value)


def plan_sync(
    *,
    owner: str,
    source_project: Project,
    target_project: Project,
    source_items: list[dict[str, Any]],
    target_by_url: dict[str, dict[str, Any]],
    mirror_source: str,
) -> list[SyncAction]:
    actions: list[SyncAction] = []
    target_status = target_project.fields.get("Status")
    target_mirror = target_project.fields.get("Mirror source")
    mirror_option = select_option_id(target_mirror, mirror_source) or select_option_id(
        target_mirror, "other"
    )

    for source_item in source_items:
        url = content_url(source_item)
        if not url:
            continue

        target_item = target_by_url.get(url)
        if target_item is None:
            actions.append(
                SyncAction(
                    kind="add",
                    message=f"add missing target item: {url}",
                    command=[
                        "project",
                        "item-add",
                        str(target_project.number),
                        "--owner",
                        owner,
                        "--url",
                        url,
                    ],
                )
            )
            continue

        source_status = source_item.get("status")
        target_status_value = target_item.get("status")
        status_option = select_option_id(target_status, source_status)
        if status_option and source_status != target_status_value:
            actions.append(
                SyncAction(
                    kind="field",
                    message=(
                        f"set Status={source_status!r} for "
                        f"{target_item.get('title', url)}"
                    ),
                    command=[
                        "project",
                        "item-edit",
                        "--id",
                        target_item["id"],
                        "--project-id",
                        target_project.id,
                        "--field-id",
                        target_status.id,
                        "--single-select-option-id",
                        status_option,
                    ],
                )
            )

        if target_mirror and mirror_option:
            target_mirror_value = target_item.get("mirror source")
            expected_mirror = (
                mirror_source if mirror_source in target_mirror.options else "other"
            )
            if target_mirror_value != expected_mirror:
                actions.append(
                    SyncAction(
                        kind="field",
                        message=(
                            f"set Mirror source={expected_mirror!r} for "
                            f"{target_item.get('title', url)}"
                        ),
                        command=[
                            "project",
                            "item-edit",
                            "--id",
                            target_item["id"],
                            "--project-id",
                            target_project.id,
                            "--field-id",
                            target_mirror.id,
                            "--single-select-option-id",
                            mirror_option,
                        ],
                    )
                )

    return actions


def apply_actions(actions: list[SyncAction], *, dry_run: bool) -> None:
    for action in actions:
        print(f"{action.kind}: {action.message}")
        if action.command:
            run_gh_raw(action.command, dry_run=dry_run)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Mirror fyi-cli project items into the RIOPA umbrella project."
    )
    parser.add_argument("--owner", default=DEFAULT_OWNER)
    parser.add_argument("--source-project", type=int, default=DEFAULT_SOURCE_PROJECT)
    parser.add_argument("--target-project", type=int, default=DEFAULT_TARGET_PROJECT)
    parser.add_argument("--repository", default=DEFAULT_REPOSITORY)
    parser.add_argument("--source-limit", type=int, default=200)
    parser.add_argument("--target-limit", type=int, default=500)
    parser.add_argument("--mirror-source", default="fyi-cli")
    parser.add_argument("--dry-run", action="store_true")
    return parser.parse_args()


def main() -> int:
    args = parse_args()

    try:
        source_project = load_project(args.owner, args.source_project)
        target_project = load_project(args.owner, args.target_project)
        source_items = source_items_for_repository(
            load_items(args.owner, args.source_project, args.source_limit),
            args.repository,
        )
        target_by_url = target_items_for_repository(
            load_items(args.owner, args.target_project, args.target_limit),
            args.repository,
        )
        actions = plan_sync(
            owner=args.owner,
            source_project=source_project,
            target_project=target_project,
            source_items=source_items,
            target_by_url=target_by_url,
            mirror_source=args.mirror_source,
        )
        print(
            "sync summary: "
            f"source_items={len(source_items)} "
            f"target_items={len(target_by_url)} "
            f"actions={len(actions)} "
            f"dry_run={args.dry_run}"
        )
        apply_actions(actions, dry_run=args.dry_run)
        return 0
    except subprocess.CalledProcessError as error:
        sys.stderr.write(error.stderr)
        return error.returncode


if __name__ == "__main__":
    raise SystemExit(main())
