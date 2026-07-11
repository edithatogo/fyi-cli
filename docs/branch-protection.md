# Protected delivery policy

The default `master` branch is protected at the GitHub repository level. Pull
requests require an owner review and the complete required CI surface:
Rustfmt, Clippy, workspace tests, package smoke, security audit, coverage,
layered harness, Python tests, CodeQL analyzers, packaging assets, and Codecov
patch coverage. Stale approvals are dismissed, linear history is required,
administrators are subject to the same rules, and force-push/deletion are
disabled.

The repository CODEOWNERS file is the source of ownership review policy;
GitHub branch settings are the enforcement point.
