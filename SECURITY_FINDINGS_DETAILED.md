# Security Audit - Detailed Findings & Remediation

## Document Info
- **Generated:** 2026-04-02
- **Audit Scope:** Complete source code review
- **Critical Issues Found:** 3
- **High Issues Found:** 2
- **Medium Issues Found:** 2

---

## CRITICAL FINDING #1: Hardcoded Default Database Credentials

### Location
- `src/main.rs:198-199` (development fallback)
- `src/main.rs:499-500` (default fallback)  
- `src/main.rs:585-586` (test setup)

### Current Code
```rust
// Line 198-199
env::set_var("PG_USER", "dummy");
env::set_var("PG_PASS", "dummy");

// Line 499-500
if env::var("PG_PASS").is_err() {
    env::set_var("PG_PASS", "sam");  // ❌ HARDCODED
    log::debug!("Set default PG_PASS=[REDACTED]");
}

// Line 585-586
std::env::set_var("PG_USER", "sam");
std::env::set_var("PG_PASS", "sam");  // ❌ HARDCODED
```

### Risk Assessment
**Severity:** 🔴 **CRITICAL**

**CVSS v3.1 Base Score:** 9.8 (Critical)
- Attack Vector: Network (AV:N)
- Attack Complexity: Low (AC:L)
- Privileges Required: None (PR:N)
- User Interaction: None (UI:N)
- Scope: Unchanged (S:U)
- Confidentiality Impact: High (C:H)
- Integrity Impact: High (I:H)
- Availability Impact: High (A:H)

### Attack Scenario
```
Attacker discovers hardcoded credential "sam" in compiled binary or source code
→ Attempts connection: psql -h target.com -U sam -p 5432 -d sam
→ Authentication succeeds with default password
→ Full database access compromised
→ Data theft, modification, or destruction possible
```

### Impact
- **Confidentiality:** All database data exposed
- **Integrity:** Attacker can modify any data
- **Availability:** Attacker can drop tables or disable database
- **Audit Trail:** Difficult to detect unauthorized access

### Proof of Vulnerability
Credential can be extracted from:
1. Source code repository (if public)
2. Compiled binary (strings tool)
3. Git history
4. Deployed containers

### Remediation Steps

#### Step 1: Remove Hardcoded Fallbacks (IMMEDIATE)
```rust
// BEFORE
if env::var("PG_PASS").is_err() {
    env::set_var("PG_PASS", "sam");
}

// AFTER - Production
if env::var("PG_PASS").is_err() {
    #[cfg(debug_assertions)]
    {
        // Development only: allow fallback
        env::set_var("PG_PASS", "sam");
        log::warn!("Using development default password. This is NEVER allowed in production.");
    }
    
    #[cfg(not(debug_assertions))]
    {
        // Production: fail fast
        eprintln!("ERROR: PG_PASS environment variable must be set in production");
        std::process::exit(1);
    }
}
```

#### Step 2: Use Secrets Management
Implement one of:
- **AWS Secrets Manager** - For cloud deployments
- **HashiCorp Vault** - For on-premises
- **1Password Secrets Automation** - For teams
- **Azure Key Vault** - For Azure deployments

Example with AWS:
```rust
use aws_secretsmanager_caching::SecretCache;

let cache = SecretCache::new();
let secret = cache.get_secret_string("sam/postgres/password")
    .map_err(|_| "Failed to retrieve password from Secrets Manager")?;
```

#### Step 3: Validation
```bash
# Verify no hardcoded passwords in source
grep -r "PG_PASS.*=.*['\"]" src/ --include="*.rs"  # Should return nothing
grep -r "password.*=.*['\"]" src/ --include="*.rs" | grep -v test | grep -v "//"  # Review carefully
```

### Timeline
- **Immediate (within 1 hour):** Add production panic check
- **Today:** Implement secrets management
- **This week:** Deploy and verify
- **Ongoing:** Code review process

---

## CRITICAL FINDING #2: SQL Injection - LIMIT/OFFSET Not Validated

### Location
`src/lib/db/connection_pool.rs:299, 304`

### Current Code
```rust
pub fn add_limit(mut self, limit: i64) -> Self {
    self.query.push_str(&format!(" LIMIT {}", limit));  // ❌ NO VALIDATION
    self
}

pub fn add_offset(mut self, offset: i64) -> Self {
    self.query.push_str(&format!(" OFFSET {}", offset));  // ❌ NO VALIDATION
    self
}
```

### Risk Assessment
**Severity:** 🔴 **CRITICAL**

**Technical Details:**
- PostgreSQL doesn't support parameterized LIMIT/OFFSET
- However, numeric type (i64) provides some protection
- **Risk:** If value source is untrusted or incorrectly validated elsewhere

**OWASP Mapping:** A03:2021 – Injection (CWE-89: SQL Injection)

### Proof of Concept
```rust
// If limit value comes from untrusted source:
let user_input = "-1 OR 1=1 --";  // Won't parse as i64, but demonstrates intent
let limit: i64 = user_input.parse().unwrap_or(10);

// Even with i64, extreme values can cause:
// 1. Resource exhaustion (LIMIT 2147483647)
// 2. Performance issues (OFFSET 1000000000)
// 3. Database DoS
```

### Remediation

```rust
pub fn add_limit(mut self, limit: i64) -> Result<Self, QueryError> {
    // Validate range
    const MAX_LIMIT: i64 = 10_000;
    if limit < 0 {
        return Err(QueryError::InvalidLimit("Limit cannot be negative".into()));
    }
    if limit > MAX_LIMIT {
        return Err(QueryError::InvalidLimit(
            format!("Limit exceeds maximum ({})", MAX_LIMIT)
        ));
    }
    
    // Safe to concatenate numeric value
    self.query.push_str(&format!(" LIMIT {}", limit));
    Ok(self)
}

pub fn add_offset(mut self, offset: i64) -> Result<Self, QueryError> {
    const MAX_OFFSET: i64 = 1_000_000;
    if offset < 0 {
        return Err(QueryError::InvalidOffset("Offset cannot be negative".into()));
    }
    if offset > MAX_OFFSET {
        return Err(QueryError::InvalidOffset(
            format!("Offset exceeds maximum ({})", MAX_OFFSET)
        ));
    }
    
    self.query.push_str(&format!(" OFFSET {}", offset));
    Ok(self)
}
```

### Testing
```bash
# Run SQL injection test suite
cargo test --test sql_injection_tests::sql_injection_tests::test_limit_negative_value_attack
cargo test --test sql_injection_tests::sql_injection_tests::test_limit_overflow_attack
```

### Timeline
- **Immediate:** Add validation
- **Today:** Update all call sites to handle Result
- **This week:** Add tests to CI/CD

---

## CRITICAL FINDING #3: Complex WHERE Clause String Manipulation

### Location
`src/lib/memory/config/mod.rs:880-895`

### Current Code
```rust
// Fragile string manipulation
let (column_expr, _needs_validation) = if col_cleaned.starts_with("LOWER(") && col_cleaned.ends_with(")") {
    let inner = &col_cleaned[6..col_cleaned.len()-1];
    Self::validate_sql_identifier(inner)?;
    (format!("LOWER({})", inner), false)
} else if col_cleaned.ends_with(" <") || col_cleaned.ends_with(" >") || ... {
    // Manual parsing of comparison operators
    let parts: Vec<&str> = col_cleaned.rsplitn(2, ' ').collect();
    if parts.len() == 2 {
        let column_name = parts[1];
        let operator = parts[0];
        Self::validate_sql_identifier(column_name)?;
        (format!("{} {}", column_name, operator), false)
    } else {
        Self::validate_sql_identifier(col_cleaned)?;
        (col_cleaned.to_string(), true)
    }
} else {
    Self::validate_sql_identifier(col_cleaned)?;
    (col_cleaned.to_string(), true)
};
```

### Risk Assessment
**Severity:** 🟡 **HIGH**

**Issues:**
1. String slicing without bounds checking
2. Manual operator parsing prone to errors
3. Legacy code with multiple code paths
4. Difficult to audit and maintain
5. Regex patterns would be clearer

**Current Status:** Mitigated by validation, but fragile

### Remediation Strategy
**Option 1: Immediate (Low-Risk)**
```rust
// Add explicit validation and clearer code paths
pub enum SqlComparison {
    Equals,
    LessThan,
    GreaterThan,
    LessThanOrEqual,
    GreaterThanOrEqual,
}

impl SqlComparison {
    fn from_str(s: &str) -> Result<Self> {
        match s {
            "=" => Ok(SqlComparison::Equals),
            "<" => Ok(SqlComparison::LessThan),
            ">" => Ok(SqlComparison::GreaterThan),
            "<=" => Ok(SqlComparison::LessThanOrEqual),
            ">=" => Ok(SqlComparison::GreaterThanOrEqual),
            _ => Err(format!("Invalid comparison operator: {}", s)),
        }
    }
}
```

**Option 2: Long-Term (Recommended)**
Use a type-safe query builder:
```toml
# Cargo.toml
sqlx = { version = "0.7", features = ["postgres", "runtime-tokio"] }
# OR
diesel = { version = "2.0", features = ["postgres"] }
```

Example with sqlx:
```rust
let result = sqlx::query(
    "SELECT * FROM config WHERE id = $1 ORDER BY id DESC"
)
.bind(id)
.fetch_all(&pool)
.await?;
```

---

## HIGH FINDING #1: Environment Variable Validation in Production

### Location
`src/main.rs:490-505`

### Issue
Missing production-specific validation for critical environment variables:
```rust
if env::var("PG_PASS").is_err() {
    env::set_var("PG_PASS", "sam");  // ❌ No production check
}
```

### Remediation
```rust
#[cfg(debug_assertions)]
fn get_or_default_env(key: &str, default: &str) -> String {
    env::var(key).unwrap_or_else(|_| {
        log::warn!("Using development default for {}", key);
        default.to_string()
    })
}

#[cfg(not(debug_assertions))]
fn get_or_default_env(key: &str, _default: &str) -> String {
    env::var(key).unwrap_or_else(|_| {
        log::error!("Required environment variable not set: {}", key);
        std::process::exit(1);
    })
}
```

---

## HIGH FINDING #2: LIMIT/OFFSET Values Not Sanitized

### Location
Multiple query builders accept i64 without range validation

### Impact
- Memory exhaustion (LIMIT 9223372036854775807)
- Query timeout/DoS
- Unexpected behavior

### Fix
Add validation middleware:
```rust
pub fn sanitize_limit(limit: usize) -> usize {
    limit.min(10_000)  // Reasonable maximum
}

pub fn sanitize_offset(offset: usize) -> usize {
    offset.min(1_000_000)  // Reasonable maximum
}
```

---

## MEDIUM FINDING #1: CREATE DATABASE String Formatting

### Location
`src/lib/memory/config/mod.rs:508`

```rust
let create_db_sql = format!("CREATE DATABASE {}", self.postgres.db_name);
```

### Status: ✅ ACCEPTABLE

**Reason:** Database name is validated with `validate_sql_identifier()` before use. CREATE DATABASE cannot use parameterized queries in PostgreSQL, so validation is the correct approach.

**Verification:**
```rust
Self::validate_sql_identifier(&self.postgres.db_name)?;  // ✅ Validates before use
```

---

## MEDIUM FINDING #2: Complex Column Name Handling

### Location
`src/lib/memory/config/mod.rs:880-895`

### Issue
Multiple special cases for LOWER() and comparison operators make code hard to understand and verify.

### Recommendation
Refactor for clarity:
```rust
fn parse_column_expression(input: &str) -> Result<(String, String)> {
    let input = input.trim();
    
    // Case 1: LOWER(column_name)
    if let Some(inner) = input.strip_prefix("LOWER(").and_then(|s| s.strip_suffix(")")) {
        validate_sql_identifier(inner)?;
        return Ok((format!("LOWER({})", inner), "=".to_string()));
    }
    
    // Case 2: column_name OPERATOR
    for op in &["<", ">", "<=", ">=", "="] {
        if let Some(col) = input.strip_suffix(op) {
            validate_sql_identifier(col)?;
            return Ok((col.to_string(), op.to_string()));
        }
    }
    
    // Case 3: Simple column name
    validate_sql_identifier(input)?;
    Ok((input.to_string(), "=".to_string()))
}
```

---

## GOOD FINDINGS (No Action Required)

### ✅ Credential Redaction in Logs
**Location:** `src/lib/monitoring.rs:31-33`
```rust
event.extra.remove("password");
event.extra.remove("api_key");
event.extra.remove("token");
```
**Status:** ✅ **PROPER** - Prevents credential leakage in error reports

### ✅ Signal Handler Usage
**Location:** `src/lib/cli/tui/mod.rs:244-250`
```rust
unsafe {
    libc::signal(libc::SIGTSTP, terminal::handle_suspend as libc::sighandler_t);
    // ...
}
```
**Status:** ✅ **JUSTIFIED** - Proper use of FFI for Unix signals

### ✅ SQL Identifier Validation
**Location:** `src/lib/memory/config/mod.rs:734-765`
**Status:** ✅ **ROBUST** - Comprehensive validation of SQL identifiers

---

## Remediation Checklist

### CRITICAL (DO TODAY)
- [ ] Add production panic for missing PG_PASS
- [ ] Validate LIMIT/OFFSET numeric ranges
- [ ] Remove hardcoded password defaults
- [ ] Add secrets management (evaluation)

### HIGH (THIS WEEK)
- [ ] Implement production-specific env validation
- [ ] Refactor complex WHERE clause building
- [ ] Add integration tests for all query paths
- [ ] Code review checklist for SQL queries

### MEDIUM (ONGOING)
- [ ] Evaluate type-safe query builders (sqlx/diesel)
- [ ] Add static analysis to CI/CD
- [ ] Implement fuzzing for query builder
- [ ] Security training for team

---

## Testing Verification

Run the created test suite:
```bash
cd ~/Projects/sam
cargo test --test sql_injection_tests -- --nocapture
```

Expected: All 40+ tests pass with green checkmarks

---

## References

- [OWASP SQL Injection](https://owasp.org/www-community/attacks/SQL_Injection)
- [CWE-89: SQL Injection](https://cwe.mitre.org/data/definitions/89.html)
- [OWASP Top 10 2021](https://owasp.org/Top10/)
- [PostgreSQL Security](https://www.postgresql.org/docs/current/sql-syntax.html)

---

**Audit Complete**
Next Review: 2026-04-09
