#!/bin/bash
# Export public files for GitHub repository
# This script copies only public-facing code, excluding personal data

set -e

echo "=============================================="
echo "FYI Request System - Public Repo Export"
echo "=============================================="

# Define directories
PROJECT_ROOT="$(pwd)"
PUBLIC_REPO="${PROJECT_ROOT}/../fyi-cli-public"

# Create public repo directory
echo "Creating public repo directory: ${PUBLIC_REPO}"
mkdir -p "${PUBLIC_REPO}"

# Copy public files (excluding personal data)
echo "Copying public files..."
rsync -av \
    --exclude='.git/' \
    --exclude='*.db' \
    --exclude='*.sqlite' \
    --exclude='data/' \
    --exclude='outputs/' \
    --exclude='.bundle/' \
    --exclude='.env' \
    --exclude='settings.json' \
    --exclude='*.local.*' \
    --exclude='*.log' \
    --exclude='logs/' \
    --exclude='.pytest_cache/' \
    --exclude='.coverage' \
    --exclude='htmlcov/' \
    --exclude='__pycache__/' \
    --exclude='*.pyc' \
    --exclude='.hypothesis/' \
    --exclude='session.sqlite' \
    --exclude='cosmic-ray.toml' \
    "${PROJECT_ROOT}/" "${PUBLIC_REPO}/"

# Setup public files
echo "Setting up public files..."
cp "${PUBLIC_REPO}/PUBLIC_README.md" "${PUBLIC_REPO}/README.md"
cp "${PUBLIC_REPO}/.gitignore.public" "${PUBLIC_REPO}/.gitignore"

# Remove internal documents
echo "Removing internal documents..."
rm -f "${PUBLIC_REPO}/PUBLIC_README.md"
rm -f "${PUBLIC_REPO}/PUBLIC_VS_PRIVATE.md"
rm -f "${PUBLIC_REPO}/.gitignore.public"

echo ""
echo "=============================================="
echo "Export Complete!"
echo "=============================================="
echo ""
echo "Public repo location: ${PUBLIC_REPO}"
echo ""
echo "Next steps:"
echo "1. cd ${PUBLIC_REPO}"
echo "2. git init"
echo "3. git add ."
echo "4. git commit -m 'Initial release: FYI Request System'"
echo "5. git remote add origin git@github.com:YOUR_USERNAME/fyi-cli.git"
echo "6. git push -u origin main"
echo ""
echo "=============================================="
