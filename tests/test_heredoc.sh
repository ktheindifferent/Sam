#!/bin/bash

# Test heredoc command directly
cd /tmp
rm -rf test_heredoc_project 2>/dev/null

echo "Creating test project..."
mkdir test_heredoc_project
cd test_heredoc_project

echo "Testing heredoc..."
cat > test.txt << 'EOF'
Line 1
Line 2
Line 3
EOF

echo "Contents of test.txt:"
cat test.txt

echo ""
echo "Now testing with coding agent..."
cd /Users/calebsmith/Documents/ktheindifferent/Sam
rm -rf test_heredoc_rust 2>/dev/null

cargo run --bin coding_agent -- "create a new rust project called test_heredoc_rust that prints hello world"

if [ -f "test_heredoc_rust/src/main.rs" ]; then
    echo ""
    echo "test_heredoc_rust/src/main.rs content:"
    cat test_heredoc_rust/src/main.rs
else
    echo "File not found: test_heredoc_rust/src/main.rs"
fi