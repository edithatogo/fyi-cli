# Non-Alaveteli provider smoke checks

The smoke sensor is disabled by default and never runs in CI. It performs one
anonymous public GET per selected provider, with a ten-second timeout, a one
megabyte response cap, a contactable User-Agent, and no credentials.

```powershell
uv run python scripts/provider_live_smoke.py muckrock --live
uv run python scripts/provider_live_smoke.py fragdenstaat --live
```

The sensor emits only status, sample count, and a schema fingerprint. A changed
or incomplete response fails closed; response bodies are never printed. A live
HTTP denial or outage is evidence to record, not permission to add credentials
or enable writes.
