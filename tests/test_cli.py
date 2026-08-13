"""Tests for CLI argument parsing and commands."""
import pytest
from fyi_system.cli import build_parser, cmd_dry_plan, cmd_list_authorities, cmd_list_requests


class TestBuildParser:
    """Test CLI parser construction and argument parsing."""
    
    def test_build_parser_returns_parser(self):
        """Test that build_parser returns an ArgumentParser."""
        parser = build_parser()
        assert parser is not None
    
    def test_parser_has_required_subparsers(self):
        """Test that parser requires a subcommand."""
        parser = build_parser()
        with pytest.raises(SystemExit):
            parser.parse_args([])
    
    def test_init_db_command(self):
        """Test init-db command parsing."""
        parser = build_parser()
        args = parser.parse_args(['init-db', '--db', 'test.db'])
        assert args.cmd == 'init-db'
        assert args.db == 'test.db'

    def test_dry_plan_command_parsing(self):
        args = build_parser().parse_args([
            'dry-plan', '--instance-id', 'nz-fyi', '--recursive-unbounded',
            '--date-from', '2026-01-01', '--date-to', '2026-01-31',
        ])
        assert args.cmd == 'dry-plan'
        assert args.instance_id == 'nz-fyi'
        assert args.recursive_unbounded is True

    def test_discover_bodies_jsonl_contract_options(self):
        args = build_parser().parse_args([
            'discover-bodies',
            '--base-url', 'https://www.righttoknow.org.au',
            '--format', 'jsonl',
            '--rate-limit-name', 'archive-discovery-au-rtk',
        ])
        assert args.cmd == 'discover-bodies'
        assert args.format == 'jsonl'
        assert args.rate_limit_name == 'archive-discovery-au-rtk'

    @pytest.mark.parametrize(
        "arguments",
        [
            ["import-authorities", "--receipt", "receipt.json"],
            ["discover-bodies", "--receipt", "receipt.json"],
            ["fetch-request-page", "1", "--receipt", "receipt.json"],
            ["discover", "--max-pages", "1", "--receipt", "receipt.json"],
            ["capture", "1", "--receipt", "receipt.json"],
            [
                "internet-archive-cdx",
                "--url-pattern", "example.test/request/*",
                "--allowed-host", "example.test",
                "--output", "cdx.json",
                "--checkpoint", "checkpoint.json",
                "--receipt", "receipt.json",
            ],
            [
                "internet-archive-replay",
                "--selection", "selection.json",
                "--allowed-target-host", "example.test",
                "--output-dir", "objects",
                "--result", "result.json",
                "--checkpoint", "checkpoint.json",
                "--receipt", "receipt.json",
            ],
        ],
    )
    def test_network_acquisition_commands_accept_receipt(self, arguments):
        args = build_parser().parse_args(arguments)
        assert args.receipt == "receipt.json"

    def test_internet_archive_cdx_parses_bounded_modes(self):
        args = build_parser().parse_args([
            "internet-archive-cdx",
            "--url-pattern", "example.test/request/*",
            "--allowed-host", "example.test",
            "--pagination-mode", "page_count",
            "--max-pages", "4",
            "--max-rows", "20",
            "--output", "cdx.json",
            "--checkpoint", "checkpoint.json",
            "--receipt", "receipt.json",
        ])
        assert args.pagination_mode == "page_count"
        assert args.max_pages == 4
        assert args.max_rows == 20

    def test_internet_archive_replay_parses_bounded_controls(self):
        args = build_parser().parse_args([
            "internet-archive-replay",
            "--selection", "selection.json",
            "--allowed-target-host", "example.test",
            "--output-dir", "objects",
            "--result", "result.json",
            "--checkpoint", "checkpoint.json",
            "--receipt", "receipt.json",
            "--max-rows", "4",
            "--max-payload-bytes", "4096",
            "--max-redirects", "1",
        ])
        assert args.max_rows == 4
        assert args.max_payload_bytes == 4096
        assert args.max_redirects == 1

    def test_dry_plan_rejects_unbounded_without_network(self, capsys):
        args = build_parser().parse_args([
            'dry-plan', '--instance-id', 'nz-fyi', '--recursive-unbounded',
        ])
        cmd_dry_plan(args)
        assert 'reject' in capsys.readouterr().out

    def test_rate_limit_status_accepts_agent_memory(self):
        args = build_parser().parse_args([
            'rate-limit-status', '--agent-memory', 'agent-memory.json',
        ])
        assert args.agent_memory == 'agent-memory.json'
    
    def test_init_db_default_db(self):
        """Test init-db command uses default database."""
        parser = build_parser()
        args = parser.parse_args(['init-db'])
        assert args.db == 'fyi_system.db'
    
    def test_import_authorities_command(self):
        """Test import-authorities command parsing."""
        parser = build_parser()
        args = parser.parse_args(['import-authorities', 'data.csv'])
        assert args.cmd == 'import-authorities'
        assert args.csv_path == 'data.csv'

    def test_import_authorities_default_source_url(self):
        """Test import-authorities can use the official upstream CSV."""
        parser = build_parser()
        args = parser.parse_args(['import-authorities'])
        assert args.cmd == 'import-authorities'
        assert args.csv_path is None
        assert args.source_url == 'https://fyi.org.nz/body/all-authorities.csv'
    
    def test_list_authorities_command(self):
        """Test list-authorities command parsing."""
        parser = build_parser()
        args = parser.parse_args(['list-authorities', '--db', 'test.db'])
        assert args.cmd == 'list-authorities'
        assert args.db == 'test.db'
    
    def test_register_request_command(self):
        """Test register-request command parsing."""
        parser = build_parser()
        args = parser.parse_args([
            'register-request',
            'ministry',
            'Test Title',
            'Test Body',
            '--tags', 'tag1,tag2',
            '--status', 'pending',
            '--fyi-request-id', '123',
            '--db', 'test.db'
        ])
        assert args.cmd == 'register-request'
        assert args.authority_slug == 'ministry'
        assert args.title == 'Test Title'
        assert args.body == 'Test Body'
        assert args.tags == 'tag1,tag2'
        assert args.status == 'pending'
        assert args.fyi_request_id == 123
        assert args.db == 'test.db'
    
    def test_register_request_defaults(self):
        """Test register-request command defaults."""
        parser = build_parser()
        args = parser.parse_args([
            'register-request',
            'ministry',
            'Test Title',
            'Test Body'
        ])
        assert args.status == 'draft'
        assert args.fyi_request_id is None
        assert args.db == 'fyi_system.db'
    
    def test_set_status_command(self):
        """Test set-status command parsing."""
        parser = build_parser()
        args = parser.parse_args(['set-status', '1', 'pending'])
        assert args.cmd == 'set-status'
        assert args.request_id == 1
        assert args.status == 'pending'
    
    def test_request_timeline_command(self):
        """Test request-timeline command parsing."""
        parser = build_parser()
        args = parser.parse_args(['request-timeline', '1', '--db', 'test.db'])
        assert args.cmd == 'request-timeline'
        assert args.request_id == 1
    
    def test_export_requests_command(self):
        """Test export-requests command parsing."""
        parser = build_parser()
        args = parser.parse_args(['export-requests', '--output', 'out.json'])
        assert args.cmd == 'export-requests'
        assert args.output == 'out.json'
        assert args.output == 'out.json'
    
    def test_import_requests_command(self):
        """Test import-requests command parsing."""
        parser = build_parser()
        args = parser.parse_args(['import-requests', 'input.json', '--replace'])
        assert args.cmd == 'import-requests'
        assert args.input == 'input.json'
        assert args.replace is True
    
    def test_import_requests_no_replace(self):
        """Test import-requests without replace flag."""
        parser = build_parser()
        args = parser.parse_args(['import-requests', 'input.json'])
        assert args.replace is False
    
    def test_build_prefilled_url_command(self):
        """Test build-prefilled-url command parsing."""
        parser = build_parser()
        args = parser.parse_args([
            'build-prefilled-url',
            'ministry',
            'Test Title',
            'Test Body',
            '--tags', 'tag1',
            '--base-url', 'https://example.com'
        ])
        assert args.cmd == 'build-prefilled-url'
        assert args.base_url == 'https://example.com'
    
    def test_build_prefilled_url_default_base_url(self):
        """Test build-prefilled-url uses default base URL."""
        parser = build_parser()
        args = parser.parse_args([
            'build-prefilled-url',
            'ministry',
            'Test Title',
            'Test Body'
        ])
        assert args.base_url == 'https://fyi.org.nz'

    def test_discover_reconcile_command(self):
        """Test discover-reconcile command parsing."""
        parser = build_parser()
        args = parser.parse_args([
            'discover-reconcile',
            '--feed', 'feed.jsonl',
            '--backfill', 'backfill.jsonl',
            '--output', 'report.json',
        ])
        assert args.cmd == 'discover-reconcile'
        assert args.feed == 'feed.jsonl'
        assert args.backfill == 'backfill.jsonl'
        assert args.output == 'report.json'
    
    def test_serve_command(self):
        """Test serve command parsing."""
        parser = build_parser()
        args = parser.parse_args([
            'serve',
            '--host', '0.0.0.0',
            '--port', '9000',
            '--db', 'test.db'
        ])
        assert args.cmd == 'serve'
        assert args.host == '0.0.0.0'
        assert args.port == 9000
    
    def test_serve_default_port(self):
        """Test serve command uses default port."""
        parser = build_parser()
        args = parser.parse_args(['serve'])
        assert args.port == 8000
    
    def test_scheduler_command(self):
        """Test scheduler command parsing."""
        parser = build_parser()
        args = parser.parse_args([
            'scheduler',
            'https://example.com/feed',
            '--interval-seconds', '7200',
            '--once'
        ])
        assert args.cmd == 'scheduler'
        assert args.feed_url == 'https://example.com/feed'
        assert args.interval_seconds == 7200
        assert args.once is True
    
    def test_scheduler_no_once_flag(self):
        """Test scheduler without --once flag."""
        parser = build_parser()
        args = parser.parse_args([
            'scheduler',
            'https://example.com/feed'
        ])
        assert args.once is False
        assert args.interval_seconds == 3600
    
    def test_export_bundle_command(self):
        """Test export-bundle command parsing."""
        parser = build_parser()
        args = parser.parse_args([
            'export-bundle',
            '1',
            '--output-dir', 'output/',
            '--profile', 'standard',
            '--no-sanitize'
        ])
        assert args.cmd == 'export-bundle'
        assert args.request_id == 1
        assert args.output_dir == 'output/'
        assert args.profile == 'standard'
        assert args.no_sanitize is True
    
    def test_export_bundle_default_profile(self):
        """Test export-bundle uses strict profile by default."""
        parser = build_parser()
        args = parser.parse_args(['export-bundle', '1'])
        assert args.profile == 'strict'
        assert args.no_sanitize is False
    
    def test_correspondence_pack_command(self):
        """Test correspondence-pack command parsing."""
        parser = build_parser()
        args = parser.parse_args([
            'correspondence-pack',
            '1',
            '--format', 'markdown'
        ])
        assert args.cmd == 'correspondence-pack'
        assert args.format == 'markdown'
    
    def test_correspondence_pack_default_format(self):
        """Test correspondence-pack uses JSON by default."""
        parser = build_parser()
        args = parser.parse_args(['correspondence-pack', '1'])
        assert args.format == 'json'
    
    def test_privacy_audit_command(self):
        """Test privacy-audit command parsing."""
        parser = build_parser()
        args = parser.parse_args([
            'privacy-audit',
            '--host', 'localhost',
            '--outputs-dir', 'outputs/',
            '--profile', 'strict'
        ])
        assert args.cmd == 'privacy-audit'
        assert args.host == 'localhost'
        assert args.outputs_dir == 'outputs/'
        assert args.profile == 'strict'
    
    def test_all_commands_have_db_argument(self):
        """Test that all commands that need DB have db argument."""
        parser = build_parser()
        # Commands that should have --db argument
        db_commands = [
            'init-db', 'import-authorities', 'list-authorities',
            'register-request', 'list-requests', 'set-status',
            'request-timeline', 'export-requests', 'import-requests',
            'build-prefilled-url', 'ingest-feed', 'reconcile-events',
            'fetch-request-page', 'attention-report', 'handover',
            'dashboard', 'run-cycle', 'scheduler', 'serve',
            'request-detail', 'export-request', 'follow-up-draft',
            'attachment-manifest', 'attachment-manifest-csv',
            'follow-up-variants', 'follow-up-pack', 'triage-report',
            'response-analysis', 'next-best-action', 'correspondence-pack',
            'export-bundle', 'privacy-audit'
        ]
        
        for cmd in db_commands:
            try:
                args = parser.parse_args([cmd, '--help'])
            except SystemExit:
                pass  # --help always exits


class TestCommands:
    """Test CLI command functions."""
    
    def test_cmd_list_authorities_exists(self):
        """Test that cmd_list_authorities function exists."""
        assert callable(cmd_list_authorities)
    
    def test_cmd_list_requests_exists(self):
        """Test that cmd_list_requests function exists."""
        assert callable(cmd_list_requests)
    
    def test_cmd_init_db(self, tmp_path):
        """Test cmd_init_db creates database."""
        from fyi_system.cli import cmd_init_db
        db_path = tmp_path / "test.db"
        
        class Args:
            db = str(db_path)
        
        cmd_init_db(Args())
        assert db_path.exists()
    
    def test_cmd_list_authorities_empty(self, tmp_path):
        """Test cmd_list_authorities with empty database."""
        from fyi_system.cli import cmd_init_db, cmd_list_authorities
        from fyi_system.db import init_db
        db_path = tmp_path / "test.db"
        init_db(str(db_path))
        
        class Args:
            db = str(db_path)
        
        # Should not raise
        cmd_list_authorities(Args())
    
    def test_cmd_list_requests_empty(self, tmp_path):
        """Test cmd_list_requests with empty database."""
        from fyi_system.cli import cmd_list_requests
        from fyi_system.db import init_db
        db_path = tmp_path / "test.db"
        init_db(str(db_path))
        
        class Args:
            db = str(db_path)
        
        # Should not raise
        cmd_list_requests(Args())
    
    def test_cmd_register_request(self, tmp_path):
        """Test cmd_register_request creates request."""
        from fyi_system.cli import cmd_init_db, cmd_register_request
        from fyi_system.db import init_db, query_all
        db_path = tmp_path / "test.db"
        init_db(str(db_path))
        
        class Args:
            db = str(db_path)
            authority_slug = 'test'
            title = 'Test Request'
            body = 'Test body'
            tags = None
            status = 'draft'
            fyi_request_id = None
        
        cmd_register_request(Args())
        
        # Verify request was created
        requests = query_all(str(db_path), 'SELECT * FROM tracked_requests')
        assert len(requests) == 1
    
    def test_cmd_set_status(self, tmp_path):
        """Test cmd_set_status updates status."""
        from fyi_system.cli import cmd_init_db, cmd_register_request, cmd_set_status
        from fyi_system.db import init_db, get_tracked_request
        db_path = tmp_path / "test.db"
        init_db(str(db_path))
        
        # Create a request first
        class RegisterArgs:
            db = str(db_path)
            authority_slug = 'test'
            title = 'Test'
            body = 'Test'
            tags = None
            status = 'draft'
            fyi_request_id = None
        
        cmd_register_request(RegisterArgs())
        
        class StatusArgs:
            db = str(db_path)
            request_id = 1
            status = 'pending'
        
        cmd_set_status(StatusArgs())
        
        # Verify status was updated
        request = get_tracked_request(str(db_path), 1)
        assert request['status'] == 'pending'
    
    def test_cmd_export_requests(self, tmp_path):
        """Test cmd_export_requests exports to JSON."""
        from fyi_system.cli import cmd_init_db, cmd_register_request, cmd_export_requests
        from fyi_system.db import init_db
        db_path = tmp_path / "test.db"
        init_db(str(db_path))
        
        # Create a request first
        class RegisterArgs:
            db = str(db_path)
            authority_slug = 'test'
            title = 'Test'
            body = 'Test'
            tags = None
            status = 'draft'
            fyi_request_id = None
        
        cmd_register_request(RegisterArgs())
        
        output_path = tmp_path / "export.json"
        
        class ExportArgs:
            db = str(db_path)
            output = str(output_path)
        
        cmd_export_requests(ExportArgs())
        
        assert output_path.exists()
        import json
        data = json.loads(output_path.read_text())
        assert isinstance(data, list)
    
    def test_cmd_attention_report(self, tmp_path):
        """Test cmd_attention_report generates report."""
        from fyi_system.cli import cmd_init_db, cmd_attention_report
        from fyi_system.db import init_db
        db_path = tmp_path / "test.db"
        init_db(str(db_path))
        
        output_path = tmp_path / "report.json"
        
        class Args:
            db = str(db_path)
            output = str(output_path)
        
        cmd_attention_report(Args())
        
        assert output_path.exists()
    
    def test_cmd_handover(self, tmp_path):
        """Test cmd_handover generates handover document."""
        from fyi_system.cli import cmd_init_db, cmd_handover
        from fyi_system.db import init_db
        db_path = tmp_path / "test.db"
        init_db(str(db_path))
        
        output_path = tmp_path / "handover.md"
        
        class Args:
            db = str(db_path)
            output = str(output_path)
        
        cmd_handover(Args())
        
        assert output_path.exists()
    
    def test_cmd_dashboard(self, tmp_path):
        """Test cmd_dashboard generates HTML dashboard."""
        from fyi_system.cli import cmd_init_db, cmd_dashboard
        from fyi_system.db import init_db
        db_path = tmp_path / "test.db"
        init_db(str(db_path))
        
        output_path = tmp_path / "dashboard.html"
        
        class Args:
            db = str(db_path)
            output = str(output_path)
            json_output = None
        
        cmd_dashboard(Args())
        
        assert output_path.exists()
        content = output_path.read_text().strip()
        assert content.startswith('<!doctype html>')
    
    def test_cmd_request_detail(self, tmp_path):
        """Test cmd_request_detail outputs request details."""
        from fyi_system.cli import cmd_init_db, cmd_register_request, cmd_request_detail
        from fyi_system.db import init_db
        db_path = tmp_path / "test.db"
        init_db(str(db_path))
        
        # Create a request first
        class RegisterArgs:
            db = str(db_path)
            authority_slug = 'test'
            title = 'Test'
            body = 'Test'
            tags = None
            status = 'draft'
            fyi_request_id = None
        
        cmd_register_request(RegisterArgs())
        
        class DetailArgs:
            db = str(db_path)
            request_id = 1
        
        # Should not raise
        cmd_request_detail(DetailArgs())
    
    def test_cmd_build_prefilled_url(self, tmp_path, capsys):
        """Test cmd_build_prefilled_url outputs URL."""
        from fyi_system.cli import cmd_build_prefilled_url
        
        class Args:
            authority_slug = 'test'
            title = 'Test'
            body = 'Test body'
            tags = None
            base_url = 'https://fyi.org.nz'
        
        cmd_build_prefilled_url(Args())
        
        captured = capsys.readouterr()
        assert 'fyi.org.nz' in captured.out
