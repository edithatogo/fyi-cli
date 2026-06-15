# run-mutants.ps1
# Runs cargo-mutants for mutation testing audits

Write-Host "Running cargo-mutants mutation audit..." -ForegroundColor Cyan

# Execute cargo-mutants on the workspace
cargo mutants --workspace --all-features
