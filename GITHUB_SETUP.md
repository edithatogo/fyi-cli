# GitHub Repository Setup Instructions

**FYI CLI** - Privacy-focused CLI tool for managing FYI.org.nz official information requests

---

## Step 1: Create GitHub Repository

### 1.1 Create Repository on GitHub

1. Go to https://github.com/new
2. **Repository name:** `fyi-cli`
3. **Description:** "Privacy-focused CLI tool for managing FYI.org.nz official information requests"
4. **Visibility:** Public
5. **DO NOT** initialize with README, .gitignore, or license (we already have these)
6. Click **Create repository**

### 1.2 Connect Local Repository

```bash
# Navigate to project directory
cd "C:\Users\60217257\OneDrive - Flinders\Project - 2026.03 - FYI NZ"

# Add GitHub remote (replace YOUR_USERNAME with your GitHub username)
git remote add origin https://github.com/YOUR_USERNAME/fyi-cli.git

# Verify remote
git remote -v
# Should show:
# origin  https://github.com/YOUR_USERNAME/fyi-cli.git (fetch)
# origin  https://github.com/YOUR_USERNAME/fyi-cli.git (push)

# Push all branches
git branch -M main
git push -u origin main

# Push all tags
git push --tags
```

---

## Step 2: Configure GitHub Settings

### 2.1 Disable Unwanted Features

Go to: `https://github.com/YOUR_USERNAME/fyi-cli/settings`

**Features section:**
- ☑ **Issues** - KEEP ENABLED (for bug reports)
- ☑ **Pull requests** - KEEP ENABLED
- ☐ **Discussions** - DISABLE (as requested)
- ☐ **Projects** - DISABLE (as requested)
- ☐ **Wiki** - DISABLE (as requested)

### 2.2 Enable Security Features

Go to: `https://github.com/YOUR_USERNAME/fyi-cli/settings/security`

**Code security and analysis:**
- ☑ **Code scanning alerts** - ENABLE (CodeQL)
- ☑ **Secret scanning alerts** - ENABLE
- ☑ **Secret scanning push protection** - ENABLE
- ☑ **Dependency graph** - ENABLE
- ☑ **Dependabot alerts** - ENABLE
- ☑ **Dependabot security updates** - ENABLE

### 2.3 Add Repository Topics

Go to: `https://github.com/YOUR_USERNAME/fyi-cli`

Click "Manage topics" and add:
```
fyi alaveteli cli official-information oia transparency privacy new-zealand python
```

---

## Step 3: Configure GitHub Secrets

### 3.1 Get PyPI API Token

1. Go to https://pypi.org/manage/account/token/
2. Click **Add API token**
3. **Token name:** `fyi-cli-publisher`
4. **Scope:** All projects
5. Click **Create token**
6. **Copy the token** (starts with `pypi-`)

### 3.2 Get TestPyPI API Token

1. Go to https://test.pypi.org/manage/account/token/
2. Click **Add API token**
3. **Token name:** `fyi-cli-test-publisher`
4. **Scope:** All projects
5. Click **Create token**
6. **Copy the token** (starts with `pypi-`)

### 3.3 Get Anaconda.org API Token

1. Go to https://anaconda.org/YOUR_USERNAME/settings/access
2. Click **Create Token**
3. **Token name:** `fyi-cli-publisher`
4. **Expires in:** 1 year
5. **Permissions:** write
6. Click **Create**
7. **Copy the token**

### 3.4 Add Secrets to GitHub

Go to: `https://github.com/YOUR_USERNAME/fyi-cli/settings/secrets/actions`

Click **New repository secret** for each:

| Secret Name | Value |
|-------------|-------|
| `PYPI_API_TOKEN` | pypi-AgEIcHlwaS5vcmc... (from step 3.1) |
| `TESTPYPI_API_TOKEN` | pypi-AgEIcHlwaS5vcmc... (from step 3.2) |
| `ANACONDA_API_TOKEN` | <token from step 3.3> |

---

## Step 4: Verify Workflows

### 4.1 Check Workflows

Go to: `https://github.com/YOUR_USERNAME/fyi-cli/actions`

You should see these workflows:
- ✅ **CI** - Runs on every push/PR
- ✅ **CodeQL** - Security scanning
- ✅ **Release** - PyPI/TestPyPI publishing
- ✅ **Release Please** - Automated releases
- ✅ **Conda Publish** - Conda package publishing
- ✅ **Profiling** - Performance profiling

### 4.2 Trigger Test Release

1. Go to: `https://github.com/YOUR_USERNAME/fyi-cli/actions/workflows/release.yml`
2. Click **Run workflow**
3. **Branch:** main
4. **Publish to:** testpypi
5. Click **Run workflow**

This will:
- Build the package
- Upload to TestPyPI
- Verify publishing works

### 4.3 Verify TestPyPI Upload

After workflow completes:
1. Go to https://test.pypi.org/project/fyi-cli/
2. You should see the package
3. Test installation:
   ```bash
   pip install -i https://test.pypi.org/simple/ fyi-cli
   fyi --version
   ```

---

## Step 5: Create First Release

### 5.1 Tag Release

```bash
# Create v1.0.0 tag
git tag v1.0.0
git push origin v1.0.0
```

### 5.2 Automated Publishing

The tag push will automatically trigger:
1. **CI workflow** - Run all tests
2. **Release workflow** - Build and publish to PyPI
3. **Conda workflow** - Build and publish to Anaconda.org
4. **Release Please** - Create GitHub release with changelog

### 5.3 Verify Releases

**PyPI:**
- https://pypi.org/project/fyi-cli/
- ```bash
  pip install fyi-cli
  fyi --help
  ```

**Anaconda.org:**
- https://anaconda.org/YOUR_USERNAME/fyi-cli
- ```bash
  conda install -c YOUR_USERNAME fyi-cli
  ```

**GitHub Releases:**
- https://github.com/YOUR_USERNAME/fyi-cli/releases
- Should show v1.0.0 with auto-generated changelog

---

## Step 6: Enable Renovate Bot

### 6.1 Install Renovate

1. Go to https://github.com/apps/renovate
2. Click **Configure**
3. Select your repository: `fyi-cli`
4. Click **Save**

### 6.2 Verify Renovate

Renovate will:
- Scan dependencies weekly (Monday 6am NZST)
- Create PRs for updates
- Auto-merge minor/patch updates
- Alert on security vulnerabilities

Check: https://github.com/YOUR_USERNAME/fyi-cli/pulls

---

## Step 7: Final Verification

### 7.1 Repository Checklist

- [ ] Repository created at https://github.com/YOUR_USERNAME/fyi-cli
- [ ] All code pushed (main branch + tags)
- [ ] Discussions/Projects/Wiki disabled
- [ ] Security features enabled
- [ ] Topics added
- [ ] Secrets configured (PYPI, TESTPYPI, ANACONDA)
- [ ] Workflows running
- [ ] TestPyPI test successful
- [ ] v1.0.0 released
- [ ] Renovate installed

### 7.2 Installation Tests

**PyPI:**
```bash
pip install fyi-cli
fyi --version
# Should show: fyi 1.0.0
```

**TestPyPI:**
```bash
pip install -i https://test.pypi.org/simple/ fyi-cli
fyi --help
```

**Conda:**
```bash
conda install -c YOUR_USERNAME fyi-cli
fyi --version
```

---

## Troubleshooting

### Workflow Fails

**Check logs:**
1. Go to Actions tab
2. Click failed workflow
3. Review error logs

**Common issues:**
- Missing secrets → Add to Settings → Secrets
- Python version mismatch → Check workflow YAML
- Build errors → Check pyproject.toml

### PyPI Upload Fails

**Check:**
1. Token is valid (not expired)
2. Token has correct permissions
3. Package name is unique on PyPI
4. Version hasn't been published before

**Fix:**
```bash
# Delete and recreate token on PyPI
# Update GitHub secret
# Re-run workflow
```

### Conda Build Fails

**Check:**
1. SHA256 hash in conda/meta.yaml matches PyPI source
2. All dependencies available on conda-forge
3. Recipe syntax is valid

**Fix:**
```bash
# Get correct hash
curl -sL https://pypi.org/packages/source/f/fyi-cli/fyi-cli-1.0.0.tar.gz | sha256sum

# Update conda/meta.yaml
# Re-run workflow
```

---

## Next Steps

After successful release:

1. **Announce release:**
   - Twitter/X
   - LinkedIn
   - Relevant forums (Reddit r/newzealand, r/privacy)
   - FYI.org.nz community

2. **Monitor:**
   - PyPI downloads: https://pypistats.org/packages/fyi-cli
   - GitHub issues: https://github.com/YOUR_USERNAME/fyi-cli/issues
   - Dependabot alerts

3. **Plan v1.1.0:**
   - Collect user feedback
   - Review issues
   - Plan new features

---

**Repository is ready for v1.0.0 release!** 🎉
