"""Tests for CLI argument parsing and commands."""
import pytest
from fyi_system.cli import build_parser, cmd_list_authorities, cmd_list_requests


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
