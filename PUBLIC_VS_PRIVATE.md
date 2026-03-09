# FYI Tool - Public vs Private Separation

This document describes how the FYI Request System tool is structured to separate:
1. **Public Tool Code** - Open source software anyone can use
2. **Private Personal Use** - Your personal data, accounts, and configuration

## Directory Structure

```
Project Root/
├── PUBLIC FILES (can be published)
│   ├── src/fyi_system/       # Tool source code
│   ├── tests/                 # Test suite
│   ├── pyproject.toml         # Project configuration
│   ├── PUBLIC_README.md       # Public documentation
│   ├── .gitignore.public      # Git ignore for public repo
│   └── conductor/             # Project management (optional)
│
└── PRIVATE FILES (never publish)
    ├── *.db                   # SQLite databases with your requests
    ├── data/*.csv             # Your authority lists
    ├── outputs/               # Generated reports
    ├── .bundle/               # Export bundles
    ├── .env                   # Personal API keys, settings
    ├── settings.json          # Personal configuration
    └── *.local.*              # Local-only files
```

## Creating the Public Repository

### Option 1: GitHub Public Repo

1. Create new public repository on GitHub: `fyi-cli`
2. Clone to a separate directory:
   ```bash
   git clone git@github.com:yourusername/fyi-cli.git
   ```
3. Copy only public files:
   ```bash
   # From project root
   rsync -av --exclude='*.db' --exclude='data/' --exclude='outputs/' \
         --exclude='.env' --exclude='settings.json' --exclude='.bundle/' \
         ./ fyi-cli/
   ```
4. Add public README:
   ```bash
   cp PUBLIC_README.md fyi-cli/README.md
   cp .gitignore.public fyi-cli/.gitignore
   ```
5. Commit and push:
   ```bash
   cd fyi-cli
   git add .
   git commit -m "Initial public release of FYI Request System"
   git push -u origin main
   ```

### Option 2: Git Submodule

Keep public tool as a submodule in your private repo:
```bash
# In your private repo
git submodule add https://github.com/yourusername/fyi-cli.git tool
```

## Personal Configuration

Create a `.env` file for personal settings (never commit):

```bash
# .env (DO NOT COMMIT)
FYI_SYSTEM_PRIVACY_PROFILE=strict
FYI_SYSTEM_BIND_HOST=127.0.0.1
FYI_SYSTEM_SANITIZE_BUNDLE_EXPORTS=true

# Personal FYI.org.nz account (if using API)
FYI_API_TOKEN=your_token_here
```

## Data Separation

### Public (Safe to Share)
- Source code
- Test suite
- Sample data (`data/sample_*.csv`)
- Documentation
- Configuration templates

### Private (Never Share)
- SQLite databases (`*.db`)
- Your authority lists
- Generated reports
- Export bundles
- API tokens
- Personal settings

## Workflow

### Tool Development (Public)
1. Work on tool features in public repo
2. Write tests, achieve >95% coverage
3. Run mutation testing
4. Publish releases

### Personal Use (Private)
1. Install tool from public repo: `pip install -e /path/to/tool`
2. Store personal data in separate directory
3. Configure via `.env` (not committed)
4. Run tool against your data

## Security Considerations

1. **Never commit databases**: They contain your actual requests
2. **Never commit API tokens**: Use environment variables
3. **Sanitize exports**: Use `--profile=strict` for bundles
4. **Review before sharing**: Check for accidental data leaks

## Example Commands

### Public Tool Development
```bash
cd fyi-cli  # Public repo
pytest --cov=fyi_system
ruff check .
git commit -m "feat: Add new feature"
```

### Private Personal Use
```bash
cd ~/private/fyi-data  # Private directory
fyi-system init-db
fyi-system import-authorities data/my_authorities.csv
fyi-system serve
```

## Questions?

See `PUBLIC_README.md` for tool documentation.
See `handover/migration-report.md` for project history.
