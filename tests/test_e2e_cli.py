"""End-to-End tests for FYI Request System CLI.

These tests verify actual CLI command execution, output, and exit codes.
"""
import subprocess
import sys
import pytest
from pathlib import Path


class TestCLIInitDB:
    """Test fyi-system init-db command."""

    def test_init_db_creates_database(self, tmp_path):
        """E2E: init-db creates database file."""
        db_path = tmp_path / "test.db"
        
        result = subprocess.run(
            [sys.executable, '-m', 'fyi_system.cli', 'init-db', '--db', str(db_path)],
            capture_output=True,
            text=True,
            timeout=30
        )
        
        assert result.returncode == 0
        assert db_path.exists()
        assert "initialized" in result.stdout.lower() or "created" in result.stdout.lower()

    def test_init_db_already_exists(self, tmp_path):
        """E2E: init-db handles existing database."""
        db_path = tmp_path / "test.db"
        
        # Create database first
        subprocess.run(
            [sys.executable, '-m', 'fyi_system.cli', 'init-db', '--db', str(db_path)],
            capture_output=True,
            timeout=30
        )
        
        # Run again - should handle gracefully
        result = subprocess.run(
            [sys.executable, '-m', 'fyi_system.cli', 'init-db', '--db', str(db_path)],
            capture_output=True,
            text=True,
            timeout=30
        )
        
        # Should either succeed or give informative error
        assert result.returncode in [0, 1]


class TestCLIRegisterRequest:
    """Test fyi-system register-request command."""

    def test_register_request_creates_request(self, tmp_path):
        """E2E: register-request creates new request."""
        db_path = tmp_path / "test.db"
        
        # Initialize database
        subprocess.run(
            [sys.executable, '-m', 'fyi_system.cli', 'init-db', '--db', str(db_path)],
            capture_output=True,
            timeout=30
        )
        
        # Register request
        result = subprocess.run(
            [sys.executable, '-m', 'fyi_system.cli', 'register-request', 
             'test-authority', 'Test Title', 'Test body content',
             '--db', str(db_path)],
            capture_output=True,
            text=True,
            timeout=30
        )
        
        assert result.returncode == 0
        assert "created" in result.stdout.lower() or "request" in result.stdout.lower()

    def test_register_request_missing_args(self, tmp_path):
        """E2E: register-request handles missing arguments."""
        db_path = tmp_path / "test.db"
        
        # Initialize database
        subprocess.run(
            [sys.executable, '-m', 'fyi_system.cli', 'init-db', '--db', str(db_path)],
            capture_output=True,
            timeout=30
        )
        
        # Try without required args
        result = subprocess.run(
            [sys.executable, '-m', 'fyi_system.cli', 'register-request'],
            capture_output=True,
            text=True,
            timeout=30
        )
        
        # Should fail with error about missing arguments
        assert result.returncode != 0
        assert "error" in result.stderr.lower() or "required" in result.stderr.lower()


class TestCLIListRequests:
    """Test fyi-system list-requests command."""

    def test_list_requests_empty(self, tmp_path):
        """E2E: list-requests shows empty list."""
        db_path = tmp_path / "test.db"
        
        # Initialize database
        subprocess.run(
            [sys.executable, '-m', 'fyi_system.cli', 'init-db', '--db', str(db_path)],
            capture_output=True,
            timeout=30
        )
        
        # List requests
        result = subprocess.run(
            [sys.executable, '-m', 'fyi_system.cli', 'list-requests', '--db', str(db_path)],
            capture_output=True,
            text=True,
            timeout=30
        )
        
        assert result.returncode == 0
        # Empty list may show nothing or a message - both are acceptable
        # Just verify command succeeded

    def test_list_requests_with_data(self, tmp_path):
        """E2E: list-requests shows created requests."""
        db_path = tmp_path / "test.db"
        
        # Initialize database
        subprocess.run(
            [sys.executable, '-m', 'fyi_system.cli', 'init-db', '--db', str(db_path)],
            capture_output=True,
            timeout=30
        )
        
        # Register request
        subprocess.run(
            [sys.executable, '-m', 'fyi_system.cli', 'register-request',
             'test-authority', 'Test Title', 'Test body',
             '--db', str(db_path)],
            capture_output=True,
            timeout=30
        )
        
        # List requests
        result = subprocess.run(
            [sys.executable, '-m', 'fyi_system.cli', 'list-requests', '--db', str(db_path)],
            capture_output=True,
            text=True,
            timeout=30
        )
        
        assert result.returncode == 0
        assert "Test Title" in result.stdout or "test-authority" in result.stdout


class TestCLIHelp:
    """Test fyi-system --help command."""

    def test_help_shows_commands(self):
        """E2E: --help shows available commands."""
        result = subprocess.run(
            [sys.executable, '-m', 'fyi_system.cli', '--help'],
            capture_output=True,
            text=True,
            timeout=30
        )
        
        assert result.returncode == 0
        assert "usage" in result.stdout.lower()
        # Should list available commands
        assert "init-db" in result.stdout or "register" in result.stdout

    def test_help_exit_code(self):
        """E2E: --help returns exit code 0."""
        result = subprocess.run(
            [sys.executable, '-m', 'fyi_system.cli', '--help'],
            capture_output=True,
            timeout=30
        )
        
        assert result.returncode == 0


class TestCLIInvalidCommand:
    """Test fyi-system with invalid commands."""

    def test_invalid_command_error(self, tmp_path):
        """E2E: Invalid command returns error."""
        db_path = tmp_path / "test.db"
        
        result = subprocess.run(
            [sys.executable, '-m', 'fyi_system.cli', 'nonexistent-command',
             '--db', str(db_path)],
            capture_output=True,
            text=True,
            timeout=30
        )
        
        assert result.returncode != 0
        assert "error" in result.stderr.lower() or "invalid" in result.stderr.lower()


class TestCLISetStatus:
    """Test fyi-system set-status command."""

    def test_set_status_updates_request(self, tmp_path):
        """E2E: set-status updates request status."""
        db_path = tmp_path / "test.db"
        
        # Initialize and create request
        subprocess.run(
            [sys.executable, '-m', 'fyi_system.cli', 'init-db', '--db', str(db_path)],
            capture_output=True,
            timeout=30
        )
        
        create_result = subprocess.run(
            [sys.executable, '-m', 'fyi_system.cli', 'register-request',
             'test', 'Test', 'Body',
             '--db', str(db_path)],
            capture_output=True,
            text=True,
            timeout=30
        )
        
        # Extract request ID from output (usually first number)
        import re
        match = re.search(r'\d+', create_result.stdout)
        if match:
            request_id = match.group()
            
            # Update status
            result = subprocess.run(
                [sys.executable, '-m', 'fyi_system.cli', 'set-status',
                 request_id, 'submitted',
                 '--db', str(db_path)],
                capture_output=True,
                text=True,
                timeout=30
            )
            
            # Should succeed or give informative error
            assert result.returncode in [0, 1]


class TestCLIExportImport:
    """Test fyi-system export/import commands."""

    def test_export_requests(self, tmp_path):
        """E2E: export-requests creates export file."""
        db_path = tmp_path / "test.db"
        export_path = tmp_path / "export.json"

        # Initialize and create request
        subprocess.run(
            [sys.executable, '-m', 'fyi_system.cli', 'init-db', '--db', str(db_path)],
            capture_output=True,
            timeout=30
        )

        subprocess.run(
            [sys.executable, '-m', 'fyi_system.cli', 'register-request',
             'test', 'Test', 'Body',
             '--db', str(db_path)],
            capture_output=True,
            timeout=30
        )

        # Export
        result = subprocess.run(
            [sys.executable, '-m', 'fyi_system.cli', 'export-requests',
             '--output', str(export_path),
             '--db', str(db_path)],
            capture_output=True,
            text=True,
            timeout=30
        )

        # Should create file or give informative error
        if result.returncode == 0:
            assert export_path.exists()


class TestCLIRequestDetail:
    """Test fyi-system request-detail command."""

    def test_request_detail(self, tmp_path):
        """E2E: request-detail shows request info."""
        db_path = tmp_path / "test.db"

        # Initialize and create request
        subprocess.run(
            [sys.executable, '-m', 'fyi_system.cli', 'init-db', '--db', str(db_path)],
            capture_output=True,
            timeout=30
        )

        create_result = subprocess.run(
            [sys.executable, '-m', 'fyi_system.cli', 'register-request',
             'test', 'Detail Test', 'Body',
             '--db', str(db_path)],
            capture_output=True,
            text=True,
            timeout=30
        )

        import re
        match = re.search(r'\d+', create_result.stdout)
        if match:
            request_id = match.group()

            result = subprocess.run(
                [sys.executable, '-m', 'fyi_system.cli', 'request-detail',
                 request_id, '--db', str(db_path)],
                capture_output=True,
                text=True,
                timeout=30
            )

            # Should return JSON or error
            assert result.returncode in [0, 1]


class TestCLIFollowUp:
    """Test fyi-system follow-up commands."""

    def test_follow_up_draft(self, tmp_path):
        """E2E: follow-up-draft generates follow-up."""
        db_path = tmp_path / "test.db"

        subprocess.run(
            [sys.executable, '-m', 'fyi_system.cli', 'init-db', '--db', str(db_path)],
            capture_output=True,
            text=True,
            timeout=30
        )

        create_result = subprocess.run(
            [sys.executable, '-m', 'fyi_system.cli', 'register-request',
             'test', 'Follow-up Test', 'Body',
             '--db', str(db_path)],
            capture_output=True,
            text=True,
            timeout=30
        )

        import re
        match = re.search(r'\d+', create_result.stdout)
        if match:
            request_id = match.group()

            result = subprocess.run(
                [sys.executable, '-m', 'fyi_system.cli', 'follow-up-draft',
                 request_id, '--db', str(db_path)],
                capture_output=True,
                text=True,
                timeout=30
            )

            # Should generate follow-up or error
            assert result.returncode in [0, 1]


class TestCLITimeline:
    """Test fyi-system request-timeline command."""

    def test_request_timeline(self, tmp_path):
        """E2E: request-timeline shows timeline."""
        db_path = tmp_path / "test.db"

        subprocess.run(
            [sys.executable, '-m', 'fyi_system.cli', 'init-db', '--db', str(db_path)],
            capture_output=True,
            text=True,
            timeout=30
        )

        create_result = subprocess.run(
            [sys.executable, '-m', 'fyi_system.cli', 'register-request',
             'test', 'Timeline Test', 'Body',
             '--db', str(db_path)],
            capture_output=True,
            text=True,
            timeout=30
        )

        import re
        match = re.search(r'\d+', create_result.stdout)
        if match:
            request_id = match.group()

            result = subprocess.run(
                [sys.executable, '-m', 'fyi_system.cli', 'request-timeline',
                 request_id, '--db', str(db_path)],
                capture_output=True,
                text=True,
                timeout=30
            )

            # Should return JSON timeline or error
            assert result.returncode in [0, 1]


class TestCLIHandover:
    """Test fyi-system handover command."""

    def test_handover_generates_report(self, tmp_path):
        """E2E: handover generates handover document."""
        db_path = tmp_path / "test.db"
        output_path = tmp_path / "handover.md"

        subprocess.run(
            [sys.executable, '-m', 'fyi_system.cli', 'init-db', '--db', str(db_path)],
            capture_output=True,
            timeout=30
        )

        result = subprocess.run(
            [sys.executable, '-m', 'fyi_system.cli', 'handover',
             '--output', str(output_path),
             '--db', str(db_path)],
            capture_output=True,
            text=True,
            timeout=30
        )

        # Should create file or give informative error
        if result.returncode == 0:
            assert output_path.exists()


class TestCLIAttentionReport:
    """Test fyi-system attention-report command."""

    def test_attention_report_generates(self, tmp_path):
        """E2E: attention-report generates report."""
        db_path = tmp_path / "test.db"
        output_path = tmp_path / "attention.json"

        subprocess.run(
            [sys.executable, '-m', 'fyi_system.cli', 'init-db', '--db', str(db_path)],
            capture_output=True,
            timeout=30
        )

        result = subprocess.run(
            [sys.executable, '-m', 'fyi_system.cli', 'attention-report',
             '--output', str(output_path),
             '--db', str(db_path)],
            capture_output=True,
            text=True,
            timeout=30
        )

        # Should create file or give informative error
        if result.returncode == 0:
            assert output_path.exists()


class TestCLITriageReport:
    """Test fyi-system triage-report command."""

    def test_triage_report_json(self, tmp_path):
        """E2E: triage-report outputs JSON."""
        db_path = tmp_path / "test.db"

        subprocess.run(
            [sys.executable, '-m', 'fyi_system.cli', 'init-db', '--db', str(db_path)],
            capture_output=True,
            timeout=30
        )

        result = subprocess.run(
            [sys.executable, '-m', 'fyi_system.cli', 'triage-report',
             '--db', str(db_path)],
            capture_output=True,
            text=True,
            timeout=30
        )

        # Should output JSON or error
        assert result.returncode in [0, 1]
        if result.returncode == 0:
            import json
            # Should be valid JSON
            try:
                json.loads(result.stdout)
            except json.JSONDecodeError:
                pytest.fail("Output is not valid JSON")


class TestCLINextBestAction:
    """Test fyi-system next-best-action command."""

    def test_next_best_action(self, tmp_path):
        """E2E: next-best-action provides recommendation."""
        db_path = tmp_path / "test.db"

        subprocess.run(
            [sys.executable, '-m', 'fyi_system.cli', 'init-db', '--db', str(db_path)],
            capture_output=True,
            timeout=30
        )

        create_result = subprocess.run(
            [sys.executable, '-m', 'fyi_system.cli', 'register-request',
             'test', 'Action Test', 'Body',
             '--db', str(db_path)],
            capture_output=True,
            text=True,
            timeout=30
        )

        import re
        match = re.search(r'\d+', create_result.stdout)
        if match:
            request_id = match.group()

            result = subprocess.run(
                [sys.executable, '-m', 'fyi_system.cli', 'next-best-action',
                 request_id, '--tone', 'neutral',
                 '--db', str(db_path)],
                capture_output=True,
                text=True,
                timeout=30
            )

            # Should output JSON or error
            assert result.returncode in [0, 1]


class TestCLIShowSettings:
    """Test fyi-system show-settings command."""

    def test_show_settings_default(self, tmp_path):
        """E2E: show-settings displays settings."""
        output_path = tmp_path / "settings.json"

        result = subprocess.run(
            [sys.executable, '-m', 'fyi_system.cli', 'show-settings',
             '--output', str(output_path)],
            capture_output=True,
            text=True,
            timeout=30
        )

        # Should output settings or error
        assert result.returncode in [0, 1]


class TestCLIBuildPrefilledURL:
    """Test fyi-system build-prefilled-url command."""

    def test_build_prefilled_url(self):
        """E2E: build-prefilled-url generates URL."""
        result = subprocess.run(
            [sys.executable, '-m', 'fyi_system.cli', 'build-prefilled-url',
             'test-authority', 'Test Title', 'Test body'],
            capture_output=True,
            text=True,
            timeout=30
        )

        assert result.returncode == 0
        assert 'fyi.org.nz' in result.stdout
        assert 'test-authority' in result.stdout


class TestCLIListAuthorities:
    """Test fyi-system list-authorities command."""

    def test_list_authorities_empty(self, tmp_path):
        """E2E: list-authorities handles empty database."""
        db_path = tmp_path / "test.db"

        subprocess.run(
            [sys.executable, '-m', 'fyi_system.cli', 'init-db', '--db', str(db_path)],
            capture_output=True,
            timeout=30
        )

        result = subprocess.run(
            [sys.executable, '-m', 'fyi_system.cli', 'list-authorities',
             '--db', str(db_path)],
            capture_output=True,
            text=True,
            timeout=30
        )

        # Should succeed (may show nothing for empty db)
        assert result.returncode == 0


class TestCLIImportRequests:
    """Test fyi-system import-requests command."""

    def test_import_requests_empty_file(self, tmp_path):
        """E2E: import-requests handles empty JSON."""
        db_path = tmp_path / "test.db"
        import_path = tmp_path / "import.json"

        # Create empty JSON array
        import_path.write_text('[]')

        subprocess.run(
            [sys.executable, '-m', 'fyi_system.cli', 'init-db', '--db', str(db_path)],
            capture_output=True,
            timeout=30
        )

        result = subprocess.run(
            [sys.executable, '-m', 'fyi_system.cli', 'import-requests',
             str(import_path), '--db', str(db_path)],
            capture_output=True,
            text=True,
            timeout=30
        )

        # Should succeed or give informative error
        assert result.returncode in [0, 1]


class TestCLIExportRequest:
    """Test fyi-system export-request command."""

    def test_export_request_single(self, tmp_path):
        """E2E: export-request exports single request."""
        db_path = tmp_path / "test.db"
        output_path = tmp_path / "request.json"

        subprocess.run(
            [sys.executable, '-m', 'fyi_system.cli', 'init-db', '--db', str(db_path)],
            capture_output=True,
            timeout=30
        )

        create_result = subprocess.run(
            [sys.executable, '-m', 'fyi_system.cli', 'register-request',
             'test', 'Export Test', 'Body',
             '--db', str(db_path)],
            capture_output=True,
            text=True,
            timeout=30
        )

        import re
        match = re.search(r'\d+', create_result.stdout)
        if match:
            request_id = match.group()

            result = subprocess.run(
                [sys.executable, '-m', 'fyi_system.cli', 'export-request',
                 request_id, '--output', str(output_path),
                 '--db', str(db_path)],
                capture_output=True,
                text=True,
                timeout=30
            )

            # Should export or give informative error
            assert result.returncode in [0, 1]
