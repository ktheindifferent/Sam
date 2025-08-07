# Security Guidelines for S.A.M.

This document outlines security considerations, known vulnerabilities, and best practices for the S.A.M. (Smart Artificial Mind) project.

## 🚨 Critical Security Issues Fixed

### 1. Command Injection Vulnerabilities (FIXED)
**Impact**: Remote Code Execution  
**Location**: `src/sam/tools.rs`  
**Status**: ✅ FIXED

**Issue**: The `cmd()` and `uinx_cmd()` functions were vulnerable to command injection attacks by passing user input directly to shell execution.

**Fix**: 
- Added secure alternatives: `safe_cmd()` and `safe_uinx_cmd()`
- Deprecated vulnerable functions with warnings
- Updated usage throughout codebase to use safe functions

**Migration Guide**:
```rust
// OLD (VULNERABLE):
tools::cmd("ls -la /tmp");
tools::uinx_cmd("ffmpeg -i input.wav output.wav");

// NEW (SECURE):
tools::safe_cmd("ls", &["-la", "/tmp"]);
tools::safe_uinx_cmd("ffmpeg", &["-i", "input.wav", "output.wav"]);
```

### 2. SQL Injection Prevention (FIXED)
**Impact**: Data breach, database corruption  
**Location**: `src/sam/memory/config/mod.rs`  
**Status**: ✅ FIXED

**Issue**: Dynamic SQL query construction without proper validation allowed SQL injection through table names, column names, and ORDER BY clauses.

**Fix**:
- Added comprehensive input validation for all SQL identifiers
- Implemented allowlists for valid SQL identifier characters
- Added length limits and keyword checking
- Enhanced error messages for debugging

### 3. Network Error Panic (FIXED)
**Impact**: Application crashes  
**Location**: `src/sam/services/lifx/lifx_api_server.rs:565`  
**Status**: ✅ FIXED

**Issue**: Network errors caused the entire application to crash with `panic!()`.

**Fix**: Replaced panic with proper error handling and graceful degradation.

### 4. Unsafe unwrap() Usage (PARTIALLY FIXED)
**Impact**: Application crashes  
**Status**: 🔄 IN PROGRESS

**Issue**: Excessive use of `.unwrap()` calls throughout the codebase that can cause panics.

**Progress**:
- ✅ Fixed critical unwrap() calls in sound processing
- ✅ Fixed database operation unwraps
- ⚠️ 200+ instances remain (see scan results for details)

## 🔒 Security Best Practices

### Input Validation
All user inputs must be validated before processing:
- Use type-safe parsing where possible
- Validate string lengths and character sets
- Implement allowlists for known-good values
- Sanitize file paths and prevent directory traversal

### Command Execution
- ❌ Never use `tools::cmd()` or `tools::uinx_cmd()` with user input
- ✅ Always use `tools::safe_cmd()` or `tools::safe_uinx_cmd()`
- ✅ Separate command and arguments
- ✅ Validate all file paths

### Database Operations
- ✅ Use parameterized queries for user data
- ✅ Validate all SQL identifiers
- ✅ Implement connection pooling with limits
- ✅ Log all database errors appropriately

### Error Handling
- ❌ Never use `.unwrap()` on operations that can fail
- ❌ Never use `panic!()` for recoverable errors
- ✅ Use proper `Result<T, E>` types
- ✅ Implement graceful error recovery
- ✅ Log errors appropriately

## 🚨 Known Remaining Issues

### High Priority
1. **Command injection in older usage** - Some files still use deprecated `uinx_cmd()`
2. **File system race conditions** - TOCTOU vulnerabilities in file operations
3. **Unchecked array access** - Some array indexing without bounds checking

### Medium Priority
1. **Integer overflow risks** - Unchecked `as usize` conversions
2. **Resource leaks** - Some file handles not properly closed
3. **Async race conditions** - Mutex usage in async contexts

### Low Priority
1. **Logic errors** - Some unnecessary unwrap() calls after null checks
2. **Dead code** - Unreachable code paths that should be removed

## 🛡️ Defensive Programming Guidelines

### For Contributors

1. **Never ignore errors**
   ```rust
   // BAD
   let result = risky_operation().unwrap();
   
   // GOOD
   let result = match risky_operation() {
       Ok(value) => value,
       Err(e) => {
           log::error!("Operation failed: {}", e);
           return Err(e.into());
       }
   };
   ```

2. **Validate all inputs**
   ```rust
   // BAD
   fn process_file(filename: &str) {
       let content = fs::read(filename).unwrap();
   }
   
   // GOOD
   fn process_file(filename: &str) -> Result<()> {
       // Validate filename
       if filename.contains("..") || filename.starts_with('/') {
           return Err("Invalid filename".into());
       }
       
       let content = fs::read(filename)?;
       // ... process content
       Ok(())
   }
   ```

3. **Use secure command execution**
   ```rust
   // BAD
   tools::cmd(&format!("convert {} {}", input, output));
   
   // GOOD
   tools::safe_cmd("convert", &[input, output]);
   ```

## 🚨 Security Reporting

If you discover a security vulnerability, please:
1. **DO NOT** open a public issue
2. Email security details privately to the maintainers
3. Include steps to reproduce
4. Allow reasonable time for fix before disclosure

## 🔍 Security Scanning

Regular security scanning should be performed:

```bash
# Scan for unwrap() usage
grep -r "\.unwrap()" src/

# Check for command injection patterns
grep -r "tools::cmd\|tools::uinx_cmd" src/

# Look for SQL injection patterns
grep -r "format!\|format_args!" src/ | grep -i sql

# Find unsafe code blocks
grep -r "unsafe" src/
```

## 📚 References

- [Rust Security Guidelines](https://anssi-fr.github.io/rust-guide/)
- [OWASP Secure Coding Practices](https://owasp.org/www-project-secure-coding-practices-quick-reference-guide/)
- [Common Weakness Enumeration (CWE)](https://cwe.mitre.org/)

---

**Last Updated**: 2025-08-07  
**Version**: 0.0.2  
**Contributors**: Terragon Labs Security Team