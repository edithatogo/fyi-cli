# Pinned Python development and CI runtime

This project retains its legacy Python application and public compatibility metadata during Rust/consumer transition, but **active development and CI jobs execute from the repository-local CPython 3.15.1 virtual environment**.

The expected version is recorded in `.python-version`. For local work:

```sh
uv python install 3.15.1
uv sync --python 3.15.1 --extra dev --locked
uv run --no-sync --python 3.15.1 python scripts/verify_python315_runtime.py
```

The verifier rejects any other CPython patch release, alternate implementation, global interpreter, or virtualenv outside `.venv`. It prints only the successful version, not sensitive paths or environment values. Jobs using only the standard library still create `.venv` and run the verifier before operating. Dependency-selected jobs use `uv sync` with an explicit 3.15.1 interpreter. External release and registry authority remain unchanged.

No test, coverage, security, audit, lint, action or publication gate is disabled by this pin. The existing `requires-python >=3.11` metadata and Python application fallback remain until full migration/consumer gates qualify. The Python 3.15 compatibility sensor remains separately identified; its optional status is never reported as proof of passing the mandatory test suite. A future runtime upgrade requires its own reviewed change rather than silently selecting a different interpreter.

If a dependency cannot operate on 3.15.1, the correct resolution is to repair/replace the dependency or document a held integration blocker, not to switch a CI job back to a different Python version or suppress its failure.
