"""Performance profiling for FYI Request System.

Run with: python -m cProfile -o profile.stats src/fyi_system/cli.py --help
Visualize with: snakeviz profile.stats
"""
import cProfile
import pstats
import io
from pathlib import Path
import time
from contextlib import contextmanager

from fyi_system.db import init_db, insert_tracked_request, query_all
from fyi_system.security import redact_text, sanitize_payload
from fyi_system.reporting import attention_report, normalize_snapshot_state


@contextmanager
def profile_context(name: str = "Profile"):
    """Context manager for profiling code blocks."""
    profiler = cProfile.Profile()
    profiler.enable()
    start_time = time.perf_counter()
    try:
        yield profiler
    finally:
        profiler.disable()
        elapsed = time.perf_counter() - start_time
        print(f"\n{name}: {elapsed*1000:.2f}ms")
        
        # Print top 20 functions by cumulative time
        stream = io.StringIO()
        stats = pstats.Stats(profiler, stream=stream)
        stats.sort_stats('cumulative')
        stats.print_stats(20)
        print(stream.getvalue())


def profile_redaction():
    """Profile text redaction performance."""
    print("=" * 60)
    print("REDACTION PERFORMANCE PROFILE")
    print("=" * 60)
    
    test_cases = [
        "Contact test@example.com for info",
        "Email admin@fyi.org.nz and support@test.com",
        "No emails here, just plain text " * 100,
        "Mixed: test@a.com and https://example.com?api_key=secret",
    ]
    
    with profile_context("Redaction"):
        for i in range(1000):
            for text in test_cases:
                redact_text(text)


def profile_sanitization():
    """Profile payload sanitization performance."""
    print("=" * 60)
    print("SANITIZATION PERFORMANCE PROFILE")
    print("=" * 60)
    
    test_payloads = [
        {'email': 'test@example.com', 'name': 'Test'},
        {'body': 'sensitive content', 'title': 'Public'},
        {f'key_{i}': f'value_{i}' for i in range(100)},
    ]
    
    with profile_context("Sanitization"):
        for i in range(1000):
            for payload in test_payloads:
                sanitize_payload(payload)


def profile_database(tmp_path):
    """Profile database operations performance."""
    print("=" * 60)
    print("DATABASE PERFORMANCE PROFILE")
    print("=" * 60)
    
    db_path = tmp_path / "test.db"
    init_db(str(db_path))
    
    # Profile inserts
    with profile_context("Insert 100 requests"):
        for i in range(100):
            insert_tracked_request(
                db_path=str(db_path),
                authority_slug=f'authority_{i}',
                title=f'Request {i}',
                body=f'Body text for request {i}'
            )
    
    # Profile queries
    with profile_context("Query all requests"):
        for i in range(100):
            query_all(str(db_path), "SELECT * FROM tracked_requests")
    
    # Profile attention report
    with profile_context("Generate attention report"):
        for i in range(10):
            attention_report(str(db_path))


def profile_state_normalization():
    """Profile state normalization performance."""
    print("=" * 60)
    print("STATE NORMALIZATION PERFORMANCE PROFILE")
    print("=" * 60)
    
    test_states = [
        'pending', 'PENDING', 'Pending', '  pending  ',
        'responded', 'RESPONDED', 'Responded',
        'draft', 'submitted', 'completed', 'closed',
        '', None, 'unknown_state',
    ]
    
    with profile_context("Normalize states"):
        for i in range(10000):
            for state in test_states:
                normalize_snapshot_state(state)


def run_all_profiles(tmp_path):
    """Run all profiling tests."""
    print("\n" + "=" * 60)
    print("FYI REQUEST SYSTEM - PERFORMANCE PROFILE")
    print("=" * 60 + "\n")
    
    profile_redaction()
    profile_sanitization()
    profile_state_normalization()
    profile_database(tmp_path)
    
    print("\n" + "=" * 60)
    print("PROFILING COMPLETE")
    print("=" * 60)


if __name__ == '__main__':
    import tempfile
    with tempfile.TemporaryDirectory() as tmp_dir:
        from pathlib import Path
        run_all_profiles(Path(tmp_dir))
