"""Performance benchmarks for FYI CLI using pytest-benchmark.

Run with:
    pytest tests/test_benchmarks.py --benchmark-only
"""

import pytest
import time
from pathlib import Path
import tempfile

from fyi_system.db import init_db, connect
from fyi_system.monitor import ingest_feed
from fyi_system.dashboard import write_dashboard
from fyi_system.security import redact_text


class TestDatabaseBenchmarks:
    """Benchmark database operations."""
    
    @pytest.fixture
    def test_db(self, tmp_path):
        """Create test database."""
        db_path = tmp_path / "benchmark.db"
        init_db(db_path)
        return str(db_path)
    
    @pytest.fixture
    def populated_db(self, test_db):
        """Create database with test data."""
        conn = connect(test_db)
        
        # Add 100 requests
        for i in range(100):
            conn.execute(
                'INSERT INTO tracked_requests (authority_slug, title, body, status) VALUES (?, ?, ?, ?)',
                (f'authority-{i}', f'Request {i}', f'Body {i}', 'draft')
            )
        conn.commit()
        conn.close()
        
        return test_db
    
    def benchmark_db_init(self, benchmark):
        """Benchmark database initialization."""
        def init():
            with tempfile.TemporaryDirectory() as tmp:
                db_path = Path(tmp) / "test.db"
                init_db(db_path)
        
        benchmark(init)
    
    def benchmark_db_insert(self, benchmark, test_db):
        """Benchmark inserting requests."""
        def insert():
            conn = connect(test_db)
            conn.execute(
                'INSERT INTO tracked_requests (authority_slug, title, body, status) VALUES (?, ?, ?, ?)',
                ('test-auth', 'Test Request', 'Test body', 'draft')
            )
            conn.commit()
            conn.close()
        
        benchmark(insert)
    
    def benchmark_db_query(self, benchmark, populated_db):
        """Benchmark querying requests."""
        def query():
            conn = connect(populated_db)
            results = conn.execute(
                'SELECT * FROM tracked_requests WHERE status = ?',
                ('draft',)
            ).fetchall()
            conn.close()
            return len(results)
        
        benchmark(query)


class TestReportingBenchmarks:
    """Benchmark reporting operations."""
    
    @pytest.fixture
    def populated_db(self, tmp_path):
        """Create database with test data."""
        db_path = tmp_path / "benchmark.db"
        init_db(db_path)
        
        conn = connect(db_path)
        for i in range(100):
            conn.execute(
                'INSERT INTO tracked_requests (authority_slug, title, body, status) VALUES (?, ?, ?, ?)',
                (f'authority-{i}', f'Request {i}', f'Body {i}', 'draft')
            )
        conn.commit()
        conn.close()
        
        return str(db_path)
    
    def benchmark_dashboard_generation(self, benchmark, populated_db):
        """Benchmark dashboard generation."""
        def generate():
            with tempfile.TemporaryDirectory() as tmp:
                output = Path(tmp) / "dashboard.html"
                write_dashboard(output, populated_db)
        
        benchmark(generate)


class TestSecurityBenchmarks:
    """Benchmark security operations."""
    
    def benchmark_email_redaction_simple(self, benchmark):
        """Benchmark simple email redaction."""
        text = "Contact test@example.com for more info"
        benchmark(redact_text, text)
    
    def benchmark_email_redaction_multiple(self, benchmark):
        """Benchmark multiple email redaction."""
        text = "Email us at support@example.com or sales@example.org"
        benchmark(redact_text, text)
    
    def benchmark_email_redaction_large(self, benchmark):
        """Benchmark redaction in large text."""
        text = "Contact " + "user@example.com, " * 100
        benchmark(redact_text, text)


class TestConcurrencyBenchmarks:
    """Benchmark concurrent operations."""
    
    @pytest.fixture
    def populated_db(self, tmp_path):
        """Create database with test data."""
        db_path = tmp_path / "benchmark.db"
        init_db(db_path)
        
        conn = connect(db_path)
        for i in range(1000):
            conn.execute(
                'INSERT INTO tracked_requests (authority_slug, title, body, status) VALUES (?, ?, ?, ?)',
                (f'authority-{i}', f'Request {i}', f'Body {i}', 'draft')
            )
        conn.commit()
        conn.close()
        
        return str(db_path)
    
    def benchmark_concurrent_reads(self, benchmark, populated_db):
        """Benchmark concurrent read operations."""
        from concurrent.futures import ThreadPoolExecutor
        
        def read(db_path, request_id):
            conn = connect(db_path)
            result = conn.execute(
                'SELECT * FROM tracked_requests WHERE id = ?',
                (request_id,)
            ).fetchone()
            conn.close()
            return result
        
        def concurrent_read():
            with ThreadPoolExecutor(max_workers=10) as executor:
                futures = [
                    executor.submit(read, populated_db, i)
                    for i in range(1, 11)
                ]
                results = [f.result() for f in futures]
            return results
        
        benchmark(concurrent_read)
