from __future__ import annotations

import csv
import json
from pathlib import Path
from typing import Any

from .db import get_tracked_request, query_all, request_timeline
from .fetch import latest_snapshot_summary
from .security import ensure_private_path, sanitize_payload, secure_write_text


def write_dashboard(
    html_output: str | Path,
    db_path: str | Path = "fyi_system.db",
    json_output: str | Path | None = None,
) -> Path:
    """Compatibility export for benchmark and legacy reporting callers."""
    from .dashboard import write_dashboard as _write_dashboard

    return _write_dashboard(html_output, db_path=db_path, json_output=json_output)


def normalize_snapshot_state(raw_state: str | None) -> str:
    state = (raw_state or '').strip().lower()
    mapping = {
        'successful': 'responded_full',
        'partially_successful': 'responded_partial',
        'waiting_response': 'awaiting_response',
        'internal_review': 'awaiting_response',
        'gone_postal': 'awaiting_response',
        'rejected': 'refused',
        'not_held': 'not_held',
    }
    return mapping.get(state, state or 'unknown')


def attention_report(db_path: str | Path = 'fyi_system.db') -> dict:
    rows = query_all(db_path, 'SELECT * FROM tracked_requests ORDER BY updated_at DESC, id DESC')
    items = []
    for row in rows:
        analysis = response_analysis(db_path, row['id'])
        needs_attention = analysis['priority'] in {'now', 'soon'} or row['status'] in {'draft', 'open', 'awaiting'} or row['last_event_title'] is None
        items.append(
            {
                'id': row['id'],
                'title': row['title'],
                'status': row['status'],
                'fyi_request_id': row['fyi_request_id'],
                'needs_attention': bool(needs_attention),
                'last_event_title': row['last_event_title'],
                'action_bucket': analysis['action_bucket'],
                'priority': analysis['priority'],
            }
        )
    return {'count': len(items), 'items': items}


def write_attention_report(output_path: str | Path, db_path: str | Path = 'fyi_system.db') -> Path:
    output = Path(output_path)
    secure_write_text(output, json.dumps(attention_report(db_path), indent=2, ensure_ascii=False))
    return output


def build_handover_markdown(db_path: str | Path = 'fyi_system.db') -> str:
    report = attention_report(db_path)
    triage = triage_report(db_path)
    lines = ['# FYI Request System Handover', '', f"Tracked requests: {report['count']}", '', '## Needs action now', '']
    for item in triage['action_now']:
        lines.append(
            f"- [NOW] #{item['id']} {item['title']} ({item['action_bucket']}; status: {item['tracked_status']}; state: {item['snapshot_state']})"
        )
    if not triage['action_now']:
        lines.append('- None currently in the action-now queue.')
    lines.extend(['', '## Attention queue', ''])
    for item in report['items']:
        marker = 'ACTION' if item['needs_attention'] else 'OK'
        lines.append(
            f"- [{marker}] #{item['id']} {item['title']} (status: {item['status']}, FYI: {item['fyi_request_id']}, action: {item['action_bucket']}, priority: {item['priority']})"
        )
        tl = request_timeline(db_path, item['id'])[:2]
        for ev in tl:
            lines.append(f"  - {ev['ts']}: {ev['title']}")
    return "\n".join(lines) + "\n"


def write_handover(output_path: str | Path, db_path: str | Path = 'fyi_system.db') -> Path:
    output = Path(output_path)
    secure_write_text(output, build_handover_markdown(db_path))
    return output


def _base_follow_up_context(db_path: str | Path, tracked_request_id: int) -> tuple[dict[str, Any], dict[str, Any] | None]:
    row = get_tracked_request(db_path, tracked_request_id)
    if not row:
        raise ValueError(f'Request {tracked_request_id} not found')
    snapshot = latest_snapshot_summary(db_path, row['fyi_request_id'])
    return dict(row), snapshot


def suggest_follow_up(db_path: str | Path, tracked_request_id: int) -> dict[str, Any]:
    row, snapshot = _base_follow_up_context(db_path, tracked_request_id)
    title = row['title']
    authority = row['authority_slug']
    status = (row['status'] or '').lower()
    snapshot_state = ((snapshot or {}).get('described_state') or '').lower()
    normalized_state = normalize_snapshot_state(snapshot_state)
    has_attachments = bool((snapshot or {}).get('attachments'))

    body_lines = [
        f'Kia ora {authority},',
        '',
        f'I’m following up on my request "{title}".',
    ]
    rationale = 'Generic follow-up based on tracked status.'
    stage = 'follow_up'

    if status == 'draft':
        rationale = 'Tracked request is still in draft and has not been submitted on FYI.'
        stage = 'submit_prompt'
        body_lines = [
            f'Draft ready for submission to {authority}.',
            '',
            f'Title: {title}',
            '',
            'Check the wording, confirm the authority, and submit through the prefilled FYI link.',
        ]
    elif normalized_state in {'responded_full', 'responded_partial'} or has_attachments:
        rationale = 'Latest snapshot suggests a response or attachment is available.'
        stage = 'review_response'
        body_lines.extend(
            [
                'I can see that material may now be available on the request page.',
                'Could you please confirm whether this constitutes your full response, and whether any further material remains outstanding?',
                '',
                'If documents have been released, please identify any withheld material and the basis for any redactions or refusals.',
            ]
        )
    elif normalized_state == 'awaiting_response' or status in {'open', 'awaiting', 'submitted', 'awaiting_response'}:
        rationale = 'Latest tracked state suggests the request is still active without a complete response.'
        stage = 'nudge'
        body_lines.extend(
            [
                'I would be grateful for an update on the status of this request.',
                'If a decision has been made, please provide the response and any released material through the FYI request page.',
                '',
                'If you need refinement or clarification, please let me know.',
            ]
        )
    elif normalized_state in {'refused', 'not_held'}:
        rationale = 'Latest snapshot indicates a refusal or not-held outcome.'
        stage = 'challenge_or_refine'
        body_lines.extend(
            [
                'I note the current outcome recorded on the request page.',
                'Please clarify whether any part of the requested information is held elsewhere within the agency, or whether the request can be narrowed or transferred.',
            ]
        )
    else:
        body_lines.extend(
            [
                'I would be grateful for an update on the current status of this request.',
                'Please let me know if any clarification would assist.',
            ]
        )

    body_lines.extend(['', 'Ngā mihi'])
    subject = f'Follow-up: {title}'
    return {
        'tracked_request_id': tracked_request_id,
        'subject': subject,
        'body': "\n".join(body_lines).strip() + "\n",
        'stage': stage,
        'rationale': rationale,
        'snapshot_state': snapshot_state,
        'normalized_snapshot_state': normalized_state,
    }


def _tone_instructions(tone: str) -> tuple[str, str]:
    tone = tone.strip().lower()
    if tone == 'warm':
        return ('Thank you for your work on this request so far.', 'I appreciate your help with this.')
    if tone == 'firm':
        return ('I am seeking a clear update on this request.', 'Please address the outstanding points directly.')
    return ('', '')


def _variant_body(authority: str, title: str, strategy: str, tone: str, response_present: bool) -> str:
    prefix, suffix = _tone_instructions(tone)
    lines = [f'Kia ora {authority},', '']
    if prefix:
        lines.extend([prefix, ''])
    if strategy == 'polite_nudge':
        lines.extend([
            f'I’m following up on my request "{title}".',
            'I would be grateful for a brief status update when convenient.',
        ])
    elif strategy == 'firm_deadline_check':
        lines.extend([
            f'I am writing to seek a clear status update on my request "{title}".',
            'Please confirm whether a decision has now been made and whether any released material is available on the FYI request page.',
        ])
    elif strategy == 'review_released_material':
        lines.extend([
            f'Thank you for the material released in relation to "{title}".',
            'Please confirm whether this is the full response and identify any withheld material or outstanding parts.',
        ])
    else:
        lines.extend([
            f'If it would assist to process my request "{title}", I am happy to clarify or refine the scope.',
            'Please let me know what narrowing or framing would be most useful while still capturing the information sought.',
        ])
    if response_present and strategy == 'review_released_material':
        lines.extend(['', 'If documents have already been uploaded, please indicate whether they are complete.'])
    if suffix:
        lines.extend(['', suffix])
    lines.extend(['', 'Ngā mihi'])
    return "\n".join(lines) + "\n"


def follow_up_variants(db_path: str | Path, tracked_request_id: int) -> dict[str, Any]:
    base = suggest_follow_up(db_path, tracked_request_id)
    row, snapshot = _base_follow_up_context(db_path, tracked_request_id)
    title = row['title']
    authority = row['authority_slug']
    snapshot_state = normalize_snapshot_state((snapshot or {}).get('described_state'))
    attachment_count = len((snapshot or {}).get('attachments', []))
    response_present = bool(attachment_count or snapshot_state in {'responded_full', 'responded_partial'})

    variants: list[dict[str, Any]] = []
    variants.append(
        {
            'strategy': 'polite_nudge',
            'subject': base['subject'],
            'body': base['body'],
            'why': 'Default, lowest-friction follow-up.',
        }
    )
    variants.append(
        {
            'strategy': 'firm_deadline_check',
            'subject': f'Status update requested: {title}',
            'body': _variant_body(authority, title, 'firm_deadline_check', 'neutral', response_present),
            'why': 'More direct wording when a request appears to have stalled.',
        }
    )
    if response_present:
        variants.append(
            {
                'strategy': 'review_released_material',
                'subject': f'Clarification on released material: {title}',
                'body': _variant_body(authority, title, 'review_released_material', 'neutral', response_present),
                'why': 'Best when documents or a substantive response appear to be available.',
            }
        )
    else:
        variants.append(
            {
                'strategy': 'clarify_scope',
                'subject': f'Clarification offered: {title}',
                'body': _variant_body(authority, title, 'clarify_scope', 'neutral', response_present),
                'why': 'Useful when delay may reflect uncertainty about scope.',
            }
        )
    return {
        'tracked_request_id': tracked_request_id,
        'snapshot_state': snapshot_state,
        'variants': variants,
    }


TONE_ORDER = ['neutral', 'warm', 'firm']


def follow_up_pack(db_path: str | Path, tracked_request_id: int) -> dict[str, Any]:
    row, snapshot = _base_follow_up_context(db_path, tracked_request_id)
    title = row['title']
    authority = row['authority_slug']
    snapshot_state = normalize_snapshot_state((snapshot or {}).get('described_state'))
    response_present = bool((snapshot or {}).get('attachments')) or snapshot_state in {'responded_full', 'responded_partial'}
    strategies = ['polite_nudge', 'firm_deadline_check', 'review_released_material' if response_present else 'clarify_scope']
    items = []
    for strategy in strategies:
        for tone in TONE_ORDER:
            subject_prefix = {
                'polite_nudge': 'Follow-up',
                'firm_deadline_check': 'Status update requested',
                'review_released_material': 'Clarification on released material',
                'clarify_scope': 'Clarification offered',
            }[strategy]
            items.append(
                {
                    'strategy': strategy,
                    'tone': tone,
                    'subject': f'{subject_prefix}: {title}',
                    'body': _variant_body(authority, title, strategy, tone, response_present),
                }
            )
    return {
        'tracked_request_id': tracked_request_id,
        'snapshot_state': snapshot_state,
        'items': items,
    }


def write_follow_up_pack(output_path: str | Path, db_path: str | Path, tracked_request_id: int) -> Path:
    output = Path(output_path)
    payload = follow_up_pack(db_path, tracked_request_id)
    secure_write_text(output, json.dumps(payload, indent=2, ensure_ascii=False))
    return output


def write_follow_up_variants(output_path: str | Path, db_path: str | Path, tracked_request_id: int) -> Path:
    output = Path(output_path)
    payload = follow_up_variants(db_path, tracked_request_id)
    secure_write_text(output, json.dumps(payload, indent=2, ensure_ascii=False))
    return output


def write_follow_up(output_path: str | Path, db_path: str | Path, tracked_request_id: int) -> Path:
    output = Path(output_path)
    payload = suggest_follow_up(db_path, tracked_request_id)
    secure_write_text(output, json.dumps(payload, indent=2, ensure_ascii=False))
    return output


def attachment_manifest(db_path: str | Path, tracked_request_id: int) -> dict[str, Any]:
    row = get_tracked_request(db_path, tracked_request_id)
    if not row:
        raise ValueError(f'Request {tracked_request_id} not found')
    snapshot = latest_snapshot_summary(db_path, row['fyi_request_id']) or {}
    attachments = snapshot.get('attachments', [])
    return {
        'tracked_request_id': tracked_request_id,
        'fyi_request_id': row['fyi_request_id'],
        'title': row['title'],
        'attachments_count': len(attachments),
        'attachments': attachments,
    }



def write_attachment_manifest(output_path: str | Path, db_path: str | Path, tracked_request_id: int) -> Path:
    output = Path(output_path)
    payload = attachment_manifest(db_path, tracked_request_id)
    secure_write_text(output, json.dumps(payload, indent=2, ensure_ascii=False))
    return output



def write_attachment_manifest_csv(output_path: str | Path, db_path: str | Path, tracked_request_id: int) -> Path:
    output = Path(output_path)
    ensure_private_path(output.parent, is_dir=True)
    payload = attachment_manifest(db_path, tracked_request_id)
    with output.open('w', encoding='utf-8', newline='') as fh:
        writer = csv.DictWriter(
            fh,
            fieldnames=['tracked_request_id', 'fyi_request_id', 'title', 'name', 'url', 'content_type', 'size', 'path'],
        )
        writer.writeheader()
        for att in payload['attachments']:
            writer.writerow(
                {
                    'tracked_request_id': payload['tracked_request_id'],
                    'fyi_request_id': payload['fyi_request_id'],
                    'title': payload['title'],
                    'name': att.get('name') or '',
                    'url': att.get('url') or '',
                    'content_type': att.get('content_type') or '',
                    'size': att.get('size') or '',
                    'path': att.get('path') or '',
                }
            )
    ensure_private_path(output, is_dir=False)
    return output


def response_analysis(db_path: str | Path, tracked_request_id: int) -> dict[str, Any]:
    row, snapshot = _base_follow_up_context(db_path, tracked_request_id)
    snapshot = snapshot or {}
    attachments = snapshot.get('attachments', [])
    events = snapshot.get('events', [])
    raw_state = snapshot.get('described_state')
    normalized_state = normalize_snapshot_state(raw_state)
    tracked_status = (row.get('status') or '').lower()
    likely_response_received = bool(attachments or normalized_state in {'responded_full', 'responded_partial'} or events)
    likely_incomplete = normalized_state in {'responded_partial', 'awaiting_response'} or tracked_status in {'submitted', 'awaiting_response', 'partial'}

    if normalized_state == 'responded_full':
        action_bucket = 'review_release'
        priority = 'now'
        recommendation = 'Review the released material and decide whether any completeness or redaction follow-up is needed.'
    elif normalized_state == 'responded_partial':
        action_bucket = 'review_release'
        priority = 'now'
        recommendation = 'A partial response is indicated; review the material and consider a completeness follow-up.'
    elif normalized_state in {'refused', 'not_held'}:
        action_bucket = 'challenge_or_refine'
        priority = 'now'
        recommendation = 'Consider challenging the refusal, seeking transfer, or refining the request.'
    elif tracked_status == 'draft':
        action_bucket = 'submit_or_review_draft'
        priority = 'soon'
        recommendation = 'The tracked request remains a draft; review and submit when ready.'
    else:
        action_bucket = 'chase_status'
        priority = 'soon'
        recommendation = 'No complete response is evident yet; send a status-check follow-up if appropriate.'

    return {
        'tracked_request_id': tracked_request_id,
        'title': row['title'],
        'tracked_status': row['status'],
        'snapshot_state': raw_state,
        'normalized_snapshot_state': normalized_state,
        'latest_snapshot_at': snapshot.get('fetched_at'),
        'attachments_count': len(attachments),
        'events_count': len(events),
        'likely_response_received': likely_response_received,
        'likely_incomplete': likely_incomplete,
        'action_bucket': action_bucket,
        'priority': priority,
        'recommendation': recommendation,
        'attachment_names': [a.get('name') for a in attachments if a.get('name')],
        'recent_event_titles': [e.get('title') for e in events[:5] if e.get('title')],
    }



def write_response_analysis(output_path: str | Path, db_path: str | Path, tracked_request_id: int) -> Path:
    output = Path(output_path)
    payload = response_analysis(db_path, tracked_request_id)
    secure_write_text(output, json.dumps(payload, indent=2, ensure_ascii=False))
    return output


def triage_report(db_path: str | Path = 'fyi_system.db') -> dict[str, Any]:
    rows = query_all(db_path, 'SELECT id FROM tracked_requests ORDER BY updated_at DESC, id DESC')
    analyses = [response_analysis(db_path, int(row['id'])) for row in rows]
    action_now = [a for a in analyses if a['priority'] == 'now']
    action_soon = [a for a in analyses if a['priority'] == 'soon']
    parked = [a for a in analyses if a['priority'] not in {'now', 'soon'}]
    return {
        'summary': {
            'total': len(analyses),
            'action_now': len(action_now),
            'action_soon': len(action_soon),
            'parked': len(parked),
        },
        'action_now': action_now,
        'action_soon': action_soon,
        'parked': parked,
    }




def next_best_action(db_path: str | Path, tracked_request_id: int, tone: str = "neutral") -> dict[str, Any]:
    analysis = response_analysis(db_path, tracked_request_id)
    pack = follow_up_pack(db_path, tracked_request_id)
    strategy_map = {
        'review_release': 'review_released_material',
        'challenge_or_refine': 'clarify_scope',
        'chase_status': 'firm_deadline_check' if analysis['priority'] == 'now' else 'polite_nudge',
        'submit_or_review_draft': 'polite_nudge',
    }
    recommended_strategy = strategy_map.get(analysis['action_bucket'], 'polite_nudge')
    selected = None
    for item in pack.get('items', []):
        if item.get('strategy') == recommended_strategy and item.get('tone') == tone:
            selected = item
            break
    if selected is None and pack.get('items'):
        for item in pack['items']:
            if item.get('strategy') == recommended_strategy:
                selected = item
                break
    selected = selected or {'strategy': recommended_strategy, 'tone': tone, 'subject': '', 'body': ''}

    checklist_map = {
        'review_release': [
            'Open the latest FYI snapshot or request page.',
            'Review attachments and event trail for completeness.',
            'Decide whether released material appears complete or needs a follow-up.',
        ],
        'challenge_or_refine': [
            'Review the refusal or not-held outcome.',
            'Decide whether to challenge, refine, or seek transfer.',
            'Send a clarification-oriented follow-up if useful.',
        ],
        'chase_status': [
            'Check whether the request is still awaiting response.',
            'Confirm the latest tracked status and FYI snapshot state.',
            'Send a status-check follow-up.',
        ],
        'submit_or_review_draft': [
            'Review the draft wording and tags.',
            'Open the FYI prefilled draft URL.',
            'Submit the request or continue refining it locally.',
        ],
    }

    available = []
    seen = set()
    for item in pack.get('items', []):
        key = (item.get('strategy'), item.get('tone'))
        if key in seen:
            continue
        seen.add(key)
        available.append({'strategy': key[0], 'tone': key[1], 'subject': item.get('subject', '')})

    return {
        'tracked_request_id': tracked_request_id,
        'title': analysis['title'],
        'priority': analysis['priority'],
        'action_bucket': analysis['action_bucket'],
        'recommended_strategy': recommended_strategy,
        'recommended_tone': selected.get('tone') or tone,
        'subject': selected.get('subject') or '',
        'body': selected.get('body') or '',
        'recommendation': analysis['recommendation'],
        'checklist': checklist_map.get(analysis['action_bucket'], []),
        'available_options': available,
        'response_analysis': analysis,
        'open_draft_path': f'/requests/{tracked_request_id}/recommended-draft?strategy={recommended_strategy}&tone={selected.get("tone") or tone}',
    }


def correspondence_pack(db_path: str | Path, tracked_request_id: int) -> dict[str, Any]:
    row, snapshot = _base_follow_up_context(db_path, tracked_request_id)
    pack = follow_up_pack(db_path, tracked_request_id)
    analysis = response_analysis(db_path, tracked_request_id)
    grouped: dict[str, list[dict[str, Any]]] = {}
    for item in pack.get('items', []):
        grouped.setdefault(item.get('strategy') or 'other', []).append(item)
    return {
        'tracked_request_id': tracked_request_id,
        'title': row['title'],
        'authority_slug': row['authority_slug'],
        'tracked_status': row['status'],
        'fyi_request_id': row['fyi_request_id'],
        'snapshot_state': (snapshot or {}).get('described_state'),
        'analysis': analysis,
        'recommended_action': next_best_action(db_path, tracked_request_id),
        'strategies': grouped,
        'bundle_basename': f'request-{tracked_request_id}-bundle',
    }



def write_correspondence_pack(output_path: str | Path, db_path: str | Path, tracked_request_id: int) -> Path:
    output = Path(output_path)
    payload = correspondence_pack(db_path, tracked_request_id)
    secure_write_text(output, json.dumps(payload, indent=2, ensure_ascii=False))
    return output



def write_correspondence_pack_markdown(output_path: str | Path, db_path: str | Path, tracked_request_id: int) -> Path:
    output = Path(output_path)
    payload = correspondence_pack(db_path, tracked_request_id)
    lines = [
        f"# Correspondence pack for request #{tracked_request_id}",
        '',
        f"**Title:** {payload['title']}",
        f"**Authority:** {payload['authority_slug']}",
        f"**Tracked status:** {payload['tracked_status']}",
        f"**FYI request ID:** {payload['fyi_request_id'] or ''}",
        f"**Snapshot state:** {payload['snapshot_state'] or ''}",
        '',
        '## Recommended next action',
        '',
        f"- Action bucket: {payload['recommended_action']['action_bucket']}",
        f"- Priority: {payload['recommended_action']['priority']}",
        f"- Strategy: {payload['recommended_action']['recommended_strategy']}",
        f"- Tone: {payload['recommended_action']['recommended_tone']}",
        '',
        payload['recommended_action']['recommendation'],
        '',
        '### Checklist',
        '',
    ]
    for item in payload['recommended_action']['checklist']:
        lines.append(f'- {item}')
    for strategy, items in payload['strategies'].items():
        lines.extend(['', f'## {strategy}', ''])
        for item in items:
            lines.extend([
                f"### {item.get('tone','neutral')}",
                '',
                f"**Subject:** {item.get('subject','')}",
                '',
                item.get('body','').rstrip(),
                '',
            ])
    secure_write_text(output, '\n'.join(lines).rstrip() + '\n')
    return output


def write_triage_report(output_path: str | Path, db_path: str | Path = 'fyi_system.db') -> Path:
    output = Path(output_path)
    secure_write_text(output, json.dumps(triage_report(db_path), indent=2, ensure_ascii=False))
    return output



def select_correspondence_variant(db_path: str | Path, tracked_request_id: int, strategy: str, tone: str = 'neutral') -> dict[str, Any]:
    pack = follow_up_pack(db_path, tracked_request_id)
    for item in pack.get('items', []):
        if item.get('strategy') == strategy and item.get('tone') == tone:
            return item
    for item in pack.get('items', []):
        if item.get('strategy') == strategy:
            return item
    raise ValueError(f'No correspondence variant for strategy={strategy!r}, tone={tone!r}')



def export_request_bundle(output_dir: str | Path, db_path: str | Path, tracked_request_id: int, *, sanitize: bool = True, profile: str = 'standard') -> Path:
    bundle_dir = Path(output_dir)
    ensure_private_path(bundle_dir, is_dir=True)
    write_correspondence_pack(bundle_dir / 'correspondence-pack.json', db_path, tracked_request_id)
    write_correspondence_pack_markdown(bundle_dir / 'correspondence-pack.md', db_path, tracked_request_id)
    write_attachment_manifest(bundle_dir / 'attachment-manifest.json', db_path, tracked_request_id)
    write_attachment_manifest_csv(bundle_dir / 'attachment-manifest.csv', db_path, tracked_request_id)
    write_response_analysis(bundle_dir / 'response-analysis.json', db_path, tracked_request_id)
    nba = next_best_action(db_path, tracked_request_id)
    secure_write_text(bundle_dir / 'next-best-action.json', json.dumps(sanitize_payload(nba, profile=profile) if sanitize else nba, indent=2, ensure_ascii=False))
    detail = {
        'tracked_request_id': tracked_request_id,
        'next_best_action': nba,
        'correspondence_pack': correspondence_pack(db_path, tracked_request_id),
        'attachment_manifest': attachment_manifest(db_path, tracked_request_id),
        'response_analysis': response_analysis(db_path, tracked_request_id),
    }
    secure_write_text(bundle_dir / 'request-detail.json', json.dumps(sanitize_payload(detail, profile=profile) if sanitize else detail, indent=2, ensure_ascii=False))
    manifest = {
        'sanitized': sanitize,
        'privacy_profile': profile,
        'tracked_request_id': tracked_request_id,
        'files': [
            'correspondence-pack.json',
            'correspondence-pack.md',
            'attachment-manifest.json',
            'attachment-manifest.csv',
            'response-analysis.json',
            'next-best-action.json',
            'request-detail.json',
        ],
    }
    secure_write_text(bundle_dir / 'manifest.json', json.dumps(manifest, indent=2, ensure_ascii=False))
    return bundle_dir
