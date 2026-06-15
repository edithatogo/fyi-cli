# run-coverage.ps1
# Runs cargo-llvm-cov to ensure code coverage meets the >90% threshold.

Write-Host "Running Cargo LLVM Cov to audit coverage (>90% threshold)..." -ForegroundColor Cyan

# Run coverage check with fail threshold
cargo llvm-cov --all-features --workspace --fail-under-lines 90 --html

if ($LASTEXITCODE -ne 0) {
    Write-Error "Coverage check failed! Coverage is below the required 90% threshold."
    exit 1
} else {
    Write-Host "Coverage check passed! Coverage is above 90%." -ForegroundColor Green
}
