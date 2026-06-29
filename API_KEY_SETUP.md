# API Key Setup Guide

**How to obtain and configure your FYI.org.nz API key**

**Last Updated:** 2026-03-09

---

## Overview

An API key allows the FYI Request System to interact with FYI.org.nz programmatically. With an API key, you can:

- ✅ Create requests automatically
- ✅ Add correspondence to requests
- ✅ Update request states
- ✅ Access full request data
- ✅ Upload attachments

**Without an API key**, you can still:
- ✅ Track requests locally
- ✅ Generate prefilled URLs for manual submission
- ✅ Monitor public feeds
- ✅ Generate reports

---

## Do You Need an API Key?

| Use Case | API Key Required? |
|----------|-------------------|
| Manual request submission | ❌ No |
| Track requests locally | ❌ No |
| Generate reports | ❌ No |
| Import public authorities | ❌ No |
| Discover public archive requests | ❌ No |
| **Automatic request creation** | ✅ **Yes** |
| **Add correspondence via API** | ✅ **Yes** |
| **Bulk operations** | ✅ **Yes** |

**Recommendation:** Start without an API key, then get one if you need automation.

### Public Archive Discovery

The archive discovery workflow uses public FYI.org.nz endpoints and does not
need an API key:

```bash
fyi import-authorities
fyi discover --date-from 2024-01-01 --date-to 2024-02-01 --output discovered.jsonl
fyi discover --backfill-ids --id-from 1 --id-to 5000 --output backfill.jsonl
fyi discover-reconcile --feed discovered.jsonl --backfill backfill.jsonl
fyi capture 12345 --max-runtime-minutes 30 --max-bytes 500000000
```

Use small windows, keep rate limits conservative, and run live smoke tests only
when explicitly opted in with `FYI_LIVE_SMOKE=1`.

---

## Step 1: Request an API Key

### Option A: FYI.org.nz Admin Interface

**If you are an authority administrator:**

1. Go to https://fyi.org.nz/admin
2. Log in with your authority account
3. Navigate to **API Keys** section
4. Click **Generate New API Key**
5. Copy the generated key

**Note:** API keys are typically only available to:
- Government authorities
- Registered organizations
- Researchers with approved projects

### Option B: Contact FYI Support

**If you don't have admin access:**

1. Email: api@fyi.org.nz (or support contact)
2. Include:
   - Your name and organization
   - Intended use of API
   - Estimated request volume
   - Contact information

**Response time:** 2-5 business days

### Option C: Use Without API Key

**You can use the system fully without an API key:**

```bash
# All features work except automatic submission
fyi-system register-request ...  # Creates local record
fyi-system build-prefilled-url ...  # Opens browser for submission
```

---

## Step 2: Configure API Key

### Method 1: Setup Wizard (Recommended)

```bash
fyi-system setup
```

Follow the prompts:
```
Welcome to FYI Request System Setup

? Do you have an FYI API key? Yes
? Enter your API key: [paste key here]
✓ API key configured successfully
```

### Method 2: CLI Command

```bash
fyi-system config set api-key YOUR_API_KEY_HERE
```

**Verify configuration:**
```bash
fyi-system config show
```

**Expected output:**
```
Configuration:
  Database: fyi_system.db
  API Key: configured (sk...ed)  # Key is masked for security
  Base URL: https://fyi.org.nz
```

### Method 3: Environment Variable

**For automated deployments:**

```bash
# Set environment variable
export FYI_API_KEY="your-api-key-here"

# Or add to .bashrc / .zshrc
echo 'export FYI_API_KEY="your-key"' >> ~/.bashrc
```

**The system will automatically use the environment variable.**

### Method 4: Configuration File

**Edit `~/.fyi-system/config.json`:**

```json
{
  "api_key": "your-api-key-here",
  "database": "~/.fyi-system/fyi_system.db",
  "base_url": "https://fyi.org.nz"
}
```

---

## Step 3: Test API Key

### Test Connection

```bash
fyi-system health-check
```

**Expected output:**
```
✓ FYI.org.nz: Connected
✓ API Key: Valid
✓ Database: OK
✓ All systems operational
```

### Test API Access

```bash
# Try to fetch a request
fyi-system request-detail 1
```

**If API key is working:**
```
✓ Request retrieved successfully
```

**If API key is invalid:**
```
✗ API Error: 401 Unauthorized
  Please check your API key configuration
  Run: fyi-system config set api-key YOUR_KEY
```

---

## Security Best Practices

### ✅ DO: Store API Key Securely

```bash
# Use config command (encrypted storage)
fyi-system config set api-key YOUR_KEY

# Or use environment variable
export FYI_API_KEY="your-key"
```

### ❌ DON'T: Expose API Key

```bash
# Don't commit to git
git add .fyi-system/config.json  # WRONG!

# Don't share in public forums
# Don't include in error reports
# Don't hardcode in scripts
```

### Rotate Key Periodically

```bash
# Generate new key in FYI admin
# Then update configuration
fyi-system config set api-key NEW_API_KEY
```

---

## Troubleshooting

### "API key not configured"

**Solution:**
```bash
fyi-system config set api-key YOUR_KEY
```

### "Invalid API key"

**Causes:**
- Typo in API key
- Key has been revoked
- Wrong key for the instance

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

### "API rate limit exceeded"

**Cause:** Too many requests in short time

**Solution:**
1. Wait 1 hour for rate limit to reset
2. Reduce request frequency
3. Contact FYI support for higher limits

### "API key permissions insufficient"

**Cause:** API key lacks required permissions

**Solution:**
1. Check key permissions in FYI admin
2. Request additional permissions if needed
3. Use appropriate key for operation

---

## API Key Limits

### Rate Limits

| Operation | Limit | Reset Period |
|-----------|-------|--------------|
| Read requests | 100/minute | 1 minute |
| Create requests | 10/minute | 1 minute |
| Add correspondence | 20/minute | 1 minute |

### Quotas

| Plan | Requests/Month | Correspondence/Month |
|------|----------------|---------------------|
| Free | 50 | 100 |
| Standard | 500 | 1000 |
| Premium | Unlimited | Unlimited |

**Contact FYI support for quota increases.**

---

## Using API Key in Scripts

### Example: Python Script

```python
import os
from fyi_system.alaveteli_client import create_fyi_client

# Get API key from environment
api_key = os.environ.get('FYI_API_KEY')

# Create client
client = create_fyi_client(api_key=api_key)

# Create request
result = client.create_request(
    title='API Test Request',
    body='This request was created via API',
    external_user_name='Script User',
    external_url='https://example.com/test'
)

print(f"Created request {result['id']}")
```

### Example: Bash Script

```bash
#!/bin/bash

# Load API key from environment
if [ -z "$FYI_API_KEY" ]; then
    echo "Error: FYI_API_KEY not set"
    exit 1
fi

# Create request
fyi-system register-request \
  ministry-of-justice \
  "API Test Request" \
  "Created via script" \
  --status submitted

echo "Request created successfully"
```

---

## Revoking API Key

### Via FYI Admin

1. Go to https://fyi.org.nz/admin/api
2. Find your API key
3. Click **Revoke**
4. Confirm revocation

### After Revocation

**Update local configuration:**
```bash
# Remove old key
fyi-system config unset api-key

# Or set new key
fyi-system config set api-key NEW_KEY
```

---

## Multiple API Keys

**For managing multiple accounts:**

```bash
# Set key for account 1
fyi-system config set api-key KEY_1 --profile account1

# Set key for account 2
fyi-system config set api-key KEY_2 --profile account2

# Switch profiles
fyi-system config set active-profile account2
```

---

## Getting Help

- **API Documentation:** https://alaveteli.org/docs/developers/api/
- **FYI Support:** api@fyi.org.nz
- **System Issues:** See [TROUBLESHOOTING.md](TROUBLESHOOTING.md)

---

**API key configured?** Continue to [QUICKSTART.md](QUICKSTART.md) to start using the system!
