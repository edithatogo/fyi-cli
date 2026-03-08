from __future__ import annotations

import html
import json
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
from urllib.parse import parse_qs, urlparse

from .dashboard import dashboard_payload
from .db import (
    get_tracked_request,
    init_db,
    insert_tracked_request,
    query_all,
    request_timeline,
    update_request_status,
    update_tracked_request,
)
from .fetch import latest_snapshot_summary
from .fyi import build_prefilled_url
from .importers import import_authorities_csv
from .reporting import (
    correspondence_pack,
    export_request_bundle,
    follow_up_pack,
    follow_up_variants,
    next_best_action,
    response_analysis,
    select_correspondence_variant,
    suggest_follow_up,
)

STATUSES = ["draft", "submitted", "awaiting_response", "partial", "completed", "closed"]


def _escape(value: str | None) -> str:
    return html.escape(value or "", quote=True)


def _layout(title: str, body: str) -> str:
    return f"""<!doctype html>
<html><head><meta charset='utf-8'><title>{_escape(title)}</title>
<style>
body {{ font-family: system-ui, sans-serif; margin: 2rem; color: #0f172a; }}
a {{ color: #0f62fe; text-decoration: none; }}
a:hover {{ text-decoration: underline; }}
nav {{ margin-bottom: 1rem; display:flex; gap:1rem; flex-wrap: wrap; }}
.card {{ border:1px solid #ddd; border-radius: 12px; padding:1rem; margin: 1rem 0; }}
.cards {{ display:flex; gap:1rem; flex-wrap:wrap; }}
.grid {{ display:grid; grid-template-columns: repeat(auto-fit, minmax(280px, 1fr)); gap: 1rem; }}
label {{ display:block; font-weight:600; margin-top:0.7rem; }}
input[type=text], input[type=number], textarea, select {{ width:100%; padding:0.65rem; border:1px solid #cbd5e1; border-radius:8px; box-sizing:border-box; }}
textarea {{ min-height: 120px; }}
button {{ margin-top: 1rem; padding:0.7rem 1rem; border:0; border-radius:10px; background:#0f62fe; color:white; cursor:pointer; }}
.btn-link {{ display:inline-block; margin-top:0.5rem; padding:0.55rem 0.9rem; border-radius:10px; background:#e2e8f0; color:#0f172a; }}
table {{ border-collapse: collapse; width:100%; }}
th, td {{ padding:0.6rem; border-bottom:1px solid #e2e8f0; text-align:left; vertical-align: top; }}
.muted {{ color:#475569; }}
.success {{ background:#ecfdf5; border:1px solid #86efac; padding:0.8rem; border-radius:10px; margin: 1rem 0; }}
pre {{ white-space: pre-wrap; background:#f8fafc; padding:0.8rem; border-radius:8px; }}
.pill {{ display:inline-block; padding:0.2rem 0.5rem; border-radius:999px; background:#f1f5f9; }}
.actions {{ display:flex; gap:0.6rem; flex-wrap:wrap; }}
</style></head><body>
<nav>
  <a href='/'>Dashboard</a>
  <a href='/requests'>Requests</a>
  <a href='/requests/new'>New request</a>
  <a href='/authorities'>Authorities</a>
  <a href='/authorities/import'>Import authorities</a>
</nav>
{body}
</body></html>"""


def _status_options(current: str) -> str:
    return "".join(
        f"<option value='{_escape(status)}'{' selected' if status == current else ''}>{_escape(status)}</option>"
        for status in STATUSES
    )


def _authority_options(db_path: str, current: str = "") -> str:
    rows = query_all(db_path, "SELECT slug, name FROM authorities ORDER BY name")
    return "".join(
        f"<option value='{_escape(r['slug'])}'{' selected' if r['slug'] == current else ''}>{_escape(r['name'])} ({_escape(r['slug'])})</option>"
        for r in rows
    )


def _render_dashboard(db_path: str, flash: str = "") -> str:
    payload = dashboard_payload(db_path)
    items = payload["items"][:10]
    flash_html = f"<div class='success'>{_escape(flash)}</div>" if flash else ""
    rows = "".join(
        f"<tr><td>{i['id']}</td><td>{_escape(i['authority_slug'])}</td><td><a href='/requests/{i['id']}'>{_escape(i['title'])}</a></td><td>{_escape(i['status'])}</td><td>{_escape(str(i.get('fyi_request_id') or ''))}</td></tr>"
        for i in items
    )
    body = f"""
    <h1>FYI Request System</h1>
    {flash_html}
    <div class='cards'>
      <div class='card'><div>Total tracked</div><strong>{payload['summary']['total']}</strong></div>
      <div class='card'><div>Needs attention</div><strong>{payload['summary']['attention']}</strong></div>
      <div class='card'><div>Action now</div><strong>{payload['summary'].get('action_now', 0)}</strong></div>
      <div class='card'><div>Authorities</div><strong>{payload['summary']['authorities']}</strong></div>
      <div class='card'><div>Recent updates (7d)</div><strong>{payload['summary']['recent_updates']}</strong></div>
    </div>
    <div class='grid'>
      <div class='card'>
        <h2>Quick actions</h2>
        <p><a href='/requests/new'>Create a tracked request</a></p>
        <p><a href='/authorities/import'>Import authority CSV</a></p>
        <p><a href='/api/dashboard'>View dashboard JSON</a></p>
        <p><a href='/requests?priority=now'>Review action-now queue</a></p>
      </div>
      <div class='card'>
        <h2>Recently tracked</h2>
        <table><thead><tr><th>ID</th><th>Authority</th><th>Title</th><th>Status</th><th>FYI</th></tr></thead><tbody>{rows}</tbody></table>
      </div>
    </div>
    """
    return _layout("FYI Request System", body)


def _render_requests(db_path: str, q: str = "", flash: str = "", priority: str = "") -> str:
    params = []
    sql = "SELECT id, authority_slug, title, status, fyi_request_id, tags, updated_at FROM tracked_requests"
    if q:
        sql += " WHERE lower(title) LIKE ? OR lower(authority_slug) LIKE ? OR lower(tags) LIKE ?"
        wildcard = f"%{q.lower()}%"
        params = [wildcard, wildcard, wildcard]
    sql += " ORDER BY updated_at DESC, id DESC"
    rows = query_all(db_path, sql, params)
    if priority:
        wanted = priority.strip().lower()
        rows = [r for r in rows if response_analysis(db_path, int(r["id"]))["priority"] == wanted]
    rows_html_parts = []
    for r in rows:
        action = next_best_action(db_path, int(r["id"]))
        rows_html_parts.append(
            f"<tr><td>{r['id']}</td><td>{_escape(r['authority_slug'])}</td><td><a href='/requests/{r['id']}'>{_escape(r['title'])}</a></td>"
            f"<td><form method='post' action='/requests/{r['id']}/status' style='display:flex;gap:0.4rem;align-items:center;'><select name='status'>{_status_options(r['status'])}</select><button type='submit'>Save</button></form></td>"
            f"<td>{_escape(str(r['fyi_request_id'] or ''))}</td><td>{_escape(r['tags'] or '')}</td><td>{_escape(r['updated_at'] or '')}</td><td><span class='pill'>{_escape(action['action_bucket'])}</span></td><td><a href='/requests/{r['id']}'>View</a> · <a href='/requests/{r['id']}/edit'>Edit</a> · <a href='/requests/{r['id']}/timeline'>Timeline</a> · <a href='/requests/{r['id']}/correspondence'>Correspondence</a> · <a href='{_escape(action['open_draft_path'])}'>Open recommended draft</a></td></tr>"
        )
    rows_html = "".join(rows_html_parts) or "<tr><td colspan='9' class='muted'>No tracked requests yet.</td></tr>"
    flash_html = f"<div class='success'>{_escape(flash)}</div>" if flash else ""
    body = f"""
    <h1>Tracked requests</h1>
    {flash_html}
    <form method='get' action='/requests' class='card'>
      <label for='q'>Search</label>
      <input type='text' id='q' name='q' value='{_escape(q)}' placeholder='title, authority, tag'>
      <label for='priority'>Priority</label>
      <select id='priority' name='priority'><option value=''>all</option><option value='now'{' selected' if priority=='now' else ''}>now</option><option value='soon'{' selected' if priority=='soon' else ''}>soon</option></select>
      <button type='submit'>Search</button>
    </form>
    <div class='card'>
      <table>
        <thead><tr><th>ID</th><th>Authority</th><th>Title</th><th>Status</th><th>FYI</th><th>Tags</th><th>Updated</th><th>Next action</th><th>Actions</th></tr></thead>
        <tbody>{rows_html}</tbody>
      </table>
    </div>
    """
    return _layout("Tracked requests", body)


def _render_request_form(db_path: str, request_row=None, flash: str = "") -> str:
    editing = request_row is not None
    title = request_row["title"] if editing else ""
    authority_slug = request_row["authority_slug"] if editing else ""
    body_val = request_row["body"] if editing else ""
    tags = request_row["tags"] if editing else ""
    status = request_row["status"] if editing else "draft"
    fyi_request_id = request_row["fyi_request_id"] if editing else ""
    fyi_url = request_row["fyi_url"] if editing else ""
    action = f"/requests/{request_row['id']}/update" if editing else "/requests/create"
    legend = f"Edit tracked request #{request_row['id']}" if editing else "Create tracked request"
    flash_html = f"<div class='success'>{_escape(flash)}</div>" if flash else ""
    preview = ""
    if authority_slug and title and body_val:
        preview_url = build_prefilled_url(authority_slug, title, body_val, tags=[t.strip() for t in tags.split(",") if t.strip()])
        preview = f"<p><a href='{_escape(preview_url)}' target='_blank' rel='noreferrer'>Open FYI prefilled draft</a></p>"
    body = f"""
    <h1>{legend}</h1>
    {flash_html}
    <form method='post' action='{action}' class='card'>
      <label for='authority_slug'>Authority</label>
      <select id='authority_slug' name='authority_slug'>{_authority_options(db_path, authority_slug)}</select>
      <label for='title'>Title</label>
      <input type='text' id='title' name='title' value='{_escape(title)}' required>
      <label for='body'>Body</label>
      <textarea id='body' name='body' required>{_escape(body_val)}</textarea>
      <label for='tags'>Tags (comma-separated)</label>
      <input type='text' id='tags' name='tags' value='{_escape(tags)}'>
      <label for='status'>Status</label>
      <select id='status' name='status'>{_status_options(status)}</select>
      <label for='fyi_request_id'>FYI request ID</label>
      <input type='number' id='fyi_request_id' name='fyi_request_id' value='{_escape(str(fyi_request_id or ''))}'>
      <label for='fyi_url'>FYI request URL</label>
      <input type='text' id='fyi_url' name='fyi_url' value='{_escape(fyi_url or '')}'>
      <button type='submit'>{'Save changes' if editing else 'Create request'}</button>
    </form>
    <div class='card'>
      <h2>FYI draft preview</h2>
      <p class='muted'>This uses FYI's documented prefilled request URL pattern.</p>
      {preview}
    </div>
    """
    return _layout(legend, body)


def _render_recommended_draft(db_path: str, request_id: int, strategy: str | None = None, tone: str = "neutral") -> str:
    row = get_tracked_request(db_path, request_id)
    if not row:
        return _layout("Not found", "<h1>Request not found</h1>")
    nba = next_best_action(db_path, request_id, tone=tone)
    strategy = strategy or nba.get("recommended_strategy") or "polite_nudge"
    selected = select_correspondence_variant(db_path, request_id, strategy, tone)
    body = f"""
    <h1>Recommended draft for request #{request_id}</h1>
    <div class='card'>
      <p><strong>{_escape(row['title'])}</strong><br><span class='muted'>{_escape(row['authority_slug'])} · {_escape(row['status'])}</span></p>
      <p><span class='pill'>{_escape(strategy)}</span> <span class='pill'>{_escape(tone)}</span></p>
      <p class='muted'>{_escape(nba.get('recommendation') or '')}</p>
      <div class='actions'>
        <a class='btn-link' href='/requests/{request_id}/correspondence'>Back to correspondence</a>
        <a class='btn-link' href='/requests/{request_id}'>Back to request</a>
      </div>
    </div>
    <div class='card'>
      <h2>{_escape(selected.get('subject') or '')}</h2>
      <pre>{_escape(selected.get('body') or '')}</pre>
    </div>
    """
    return _layout(f"Recommended draft #{request_id}", body)


def _render_request_detail(db_path: str, request_id: int, flash: str = "") -> str:
    row = get_tracked_request(db_path, request_id)
    if not row:
        return _layout("Not found", "<h1>Request not found</h1>")
    flash_html = f"<div class='success'>{_escape(flash)}</div>" if flash else ""
    snapshot = latest_snapshot_summary(db_path, row["fyi_request_id"])
    preview = ""
    if row["authority_slug"] and row["title"] and row["body"]:
        preview_url = build_prefilled_url(row["authority_slug"], row["title"], row["body"], tags=[t.strip() for t in (row["tags"] or "").split(",") if t.strip()])
        preview = f"<a href='{_escape(preview_url)}' target='_blank' rel='noreferrer'>Open FYI prefilled draft</a>"
    fyi_link = f"<a href='{_escape(row['fyi_url'])}' target='_blank' rel='noreferrer'>Open tracked FYI URL</a>" if row["fyi_url"] else ""
    snapshot_html = "<p class='muted'>No FYI request snapshot stored yet.</p>"
    attachments_html = "<li class='muted'>No attachments detected.</li>"
    events_html = "<li class='muted'>No snapshot events detected.</li>"
    follow_up = suggest_follow_up(db_path, request_id)
    followup_html = f"<p><strong>{_escape(follow_up['subject'])}</strong></p><pre>{_escape(follow_up['body'])}</pre><p class='muted'>{_escape(follow_up['rationale'])}</p>"
    analysis = response_analysis(db_path, request_id)
    nba = next_best_action(db_path, request_id)
    analysis_html = f"<p><strong>Tracked status:</strong> {_escape(str(analysis.get('tracked_status') or ''))}<br><strong>Snapshot state:</strong> {_escape(str(analysis.get('snapshot_state') or ''))}<br><strong>Attachments:</strong> {_escape(str(analysis.get('attachments_count') or 0))}<br><strong>Events:</strong> {_escape(str(analysis.get('events_count') or 0))}</p><p class='muted'>{_escape(analysis.get('recommendation') or '')}</p>"
    checklist_html = "".join(f"<li>{_escape(item)}</li>" for item in nba.get("checklist", [])) or "<li class='muted'>No checklist available.</li>"
    nba_html = f"<p><span class='pill'>{_escape(nba.get('action_bucket') or '')}</span> <span class='pill'>{_escape(nba.get('priority') or '')}</span></p><p><strong>Recommended strategy:</strong> {_escape(nba.get('recommended_strategy') or '')} / {_escape(nba.get('recommended_tone') or '')}</p><p class='muted'>{_escape(nba.get('recommendation') or '')}</p><ul>{checklist_html}</ul><details><summary>{_escape(nba.get('subject') or 'Draft')}</summary><pre>{_escape(nba.get('body') or '')}</pre></details><div class='actions'><a class='btn-link' href='{_escape(nba.get('open_draft_path') or f'/requests/{request_id}/recommended-draft')}'>Open recommended draft</a><a class='btn-link' href='/requests/{request_id}/correspondence'>Open correspondence pack</a><a class='btn-link' href='/requests/{request_id}/export-bundle'>Export bundle</a></div>"
    variants = follow_up_variants(db_path, request_id)
    variants_html = "".join(
        f"<li><strong>{_escape(v.get('strategy') or '')}</strong>: {_escape(v.get('why') or '')}<br><details><summary>{_escape(v.get('subject') or '')}</summary><pre>{_escape(v.get('body') or '')}</pre></details></li>"
        for v in variants.get("variants", [])
    ) or "<li class='muted'>No alternative variants generated.</li>"
    pack = follow_up_pack(db_path, request_id)
    pack_html = "".join(
        f"<li><strong>{_escape(item.get('strategy') or '')}</strong> / {_escape(item.get('tone') or '')}<br><a href='/requests/{request_id}/recommended-draft?strategy={_escape(item.get('strategy') or '')}&tone={_escape(item.get('tone') or '')}'>Open this draft</a><details><summary>{_escape(item.get('subject') or '')}</summary><pre>{_escape(item.get('body') or '')}</pre></details></li>"
        for item in pack.get("items", [])[:9]
    ) or "<li class='muted'>No strategy-and-tone pack generated.</li>"
    if snapshot:
        attachments_html = "".join(
            f"<li><a href='{_escape(a.get('url') or '')}' target='_blank' rel='noreferrer'>{_escape(a.get('name') or 'attachment')}</a> <span class='muted'>{_escape(a.get('content_type') or '')}</span></li>"
            for a in snapshot.get("attachments", [])[:20]
        ) or attachments_html
        events_html = "".join(
            f"<li><strong>{_escape(e.get('title') or 'event')}</strong> <span class='muted'>{_escape(e.get('created_at') or '')}</span><br>{_escape(e.get('detail') or '')}</li>"
            for e in snapshot.get("events", [])[:20]
        ) or events_html
        snapshot_html = f"""
        <p><strong>Latest snapshot:</strong> {_escape(snapshot.get('fetched_at') or '')}</p>
        <p><strong>State:</strong> {_escape(snapshot.get('described_state') or '')}</p>
        <p><strong>Attachments detected:</strong> {snapshot.get('attachments_count', 0)}<br><strong>Events detected:</strong> {snapshot.get('events_count', 0)}</p>
        <p><a href='{_escape(snapshot.get('source_url') or '')}' target='_blank' rel='noreferrer'>Open snapshot source URL</a></p>
        """
    body = f"""
    <h1>Request #{request_id}</h1>
    {flash_html}
    <div class='grid'>
      <div class='card'>
        <h2>Tracked request</h2>
        <p><strong>Title</strong><br>{_escape(row['title'])}</p>
        <p><strong>Authority</strong><br>{_escape(row['authority_slug'])}</p>
        <p><strong>Status</strong><br>{_escape(row['status'])}</p>
        <p><strong>Tags</strong><br>{_escape(row['tags'] or '')}</p>
        <p><strong>FYI request ID</strong><br>{_escape(str(row['fyi_request_id'] or ''))}</p>
        <p><strong>Body</strong><br>{_escape(row['body'])}</p>
        <p>{preview}</p>
        <p>{fyi_link}</p>
        <p><a href='/requests/{request_id}/edit'>Edit tracked request</a> · <a href='/requests/{request_id}/timeline'>Timeline</a> · <a href='/requests/{request_id}/export-bundle'>Export bundle</a></p>
      </div>
      <div class='card'>
        <h2>Latest FYI snapshot</h2>
        {snapshot_html}
      </div>
    </div>
    <div class='grid'>
      <div class='card'>
        <h2>Attachments</h2>
        <ul>{attachments_html}</ul>
      </div>
      <div class='card'>
        <h2>Snapshot events</h2>
        <ul>{events_html}</ul>
      </div>
    </div>
    <div class='grid'>
      <div class='card'>
        <h2>Response analysis</h2>
        {analysis_html}
      </div>
      <div class='card'>
        <h2>Suggested follow-up draft</h2>
        {followup_html}
      </div>
    </div>
    <div class='card'>
      <h2>Next best action</h2>
      {nba_html}
    </div>
    <div class='card'>
      <h2>Alternative follow-up variants</h2>
      <ul>{variants_html}</ul>
    </div>
    <div class='card'>
      <h2>Strategy and tone pack</h2>
      <ul>{pack_html}</ul>
    </div>
    """
    return _layout(f"Request #{request_id}", body)


def _render_correspondence(db_path: str, request_id: int, flash: str = "") -> str:
    row = get_tracked_request(db_path, request_id)
    if not row:
        return _layout("Not found", "<h1>Request not found</h1>")
    pack = correspondence_pack(db_path, request_id)
    recommended = pack["recommended_action"]
    flash_html = f"<div class='success'>{_escape(flash)}</div>" if flash else ""
    strategy_html = []
    for strategy, items in pack.get("strategies", {}).items():
        blocks = []
        for item in items:
            open_link = f"/requests/{request_id}/recommended-draft?strategy={item.get('strategy')}&tone={item.get('tone')}"
            blocks.append(f"<li><strong>{_escape(item.get('tone') or '')}</strong> · <a href='{_escape(open_link)}'>Open this draft</a><br><details><summary>{_escape(item.get('subject') or '')}</summary><pre>{_escape(item.get('body') or '')}</pre></details></li>")
        strategy_html.append(f"<div class='card'><h2>{_escape(strategy)}</h2><ul>{''.join(blocks)}</ul></div>")
    body = f"""
    <h1>Correspondence pack for request #{request_id}</h1>
    {flash_html}
    <div class='card'>
      <p><strong>{_escape(row['title'])}</strong><br><span class='muted'>{_escape(row['authority_slug'])} · {_escape(row['status'])}</span></p>
      <p><span class='pill'>{_escape(recommended.get('action_bucket') or '')}</span> <span class='pill'>{_escape(recommended.get('priority') or '')}</span></p>
      <p><strong>Recommended:</strong> {_escape(recommended.get('recommended_strategy') or '')} / {_escape(recommended.get('recommended_tone') or '')}</p>
      <p class='muted'>{_escape(recommended.get('recommendation') or '')}</p>
      <div class='actions'><a class='btn-link' href='{_escape(recommended.get('open_draft_path') or f'/requests/{request_id}/recommended-draft')}'>Open recommended draft</a><a class='btn-link' href='/requests/{request_id}/export-bundle'>Export bundle</a><a class='btn-link' href='/requests/{request_id}'>Back to request</a></div>
    </div>
    {''.join(strategy_html)}
    """
    return _layout(f"Correspondence #{request_id}", body)


def _render_authorities(db_path: str, q: str = "", flash: str = "") -> str:
    params = []
    sql = "SELECT slug, name, url FROM authorities"
    if q:
        sql += " WHERE lower(name) LIKE ? OR lower(slug) LIKE ?"
        wildcard = f"%{q.lower()}%"
        params = [wildcard, wildcard]
    sql += " ORDER BY name"
    rows = query_all(db_path, sql, params)
    links = []
    for r in rows:
        link_html = f"<a href='{_escape(r['url'])}' target='_blank' rel='noreferrer'>link</a>" if r["url"] else ""
        links.append(f"<tr><td>{_escape(r['slug'])}</td><td>{_escape(r['name'])}</td><td>{link_html}</td></tr>")
    rows_html = "".join(links) or "<tr><td colspan='3' class='muted'>No authorities loaded yet.</td></tr>"
    flash_html = f"<div class='success'>{_escape(flash)}</div>" if flash else ""
    body = f"""
    <h1>Authorities</h1>
    {flash_html}
    <form method='get' action='/authorities' class='card'>
      <label for='q'>Search</label>
      <input type='text' id='q' name='q' value='{_escape(q)}' placeholder='authority name or slug'>
      <button type='submit'>Search</button>
    </form>
    <div class='card'>
      <table>
        <thead><tr><th>Slug</th><th>Name</th><th>URL</th></tr></thead>
        <tbody>{rows_html}</tbody>
      </table>
    </div>
    """
    return _layout("Authorities", body)


def _render_import_form(message: str = "") -> str:
    message_html = f"<div class='success'>{_escape(message)}</div>" if message else ""
    body = f"""
    <h1>Import authorities</h1>
    {message_html}
    <form method='post' action='/authorities/import' enctype='multipart/form-data' class='card'>
      <label for='file'>CSV file</label>
      <input type='file' id='file' name='file'>
      <button type='submit'>Import CSV</button>
    </form>
    """
    return _layout("Import authorities", body)


def _render_timeline(db_path: str, request_id: int, flash: str = "") -> str:
    row = get_tracked_request(db_path, request_id)
    if not row:
        return _layout("Not found", "<h1>Request not found</h1>")
    items = request_timeline(db_path, request_id)
    rows = "".join(
        f"<tr><td>{_escape(ev.get('ts') or '')}</td><td>{_escape(ev.get('kind') or '')}</td><td>{_escape(ev.get('title') or '')}</td><td>{_escape(ev.get('detail') or '')}</td></tr>"
        for ev in items
    )
    flash_html = f"<div class='success'>{_escape(flash)}</div>" if flash else ""
    body = f"""
    <h1>Timeline for request #{request_id}</h1>
    {flash_html}
    <div class='card'><p><strong>{_escape(row['title'])}</strong><br><span class='muted'>{_escape(row['authority_slug'])}</span></p><p><a href='/requests/{request_id}'>Back to request</a></p></div>
    <div class='card'><table><thead><tr><th>When</th><th>Kind</th><th>Title</th><th>Detail</th></tr></thead><tbody>{rows}</tbody></table></div>
    """
    return _layout(f"Timeline #{request_id}", body)


def _parse_multipart_upload(content_type: str, body: bytes) -> bytes:
    marker = "boundary="
    if marker not in content_type:
        return b""
    boundary = content_type.split(marker, 1)[1].encode("utf-8")
    parts = body.split(b"--" + boundary)
    for part in parts:
        if b"Content-Disposition" in part and b'name="file"' in part:
            _header, _sep, payload = part.partition(b"\r\n\r\n")
            return payload.rstrip(b"\r\n-")
    return b""


def _apply_security_headers(handler: BaseHTTPRequestHandler):
    handler.send_header('Cache-Control', 'no-store')
    handler.send_header('Pragma', 'no-cache')
    handler.send_header('X-Content-Type-Options', 'nosniff')
    handler.send_header('Referrer-Policy', 'no-referrer')
    handler.send_header('Content-Security-Policy', "default-src 'self'; img-src 'self' data:; style-src 'self' 'unsafe-inline'; base-uri 'self'; form-action 'self'")


def _redirect(handler: BaseHTTPRequestHandler, location: str):
    handler.send_response(303)
    handler.send_header("Location", location)
    _apply_security_headers(handler)
    handler.end_headers()


def _parse_post_fields(handler: BaseHTTPRequestHandler) -> dict[str, str]:
    content_type = handler.headers.get("Content-Type", "")
    length = int(handler.headers.get("Content-Length", "0") or "0")
    body = handler.rfile.read(length)
    if content_type.startswith("application/json"):
        return json.loads(body.decode("utf-8")) if body else {}
    if content_type.startswith("multipart/form-data"):
        payload = _parse_multipart_upload(content_type, body)
        return {"file": payload.decode("utf-8", errors="replace")}
    parsed = parse_qs(body.decode("utf-8"), keep_blank_values=True)
    return {k: v[-1] for k, v in parsed.items()}


def make_handler(db_path: str):
    class Handler(BaseHTTPRequestHandler):
        def _json(self, payload: dict, status: int = 200):
            data = json.dumps(payload, ensure_ascii=False).encode("utf-8")
            self.send_response(status)
            self.send_header("Content-Type", "application/json; charset=utf-8")
            self.send_header("Content-Length", str(len(data)))
            _apply_security_headers(self)
            self.end_headers()
            self.wfile.write(data)

        def _html(self, html_text: str, status: int = 200):
            data = html_text.encode("utf-8")
            self.send_response(status)
            self.send_header("Content-Type", "text/html; charset=utf-8")
            self.send_header("Content-Length", str(len(data)))
            _apply_security_headers(self)
            self.end_headers()
            self.wfile.write(data)

        def do_GET(self):
            parsed = urlparse(self.path)
            qs = parse_qs(parsed.query)
            flash = (qs.get("flash") or [""])[0]
            if parsed.path == "/api/dashboard":
                return self._json(dashboard_payload(db_path))
            if parsed.path == "/":
                return self._html(_render_dashboard(db_path, flash=flash))
            if parsed.path == "/requests":
                return self._html(_render_requests(db_path, q=(qs.get("q") or [""])[0], flash=flash, priority=(qs.get("priority") or [""])[0]))
            if parsed.path == "/requests/new":
                return self._html(_render_request_form(db_path, flash=flash))
            if parsed.path == "/authorities":
                return self._html(_render_authorities(db_path, q=(qs.get("q") or [""])[0], flash=flash))
            if parsed.path == "/authorities/import":
                return self._html(_render_import_form(message=flash))
            if parsed.path.startswith("/requests/"):
                parts = [p for p in parsed.path.split("/") if p]
                try:
                    request_id = int(parts[1])
                except (ValueError, IndexError):
                    return self._json({"error": "not found"}, status=404)
                if len(parts) == 2:
                    row = get_tracked_request(db_path, request_id)
                    if not row:
                        return self._json({"error": "not found"}, status=404)
                    return self._html(_render_request_detail(db_path, request_id, flash=flash))
                if len(parts) == 3 and parts[2] == "timeline":
                    return self._html(_render_timeline(db_path, request_id, flash=flash))
                if len(parts) == 3 and parts[2] == "correspondence":
                    return self._html(_render_correspondence(db_path, request_id, flash=flash))
                if len(parts) == 3 and parts[2] == "edit":
                    row = get_tracked_request(db_path, request_id)
                    if not row:
                        return self._json({"error": "not found"}, status=404)
                    return self._html(_render_request_form(db_path, request_row=row, flash=flash))
                if len(parts) == 3 and parts[2] == "recommended-draft":
                    return self._html(_render_recommended_draft(db_path, request_id, strategy=(qs.get("strategy") or [None])[0], tone=(qs.get("tone") or ["neutral"])[0]))
                if len(parts) == 3 and parts[2] == "export-bundle":
                    out_dir = export_request_bundle(Path("outputs") / f"request-{request_id}-bundle", db_path, request_id)
                    return self._html(_render_request_detail(db_path, request_id, flash=f"Exported bundle to {out_dir}"))
            return self._json({"error": "not found"}, status=404)

        def do_POST(self):
            parsed = urlparse(self.path)
            if parsed.path == "/requests/create":
                fields = _parse_post_fields(self)
                insert_tracked_request(
                    db_path,
                    authority_slug=fields.get("authority_slug", ""),
                    title=fields.get("title", ""),
                    body=fields.get("body", ""),
                    tags=fields.get("tags", ""),
                    status=fields.get("status", "draft"),
                    fyi_request_id=int(fields["fyi_request_id"]) if fields.get("fyi_request_id") else None,
                    fyi_url=fields.get("fyi_url") or None,
                )
                return _redirect(self, "/requests?flash=Tracked+request+created")
            if parsed.path == "/authorities/import":
                length = int(self.headers.get("Content-Length", "0") or "0")
                raw = self.rfile.read(length) if length else b""
                payload = _parse_multipart_upload(self.headers.get("Content-Type", ""), raw)
                tmp = Path("outputs/uploaded-authorities.csv")
                tmp.parent.mkdir(parents=True, exist_ok=True)
                tmp.write_bytes(payload)
                count = import_authorities_csv(tmp, db_path=db_path)
                return _redirect(self, f"/authorities?flash=Imported+{count}+authorities")
            if parsed.path.startswith("/requests/"):
                parts = [p for p in parsed.path.split("/") if p]
                try:
                    request_id = int(parts[1])
                except (ValueError, IndexError):
                    return _redirect(self, "/")
                if len(parts) == 3 and parts[2] == "update":
                    fields = _parse_post_fields(self)
                    update_tracked_request(
                        db_path,
                        request_id,
                        authority_slug=fields.get("authority_slug", ""),
                        title=fields.get("title", ""),
                        body=fields.get("body", ""),
                        tags=fields.get("tags", ""),
                        status=fields.get("status", "draft"),
                        fyi_request_id=int(fields["fyi_request_id"]) if fields.get("fyi_request_id") else None,
                        fyi_url=fields.get("fyi_url") or None,
                    )
                    return _redirect(self, f"/requests/{request_id}?flash=Tracked+request+updated")
                if len(parts) == 3 and parts[2] == "status":
                    fields = _parse_post_fields(self)
                    update_request_status(db_path, request_id, fields.get("status", "draft"))
                    return _redirect(self, "/requests?flash=Status+updated")
            return _redirect(self, "/")

    return Handler


def serve(host: str = "127.0.0.1", port: int = 8000, db_path: str = "fyi_system.db"):
    init_db(db_path)
    Handler = make_handler(db_path)
    httpd = ThreadingHTTPServer((host, port), Handler)
    print(f"Serving on http://{host}:{port}")
    httpd.serve_forever()
