# S.A.M. Development TODO List - Updated 2026-04-02 11:19 UTC

## 🚨 CRITICAL SECURITY (P0 - DO FIRST)

### Command Injection Fix [BLOCKER]
- Replace all `uinx_cmd()` calls with `safe_uinx_cmd()`
  - src/sam/tools.rs
  - src/sam/services/media/snapcast.rs
  - src/sam/services/who.rs
  - src/sam/http/api/observations.rs
- Status: HIGH PRIORITY - Multiple active exploits

### Memory Safety
- Remove unsafe transmute in connection_pool.rs:306
- Audit all unsafe blocks
- Status: CRITICAL - Can cause undefined behavior

### Credential Management
- Remove hardcoded credentials in main.rs:289-293
- Implement env var validation
- Status: CRITICAL - Secrets exposed

---

## 🔴 HIGH PRIORITY (P1 - This Week)

### Error Handling Overhaul
- Replace 100+ unwrap()/expect() calls with Result types
- Focus on main.rs (lines 71,225,233,234)
- Focus on services/spotify.rs
- Focus on logging/mod.rs:379,380
- Status: IN PROGRESS

### SQL Security
- Audit memory/config/mod.rs:777-792
- Migrate to parameterized queries
- Add SQL injection test suite
- Status: AUDIT NEEDED

### Resource Management
- Fix leaks in backup.rs, ssh.rs, p2p/file_sharing.rs
- Implement RAII patterns
- Add leak detection tests
- Status: NOT STARTED

---

## 🟡 MEDIUM PRIORITY (P2 - This Month)

### Testing & CI
- Increase test coverage to 75%+
- Add integration tests for API endpoints
- Implement property-based testing
- Fix flaky tests in concurrent modules

### Documentation
- Update API documentation (docs/API.md)
- Add architecture diagrams
- Create deployment guides for Docker
- Document all configuration options

### Performance
- Optimize database query performance
- Reduce memory allocation in hot paths
- Profile and optimize WebSocket handling
- Cache improvements

---

## 🟢 LOW PRIORITY (P3 - Nice to Have)

### Code Quality
- Remove deprecated functions
- Update dependencies to latest versions
- Refactor large functions (>300 lines)
- Improve error messages

### Features
- Add metrics/monitoring
- Improve logging output
- Add health check endpoints
- Enhance touch UI responsiveness

---

## Session Assignments (Parallel Workers)

### WORKER 1: CRITICAL BUGS
- Focus: Command injection & memory safety fixes
- Time: 25 min
- Deliverable: PR with all unsafe transmutes removed

### WORKER 2: FEATURES
- Focus: New feature development & refactoring
- Time: 25 min
- Deliverable: Feature branch with enhancements

### WORKER 3: TESTING
- Focus: Test coverage & CI improvements
- Time: 25 min
- Deliverable: New tests and CI config

### WORKER 4: DOCUMENTATION
- Focus: API docs & deployment guides
- Time: 25 min
- Deliverable: Updated docs/

### WORKER 5: SECURITY AUDIT
- Focus: SQL injection & credential management
- Time: 25 min
- Deliverable: Security audit report

---

## Current Status (2026-04-02 11:19)
- Repo: feature/error-handling branch
- Staged: 16+ files (tests, security fixes, API docs)
- Build: In progress (release mode)
- Test run: Queued
- 5 parallel workers spawning now

Last Updated: 2026-04-02 11:19 UTC
Session Time Budget: 25 minutes max
Next: Commit & push all worker results
