# S.A.M. Bug Tracker

## Critical Severity Bugs 🔴

### 1. Command Injection Vulnerabilities
**Status:** Active  
**Severity:** CRITICAL  
**Files Affected:**
- `src/sam/tools.rs` - Deprecated `uinx_cmd()` function still in use
- `src/sam/services/media/snapcast.rs`
- `src/sam/services/who.rs`
- `src/sam/http/api/observations.rs`

**Description:** The deprecated `uinx_cmd()` function allows direct command injection through unsanitized user input. Multiple files still use this function with format strings, creating severe security vulnerabilities.

**Example:**
```rust
crate::sam::tools::uinx_cmd(&format!("rm -rf {obama_zip}"));
```

**Fix Required:** Replace all instances of `uinx_cmd()` with `safe_uinx_cmd()` immediately.

---

### 2. Unsafe Memory Transmutation
**Status:** Active  
**Severity:** CRITICAL  
**File:** `src/sam/db/connection_pool.rs:306`

**Description:** Unsafe memory transmutation to change lifetime parameters can lead to undefined behavior and memory safety violations.

**Code:**
```rust
unsafe { std::mem::transmute(params.as_slice()) };
```

**Fix Required:** Redesign parameter handling to avoid unsafe operations.

---

### 3. Hardcoded Database Credentials
**Status:** Active  
**Severity:** CRITICAL  
**File:** `src/main.rs:289-293`

**Description:** Database credentials are hardcoded in the source code, exposing the database to unauthorized access.

**Code:**
```rust
std::env::set_var("PG_DBNAME", "sam");
std::env::set_var("PG_USER", "sam");
std::env::set_var("PG_PASS", "sam");
```

**Fix Required:** Implement secure credential management using environment variables or secure configuration files.

---

## High Severity Bugs 🟠

### 4. Extensive Use of unwrap()/expect()
**Status:** Active  
**Severity:** HIGH  
**Files Affected:** 100+ instances across multiple files

**Key Locations:**
- `src/main.rs:71,225,233,234` - Critical runtime initialization
- `src/sam/services/spotify.rs` - Multiple panic points
- `src/sam/logging/mod.rs:379,380` - Prometheus metrics encoding

**Description:** Excessive use of `unwrap()` and `expect()` can cause application crashes and denial of service.

**Fix Required:** Replace with proper error handling using Result types and error propagation.

---

### 5. SQL Injection Risks
**Status:** Partially Mitigated  
**Severity:** HIGH  
**File:** `src/sam/memory/config/mod.rs:777-792`

**Description:** Dynamic SQL construction using format macros, though some validation exists through `validate_sql_identifier()`.

**Code Examples:**
```rust
execquery = format!("{execquery} WHERE {col} ${counter}");
execquery = format!("{execquery} ORDER BY {order_val}");
```

**Fix Required:** Use parameterized queries exclusively, avoid dynamic SQL construction.

---

### 6. Resource Leaks
**Status:** Active  
**Severity:** HIGH  
**Files Affected:**
- `src/sam/services/backup.rs`
- `src/sam/services/ssh.rs`
- `src/sam/services/p2p/file_sharing.rs`

**Description:** Many File::open() and TcpStream::connect() operations without explicit resource cleanup.

**Fix Required:** Implement proper RAII patterns and Drop traits where needed.

---

## Medium Severity Bugs 🟡

### 7. Potential Deadlocks in Concurrent Code
**Status:** Active  
**Severity:** MEDIUM  
**Files Affected:**
- `src/sam/services/thread_manager.rs:35` - Global static mutex
- `src/sam/services/p2p/enhanced.rs:116-120` - Multiple nested locks
- `src/sam/services/spotify.rs:629` - Panic on lock poisoning

**Description:** Extensive use of Arc<Mutex<T>> without clear deadlock prevention strategies.

**Fix Required:** Review lock ordering, implement timeout mechanisms, use async-aware locks.

---

### 8. Inconsistent Input Validation
**Status:** Partially Implemented  
**Severity:** MEDIUM  
**File:** `src/sam/security/validation_middleware.rs`

**Description:** Validation middleware exists but is not consistently applied across all API endpoints.

**Fix Required:** Enforce validation middleware on all HTTP endpoints systematically.

---

### 9. Path Traversal Vulnerabilities
**Status:** Active  
**Severity:** MEDIUM  
**File:** `src/sam/tools.rs:418`

**Description:** File operations on user-controlled paths without proper validation.

**Code:**
```rust
let mut outfile = fs::File::create(&outpath)?;
```

**Fix Required:** Canonicalize and validate all file paths before operations.

---

## Low Severity Bugs 🟢

### 10. Inefficient Sorting Operations
**Status:** Active  
**Severity:** LOW  
**File:** `src/sam/services/monitoring.rs:145`

**Description:** Sorting operations using unwrap() on partial comparisons can panic with NaN values.

**Code:**
```rust
sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
```

**Fix Required:** Handle NaN cases properly in floating-point comparisons.

---

### 11. Missing Error Context
**Status:** Active  
**Severity:** LOW  
**Multiple Files**

**Description:** Many error conditions lack sufficient context for debugging, using generic error messages.

**Fix Required:** Add structured logging and detailed error context.

---

### 12. Use of Deprecated Functions
**Status:** Active  
**Severity:** LOW  
**Multiple Files**

**Description:** Continued use of functions marked with #[deprecated] attribute.

**Fix Required:** Migrate to recommended alternatives.

---

## Bug Statistics

| Severity | Count | Status |
|----------|-------|--------|
| Critical | 3 | Active |
| High | 3 | Active/Partial |
| Medium | 3 | Active/Partial |
| Low | 3 | Active |

**Total Bugs Tracked:** 12

## Priority Action Items

1. **IMMEDIATE:** Fix all command injection vulnerabilities (Bug #1)
2. **IMMEDIATE:** Remove unsafe memory operations (Bug #2)
3. **IMMEDIATE:** Secure database credentials (Bug #3)
4. **HIGH:** Replace critical unwrap()/expect() calls (Bug #4)
5. **HIGH:** Audit SQL query construction (Bug #5)
6. **HIGH:** Fix resource leaks (Bug #6)

## Testing Requirements

- Security penetration testing after fixing critical vulnerabilities
- Unit tests for all error handling paths
- Integration tests for SQL injection prevention
- Resource leak detection tests
- Concurrency stress tests for deadlock detection

## Notes

- This bug list was generated from static code analysis on 2025-09-06
- Additional runtime testing may reveal more issues
- Regular security audits recommended using cargo-audit and cargo-geiger
- Consider implementing a bug bounty program once critical issues are resolved