# Conda Publishing Guide

**FYI CLI** is available on conda via conda-forge and anaconda.org.

---

## Installation

### From conda-forge (Recommended)

```bash
conda install -c conda-forge fyi-cli
```

### From anaconda.org

```bash
conda install -c yourusername fyi-cli
```

---

## Publishing to Conda

### Option 1: Anaconda.org (Automatic)

**Workflow:** `.github/workflows/conda-publish.yml`

**Triggers:**
- On GitHub release published
- Manual workflow dispatch

**Process:**
1. Release is published on GitHub
2. Workflow builds conda package
3. Uploads to anaconda.org/yourusername

**Required Secret:**
```
ANAconda_API_TOKEN = your-anaconda-token
```

**Get Token:**
1. Go to https://anaconda.org/yourusername/settings/access
2. Create API token with `write` permissions
3. Add to GitHub: Settings → Secrets → `ANAconda_API_TOKEN`

---

### Option 2: conda-forge (Recommended for Production)

**Process:**

#### Step 1: Create Feedstock Repository

```bash
# Fork conda-forge/staged-recipes
git clone https://github.com/conda-forge/staged-recipes.git
cd staged-recipes

# Copy recipe
cp ../fyi-cli/conda/meta.yaml recipes/fyi-cli/

# Commit and push
git checkout -b fyi-cli
git add recipes/fyi-cli
git commit -m "Add fyi-cli recipe"
git push origin fyi-cli
```

#### Step 2: Submit PR

1. Go to https://github.com/conda-forge/staged-recipes
2. Create Pull Request from your branch
3. Wait for review (usually 1-2 weeks)
4. Once approved, feedstock is created

#### Step 3: Auto-Updates

After feedstock is created:
- New versions are auto-detected via `regro-cf-autotick-bot`
- PRs are created automatically
- Merged PRs trigger conda-forge builds

---

## Manual Build

### Build Locally

```bash
# Install conda-build
conda install -c conda-forge conda-build

# Build package
conda-build conda/ --output-folder conda-dist

# Test installation
conda create -n test-fyi -c ./conda-dist fyi-cli
conda activate test-fyi
fyi --help
```

### Upload Manually

```bash
# Install anaconda-client
conda install -c conda-forge anaconda-client

# Login
anaconda login

# Upload
anaconda upload conda-dist/noarch/fyi-cli-*.tar.bz2
```

---

## Recipe Structure

```
conda/
├── meta.yaml              # Package recipe
└── conda_build_config.yaml  # Build configuration
```

### meta.yaml Sections

| Section | Purpose |
|---------|---------|
| `package` | Name and version |
| `source` | PyPI tarball URL and hash |
| `build` | Build number, script, entry points |
| `requirements` | Host (build) and run (runtime) deps |
| `test` | Import tests and CLI tests |
| `about` | License, summary, URLs |
| `extra` | Maintainers |

---

## Version Updates

### Automatic (conda-forge)

The `regro-cf-autotick-bot` automatically:
1. Detects new PyPI releases
2. Creates PR with updated version
3. Updates SHA256 hash
4. Merges after CI passes

### Manual

Update `conda/meta.yaml`:

```yaml
{% set version = "1.0.1" %}  # Update version

source:
  sha256: new_sha256_hash  # Update hash
```

Get new hash:
```bash
curl -sL https://pypi.org/packages/source/f/fyi-cli/fyi-cli-1.0.1.tar.gz | sha256sum
```

---

## Testing

### Test Conda Package

```bash
# Create test environment
conda create -n test-fyi -c ./conda-dist fyi-cli pytest
conda activate test-fyi

# Run tests
fyi --help
pytest tests/ -v

# Cleanup
conda deactivate
conda env remove -n test-fyi
```

### Test on Multiple Platforms

Conda builds are noarch (platform-independent), but test on:
- Linux (Ubuntu)
- macOS (Intel + Apple Silicon)
- Windows

---

## Troubleshooting

### Build Fails

**Error:** `conda-build` fails

**Solution:**
```bash
# Clean build cache
conda-build purge

# Rebuild with verbose output
conda-build conda/ -vv
```

### Import Error

**Error:** `ImportError: No module named 'fyi_system'`

**Solution:** Check `build/script` in meta.yaml:
```yaml
build:
  script: {{ PYTHON }} -m pip install . -vv
```

### Entry Point Not Found

**Error:** `fyi: command not found`

**Solution:** Check entry_points in meta.yaml:
```yaml
build:
  entry_points:
    - fyi = fyi_system.cli:main
```

### Dependency Conflict

**Error:** `UnsatisfiableError: fyi-cli conflicts with...`

**Solution:** 
1. Check runtime dependencies in meta.yaml
2. Ensure version ranges are compatible
3. Test in clean environment

---

## Comparison: PyPI vs Conda

| Feature | PyPI | Conda |
|---------|------|-------|
| **Package Format** | Wheel/sdist | .tar.bz2/.conda |
| **Dependencies** | pip | conda |
| **Binary Packages** | No | Yes |
| **Non-Python Deps** | No | Yes |
| **Platform** | Python-specific | Cross-platform |
| **Update Speed** | Immediate | 1-2 days (conda-forge) |

**Recommendation:** Publish to both PyPI and conda for maximum reach.

---

## Resources

- [Conda Documentation](https://docs.conda.io/)
- [Conda-build Documentation](https://docs.conda.io/projects/conda-build/en/latest/)
- [Conda-forge Documentation](https://conda-forge.org/docs/)
- [Conda Recipe Gallery](https://github.com/conda/conda-recipes)
- [Anaconda Cloud](https://anaconda.org/)

---

**Questions?** See [CONTRIBUTING.md](CONTRIBUTING.md) or open an issue.
