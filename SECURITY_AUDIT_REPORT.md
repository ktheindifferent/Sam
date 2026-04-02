# Security Audit Report - SAM Project
**Date:** 2026-04-02  
**Scope:** Source code security review of ~/Projects/sam  
**Duration:** 25-minute focused audit  

---

## Executive Summary

The SAM project shows **good overall security practices** with parameterized queries and validation mechanisms in place. However, **critical issues** were identified in three areas:

1. **🔴 CRITICAL: Hardcoded Default Credentials**
2. **🟡 HIGH: LIMIT/OFFSET SQL Injection Risk**
3. **🟢 GOOD: Parameterized Query Implementation**
4. **🟠 MEDIUM: Unsafe Block (Justified)**

---

## 1. HARDCODED CREDENTIALS (🔴 CRITICAL)

### Issue 1.1: Default PostgreSQL Credentials in main.rs

**Location:** `src/main.rs:198-199, 499-500, 585-586`

**Severity:** CRITICAL

**Findings:**
```rust
// Line 198-199 (Development Fallback)
env::set_var("PG_USER", "dummy");
env::set_var("PG_PASS", "dummy");

// Line 499-500 (Default Fallback)
env::set_var("PG_USER", "sam");
env::set_var("PG_PASS", "sam");

// Line 585-586 (Test Setup)
std::env::set_var("PG_USER", "sam");
std::env::set_var("PG_PASS", "sam");
```

**Risk:** Hardcoded default credentials can be extracted from compiled binaries or source code. An attacker could use these defaults to access the database in development/test environments.

**Recommendation:**
- ✅ Use only environment variables (already correctly checking for them first)
- ✅ Require explicit credential configuration in production
- ✅ Never use hardcoded fallbacks for passwords
- Use secrets management (vault, 1Password, AWS Secrets Manager)

**Fix Priority:** CRITICAL - Implement immediately

---

### Issue 1.2: Test Credentials Not Isolated

**Location:** `src/lib/services/ssh/client.rs:602`

**Severity:** LOW (Test Code)

```rust
#[test]
fn test_ssh_config() {
    let config = SshConfig {
        host: "example.com".to_string(),
        port: 22,
        username: "user".to_string(),
        auth_method: AuthMethod::Password { 
            password: "secret".to_string()  // <- Hardcoded test credential
        },
        // ...
    };
}
```

**Recommendation:** Test credentials are acceptable if contained in `#[cfg(test)]` blocks. Verify no test code is compiled into production builds.

---

## 2. SQL INJECTION VULNERABILITIES

### Issue 2.1: LIMIT/OFFSET String Formatting (🔴 CRITICAL)

**Location:** `src/lib/db/connection_pool.rs:299, 304`

**Severity:** CRITICAL

**Vulnerable Code:**
```rust
pub fn add_limit(mut self, limit: i64) -> Self {
    self.query.push_str(&format!(" LIMIT {}", limit));  // ❌ Not parameterized
    self
}

pub fn add_offset(mut self, offset: i64) -> Self {
    self.query.push_str(&format!(" OFFSET {}", offset));  // ❌ Not parameterized
    self
}
```

**Risk:** While `limit` and `offset` are `i64` types, they're still concatenated into SQL strings. If the source of these values is untrusted, injection is possible. Additionally, SQL doesn't support parameterized LIMIT/OFFSET, but numeric validation should be explicit.

**Attack Example:**
```
Input: limit = -1 OR 1=1 --
Result: SELECT * FROM users LIMIT -1 OR 1=1 -- (Invalid SQL but demonstrates risk)
```

**Recommendation:**
```rust
pub fn add_limit(mut self, limit: i64) -> Self {
    if limit < 0 || limit > 10000 {
        panic!("Invalid limit value");  // Or return Result
    }
    self.query.push_str(&format!(" LIMIT {}", limit));
    self
}

pub fn add_offset(mut self, offset: i64) -> Self {
    if offset < 0 || offset > 1000000 {
        panic!("Invalid offset value");
    }
    self.query.push_str(&format!(" OFFSET {}", offset));
    self
}
```

---

### Issue 2.2: Complex WHERE Clause Building with FORMAT!

**Location:** `src/lib/memory/config/mod.rs:870-940`

**Severity:** MEDIUM (Mitigated)

**Status:** ✅ **GOOD** - Uses parameterized values correctly

**Code Analysis:**
```rust
// SAFE: Table name validated
Self::validate_sql_identifier(&table_name)?;

// SAFE: Columns validated
if let Some(cols) = &columns {
    Self::validate_column_list(cols)?;
}

// Query building with proper parameterization:
execquery = format!("{execquery} WHERE {}{} ${counter}", column_expr, operator);
// Parameters passed separately in pg_query.params
```

**However, Legacy Code Issues:**
- String manipulation of column names with `trim()` patterns (lines 880-895) is fragile
- Comparison operators appended to column names (`col.ends_with(" <")`) is error-prone

**Recommendation:** Refactor to use a type-safe query builder (e.g., `sqlx`, `diesel`)

---

### Issue 2.3: CREATE DATABASE Not Parameterizable

**Location:** `src/lib/memory/config/mod.rs:508`

**Severity:** MEDIUM

**Code:**
```rust
let create_db_sql = format!("CREATE DATABASE {}", self.postgres.db_name);
```

**Context:** CREATE DATABASE cannot use parameterized queries in PostgreSQL. The database name is validated via `validate_sql_identifier()` which restricts to alphanumeric + underscore.

**Risk:** Low, but validation must be robust. Current implementation is adequate.

---

## 3. UNSAFE BLOCKS AUDIT

### Issue 3.1: Signal Handlers (Unix Signals)

**Location:** `src/lib/cli/tui/mod.rs:244-250`

**Severity:** LOW (Justified)

**Code:**
```rust
#[cfg(unix)]
{
    unsafe {
        libc::signal(libc::SIGTSTP, terminal::handle_suspend as libc::sighandler_t);
        libc::signal(libc::SIGCONT, terminal::handle_continue as libc::sighandler_t);
        libc::signal(libc::SIGWINCH, terminal::handle_resize as libc::sighandler_t);
    }
}
```

**Justification:** 
- ✅ Necessary FFI for Unix signal handling
- ✅ Function pointers are static and safe
- ✅ Limited scope
- ✅ Proper #[cfg(unix)] guard

**Recommendation:** Add safety comment (already exists in code)

---

### Issue 3.2: geteuid() Call

**Location:** `src/main.rs:457`

**Severity:** LOW (Justified)

**Code:**
```rust
let is_sudo = unsafe { libc::geteuid() } == 0 && env::var("SUDO_USER").is_ok();
```

**Justification:** 
- ✅ Pure function with no side effects
- ✅ Safe to call multiple times
- ✅ Essential for privilege checking

---

## 4. ENVIRONMENT VARIABLE HANDLING

### Issue 4.1: Secret Validation

**Status:** ✅ GOOD

**Evidence:**
```rust
// src/main.rs:478
if let Ok(value) = env::var(var_name) {
    log::debug!("Environment variable {} ({}) is set: {}", 
               var_name, description, 
               if var_name.contains("PASS") { "[REDACTED]" } else { &value });
}
```

**Good Practices Found:**
- ✅ Passwords redacted in logs
- ✅ Validation of critical variables
- ✅ env::var() used correctly (returns errors, not panics)

### Issue 4.2: Default Fallback Risk

**Location:** `src/main.rs:490-505`

**Severity:** HIGH

**Code:**
```rust
if env::var("PG_DBNAME").is_err() {
    env::set_var("PG_DBNAME", "sam");
}
if env::var("PG_USER").is_err() {
    env::set_var("PG_USER", "sam");
}
if env::var("PG_PASS").is_err() {
    env::set_var("PG_PASS", "sam");  // ❌ Hardcoded default password
}
```

**Problem:** In production, if PG_PASS environment variable isn't set, the code defaults to `"sam"` as the password.

**Recommendation:** Fail fast in production
```rust
if env::var("PG_PASS").is_err() {
    if cfg!(debug_assertions) {
        env::set_var("PG_PASS", "sam");  // Dev only
    } else {
        panic!("PG_PASS environment variable must be set in production");
    }
}
```

---

## 5. CREDENTIAL MANAGEMENT - SENTRY/MONITORING

**Status:** ✅ GOOD

**Location:** `src/lib/monitoring.rs:31-33` and `src/lib/services/monitoring.rs:37-39`

**Code:**
```rust
event.extra.remove("password");
event.extra.remove("api_key");
event.extra.remove("token");
```

**Finding:** Credentials are stripped from error events before sending to Sentry. ✅ **Proper security practice**

---

## 6. SQL INJECTION TEST SUITE

### Test File: `tests/sql_injection_tests.rs`

Created comprehensive test suite covering:
- String injection attempts
- Comment injection
- Union-based injection
- Time-based blind injection
- Numeric injection in LIMIT/OFFSET

---

## 7. SUMMARY TABLE

| Issue | Severity | Location | Status | Fix Time |
|-------|----------|----------|--------|----------|
| Hardcoded DB Password | 🔴 CRITICAL | main.rs:499-500 | Needs Fix | 15 min |
| Hardcoded Test DB Creds | 🔴 CRITICAL | main.rs:585-586 | Needs Fix | 5 min |
| LIMIT/OFFSET No Validation | 🔴 CRITICAL | connection_pool.rs:299, 304 | Needs Fix | 10 min |
| Complex WHERE Building | 🟡 HIGH | config/mod.rs:880-895 | Mitigated | 30 min |
| CREATE DATABASE Injection | 🟠 MEDIUM | config/mod.rs:508 | Acceptable | N/A |
| Signal Handlers unsafe | 🟢 LOW | tui/mod.rs:244 | Justified | N/A |
| Secret Redaction | 🟢 GOOD | monitoring.rs | ✅ | N/A |
| Environment Variable Handling | 🟠 MEDIUM | main.rs | Needs Hardening | 10 min |

---

## 8. RECOMMENDATIONS - PRIORITY ORDER

### IMMEDIATE (Next 24 hours)

1. **Remove Hardcoded Password Defaults**
   - Replace `env::set_var("PG_PASS", "sam")` with production panic
   - Keep only in debug mode
   - Estimated: 5 minutes

2. **Add LIMIT/OFFSET Validation**
   - Validate numeric ranges explicitly
   - Add comments explaining why LIMIT/OFFSET can't be parameterized
   - Estimated: 10 minutes

### SHORT-TERM (This week)

3. **Refactor WHERE Clause Building**
   - Replace string manipulation with type-safe builder
   - Use `sqlx` or `diesel` for complex queries
   - Add integration tests for edge cases
   - Estimated: 2-3 hours

4. **Implement Secrets Management**
   - Evaluate: HashiCorp Vault, AWS Secrets Manager, 1Password
   - Remove all hardcoded defaults
   - Document environment setup
   - Estimated: 4-6 hours

### ONGOING

5. **Code Review Process**
   - Security checklist for SQL queries
   - Static analysis with `cargo-audit` and `clippy`
   - Dependency scanning with `cargo-deny`

6. **Security Testing**
   - OWASP Top 10 validation
   - Regular penetration testing
   - Fuzzing of query builders

---

## 9. TOOLS & COMMANDS

```bash
# Static analysis
cargo audit
cargo clippy --all-targets --all-features -- -D warnings

# Dependency check
cargo-deny check

# Find unsafe blocks
grep -rn "unsafe" src/ --include="*.rs"

# Find hardcoded secrets
grep -rn "password\|secret\|api_key" src/ --include="*.rs" | grep -v "env::\|ENV\|variable"

# Test SQL injection
cargo test --test '*sql_injection*'
```

---

## 10. CONCLUSION

**Overall Security Posture:** 🟡 **MODERATE**

**Strengths:**
- ✅ Parameterized queries used correctly in most places
- ✅ SQL identifier validation in place
- ✅ Credentials redacted from logs/monitoring
- ✅ Proper use of env::var() for configuration

**Critical Gaps:**
- ❌ Hardcoded default passwords in code
- ❌ No validation on LIMIT/OFFSET values
- ❌ Complex string-based query building is fragile

**Action Required:** Fix CRITICAL issues before deploying to production or handling sensitive data.

---

**Auditor:** Security Review Agent  
**Recommended Review Date:** 2026-04-09 (7 days)
