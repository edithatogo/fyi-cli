# Security Policy

## Supported Versions

| Version | Supported          |
| ------- | ------------------ |
| 1.0.x   | :white_check_mark: |
| 0.14.x  | :warning: Security fixes only |
| < 0.14  | :x:                |

## Reporting a Vulnerability

We take the security of FYI Request System seriously. If you believe you have found a security vulnerability, please report it to us as described below.

### How to Report

**Please do NOT report security vulnerabilities through public GitHub issues.**

Instead, please report them via email to: **security@fyi-cli.example.com**

Or use GitHub's private vulnerability reporting feature:
https://github.com/yourusername/fyi-cli/security/advisories/new

### What to Include

Please include the following information in your report:

- A description of the vulnerability
- Steps to reproduce the issue
- Potential impact of the vulnerability
- Any suggested fixes (if applicable)
- Your contact information for follow-up

### Response Time

You can expect:
- **Acknowledgment:** Within 48 hours
- **Initial assessment:** Within 5 business days
- **Status update:** Within 10 business days
- **Resolution timeline:** Depends on severity (see below)

### Severity Levels

| Severity | Response Timeline | Description |
|----------|------------------|-------------|
| **Critical** | 24-48 hours | Remote code execution, data breach |
| **High** | 5 business days | Privilege escalation, authentication bypass |
| **Medium** | 10 business days | XSS, CSRF, information disclosure |
| **Low** | 20 business days | Minor security issues |

### Process

1. **Report** - Submit your findings via email or GitHub advisory
2. **Acknowledge** - We'll confirm receipt within 48 hours
3. **Assess** - We'll evaluate the vulnerability and determine severity
4. **Fix** - We'll develop and test a fix
5. **Release** - We'll release a patched version
6. **Disclose** - Public disclosure after users have had time to update

### Disclosure Policy

- We will notify you when the vulnerability has been fixed
- We may request that you keep the vulnerability confidential until a fix is released
- We will credit you in the security advisory (unless you prefer to remain anonymous)
- We request that you do not disclose the vulnerability publicly before we release a fix

### Security Best Practices for Users

To keep your installation secure:

1. **Keep updated** - Always use the latest version
2. **Protect API keys** - Store API keys securely, never commit to version control
3. **Use encryption** - Enable encryption for sensitive data
4. **Review permissions** - Regularly audit file permissions
5. **Monitor logs** - Check logs for suspicious activity

### Security Measures in This Project

- **Dependency scanning** - Automated vulnerability scanning on every commit
- **CodeQL analysis** - Static analysis for security issues
- **Secret scanning** - GitHub secret scanning enabled
- **Signed commits** - Commit signing encouraged
- **Branch protection** - Main branch protected
- **Required reviews** - Pull requests require review

### Past Security Advisories

For a list of past security advisories, see:
https://github.com/yourusername/fyi-cli/security/advisories

### Contact

- **Email:** security@fyi-cli.example.com
- **GitHub Advisories:** https://github.com/yourusername/fyi-cli/security/advisories/new

---

**Thank you for helping keep FYI Request System secure!**
