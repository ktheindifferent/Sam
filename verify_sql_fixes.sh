#!/bin/bash

echo "=== SQL Injection Fix Verification ==="
echo ""

echo "1. Checking for dynamic SQL query building with direct interpolation..."
echo "   Searching for potentially unsafe patterns..."

# Check for format! with direct variable interpolation in SQL contexts
if grep -r "format!.*DELETE.*WHERE.*{}" src/ --include="*.rs" | grep -v "// Safe:" | grep -v "tests"; then
    echo "   ⚠️  Found potential SQL injection patterns"
else
    echo "   ✅ No unsafe DELETE patterns found"
fi

if grep -r "format!.*SELECT.*WHERE.*{}" src/ --include="*.rs" | grep -v "// Safe:" | grep -v "tests" | grep -v "\$1"; then
    echo "   ⚠️  Found potential SQL injection patterns"
else
    echo "   ✅ No unsafe SELECT patterns found"
fi

echo ""
echo "2. Checking for parameterized queries..."
# Check for proper parameter usage
if grep -r "execute_statement.*vec!\[Value::" src/ --include="*.rs" > /dev/null; then
    echo "   ✅ Found parameterized query usage"
else
    echo "   ⚠️  No parameterized queries found"
fi

echo ""
echo "3. Checking for validation functions..."
# Check for validation functions
if grep -r "validate_sql_identifier" src/ --include="*.rs" > /dev/null; then
    echo "   ✅ SQL identifier validation found"
else
    echo "   ⚠️  SQL identifier validation not found"
fi

if grep -r "validate_column_list" src/ --include="*.rs" > /dev/null; then
    echo "   ✅ Column list validation found"
else
    echo "   ⚠️  Column list validation not found"
fi

if grep -r "validate_order_clause" src/ --include="*.rs" > /dev/null; then
    echo "   ✅ ORDER BY validation found"
else
    echo "   ⚠️  ORDER BY validation not found"
fi

echo ""
echo "4. Checking for input range validation..."
# Check for numeric validation
if grep -r "days < 0 || days > 3650" src/ --include="*.rs" > /dev/null; then
    echo "   ✅ Days parameter validation found"
else
    echo "   ⚠️  Days parameter validation not found"
fi

if grep -r "limit_val > 10000" src/ --include="*.rs" > /dev/null; then
    echo "   ✅ Limit validation found"
else
    echo "   ⚠️  Limit validation not found"
fi

echo ""
echo "5. Checking for SQL injection test coverage..."
if [ -f "tests/security/sql_injection_tests.rs" ]; then
    echo "   ✅ SQL injection test file exists"
    test_count=$(grep -c "#\[test\]" tests/security/sql_injection_tests.rs)
    echo "   ✅ Found $test_count test cases"
else
    echo "   ⚠️  SQL injection test file not found"
fi

echo ""
echo "6. Summary of changes:"
echo "   - Fixed cleanup_old_health_records() in database.rs"
echo "   - Added validation to pg_select_async() in config/mod.rs"
echo "   - Added validation to destroy_row functions"
echo "   - Added validation to database creation"
echo "   - Created comprehensive SQL injection test suite"
echo ""
echo "=== Verification Complete ==="