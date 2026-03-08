
from __future__ import annotations
import json
from pathlib import Path
from typing import Any
import requests
from .db import connect, query_all


def fetch_request_page(request_id: int, base_url: str = 'https://fyi.org.nz', db_path: str | Path = 'fyi_system.db', timeout: int = 20) -> dict:
    url = f"{base_url.rstrip('/')}/request/{request_id}.json"
    response = requests.get(url, timeout=timeout)
    response.raise_for_status()
    data = response.json()
    conn = connect(db_path)
    try:
        conn.execute(
            'INSERT INTO request_snapshots(fyi_request_id, source_url, raw_json) VALUES (?, ?, ?)',
            (request_id, url, json.dumps(data, ensure_ascii=False)),
        )
        conn.commit()
    finally:
        conn.close()
    return data


def _get_first(node: Any, *paths: tuple[str, ...] | str) -> Any:
    for path in paths:
        if isinstance(path, str):
            path = tuple(path.split('.'))
        cur = node
        ok = True
        for part in path:
            if isinstance(cur, dict) and part in cur:
                cur = cur[part]
            else:
                ok = False
                break
        if ok and cur not in (None, ''):
            return cur
    return None


def _dedupe_keep_order(items: list[dict[str, Any]], key_fields: tuple[str, ...]) -> list[dict[str, Any]]:
    seen: set[tuple[Any, ...]] = set()
    out: list[dict[str, Any]] = []
    for item in items:
        key = tuple(item.get(k) for k in key_fields)
        if key in seen:
            continue
        seen.add(key)
        out.append(item)
    return out


def _normalize_attachment(node: dict[str, Any], path: str) -> dict[str, Any] | None:
    url = node.get('url') or node.get('download_url') or node.get('file_url') or node.get('attachment_url') or node.get('public_body_url')
    name = node.get('name') or node.get('filename') or node.get('file_name') or node.get('title') or node.get('display_name')
    content_type = node.get('content_type') or node.get('mime_type') or node.get('content-type') or ''
    size = node.get('size') or node.get('byte_size') or node.get('content_length') or node.get('filesize')
    if url and name:
        return {
            'name': str(name),
            'url': str(url),
            'content_type': str(content_type),
            'size': size,
            'path': path,
        }
    return None


def _flatten_candidate_files(node: Any, path: str = 'root') -> list[dict[str, Any]]:
    found: list[dict[str, Any]] = []
    if isinstance(node, dict):
        normalized = _normalize_attachment(node, path)
        if normalized:
            found.append(normalized)
        # common FYI / Alaveteli-ish wrappers
        for wrapper_key in ('attachments', 'incoming_message_attachments', 'outgoing_message_attachments', 'documents', 'files'):
            value = node.get(wrapper_key)
            if isinstance(value, list):
                for idx, item in enumerate(value):
                    found.extend(_flatten_candidate_files(item, f'{path}.{wrapper_key}[{idx}]'))
        for key, value in node.items():
            if isinstance(value, (dict, list)):
                found.extend(_flatten_candidate_files(value, f'{path}.{key}'))
    elif isinstance(node, list):
        for idx, value in enumerate(node):
            found.extend(_flatten_candidate_files(value, f'{path}[{idx}]'))
    return _dedupe_keep_order(found, ('name', 'url'))


def _normalize_event(node: dict[str, Any], path: str) -> dict[str, Any] | None:
    if path == 'root':
        return None
    title = node.get('title') or node.get('event_type') or node.get('described_state') or node.get('state')
    detail = node.get('comment') or node.get('body') or node.get('excerpt') or node.get('details') or node.get('described_state') or node.get('event_type')
    created_at = node.get('created_at') or node.get('updated_at') or node.get('sent_at') or node.get('occurred_at') or node.get('last_event_forming_initial_request_at')
    lower_path = path.lower()
    eventish = (
        any(k in node for k in ('event_type', 'incoming_message', 'outgoing_message', 'comment'))
        or ('message' in lower_path)
        or ('event' in lower_path)
        or ('history' in lower_path)
        or ('correspondence' in lower_path)
    )
    if lower_path.endswith('.request') or lower_path.endswith('.info_request'):
        eventish = False
    if eventish and (title or detail or created_at):
        return {
            'title': str(title or 'event'),
            'detail': str(detail or ''),
            'created_at': str(created_at or ''),
            'path': path,
        }
    return None


def _flatten_candidate_events(node: Any, path: str = 'root') -> list[dict[str, Any]]:
    found: list[dict[str, Any]] = []
    if isinstance(node, dict):
        normalized = _normalize_event(node, path)
        if normalized:
            found.append(normalized)
        for wrapper_key in ('events', 'history', 'correspondence', 'messages', 'incoming_messages', 'outgoing_messages'):
            value = node.get(wrapper_key)
            if isinstance(value, list):
                for idx, item in enumerate(value):
                    found.extend(_flatten_candidate_events(item, f'{path}.{wrapper_key}[{idx}]'))
        for key, value in node.items():
            if isinstance(value, (dict, list)):
                found.extend(_flatten_candidate_events(value, f'{path}.{key}'))
    elif isinstance(node, list):
        for idx, value in enumerate(node):
            found.extend(_flatten_candidate_events(value, f'{path}[{idx}]'))
    return _dedupe_keep_order(found, ('title', 'created_at', 'detail'))


def normalize_request_payload(data: dict[str, Any]) -> dict[str, Any]:
    nested = data.get('info_request', {}) if isinstance(data, dict) else {}
    request_node = _get_first(data, 'info_request', 'request') or nested or data
    return {
        'id': _get_first(data, ('id',), ('info_request', 'id'), ('request', 'id')),
        'title': _get_first(data, ('title',), ('info_request', 'title'), ('request', 'title')),
        'described_state': _get_first(
            data,
            ('described_state',),
            ('info_request', 'described_state'),
            ('request', 'described_state'),
            ('state',),
            ('info_request', 'state'),
            ('request', 'state'),
        ),
        'url_title': _get_first(data, ('url_title',), ('info_request', 'url_title'), ('request', 'url_title')),
        'request_node': request_node,
    }


def extract_request_artifacts(data: dict) -> dict[str, Any]:
    request = normalize_request_payload(data)
    attachments = _flatten_candidate_files(data)
    events = _flatten_candidate_events(data)
    return {
        'attachments': attachments,
        'events': events,
        'counts': {
            'attachments': len(attachments),
            'events': len(events),
        },
        'request': {
            'title': request['title'],
            'described_state': request['described_state'],
            'url_title': request['url_title'],
            'id': request['id'],
        },
    }


def summarize_request_json(data: dict) -> dict:
    request = normalize_request_payload(data)
    artifacts = extract_request_artifacts(data)
    return {
        'title': request['title'],
        'described_state': request['described_state'],
        'url_title': request['url_title'],
        'attachments_count': artifacts['counts']['attachments'],
        'events_count': artifacts['counts']['events'],
    }


def latest_snapshot_summary(db_path: str | Path, fyi_request_id: int | None) -> dict | None:
    if fyi_request_id is None:
        return None
    rows = query_all(
        db_path,
        'SELECT raw_json, fetched_at, source_url FROM request_snapshots WHERE fyi_request_id=? ORDER BY fetched_at DESC, id DESC LIMIT 1',
        (fyi_request_id,),
    )
    if not rows:
        return None
    row = rows[0]
    data = json.loads(row['raw_json'])
    summary = summarize_request_json(data)
    artifacts = extract_request_artifacts(data)
    summary['attachments'] = artifacts['attachments']
    summary['events'] = artifacts['events']
    summary['fetched_at'] = row['fetched_at']
    summary['source_url'] = row['source_url']
    return summary
