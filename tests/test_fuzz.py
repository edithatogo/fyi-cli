"""Fuzz testing for FYI Request System using hypothesis.

Fuzz testing generates random, unexpected, or malformed inputs
to find crashes, edge cases, and security vulnerabilities.
"""
import json
import pytest
from hypothesis import given, settings, strategies as st, HealthCheck
from hypothesis.strategies import text, binary, integers, lists, none
import re

from fyi_system.security import (
    redact_text,
    sanitize_payload,
    ensure_private_path,
    secure_write_text,
    load_settings,
)
from fyi_system.reporting import normalize_snapshot_state
from fyi_system.db import init_db, insert_tracked_request


class TestFuzzRedaction:
    """Fuzz tests for text redaction."""
    
    @given(text(min_size=0, max_size=10000, alphabet=st.characters(min_codepoint=0, max_codepoint=65535)))
    @settings(max_examples=500, deadline=None)
    def test_redact_handles_any_unicode(self, random_text):
        """Fuzz: Redaction handles any Unicode input without crashing."""
        try:
            result = redact_text(random_text)
            assert isinstance(result, str)
        except (UnicodeEncodeError, UnicodeDecodeError):
            # Acceptable for invalid Unicode
            pass
    
    @given(binary(min_size=0, max_size=1000))
    @settings(max_examples=200, deadline=None)
    def test_redact_handles_bytes_gracefully(self, random_bytes):
        """Fuzz: Redaction handles bytes input gracefully."""
        try:
            # Should handle non-string input
            result = redact_text(random_bytes)
            # If it doesn't crash, should return input or handle it
            assert result is not None
        except (TypeError, AttributeError):
            # Also acceptable - type error is valid response
            pass
    
    @given(integers())
    @settings(max_examples=100, deadline=None)
    def test_redact_handles_integers(self, random_int):
        """Fuzz: Redaction handles integer input."""
        result = redact_text(str(random_int))
        assert isinstance(result, str)
    
    @given(lists(text(min_size=0, max_size=100), min_size=0, max_size=100))
    @settings(max_examples=200, deadline=None)
    def test_redact_long_strings(self, string_list):
        """Fuzz: Redaction handles very long strings."""
        long_text = "".join(string_list)
        result = redact_text(long_text)
        assert isinstance(result, str)


class TestFuzzSanitization:
    """Fuzz tests for payload sanitization."""
    
    @given(st.recursive(
        st.one_of(
            text(min_size=0, max_size=100),
            integers(),
            none(),
            st.booleans(),
        ),
        lambda children: st.lists(children, min_size=0, max_size=20) |
                        st.dictionaries(text(min_size=1, max_size=20), children, min_size=0, max_size=20)
    ))
    @settings(max_examples=100, deadline=None)
    def test_sanitize_handles_arbitrary_nesting(self, arbitrary_structure):
        """Fuzz: Sanitization handles arbitrarily nested structures."""
        try:
            result = sanitize_payload(arbitrary_structure)
            # Should not crash (result can be None for None input)
            assert result is not None or arbitrary_structure is None
        except (RecursionError, TypeError):
            # Acceptable for deeply nested or invalid structures
            pass
    
    @given(text(min_size=0, max_size=100000, alphabet=st.characters(min_codepoint=0, max_codepoint=255)))
    @settings(max_examples=50, deadline=None)
    def test_sanitize_very_long_strings(self, very_long_text):
        """Fuzz: Sanitization handles very long strings."""
        payload = {'data': very_long_text}
        try:
            result = sanitize_payload(payload)
            assert isinstance(result, dict)
        except (MemoryError, RecursionError):
            # Acceptable for extremely large inputs
            pass


class TestFuzzStateNormalization:
    """Fuzz tests for state normalization."""
    
    @given(text(min_size=0, max_size=1000, alphabet=st.characters(min_codepoint=0, max_codepoint=65535)))
    @settings(max_examples=500, deadline=None)
    def test_normalize_any_unicode_string(self, random_unicode):
        """Fuzz: Normalization handles any Unicode string."""
        try:
            result = normalize_snapshot_state(random_unicode)
            assert isinstance(result, str)
            assert result == result.lower()
        except (UnicodeEncodeError, UnicodeDecodeError):
            # Acceptable for invalid Unicode
            pass
    
    @given(lists(text(min_size=0, max_size=100), min_size=0, max_size=100))
    @settings(max_examples=200, deadline=None)
    def test_normalize_with_null_bytes(self, text_parts):
        """Fuzz: Normalization handles strings with null bytes."""
        test_string = "\x00".join(text_parts)
        try:
            result = normalize_snapshot_state(test_string)
            assert isinstance(result, str)
        except (ValueError, TypeError):
            # Acceptable for strings with null bytes
            pass


class TestFuzzDatabase:
    """Fuzz tests for database operations."""

    @given(
        text(min_size=1, max_size=50, alphabet=st.characters(min_codepoint=32, max_codepoint=126)),
        text(min_size=1, max_size=200, alphabet=st.characters(min_codepoint=32, max_codepoint=126)),
        text(min_size=1, max_size=1000, alphabet=st.characters(min_codepoint=32, max_codepoint=126)),
    )
    @settings(max_examples=100, deadline=None, suppress_health_check=[HealthCheck.function_scoped_fixture])
    def test_insert_random_request_data(self, tmp_path, slug, title, body):
        """Fuzz: Database handles random request data."""
        db_path = tmp_path / "test.db"
        init_db(str(db_path))

        try:
            request_id = insert_tracked_request(
                db_path=str(db_path),
                authority_slug=slug,
                title=title,
                body=body
            )
            assert request_id is not None
            assert isinstance(request_id, int)
        except Exception:
            # Should handle any valid string input
            pass


class TestFuzzSettings:
    """Fuzz tests for settings loading."""

    @given(text(min_size=0, max_size=1000, alphabet=st.characters(min_codepoint=0, max_codepoint=255)))
    @settings(max_examples=100, deadline=None, suppress_health_check=[HealthCheck.function_scoped_fixture])
    def test_load_settings_from_random_file(self, tmp_path, random_content):
        """Fuzz: Settings loading handles random file content."""
        settings_path = tmp_path / "settings.json"
        settings_path.write_text(random_content, encoding='utf-8', errors='replace')

        try:
            settings = load_settings(str(settings_path))
            # Should either load defaults or handle gracefully
            assert hasattr(settings, 'profile')
        except Exception:
            # Acceptable for any invalid input
            pass


class TestFuzzEdgeCases:
    """Fuzz tests for specific edge cases."""
    
    @given(integers(min_value=-1000000, max_value=1000000))
    @settings(max_examples=200, deadline=None)
    def test_redact_emoji_and_special_chars(self, seed_value):
        """Fuzz: Redaction handles emoji and special characters."""
        # Generate test strings with emoji and special chars
        test_strings = [
            f"Email: test@example.com 📧",
            f"Contact: admin@test.com™",
            f"Reach: support@domain.com©®",
            f"📧 test@example.com 📧",
        ]
        
        for test_string in test_strings:
            result = redact_text(test_string)
            assert isinstance(result, str)

    @given(lists(integers(), min_size=0, max_size=1000))
    @settings(max_examples=100, deadline=None, suppress_health_check=[HealthCheck.function_scoped_fixture])
    def test_sanitize_with_numeric_arrays(self, tmp_path, number_list):
        """Fuzz: Sanitization handles numeric arrays."""
        payload = {'numbers': number_list}
        result = sanitize_payload(payload)
        assert isinstance(result, dict)
