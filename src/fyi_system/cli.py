
from __future__ import annotations
import argparse
import json
from pathlib import Path
from .acquisition_receipts import AcquisitionRecorder, canonical_json_bytes
from .archive_capture import CaptureCaps, capture_request
from .archive_diff import run_diff
from .evidence_delta import emit_evidence_deltas
from .process_events import export_process_events, validate_process_event_file
from .archive_health import build_archive_health, write_archive_health
from .db import init_db, query_all, connect, get_tracked_request, export_tracked_requests, import_tracked_requests, request_timeline, update_request_status
from .discovery import backfill_ids, discover_feed, reconcile_discovery_files, shared_rate_limit_status, write_jsonl
from .agent_runtime import RetrievalPlan, agent_status_report, reflect_plan
from .fyi import build_prefilled_url
from .importers import (
    DEFAULT_AUTHORITIES_URL,
    discover_bodies_with_provenance,
    format_bodies_jsonl,
    import_authorities_csv,
    import_authorities_url,
)
from .internet_archive_cdx import CDX_ADAPTER_ID, CDX_ENDPOINT, CdxConfig, discover_cdx
from .monitor import ingest_feed, reconcile_events
from .fetch import fetch_request_page, summarize_request_json, latest_snapshot_summary
from .reporting import (
    write_attention_report,
    write_handover,
    suggest_follow_up,
    write_attachment_manifest,
    write_attachment_manifest_csv,
    follow_up_variants,
    follow_up_pack,
    response_analysis,
    triage_report,
    next_best_action,
    correspondence_pack,
    write_correspondence_pack,
    write_correspondence_pack_markdown,
    write_triage_report,
    export_request_bundle,
)
from .dashboard import write_dashboard
from .scheduler import run_scheduler, run_cycle
from .webapp import serve
from .security import load_settings, privacy_audit, secure_write_text


def _write_json_or_print(payload, output=None):
    if output:
        p = Path(output)
        secure_write_text(p, json.dumps(payload, indent=2, ensure_ascii=False))
        print(p)
    else:
        print(json.dumps(payload, indent=2, ensure_ascii=False))


def _recorder(
    args,
    *,
    adapter_id: str,
    source_url: str,
    request_bounds: dict,
    checkpoint_path: Path | None = None,
) -> AcquisitionRecorder | None:
    if not getattr(args, "receipt", None):
        return None
    return AcquisitionRecorder(
        command=args.cmd,
        adapter_id=adapter_id,
        source_url=source_url,
        request_bounds=request_bounds,
        rate_limit_name=getattr(args, "rate_limit_name", None),
        minimum_interval_seconds=getattr(args, "delay_seconds", None),
        checkpoint_path=checkpoint_path,
    )


def _write_receipt(args, recorder, result_projection: bytes, media_type: str) -> None:
    if recorder is not None:
        recorder.write(
            args.receipt,
            result_projection=result_projection,
            result_media_type=media_type,
        )


def _run_acquisition(args, recorder, operation):
    try:
        return operation()
    except Exception as error:
        if recorder is not None:
            recorder.write_failure(args.receipt, failure_type=type(error).__name__)
        raise


def cmd_init_db(args):
    init_db(args.db)
    print(f"Initialized {args.db}")


def cmd_import_authorities(args):
    if args.csv_path:
        if args.receipt:
            raise SystemExit("--receipt is only valid for network authority imports")
        n = import_authorities_csv(args.csv_path, db_path=args.db)
    else:
        recorder = _recorder(
            args,
            adapter_id="alaveteli-authority-catalog",
            source_url=args.source_url,
            request_bounds={"resource": "all-authorities.csv"},
        )
        n = _run_acquisition(
            args,
            recorder,
            lambda: import_authorities_url(args.source_url, db_path=args.db, recorder=recorder),
        )
        _write_receipt(
            args,
            recorder,
            canonical_json_bytes({"imported_authorities": n}),
            "application/json",
        )
    print(f"Imported {n} authorities")


def cmd_discover_bodies(args):
    source_url = args.catalog_url or f"{args.base_url.rstrip('/')}/body/all-authorities.csv"
    recorder = _recorder(
        args,
        adapter_id="alaveteli-authority-catalog",
        source_url=source_url,
        request_bounds={"resource": "authority_catalog"},
    )
    rows, provenance = _run_acquisition(
        args,
        recorder,
        lambda: discover_bodies_with_provenance(
            base_url=args.base_url,
            catalog_url=args.catalog_url,
            delay_seconds=args.delay_seconds,
            shared_rate_limit_db_path=args.db,
            shared_rate_limit_name=args.rate_limit_name,
            transport=None,
            recorder=recorder,
        ),
    )
    if args.format == "jsonl":
        rendered = format_bodies_jsonl(rows)
        if args.output:
            secure_write_text(Path(args.output), rendered)
            print(args.output)
        else:
            print(rendered, end="")
        _write_receipt(args, recorder, rendered.encode("utf-8"), "application/x-ndjson")
        return
    payload = {
        "base_url": args.base_url.rstrip("/"),
        "catalog_url": provenance["catalog_url"],
        "count": len(rows),
        "bodies": rows,
        "provenance": provenance,
    }
    _write_json_or_print(payload, args.output)
    rendered = canonical_json_bytes(payload)
    _write_receipt(args, recorder, rendered, "application/json")


def cmd_list_authorities(args):
    rows = query_all(args.db, 'SELECT slug, name, url FROM authorities ORDER BY name')
    for row in rows:
        print(f"{row['slug']}	{row['name']}	{row['url'] or ''}")


def cmd_register_request(args):
    conn = connect(args.db)
    try:
        conn.execute(
            'INSERT INTO tracked_requests(authority_slug, title, body, tags, status, fyi_request_id) VALUES (?, ?, ?, ?, ?, ?)',
            (args.authority_slug, args.title, args.body, args.tags or '', args.status, args.fyi_request_id),
        )
        conn.commit()
        print('Registered request')
    finally:
        conn.close()


def cmd_list_requests(args):
    rows = query_all(args.db, 'SELECT id, authority_slug, title, status, fyi_request_id FROM tracked_requests ORDER BY id')
    for row in rows:
        print(f"{row['id']}	{row['authority_slug']}	{row['title']}	{row['status']}	{row['fyi_request_id'] or ''}")


def cmd_build_prefilled_url(args):
    tags = [t for t in (args.tags or '').split(',') if t]
    print(build_prefilled_url(args.authority_slug, args.title, args.body, tags=tags, base_url=args.base_url))


def cmd_ingest_feed(args):
    print(ingest_feed(args.feed_url, db_path=args.db))


def cmd_reconcile(args):
    print(reconcile_events(db_path=args.db))


def cmd_fetch_request_page(args):
    recorder = _recorder(
        args,
        adapter_id="alaveteli-request-json",
        source_url=f"{args.base_url.rstrip('/')}/request/{args.request_id}.json",
        request_bounds={"request_id": args.request_id},
    )
    data = _run_acquisition(
        args,
        recorder,
        lambda: fetch_request_page(
            args.request_id,
            base_url=args.base_url,
            db_path=args.db,
            recorder=recorder,
        ),
    )
    summary = summarize_request_json(data)
    print(json.dumps(summary, indent=2, ensure_ascii=False))
    _write_receipt(args, recorder, canonical_json_bytes(summary), "application/json")


def cmd_discover(args):
    checkpoint_path = Path(args.checkpoint) if args.checkpoint else None
    bounds = {
        "mode": "numeric_ids" if args.backfill_ids else "search_feed",
        "date_from": args.date_from,
        "date_to": args.date_to,
        "authority": args.authority,
        "status": args.status,
        "max_pages": args.max_pages,
        "id_from": args.id_from,
        "id_to": args.id_to,
    }
    recorder = _recorder(
        args,
        adapter_id="alaveteli-numeric-id" if args.backfill_ids else "alaveteli-search-feed",
        source_url=args.base_url,
        request_bounds=bounds,
        checkpoint_path=checkpoint_path,
    )
    if args.backfill_ids:
        if args.id_from is None or args.id_to is None:
            raise SystemExit("--backfill-ids requires --id-from and --id-to")
        rows = _run_acquisition(
            args,
            recorder,
            lambda: backfill_ids(
                id_from=args.id_from,
                id_to=args.id_to,
                base_url=args.base_url,
                delay_seconds=args.delay_seconds,
                shared_rate_limit_db_path=args.db,
                shared_rate_limit_name=args.rate_limit_name,
                recorder=recorder,
            ),
        )
    else:
        rows = _run_acquisition(
            args,
            recorder,
            lambda: discover_feed(
                base_url=args.base_url,
                date_from=args.date_from,
                date_to=args.date_to,
                authority=args.authority,
                status=args.status,
                checkpoint_path=checkpoint_path,
                max_pages=args.max_pages,
                delay_seconds=args.delay_seconds,
                shared_rate_limit_db_path=args.db,
                shared_rate_limit_name=args.rate_limit_name,
                recorder=recorder,
            ),
        )
    rendered = ("\n".join(row.to_json() for row in rows) + "\n").encode("utf-8")
    if args.output:
        write_jsonl(Path(args.output), rows)
        print(args.output)
    else:
        for row in rows:
            print(row.to_json())
    _write_receipt(args, recorder, rendered, "application/x-ndjson")


def cmd_discover_reconcile(args):
    report = reconcile_discovery_files(Path(args.feed), Path(args.backfill))
    payload = report.to_dict()
    if args.output:
        Path(args.output).write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8")
        print(args.output)
    else:
        print(json.dumps(payload, indent=2, sort_keys=True))


def cmd_internet_archive_cdx(args):
    config = CdxConfig(
        url_pattern=args.url_pattern,
        allowed_host=args.allowed_host,
        pagination_mode=args.pagination_mode,
        capture_mode=args.capture_mode,
        page_size=args.page_size,
        max_pages=args.max_pages,
        max_rows=args.max_rows,
        max_runtime_seconds=args.max_runtime_seconds,
        max_stall_seconds=args.max_stall_seconds,
        from_timestamp=args.from_timestamp,
        to_timestamp=args.to_timestamp,
        include_urlkey=args.include_urlkey,
    )
    checkpoint = Path(args.checkpoint)
    recorder = _recorder(
        args,
        adapter_id=CDX_ADAPTER_ID,
        source_url=CDX_ENDPOINT,
        request_bounds=config.request_bounds(),
        checkpoint_path=checkpoint,
    )
    rows = _run_acquisition(
        args,
        recorder,
        lambda: discover_cdx(
            config,
            output_path=args.output,
            checkpoint_path=checkpoint,
            observer=recorder.observe_response if recorder is not None else None,
        ),
    )
    rendered = canonical_json_bytes(rows)
    _write_receipt(args, recorder, rendered, "application/json")
    print(args.output)


def cmd_rate_limit_status(args):
    payload = shared_rate_limit_status(args.db, name=args.name)
    if args.agent_memory:
        payload = payload or {}
        payload["agent"] = agent_status_report(args.agent_memory)
    if args.output:
        Path(args.output).write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8")
        print(args.output)
    else:
        print(json.dumps(payload, indent=2, sort_keys=True))


def cmd_dry_plan(args):
    """Reflect a bounded retrieval plan without making network calls."""
    plan = RetrievalPlan(
        instance_id=args.instance_id,
        description=args.description,
        estimated_requests=args.estimated_requests,
        date_from=args.date_from,
        date_to=args.date_to,
        max_pages=args.max_pages,
        recursive_unbounded=args.recursive_unbounded,
        is_heavy=args.heavy,
        force_schedule=args.force_schedule,
    )
    _write_json_or_print(reflect_plan(plan), args.output)


def cmd_archive_health(args):
    report = build_archive_health(
        discovered_path=Path(args.discovered),
        ledger_path=Path(args.ledger),
        manifest_path=Path(args.manifest),
        sync_state_path=Path(args.sync_state),
        db_path=Path(args.db),
        attachments_dir=Path(args.attachments_dir),
        wacz_dir=Path(args.wacz_dir),
        stale_after_days=args.stale_after_days,
    )
    if args.output:
        write_archive_health(Path(args.output), report)
        print(args.output)
    else:
        print(json.dumps(report, indent=2, sort_keys=True))


def cmd_capture(args):
    recorder = _recorder(
        args,
        adapter_id="alaveteli-request-capture",
        source_url=f"{args.base_url.rstrip('/')}/request/{args.request_ref}",
        request_bounds={"request_ref": str(args.request_ref)},
    )
    summary = _run_acquisition(
        args,
        recorder,
        lambda: capture_request(
            request_ref=str(args.request_ref),
            base_url=args.base_url,
            data_dir=Path(args.data_dir),
            dist_dir=Path(args.dist_dir),
            caps=CaptureCaps(
                max_bytes=args.max_bytes,
                max_runtime_minutes=args.max_runtime_minutes,
                max_disk_gb=args.max_disk_gb,
            ),
            recorder=recorder,
        ),
    )
    print(json.dumps(summary, indent=2, sort_keys=True))
    _write_receipt(args, recorder, canonical_json_bytes(summary), "application/json")


def cmd_diff(args):
    changes = run_diff(
        derived_dir=Path(args.derived_dir),
        previous_manifest=Path(args.previous_manifest),
        output_path=Path(args.output),
        cursor_path=Path(args.cursor) if args.cursor else None,
        sync_state_path=Path(args.sync_state) if args.sync_state else None,
        advance_cursor=args.advance_cursor,
        since=args.since,
    )
    print(json.dumps(changes, indent=2, sort_keys=True))


def cmd_emit_evidence_delta(args):
    if not args.experimental:
        raise SystemExit("EvidenceDelta emission is experimental; pass --experimental to enable it")
    deltas = emit_evidence_deltas(
        derived_dir=Path(args.derived_dir),
        output=Path(args.output),
        captured_at=args.captured_at,
        previous_manifest=Path(args.previous_manifest) if args.previous_manifest else None,
        instance_id=args.instance_id,
        jurisdiction=args.jurisdiction,
        source=args.source,
        partition=args.partition,
        base_url=args.base_url,
    )
    print(json.dumps({"output": args.output, "delta_count": len(deltas)}, sort_keys=True))


def cmd_export_process_events(args):
    result = export_process_events(
        derived_dir=Path(args.derived_dir),
        output=Path(args.output),
        captured_at=args.captured_at,
        checkpoint=Path(args.checkpoint) if args.checkpoint else None,
        instance_id=args.instance_id,
        source=args.source,
        base_url=args.base_url,
        attachments_output=Path(args.attachments_output) if args.attachments_output else None,
    )
    print(json.dumps(result, sort_keys=True))


def cmd_validate_process_events(args):
    result = validate_process_event_file(Path(args.input))
    result["input"] = args.input
    print(json.dumps(result, sort_keys=True))


def cmd_attention_report(args):
    print(write_attention_report(args.output, db_path=args.db))


def cmd_handover(args):
    print(write_handover(args.output, db_path=args.db))


def cmd_dashboard(args):
    print(write_dashboard(args.output, db_path=args.db, json_output=args.json_output))


def cmd_run_cycle(args):
    print(json.dumps(run_cycle(args.feed_url, db_path=args.db, outputs_dir=args.outputs_dir), indent=2))


def cmd_scheduler(args):
    run_scheduler(args.feed_url, interval_seconds=args.interval_seconds, db_path=args.db, outputs_dir=args.outputs_dir, once=args.once)


def cmd_serve(args):
    settings = load_settings(args.settings)
    host = args.host or settings.bind_host
    serve(host=host, port=args.port, db_path=args.db)


def cmd_export_requests(args):
    print(export_tracked_requests(args.db, args.output))


def cmd_import_requests(args):
    print(import_tracked_requests(args.db, args.input, replace=args.replace))


def cmd_request_timeline(args):
    print(json.dumps(request_timeline(args.db, args.request_id), indent=2, ensure_ascii=False))


def cmd_set_status(args):
    update_request_status(args.db, args.request_id, args.status)
    print(f"Updated request {args.request_id} to {args.status}")


def cmd_request_detail(args):
    row = get_tracked_request(args.db, args.request_id)
    if not row:
        raise SystemExit(f'Request {args.request_id} not found')
    payload = dict(row)
    payload['latest_snapshot'] = latest_snapshot_summary(args.db, row['fyi_request_id'])
    payload['follow_up'] = suggest_follow_up(args.db, args.request_id)
    payload['follow_up_variants'] = follow_up_variants(args.db, args.request_id)
    payload['follow_up_pack'] = follow_up_pack(args.db, args.request_id)
    payload['response_analysis'] = response_analysis(args.db, args.request_id)
    payload['next_best_action'] = next_best_action(args.db, args.request_id)
    print(json.dumps(payload, indent=2, ensure_ascii=False))


def cmd_export_request(args):
    row = get_tracked_request(args.db, args.request_id)
    if not row:
        raise SystemExit(f'Request {args.request_id} not found')
    _write_json_or_print(dict(row), args.output)


def cmd_follow_up_draft(args):
    _write_json_or_print(suggest_follow_up(args.db, args.request_id), args.output)


def cmd_attachment_manifest(args):
    print(write_attachment_manifest(args.output, args.db, args.request_id))


def cmd_follow_up_variants(args):
    _write_json_or_print(follow_up_variants(args.db, args.request_id), args.output)


def cmd_attachment_manifest_csv(args):
    print(write_attachment_manifest_csv(args.output, args.db, args.request_id))


def cmd_follow_up_pack(args):
    _write_json_or_print(follow_up_pack(args.db, args.request_id), args.output)


def cmd_triage_report(args):
    if args.output:
        print(write_triage_report(args.output, db_path=args.db))
    else:
        print(json.dumps(triage_report(args.db), indent=2, ensure_ascii=False))


def cmd_response_analysis(args):
    _write_json_or_print(response_analysis(args.db, args.request_id), args.output)


def cmd_next_best_action(args):
    _write_json_or_print(next_best_action(args.db, args.request_id, tone=args.tone), args.output)


def cmd_correspondence_pack(args):
    if args.format == 'markdown':
        print(write_correspondence_pack_markdown(args.output or 'outputs/correspondence-pack.md', args.db, args.request_id))
    elif args.output:
        print(write_correspondence_pack(args.output, args.db, args.request_id))
    else:
        print(json.dumps(correspondence_pack(args.db, args.request_id), indent=2, ensure_ascii=False))


def cmd_export_bundle(args):
    out_dir = args.output_dir or f'outputs/request-{args.request_id}-bundle'
    print(export_request_bundle(out_dir, args.db, args.request_id, sanitize=(not args.no_sanitize), profile=args.profile))


def cmd_show_settings(args):
    _write_json_or_print(load_settings(args.settings).__dict__, args.output)


def cmd_privacy_audit(args):
    settings = load_settings(args.settings)
    host = args.host or settings.bind_host
    _write_json_or_print(privacy_audit(args.db, host=host, outputs_dir=args.outputs_dir, profile=args.profile or settings.profile), args.output)


def build_parser():
    p = argparse.ArgumentParser(prog='fyi-system')
    sub = p.add_subparsers(dest='cmd', required=True)
    
    # Command: init-db
    sp = sub.add_parser('init-db')
    sp.add_argument('--db', default='fyi_system.db')
    sp.set_defaults(func=cmd_init_db)
    
    # Command: import-authorities
    sp = sub.add_parser('import-authorities')
    sp.add_argument('csv_path', nargs='?')
    sp.add_argument('--source-url', default=DEFAULT_AUTHORITIES_URL)
    sp.add_argument('--db', default='fyi_system.db')
    sp.add_argument('--receipt')
    sp.set_defaults(func=cmd_import_authorities)

    # Command: discover-bodies
    sp = sub.add_parser('discover-bodies')
    sp.add_argument('--base-url', default='https://fyi.org.nz')
    sp.add_argument('--catalog-url', default=None)
    sp.add_argument('--delay-seconds', type=float, default=1.0)
    sp.add_argument('--rate-limit-name', default='authority-discovery')
    sp.add_argument('--db', default='fyi_system.db')
    sp.add_argument('--output')
    sp.add_argument('--format', choices=('json', 'jsonl'), default='json')
    sp.add_argument('--receipt')
    sp.set_defaults(func=cmd_discover_bodies)
    
    # Command: list-authorities
    sp = sub.add_parser('list-authorities')
    sp.add_argument('--db', default='fyi_system.db')
    sp.set_defaults(func=cmd_list_authorities)
    
    # Command: register-request
    sp = sub.add_parser('register-request')
    sp.add_argument('authority_slug')
    sp.add_argument('title')
    sp.add_argument('body')
    sp.add_argument('--tags')
    sp.add_argument('--status', default='draft')
    sp.add_argument('--fyi-request-id', type=int)
    sp.add_argument('--db', default='fyi_system.db')
    sp.set_defaults(func=cmd_register_request)
    
    # Command: list-requests
    sp = sub.add_parser('list-requests')
    sp.add_argument('--db', default='fyi_system.db')
    sp.set_defaults(func=cmd_list_requests)
    
    # Command: set-status
    sp = sub.add_parser('set-status')
    sp.add_argument('request_id', type=int)
    sp.add_argument('status')
    sp.add_argument('--db', default='fyi_system.db')
    sp.set_defaults(func=cmd_set_status)
    
    # Command: request-timeline
    sp = sub.add_parser('request-timeline')
    sp.add_argument('request_id', type=int)
    sp.add_argument('--db', default='fyi_system.db')
    sp.set_defaults(func=cmd_request_timeline)
    
    # Command: export-requests
    sp = sub.add_parser('export-requests')
    sp.add_argument('--output', default='outputs/requests.json')
    sp.add_argument('--db', default='fyi_system.db')
    sp.set_defaults(func=cmd_export_requests)
    
    # Command: import-requests
    sp = sub.add_parser('import-requests')
    sp.add_argument('input')
    sp.add_argument('--replace', action='store_true')
    sp.add_argument('--db', default='fyi_system.db')
    sp.set_defaults(func=cmd_import_requests)
    
    # Command: build-prefilled-url
    sp = sub.add_parser('build-prefilled-url')
    sp.add_argument('authority_slug')
    sp.add_argument('title')
    sp.add_argument('body')
    sp.add_argument('--tags')
    sp.add_argument('--base-url', default='https://fyi.org.nz')
    sp.set_defaults(func=cmd_build_prefilled_url)
    
    # Command: ingest-feed
    sp = sub.add_parser('ingest-feed')
    sp.add_argument('feed_url')
    sp.add_argument('--db', default='fyi_system.db')
    sp.set_defaults(func=cmd_ingest_feed)
    
    # Command: reconcile-events
    sp = sub.add_parser('reconcile-events')
    sp.add_argument('--db', default='fyi_system.db')
    sp.set_defaults(func=cmd_reconcile)
    
    # Command: fetch-request-page
    sp = sub.add_parser('fetch-request-page')
    sp.add_argument('request_id', type=int)
    sp.add_argument('--base-url', default='https://fyi.org.nz')
    sp.add_argument('--db', default='fyi_system.db')
    sp.add_argument('--receipt')
    sp.set_defaults(func=cmd_fetch_request_page)

    # Command: discover
    sp = sub.add_parser('discover')
    sp.add_argument('--base-url', default='https://fyi.org.nz')
    sp.add_argument('--date-from')
    sp.add_argument('--date-to')
    sp.add_argument('--authority')
    sp.add_argument('--status')
    sp.add_argument('--checkpoint')
    sp.add_argument('--max-pages', type=int)
    sp.add_argument('--delay-seconds', type=float, default=1.0)
    sp.add_argument('--backfill-ids', action='store_true')
    sp.add_argument('--id-from', type=int)
    sp.add_argument('--id-to', type=int)
    sp.add_argument('--db', default='fyi_system.db')
    sp.add_argument('--rate-limit-name', default='archive-discovery')
    sp.add_argument('--output')
    sp.add_argument('--receipt')
    sp.set_defaults(func=cmd_discover)

    # Command: rate-limit-status
    sp = sub.add_parser('rate-limit-status')
    sp.add_argument('--db', default='fyi_system.db')
    sp.add_argument('--name', default='archive-discovery')
    sp.add_argument('--output')
    sp.add_argument('--agent-memory')
    sp.set_defaults(func=cmd_rate_limit_status)

    # Command: dry-plan (offline plan-and-solve reflection)
    sp = sub.add_parser('dry-plan')
    sp.add_argument('--instance-id', required=True)
    sp.add_argument('--description', default='retrieval')
    sp.add_argument('--estimated-requests', type=int, default=0)
    sp.add_argument('--date-from')
    sp.add_argument('--date-to')
    sp.add_argument('--max-pages', type=int)
    sp.add_argument('--recursive-unbounded', action='store_true')
    sp.add_argument('--heavy', action='store_true')
    sp.add_argument('--force-schedule', action='store_true')
    sp.add_argument('--output')
    sp.set_defaults(func=cmd_dry_plan)

    # Command: discover-reconcile
    sp = sub.add_parser('discover-reconcile')
    sp.add_argument('--feed', required=True)
    sp.add_argument('--backfill', required=True)
    sp.add_argument('--output')
    sp.set_defaults(func=cmd_discover_reconcile)

    # Command: internet-archive-cdx
    sp = sub.add_parser('internet-archive-cdx')
    sp.add_argument('--url-pattern', required=True)
    sp.add_argument('--allowed-host', required=True)
    sp.add_argument('--pagination-mode', choices=('page_count', 'resume_key'), default='resume_key')
    sp.add_argument('--capture-mode', choices=('url_index', 'all_captures'), default='url_index')
    sp.add_argument('--page-size', type=int, default=1000)
    sp.add_argument('--max-pages', type=int, default=100)
    sp.add_argument('--max-rows', type=int, default=1_000_000)
    sp.add_argument('--max-runtime-seconds', type=float, default=180.0)
    sp.add_argument('--max-stall-seconds', type=float)
    sp.add_argument('--from-timestamp')
    sp.add_argument('--to-timestamp')
    sp.add_argument('--include-urlkey', action='store_true')
    sp.add_argument('--output', required=True)
    sp.add_argument('--checkpoint', required=True)
    sp.add_argument('--receipt', required=True)
    sp.set_defaults(func=cmd_internet_archive_cdx)

    # Command: archive-health
    sp = sub.add_parser('archive-health')
    sp.add_argument('--discovered', default='data/_state/discovered_requests.jsonl')
    sp.add_argument('--ledger', default='data/_state/ledger.jsonl')
    sp.add_argument('--manifest', default='manifests/latest_manifest.json')
    sp.add_argument('--sync-state', default='data/_state/sync_state.json')
    sp.add_argument('--db', default='fyi_system.db')
    sp.add_argument('--attachments-dir', default='data/attachments')
    sp.add_argument('--wacz-dir', default='dist/site_snapshots')
    sp.add_argument('--stale-after-days', type=int, default=14)
    sp.add_argument('--output')
    sp.set_defaults(func=cmd_archive_health)

    # Command: capture
    sp = sub.add_parser('capture')
    sp.add_argument('request_ref')
    sp.add_argument('--base-url', default='https://fyi.org.nz')
    sp.add_argument('--data-dir', default='data')
    sp.add_argument('--dist-dir', default='dist')
    sp.add_argument('--max-bytes', type=int)
    sp.add_argument('--max-runtime-minutes', type=float)
    sp.add_argument('--max-disk-gb', type=float)
    sp.add_argument('--receipt')
    sp.set_defaults(func=cmd_capture)

    # Command: diff
    sp = sub.add_parser('diff')
    sp.add_argument('--derived-dir', default='data/raw/requests')
    sp.add_argument('--previous-manifest', default='manifests/latest_manifest.json')
    sp.add_argument('--output', default='manifests/latest_changes.json')
    sp.add_argument('--cursor')
    sp.add_argument('--sync-state', default='data/_state/sync_state.json')
    sp.add_argument('--since')
    sp.add_argument('--advance-cursor', action='store_true')
    sp.set_defaults(func=cmd_diff)

    # Command: emit-evidence-delta
    sp = sub.add_parser('emit-evidence-delta')
    sp.add_argument('--experimental', action='store_true')
    sp.add_argument('--derived-dir', default='data/raw/requests')
    sp.add_argument('--previous-manifest')
    sp.add_argument('--output', required=True)
    sp.add_argument('--captured-at', required=True)
    sp.add_argument('--instance-id', default='nz-fyi')
    sp.add_argument('--jurisdiction', default='NZ')
    sp.add_argument('--source', default='urn:fyi-cli:site:fyi.org.nz')
    sp.add_argument('--partition', default='requests')
    sp.add_argument('--base-url', default='https://fyi.org.nz')
    sp.set_defaults(func=cmd_emit_evidence_delta)

    # Command: export-process-events
    sp = sub.add_parser('export-process-events')
    sp.add_argument('--derived-dir', default='data/raw/requests')
    sp.add_argument('--output', required=True)
    sp.add_argument('--captured-at', required=True)
    sp.add_argument('--checkpoint')
    sp.add_argument('--instance-id', default='nz-fyi')
    sp.add_argument('--source', default='urn:fyi-cli:site:fyi.org.nz')
    sp.add_argument('--base-url', default='https://fyi.org.nz')
    sp.add_argument('--attachments-output')
    sp.set_defaults(func=cmd_export_process_events)

    # Command: validate-process-events
    sp = sub.add_parser('validate-process-events')
    sp.add_argument('--input', required=True)
    sp.set_defaults(func=cmd_validate_process_events)
    
    # Command: attention-report
    sp = sub.add_parser('attention-report')
    sp.add_argument('--output', default='outputs/attention-report.json')
    sp.add_argument('--db', default='fyi_system.db')
    sp.set_defaults(func=cmd_attention_report)
    
    # Command: handover
    sp = sub.add_parser('handover')
    sp.add_argument('--output', default='outputs/handover.md')
    sp.add_argument('--db', default='fyi_system.db')
    sp.set_defaults(func=cmd_handover)
    
    # Command: dashboard
    sp = sub.add_parser('dashboard')
    sp.add_argument('--output', default='outputs/dashboard.html')
    sp.add_argument('--json-output')
    sp.add_argument('--db', default='fyi_system.db')
    sp.set_defaults(func=cmd_dashboard)
    
    # Command: run-cycle
    sp = sub.add_parser('run-cycle')
    sp.add_argument('feed_url')
    sp.add_argument('--outputs-dir', default='outputs')
    sp.add_argument('--db', default='fyi_system.db')
    sp.set_defaults(func=cmd_run_cycle)
    
    # Command: scheduler
    sp = sub.add_parser('scheduler')
    sp.add_argument('feed_url')
    sp.add_argument('--interval-seconds', type=int, default=3600)
    sp.add_argument('--outputs-dir', default='outputs')
    sp.add_argument('--db', default='fyi_system.db')
    sp.add_argument('--once', action='store_true')
    sp.set_defaults(func=cmd_scheduler)
    
    # Command: serve
    sp = sub.add_parser('serve')
    sp.add_argument('--host')
    sp.add_argument('--port', type=int, default=8000)
    sp.add_argument('--settings')
    sp.add_argument('--db', default='fyi_system.db')
    sp.set_defaults(func=cmd_serve)
    
    # Command: request-detail
    sp = sub.add_parser('request-detail')
    sp.add_argument('request_id', type=int)
    sp.add_argument('--db', default='fyi_system.db')
    sp.set_defaults(func=cmd_request_detail)
    
    # Command: export-request
    sp = sub.add_parser('export-request')
    sp.add_argument('request_id', type=int)
    sp.add_argument('--output')
    sp.add_argument('--db', default='fyi_system.db')
    sp.set_defaults(func=cmd_export_request)
    
    # Command: follow-up-draft
    sp = sub.add_parser('follow-up-draft')
    sp.add_argument('request_id', type=int)
    sp.add_argument('--output')
    sp.add_argument('--db', default='fyi_system.db')
    sp.set_defaults(func=cmd_follow_up_draft)
    
    # Command: attachment-manifest
    sp = sub.add_parser('attachment-manifest')
    sp.add_argument('request_id', type=int)
    sp.add_argument('--output', default='outputs/attachment-manifest.json')
    sp.add_argument('--db', default='fyi_system.db')
    sp.set_defaults(func=cmd_attachment_manifest)
    
    # Command: attachment-manifest-csv
    sp = sub.add_parser('attachment-manifest-csv')
    sp.add_argument('request_id', type=int)
    sp.add_argument('--output', default='outputs/attachment-manifest.csv')
    sp.add_argument('--db', default='fyi_system.db')
    sp.set_defaults(func=cmd_attachment_manifest_csv)
    
    # Command: follow-up-variants
    sp = sub.add_parser('follow-up-variants')
    sp.add_argument('request_id', type=int)
    sp.add_argument('--output')
    sp.add_argument('--db', default='fyi_system.db')
    sp.set_defaults(func=cmd_follow_up_variants)
    
    # Command: follow-up-pack
    sp = sub.add_parser('follow-up-pack')
    sp.add_argument('request_id', type=int)
    sp.add_argument('--output')
    sp.add_argument('--db', default='fyi_system.db')
    sp.set_defaults(func=cmd_follow_up_pack)
    
    # Command: triage-report
    sp = sub.add_parser('triage-report')
    sp.add_argument('--output')
    sp.add_argument('--db', default='fyi_system.db')
    sp.set_defaults(func=cmd_triage_report)
    
    # Command: response-analysis
    sp = sub.add_parser('response-analysis')
    sp.add_argument('request_id', type=int)
    sp.add_argument('--output')
    sp.add_argument('--db', default='fyi_system.db')
    sp.set_defaults(func=cmd_response_analysis)
    
    # Command: next-best-action
    sp = sub.add_parser('next-best-action')
    sp.add_argument('request_id', type=int)
    sp.add_argument('--tone', default='neutral')
    sp.add_argument('--output')
    sp.add_argument('--db', default='fyi_system.db')
    sp.set_defaults(func=cmd_next_best_action)
    
    # Command: correspondence-pack
    sp = sub.add_parser('correspondence-pack')
    sp.add_argument('request_id', type=int)
    sp.add_argument('--format', choices=['json', 'markdown'], default='json')
    sp.add_argument('--output')
    sp.add_argument('--db', default='fyi_system.db')
    sp.set_defaults(func=cmd_correspondence_pack)
    
    # Command: export-bundle
    sp = sub.add_parser('export-bundle')
    sp.add_argument('request_id', type=int)
    sp.add_argument('--output-dir')
    sp.add_argument('--profile', choices=['standard', 'strict'], default='strict')
    sp.add_argument('--no-sanitize', action='store_true')
    sp.add_argument('--db', default='fyi_system.db')
    sp.set_defaults(func=cmd_export_bundle)
    
    # Command: show-settings
    sp = sub.add_parser('show-settings')
    sp.add_argument('--settings')
    sp.add_argument('--output')
    sp.set_defaults(func=cmd_show_settings)
    
    # Command: privacy-audit
    sp = sub.add_parser('privacy-audit')
    sp.add_argument('--db', default='fyi_system.db')
    sp.add_argument('--host')
    sp.add_argument('--outputs-dir', default='outputs')
    sp.add_argument('--profile', choices=['standard', 'strict'])
    sp.add_argument('--settings')
    sp.add_argument('--output')
    sp.set_defaults(func=cmd_privacy_audit)
    
    return p


def main():
    parser = build_parser()
    args = parser.parse_args()
    args.func(args)


if __name__ == '__main__':
    main()
