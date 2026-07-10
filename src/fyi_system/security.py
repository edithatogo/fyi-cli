
from __future__ import annotations

import hashlib
import json
import os
import re
import stat
from dataclasses import dataclass
from pathlib import Path
from typing import Any
from urllib.parse import parse_qsl, urlencode, urlsplit, urlunsplit

SENSITIVE_QUERY_KEYS = {
    'token', 'api_key', 'apikey', 'key', 'signature', 'sig', 'auth', 'authorization', 'access_token'
}
SENSITIVE_FIELD_KEYS = {
    'body', 'raw_json', 'summary', 'detail', 'email', 'authorization', 'token', 'api_key'
}
URL_RE = re.compile(r"https?://[^\s\]\)\>\"']+")
# Fixed email regex to handle edge cases with any non-whitespace, non-@ local part
EMAIL_RE = re.compile(r'''[^@\s]+@[^@\s]+\.[A-Za-z]{2,}''')
BEARER_RE = re.compile(r'(?i)(authorization\s*:\s*bearer\s+)[^\s]+')
SECRET_RE = re.compile(r'(?i)(?<![?&])((?:api[_-]?key|token|signature|sig|auth)\s*[=:]\s*)([^\s,;]+)')
ENCODED_SECRET_RE = re.compile(r'(?i)((?:api[_-]?key|token|signature|sig|auth)%3[dD])([^%&\s]+)')


@dataclass
class PrivacySettings:
    profile: str = 'standard'
    bind_host: str = '127.0.0.1'
    redact_logs: bool = True
    sanitize_bundle_exports: bool = True
    file_mode: int = 0o600
    dir_mode: int = 0o700


def _env_bool(name: str) -> bool | None:
    raw = os.getenv(name)
    if raw is None:
        return None
    return raw.strip().lower() in {'1', 'true', 'yes', 'on'}


def load_settings(settings_path: str | Path | None = None) -> PrivacySettings:
    payload: dict[str, Any] = {}
    if settings_path and Path(settings_path).exists():
        payload.update(json.loads(Path(settings_path).read_text(encoding='utf-8')))
    env_profile = os.getenv('FYI_SYSTEM_PRIVACY_PROFILE')
    env_bind_host = os.getenv('FYI_SYSTEM_BIND_HOST')
    env_redact_logs = _env_bool('FYI_SYSTEM_REDACT_LOGS')
    env_sanitize = _env_bool('FYI_SYSTEM_SANITIZE_BUNDLE_EXPORTS')
    if env_profile is not None:
        payload['profile'] = env_profile
    if env_bind_host is not None:
        payload['bind_host'] = env_bind_host
    if env_redact_logs is not None:
        payload['redact_logs'] = env_redact_logs
    if env_sanitize is not None:
        payload['sanitize_bundle_exports'] = env_sanitize
    return PrivacySettings(**payload)


def ensure_private_path(path: str | Path, *, is_dir: bool | None = None, file_mode: int = 0o600, dir_mode: int = 0o700) -> Path:
    p = Path(path)
    if is_dir is None:
        is_dir = p.is_dir()
    if is_dir:
        p.mkdir(parents=True, exist_ok=True)
        try:
            os.chmod(p, dir_mode)
        except OSError:
            pass
    else:
        p.parent.mkdir(parents=True, exist_ok=True)
        try:
            os.chmod(p.parent, dir_mode)
        except OSError:
            pass
        if p.exists():
            try:
                os.chmod(p, file_mode)
            except OSError:
                pass
    return p


def secure_write_text(path: str | Path, content: str, *, encoding: str = 'utf-8') -> Path:
    p = Path(path)
    ensure_private_path(p.parent, is_dir=True)
    p.write_text(content, encoding=encoding)
    ensure_private_path(p, is_dir=False)
    return p


def redact_text(value: str, *, profile: str = 'standard') -> str:
    if not isinstance(value, str):
        return value
    redacted = EMAIL_RE.sub('[redacted-email]', value)
    redacted = BEARER_RE.sub(r'\1[redacted-token]', redacted)
    redacted = _redact_url_query_secrets(redacted)
    redacted = SECRET_RE.sub(r'\1[redacted-secret]', redacted)
    redacted = ENCODED_SECRET_RE.sub(r'\1[redacted-secret]', redacted)
    if profile == 'strict' and len(redacted) > 240:
        digest = hashlib.sha256(redacted.encode('utf-8')).hexdigest()[:12]
        return f'[redacted-long-text sha256:{digest}]'
    return redacted


def _redact_url_query_secrets(value: str) -> str:
    def repl(match: re.Match[str]) -> str:
        token = match.group(0)
        try:
            parts = urlsplit(token)
            pairs = []
            changed = False
            for k, v in parse_qsl(parts.query, keep_blank_values=True):
                if k.lower() in SENSITIVE_QUERY_KEYS:
                    pairs.append((k, '[redacted]'))
                    changed = True
                else:
                    pairs.append((k, v))
            if changed:
                return urlunsplit((parts.scheme, parts.netloc, parts.path, urlencode(pairs), parts.fragment))
        except Exception:
            return token
        return token
    return URL_RE.sub(repl, value)


def sanitize_payload(payload: Any, *, profile: str = 'standard') -> Any:
    if isinstance(payload, dict):
        sanitized = {}
        for key, value in payload.items():
            lower = key.lower()
            if profile == 'strict' and lower in {'body', 'raw_json'} and isinstance(value, str):
                digest = hashlib.sha256(value.encode('utf-8')).hexdigest()[:12]
                sanitized[key] = f'[redacted sha256:{digest}]'
                continue
            if profile == 'strict' and lower in {'tags'} and isinstance(value, str):
                sanitized[key] = '[redacted-tags]'
                continue
            if lower in SENSITIVE_FIELD_KEYS and isinstance(value, str):
                sanitized[key] = redact_text(value, profile=profile)
            else:
                sanitized[key] = sanitize_payload(value, profile=profile)
        return sanitized
    if isinstance(payload, list):
        return [sanitize_payload(item, profile=profile) for item in payload]
    if isinstance(payload, str):
        return redact_text(payload, profile=profile)
    return payload


def privacy_audit(db_path: str | Path, *, host: str = '127.0.0.1', outputs_dir: str | Path = 'outputs', profile: str = 'standard') -> dict[str, Any]:
    db = Path(db_path)
    outputs = Path(outputs_dir)
    checks = [
        {
            'name': 'bind_host_local_only',
            'ok': host in {'127.0.0.1', 'localhost', '::1'},
            'detail': f'Host is {host}',
            'recommendation': 'Bind the local UI to 127.0.0.1 unless you intentionally front it with your own reverse proxy.',
        },
        _mode_check(db, expected=0o600, label='database_permissions'),
        _mode_check(outputs, expected=0o700, label='outputs_directory_permissions', is_dir=True),
        {
            'name': 'raw_snapshot_storage',
            'ok': False,
            'detail': 'FYI request snapshots are stored as raw JSON for local analysis.',
            'recommendation': 'Use a separate machine account, encrypted disk, or SQLCipher if you need stronger at-rest protection.',
        },
        {
            'name': 'sanitized_exports_default',
            'ok': profile in {'standard', 'strict'},
            'detail': f'Export sanitization profile is {profile}',
            'recommendation': 'Use strict mode for bundles you may move off-device or share.',
        },
    ]
    score = sum(1 for c in checks if c['ok'])
    return {
        'score': score,
        'total_checks': len(checks),
        'checks': checks,
        'recommendations': [
            'Prefer full-disk encryption on the host.',
            'Keep the web UI bound to localhost and place any remote access behind your own authenticated tunnel or VPN.',
            'Use strict sanitized bundles for exports that leave your main workstation.',
            'Separate browser profiles for FYI submission and local admin review.',
        ],
    }


def _mode_check(path: Path, *, expected: int, label: str, is_dir: bool | None = None) -> dict[str, Any]:
    if not path.exists():
        return {
            'name': label,
            'ok': False,
            'detail': f'{path} does not exist yet',
            'recommendation': 'Create it and keep it private to your user account.',
        }
    mode = stat.S_IMODE(path.stat().st_mode)
    ok = mode & 0o077 == 0
    target = oct(expected)
    kind = 'directory' if (is_dir if is_dir is not None else path.is_dir()) else 'file'
    return {
        'name': label,
        'ok': ok,
        'detail': f'{kind} mode is {oct(mode)}; expected private mode around {target}',
        'recommendation': f'Run chmod {target} {path.name} if this path is shared too broadly.',
    }
