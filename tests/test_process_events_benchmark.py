import pytest

from scripts.benchmark_process_events import run


@pytest.mark.performance
def test_representative_process_event_fixture_is_bounded():
    result = run(request_count=250, events_per_request=8)
    assert result["event_count"] == 2_000
    assert result["elapsed_seconds"] < 10


@pytest.mark.performance
def test_full_corpus_shaped_process_event_fixture_is_bounded():
    result = run(request_count=1_000, events_per_request=8)
    assert result["event_count"] == 8_000
    assert result["elapsed_seconds"] < 30
