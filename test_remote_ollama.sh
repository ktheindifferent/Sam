#!/bin/bash

# Test Remote Ollama Connection

echo "Testing connection to Ollama server at 172.16.0.125:11434..."
echo

# Test connection
if curl -s -f -m 5 "http://172.16.0.125:11434/api/tags" > /dev/null 2>&1; then
    echo "✓ Server is accessible!"
    echo

    # List models
    echo "Available models:"
    curl -s "http://172.16.0.125:11434/api/tags" | jq -r '.models[].name' 2>/dev/null || echo "Could not parse models"
    echo

    # Test generation with gpt-oss:20b
    echo "Testing generation with gpt-oss:20b..."
    response=$(curl -s -X POST "http://172.16.0.125:11434/api/generate" \
        -H "Content-Type: application/json" \
        -d '{
            "model": "gpt-oss:20b",
            "prompt": "Write a single line of Python code that prints Hello World",
            "stream": false,
            "options": {
                "temperature": 0.7,
                "max_tokens": 50
            }
        }' 2>/dev/null)

    if [ $? -eq 0 ]; then
        echo "Response from model:"
        echo "$response" | jq -r '.response' 2>/dev/null || echo "Could not parse response"
        echo
        echo "✓ Model is working!"
    else
        echo "✗ Failed to generate response"
    fi
else
    echo "✗ Could not connect to server at http://172.16.0.125:11434"
    echo
    echo "Please ensure:"
    echo "1. The Ollama server is running on 172.16.0.125"
    echo "2. Port 11434 is accessible"
    echo "3. No firewall is blocking the connection"
fi