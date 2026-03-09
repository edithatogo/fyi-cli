# Installation Guide

This guide covers installation on Windows, macOS, and Linux.

**Last Updated:** 2026-03-09  
**Version:** 1.0.0

---

## Quick Install

### Windows
```powershell
# Using pip (recommended)
pip install fyi-request-system

# Or download executable
# See "Windows Executable" section below
```

### macOS
```bash
# Using pip (recommended)
pip install fyi-request-system

# Or using Homebrew (coming soon)
# brew install fyi-request-system
```

### Linux
```bash
# Using pip (recommended)
pip install fyi-request-system

# Or using apt (Ubuntu/Debian, coming soon)
# sudo apt install fyi-request-system
```

---

## Prerequisites

### Python Version
- **Required:** Python 3.10 or higher
- **Recommended:** Python 3.11+

**Check your Python version:**
```bash
python --version
# or
python3 --version
```

**If you need to install Python:**
- Windows: https://www.python.org/downloads/windows/
- macOS: https://www.python.org/downloads/mac-osx/
- Linux: https://www.python.org/downloads/

### pip (Python Package Manager)

**Check if pip is installed:**
```bash
pip --version
```

**If pip is not installed:**
```bash
# Windows
python -m ensurepip --upgrade

# macOS
python3 -m ensurepip --upgrade

# Linux
sudo apt-get install python3-pip  # Ubuntu/Debian
sudo dnf install python3-pip      # Fedora/RHEL
```

---

## Installation Methods

### Method 1: pip (Recommended)

**Step 1: Install the package**
```bash
pip install fyi-request-system
```

**Step 2: Verify installation**
```bash
fyi-system --version
# Output: fyi-system 1.0.0
```

**Step 3: Run setup wizard**
```bash
fyi-system setup
```

---

### Method 2: From Source (Developers)

**Step 1: Clone the repository**
```bash
git clone https://github.com/yourusername/fyi-request-system.git
cd fyi-request-system
```

**Step 2: Create virtual environment**
```bash
# Windows
python -m venv venv
venv\Scripts\activate

# macOS/Linux
python3 -m venv venv
source venv/bin/activate
```

**Step 3: Install in development mode**
```bash
pip install -e ".[dev]"
```

**Step 4: Verify installation**
```bash
fyi-system --version
```

---

### Method 3: Standalone Executable (No Python Required)

#### Windows Executable

**Download:**
1. Go to [Releases](https://github.com/yourusername/fyi-request-system/releases)
2. Download `fyi-system-win.exe`
3. Save to `C:\Program Files\FYI-System\` or preferred location

**Install:**
```powershell
# Run installer
.\fyi-system-win.exe /install

# Or add to PATH manually
[System.Environment]::SetEnvironmentVariable(
    "Path",
    $env:Path + ";C:\Program Files\FYI-System",
    "Machine"
)
```

**Verify:**
```powershell
fyi-system --version
```

---

#### macOS Executable

**Download:**
1. Go to [Releases](https://github.com/yourusername/fyi-request-system/releases)
2. Download `fyi-system-macos`
3. Move to `/usr/local/bin/`

**Install:**
```bash
# Move to PATH
mv ~/Downloads/fyi-system-macos /usr/local/bin/fyi-system
chmod +x /usr/local/bin/fyi-system
```

**Verify:**
```bash
fyi-system --version
```

---

#### Linux Executable

**Download:**
1. Go to [Releases](https://github.com/yourusername/fyi-request-system/releases)
2. Download `fyi-system-linux`
3. Move to `/usr/local/bin/`

**Install:**
```bash
# Move to PATH
mv ~/Downloads/fyi-system-linux /usr/local/bin/fyi-system
chmod +x /usr/local/bin/fyi-system
```

**Verify:**
```bash
fyi-system --version
```

---

## Post-Installation Setup

### Step 1: Run Setup Wizard

```bash
fyi-system setup
```

The setup wizard will guide you through:
1. Database location selection
2. FYI API key configuration (optional)
3. Privacy settings
4. Default preferences

### Step 2: Verify Installation

```bash
# Check version
fyi-system --version

# Run health check
fyi-system health-check

# Initialize database
fyi-system init-db
```

### Step 3: Configure API Key (Optional)

If you have an FYI.org.nz API key:

```bash
# Store API key securely
fyi-system config set api-key YOUR_API_KEY_HERE

# Verify configuration
fyi-system config show
```

**Don't have an API key?** See [API_KEY_SETUP.md](docs/API_KEY_SETUP.md)

---

## Upgrade Instructions

### From Previous Version

```bash
# Upgrade via pip
pip install --upgrade fyi-request-system

# Verify upgrade
fyi-system --version
```

### Migration Guide

**Upgrading from v0.x to v1.0.0:**

Your existing data will be automatically migrated. No action required.

**Backup recommended before upgrade:**
```bash
# Backup database
fyi-system export-all --output backup-before-upgrade.json
```

---

## Uninstallation

### pip Installation

```bash
pip uninstall fyi-request-system
```

### Windows Executable

```powershell
# Run uninstaller
C:\Program Files\FYI-System\uninstall.exe

# Or manually remove
Remove-Item "C:\Program Files\FYI-System" -Recurse -Force
```

### macOS/Linux Executable

```bash
sudo rm /usr/local/bin/fyi-system
rm -rf ~/.fyi-system
```

---

## Troubleshooting

### "pip: command not found"

**Solution:** Install pip first (see Prerequisites section)

### "Permission denied" error

**Windows:**
```powershell
# Run as Administrator
pip install fyi-request-system
```

**macOS/Linux:**
```bash
# Use --user flag
pip install --user fyi-request-system

# Or use sudo (not recommended)
sudo pip install fyi-request-system
```

### "Module not found" error

**Solution:** Reinstall the package
```bash
pip uninstall fyi-request-system
pip install --upgrade pip
pip install fyi-request-system
```

### Database errors on first run

**Solution:** Initialize database
```bash
fyi-system init-db
```

### API key not working

**Solution:** Verify API key
```bash
# Check stored key
fyi-system config show

# Re-enter key
fyi-system config set api-key YOUR_NEW_KEY

# Test connection
fyi-system health-check
```

---

## System Requirements

### Minimum Requirements
- **OS:** Windows 10, macOS 10.15+, or Linux (Ubuntu 20.04+)
- **Python:** 3.10+
- **RAM:** 512 MB
- **Disk:** 100 MB for installation + space for database

### Recommended Requirements
- **OS:** Windows 11, macOS 12+, or Linux (Ubuntu 22.04+)
- **Python:** 3.11+
- **RAM:** 1 GB
- **Disk:** 500 MB for installation + database

---

## Next Steps

After installation:
1. ✅ Run `fyi-system setup`
2. ✅ Read [QUICKSTART.md](QUICKSTART.md)
3. ✅ Configure API key (see [API_KEY_SETUP.md](docs/API_KEY_SETUP.md))
4. ✅ Try your first command: `fyi-system --help`

---

## Getting Help

- **Documentation:** https://fyi-request-system.readthedocs.io/
- **Issues:** https://github.com/yourusername/fyi-request-system/issues
- **Discussions:** https://github.com/yourusername/fyi-request-system/discussions
- **Email:** support@fyi-request-system.example.com

---

**Installation successful?** Continue to [QUICKSTART.md](QUICKSTART.md) to get started!
