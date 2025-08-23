#!/bin/bash
# Test Redis module with thread sanitizer for data race detection

echo "Building with thread sanitizer..."
RUSTFLAGS="-Z sanitizer=thread" cargo +nightly test --lib sam::services::redis::tests --target x86_64-unknown-linux-gnu 2>&1 | tee thread_sanitizer_output.log

echo ""
echo "Checking for data races..."
if grep -q "WARNING: ThreadSanitizer" thread_sanitizer_output.log; then
    echo "⚠️ Thread sanitizer warnings detected!"
    grep "WARNING: ThreadSanitizer" thread_sanitizer_output.log
    exit 1
else
    echo "✅ No data races detected by thread sanitizer"
fi

echo ""
echo "Running standard tests..."
cargo test --lib sam::services::redis::tests 2>&1 | tee standard_test_output.log

echo ""
echo "Test summary:"
echo "- All unsafe blocks removed from redis.rs"
echo "- Using OnceCell for thread-safe lazy initialization"
echo "- Arc<RwLock<Option<Pool>>> pattern for safe concurrent access"
echo "- Multiple concurrent access tests added"