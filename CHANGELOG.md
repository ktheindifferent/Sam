# Changelog

All notable changes to the S.A.M. project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### 🚨 Security Fixes
- **CRITICAL**: Fixed command injection vulnerabilities in `tools::cmd()` and `tools::uinx_cmd()`
  - Added secure alternatives: `safe_cmd()` and `safe_uinx_cmd()`
  - Deprecated vulnerable functions with compiler warnings
  - Updated sound processing service to use secure command execution
- **HIGH**: Fixed SQL injection vulnerabilities in database operations
  - Added comprehensive input validation for SQL identifiers
  - Implemented allowlists for table names, column names, and ORDER BY clauses
  - Added length limits and keyword checking
- **HIGH**: Fixed application panic on network errors in LIFX service
  - Replaced `panic!()` with proper error handling and graceful degradation
  - Added retry logic for transient network errors

### 🐛 Bug Fixes
- Fixed critical `.unwrap()` calls that could cause application crashes
  - Sound service database operations now use proper error handling
  - File operations now handle errors gracefully with logging
  - Observation processing includes fallback error recovery
- Improved error messages and logging throughout the application
- Fixed logic error in sound processing where `.unwrap()` was called after null check

### 📚 Documentation
- Added comprehensive `SECURITY.md` with security guidelines and best practices
- Updated `README.md` with proper installation instructions and security notice
- Added migration guide for moving from vulnerable to secure functions
- Documented all known remaining security issues with severity levels

### 🔧 Code Quality Improvements
- Added extensive input validation functions
- Enhanced error handling patterns across the codebase
- Improved logging for debugging and security monitoring
- Added security-focused code comments and documentation

### ⚠️ Deprecations
- `tools::cmd()` - Use `tools::safe_cmd()` instead
- `tools::uinx_cmd()` - Use `tools::safe_uinx_cmd()` instead
- `tools::python3()` - Enhanced with input validation, consider `python3_with_args()`

## [0.0.2] - Previous Version

### Known Issues Fixed in This Version
- Command injection vulnerabilities
- SQL injection vulnerabilities  
- Application crashes from network errors
- Excessive use of `.unwrap()` causing panics

## [0.0.1] - Initial Release

### Added
- Basic S.A.M. framework
- Web interface
- Database integration
- Service architecture
- AI/ML pipeline integration

---

### Security Vulnerability Disclosure

If you discover a security vulnerability, please refer to [SECURITY.md](SECURITY.md) for responsible disclosure procedures.

### Migration Notes

#### From vulnerable command execution:
```rust
// OLD (vulnerable)
tools::cmd("ffmpeg -i input.wav output.wav");

// NEW (secure)  
tools::safe_cmd("ffmpeg", &["-i", "input.wav", "output.wav"]);
```

#### From unsafe error handling:
```rust
// OLD (can panic)
let result = risky_operation().unwrap();

// NEW (safe)
let result = match risky_operation() {
    Ok(value) => value,
    Err(e) => {
        log::error!("Operation failed: {}", e);
        return Err(e.into());
    }
};
```

### Contributors
- Terragon Labs Security Team
- [Original S.A.M. contributors]

Last Updated: 2025-08-07