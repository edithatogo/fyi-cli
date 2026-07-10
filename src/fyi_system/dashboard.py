from __future__ import annotations

import json
from pathlib import Path
from typing import Any

from jinja2 import Environment, select_autoescape

from .db import query_all
from .reporting import attention_report, triage_report

HTML_TEMPLATE = Environment(
    autoescape=select_autoescape(enabled_extensions=("html", "xml")),
).from_string("""
<!doctype html>
<html>
<head>
  <meta charset='utf-8'>
  <title>FYI Request System Dashboard</title>
  <style>
    body { font-family: system-ui, sans-serif; margin: 2rem; color: #0f172a; }
    .cards { display: flex; gap: 1rem; flex-wrap: wrap; }
    .card { border: 1px solid #ddd; border-radius: 10px; padding: 1rem; min-width: 180px; }
    table { border-collapse: collapse; width: 100%; margin-top: 1rem; }
    th, td { border-bottom: 1px solid #eee; padding: 0.6rem; text-align: left; vertical-align: top; }
    .pill { display: inline-block; padding: 0.2rem 0.5rem; border-radius: 999px; background: #f1f5f9; }
    .muted { color: #475569; }
    .warn { background: #fff7ed; }
  </style>
</head>
<body>
  <h1>FYI Request System Dashboard</h1>
  <p class='muted'>Use the local web app for create/edit workflows; this page is the static operator summary.</p>
  <div class='cards'>
    <div class='card'><div>Total tracked</div><strong>{{ summary.total }}</strong></div>
    <div class='card'><div>Needs attention</div><strong>{{ summary.attention }}</strong></div>
    <div class='card'><div>Action now</div><strong>{{ summary.action_now }}</strong></div>
    <div class='card'><div>Authorities</div><strong>{{ summary.authorities }}</strong></div>
    <div class='card'><div>Recent updates (7d)</div><strong>{{ summary.recent_updates }}</strong></div>
  </div>
  <h2>Needs action now</h2>
  <table>
    <thead><tr><th>ID</th><th>Title</th><th>Status</th><th>State</th><th>Action</th></tr></thead>
    <tbody>
      {% for item in action_now %}
      <tr class='warn'>
        <td>{{ item.tracked_request_id }}</td>
        <td>{{ item.title }}</td>
        <td>{{ item.tracked_status }}</td>
        <td>{{ item.normalized_snapshot_state }}</td>
        <td>{{ item.action_bucket }}</td>
      </tr>
      {% endfor %}
      {% if not action_now %}
      <tr><td colspan='5' class='muted'>Nothing currently in the action-now queue.</td></tr>
      {% endif %}
    </tbody>
  </table>
  <h2>Tracked requests</h2>
  <table>
    <thead><tr><th>ID</th><th>Authority</th><th>Title</th><th>Status</th><th>FYI</th><th>Last event</th><th>Priority</th><th>Updated</th></tr></thead>
    <tbody>
      {% for item in items %}
      <tr class='{% if item.needs_attention %}warn{% endif %}'>
        <td>{{ item.id }}</td>
        <td>{{ item.authority_slug }}</td>
        <td>{{ item.title }}</td>
        <td><span class='pill'>{{ item.status }}</span></td>
        <td>{{ item.fyi_request_id or '' }}</td>
        <td>{{ item.last_event_title or '' }}</td>
        <td>{{ item.priority }}</td>
        <td>{{ item.updated_at or '' }}</td>
      </tr>
      {% endfor %}
    </tbody>
  </table>
</body>
</html>
""")


def dashboard_payload(db_path: str | Path = "fyi_system.db") -> dict[str, Any]:
    report = attention_report(db_path)
    triage = triage_report(db_path)
    authorities = query_all(db_path, 'SELECT COUNT(*) AS c FROM authorities')[0]['c']
    recent_updates = query_all(db_path, "SELECT COUNT(*) AS c FROM tracked_requests WHERE updated_at >= datetime('now', '-7 day')")[0]['c']
    rows = query_all(db_path, 'SELECT id, authority_slug, title, status, fyi_request_id, last_event_title, updated_at FROM tracked_requests ORDER BY updated_at DESC, id DESC')
    attn_by_id = {i['id']: i for i in report['items']}
    items = []
    for row in rows:
        item = dict(row)
        match = attn_by_id.get(item['id'], {})
        item['needs_attention'] = bool(match.get('needs_attention'))
        item['priority'] = match.get('priority', '')
        item['action_bucket'] = match.get('action_bucket', '')
        items.append(item)
    return {
        'summary': {
            'total': report['count'],
            'attention': sum(1 for i in report['items'] if i['needs_attention']),
            'action_now': triage['summary']['action_now'],
            'authorities': authorities,
            'recent_updates': recent_updates,
        },
        'action_now': triage['action_now'][:8],
        'items': items,
    }


def write_dashboard(html_output: str | Path, db_path: str | Path = 'fyi_system.db', json_output: str | Path | None = None) -> Path:
    payload = dashboard_payload(db_path)
    html_path = Path(html_output)
    html_path.parent.mkdir(parents=True, exist_ok=True)
    html_path.write_text(HTML_TEMPLATE.render(**payload), encoding='utf-8')
    if json_output:
        json_path = Path(json_output)
        json_path.parent.mkdir(parents=True, exist_ok=True)
        json_path.write_text(json.dumps(payload, indent=2, ensure_ascii=False), encoding='utf-8')
    return html_path
