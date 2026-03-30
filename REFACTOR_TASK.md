Refactoring Task for SAM-C

Objective:
Refactor installer.rs to improve maintainability by:
1. Breaking down large functions
2. Simplifying complex conditionals
3. Eliminating duplicate code patterns

Areas to Address:
- Large functions like pre_install() and ensure_chocolatey_installed()
- Complex conditionals with deep nesting
- Duplicate code patterns that can be extracted

Implementation Plan:
1. Analyze current structure
2. Break down large functions into smaller helpers
3. Simplify conditional logic
4. Extract common patterns
5. Test with cargo build
6. Commit with message: refactor: improve installer code structure
