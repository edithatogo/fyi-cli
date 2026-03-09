# Troubleshooting Guide

**Common issues and solutions for FYI Request System**

**Version:** 1.0.0  
**Last Updated:** 2026-03-09

---

## Quick Fixes

### "Command not found: fyi-system"

**Cause:** System not installed or not in PATH

**Solution:**
```bash
# Verify installation
pip show fyi-cli

# If not installed
pip install fyi-cli

# If installed but not found, add to PATH
# Windows: Add C:\Users\<user>\AppData\Roaming\Python\Python311\Scripts to PATH
# macOS/Linux: Add ~/.local/bin to PATH
```

---

### "Database not found" or "Database locked"

**Cause:** Database file missing or in use

**Solution:**
```bash
# Initialize database
fyi-system init-db

# If locked, close other instances and retry
# Or remove lock file
rm fyi_system.db-shm
rm fyi_system.db-wal
```

---

### "API key not configured"

**Cause:** API key not set

**Solution:**
```bash
# Set API key
fyi-system config set api-key YOUR_API_KEY

# Verify
fyi-system config show
```

---

### "Invalid API key" or "401 Unauthorized"

**Cause:** API key is incorrect or revoked

**Solution:**
1. Verify key in FYI admin panel
2. Re-enter key:
   ```bash
   fyi-system config set api-key CORRECT_KEY
   ```
3. Test connection:
   ```bash
   fyi-system health-check
   ```

---

### "Permission denied" errors

**Cause:** Insufficient file permissions

**Solution:**
```bash
# Windows: Run as Administrator
# Right-click terminal → Run as Administrator

# macOS/Linux: Fix permissions
chmod 755 ~/.fyi-system
chmod 644 ~/.fyi-system/config.json
chmod 600 ~/.fyi-system/fyi_system.db
```

---

## Installation Issues

### pip install fails

**Error:** `Could not find a version that satisfies the requirement`

**Solution:**
```bash
# Upgrade pip
python -m pip install --upgrade pip

# Try again
pip install fyi-cli

# Or install from source
git clone https://github.com/yourusername/fyi-cli.git
cd fyi-cli
pip install -e .
```

---

### Python version error

**Error:** `Requires Python >=3.10`

**Solution:**
```bash
# Check Python version
python --version

# If < 3.10, install newer Python
# Download from https://www.python.org/downloads/
```

---

### Dependencies conflict

**Error:** `Cannot install ... conflicting dependencies`

**Solution:**
```bash
# Create virtual environment
python -m venv venv
source venv/bin/activate  # or venv\Scripts\activate on Windows

# Install in clean environment
pip install fyi-cli
```

---

## Database Issues

### Cannot initialize database

**Error:** `Failed to initialize database`

**Solution:**
```bash
# Check disk space
df -h  # macOS/Linux
dir    # Windows

# Check permissions
ls -la .  # macOS/Linux
dir       # Windows

# Try different location
fyi-system init-db --db /tmp/fyi_system.db
```

---

### Database corruption

**Symptoms:** Random errors, data missing

**Solution:**
```bash
# Export data first (if possible)
fyi-system export-requests --output backup.json

# Reinitialize database
rm fyi_system.db
fyi-system init-db

# Import data
fyi-system import-requests backup.json
```

---

### Slow database performance

**Symptoms:** Commands take >5 seconds

**Solution:**
```bash
# Vacuum database
sqlite3 fyi_system.db "VACUUM;"

# Analyze tables
sqlite3 fyi_system.db "ANALYZE;"

# Check database size
ls -lh fyi_system.db

# If >100MB, consider archiving old requests
```

---

## API Issues

### Connection timeout

**Error:** `Request timed out after 30 seconds`

**Solution:**
```bash
# Increase timeout
fyi-system config set timeout 60

# Check network connection
ping fyi.org.nz

# Check if FYI.org.nz is up
curl -I https://fyi.org.nz
```

---

### Rate limit exceeded

**Error:** `429 Too Many Requests`

**Solution:**
```bash
# Wait 1 hour for reset
# Or reduce polling frequency
fyi-system config set scheduler_interval 7200  # 2 hours
```

---

### API returns empty data

**Symptoms:** Commands succeed but show no data

**Solution:**
```bash
# Verify API key has permissions
fyi-system health-check

# Check if data exists
fyi-system list-requests

# Try manual feed ingestion
fyi-system ingest-feed https://www.fyi.org.nz/request/latest.rss
```

---

## Feed Monitoring Issues

### Feed ingestion fails

**Error:** `Failed to parse feed`

**Solution:**
```bash
# Test feed URL
curl https://www.fyi.org.nz/request/latest.rss

# Check if feed is valid XML
# If feed is down, wait and retry later
fyi-system ingest-feed https://www.fyi.org.nz/request/latest.rss
```

---

### Scheduler not running

**Symptoms:** No automatic feed updates

**Solution:**
```bash
# Check scheduler status
# (Scheduler runs in foreground, check if terminal is open)

# Start scheduler
fyi-system scheduler https://www.fyi.org.nz/request/latest.rss

# Or run once
fyi-system run-cycle https://www.fyi.org.nz/request/latest.rss
```

---

### Feed events not matching requests

**Symptoms:** Feed events not linked to tracked requests

**Solution:**
```bash
# Run reconciliation
fyi-system reconcile-events

# Check request IDs match
fyi-system list-requests
```

---

## Web UI Issues

### Web server won't start

**Error:** `Address already in use`

**Solution:**
```bash
# Check if port is in use
netstat -ano | findstr :8000  # Windows
lsof -i :8000                  # macOS/Linux

# Use different port
fyi-system serve --port 8080
```

---

### Cannot access web UI

**Symptoms:** Browser shows "Connection refused"

**Solution:**
```bash
# Check server is running
# Terminal should show: "Running on http://127.0.0.1:8000"

# Check firewall
# Windows: Allow Python through firewall
# macOS: System Preferences → Security → Firewall

# Try different host
fyi-system serve --host 0.0.0.0
```

---

### Web UI slow or unresponsive

**Symptoms:** Pages take >5 seconds to load

**Solution:**
```bash
# Check database size
ls -lh fyi_system.db

# If >100MB, archive old requests
fyi-system export-requests --output archive.json

# Enable debug mode to see what's slow
fyi-system serve --debug
```

---

## Export/Import Issues

### Export fails

**Error:** `Failed to export requests`

**Solution:**
```bash
# Check disk space
df -h  # macOS/Linux
dir    # Windows

# Check output directory exists
mkdir -p outputs

# Try different output file
fyi-system export-requests --output /tmp/requests.json
```

---

### Import fails

**Error:** `Failed to import requests`

**Solution:**
```bash
# Verify JSON is valid
python -m json.tool requests.json > /dev/null

# Check file format
# Should be array of request objects

# Try importing one at a time
# Edit JSON to have single request, import, repeat
```

---

### Export file too large

**Symptoms:** Export file >10MB

**Solution:**
```bash
# Export with sanitization (smaller)
fyi-system export-requests --output requests.json

# Or export in batches
# Edit export script to limit number of requests

# Or archive old requests
fyi-system export-requests --output archive-2025.json
# Then delete old requests from database
```

---

## Security Issues

### Encryption setup fails

**Error:** `Failed to initialize encryption`

**Solution:**
```bash
# Check cryptography package installed
pip show cryptography

# Reinstall if needed
pip install --upgrade cryptography

# Try setup again
fyi-system setup
```

---

### Privacy audit fails

**Error:** `Privacy check failed: file permissions`

**Solution:**
```bash
# Fix file permissions
chmod 600 fyi_system.db
chmod 600 ~/.fyi-system/config.json

# Re-run audit
fyi-system privacy-audit
```

---

### Credentials not stored securely

**Symptoms:** API key visible in config file

**Solution:**
```bash
# Use environment variable instead
export FYI_API_KEY="your-key"

# Or enable encryption
fyi-system setup
# Select "Enable encryption: Yes"
```

---

## Performance Issues

### Commands run slowly

**Symptoms:** All commands take >3 seconds

**Solution:**
```bash
# Check database size
ls -lh fyi_system.db

# Vacuum database
sqlite3 fyi_system.db "VACUUM;"

# Reduce verbose output
fyi-system config set verbose false

# Check system resources
top  # macOS/Linux
Task Manager  # Windows
```

---

### High memory usage

**Symptoms:** System uses >500MB RAM

**Solution:**
```bash
# Reduce batch sizes in scheduler
# Edit scheduler configuration

# Close web UI if not needed
# Press Ctrl+C in terminal running server

# Restart application
# Close all terminals and reopen
```

---

## Error Messages Reference

### Common Error Codes

| Error | Meaning | Solution |
|-------|---------|----------|
| `400 Bad Request` | Invalid parameters | Check command syntax |
| `401 Unauthorized` | Invalid API key | Re-enter API key |
| `403 Forbidden` | No permission | Check API key permissions |
| `404 Not Found` | Resource doesn't exist | Check ID/URL |
| `429 Too Many Requests` | Rate limited | Wait and retry |
| `500 Internal Error` | Server error | Contact FYI support |
| `503 Service Unavailable` | FYI down | Wait and retry later |

---

## Getting More Help

### Enable Verbose Mode

```bash
# Show detailed error messages
fyi-system <command> --verbose

# Or set globally
fyi-system config set verbose true
```

### Check Logs

```bash
# Logs stored in:
# Windows: %USERPROFILE%\.fyi-system\logs\
# macOS/Linux: ~/.fyi-system/logs/

# View latest log
tail -f ~/.fyi-system/logs/fyi-system.log
```

### Report a Bug

1. **Gather information:**
   - Error message (full text)
   - Command you ran
   - System information (OS, Python version)
   - Log file (last 50 lines)

2. **Create GitHub issue:**
   - Go to https://github.com/yourusername/fyi-cli/issues
   - Click "New Issue"
   - Fill in template

3. **Or email support:**
   - support@fyi-cli.example.com

---

## Still Stuck?

### Collect Debug Information

```bash
# System info
python --version
fyi-system --version

# Configuration
fyi-system config show

# Health check
fyi-system health-check --verbose

# Database info
sqlite3 fyi_system.db ".tables"
sqlite3 fyi_system.db "SELECT COUNT(*) FROM tracked_requests;"
```

### Include in Bug Report

- [ ] Error message (screenshot or text)
- [ ] Command you ran
- [ ] Output of `fyi-system --version`
- [ ] Output of `fyi-system config show`
- [ ] Last 50 lines of log file
- [ ] Steps to reproduce

---

**Still having issues?** See [FAQ.md](FAQ.md) or contact support.
