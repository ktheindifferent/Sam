#!/bin/bash

# Setup Ollama Server Script
# This script helps configure a remote Ollama server for the SAM Coding Agent

set -e

echo "==================================="
echo "SAM Coding Agent - Ollama Setup"
echo "==================================="
echo

# Function to check if server is accessible
check_server() {
    local endpoint=$1
    echo "Testing connection to $endpoint..."
    if curl -s -f -m 5 "$endpoint/api/tags" > /dev/null 2>&1; then
        echo "✓ Server is accessible!"
        return 0
    else
        echo "✗ Could not connect to server"
        return 1
    fi
}

# Function to list models on server
list_models() {
    local endpoint=$1
    echo "Available models on server:"
    curl -s "$endpoint/api/tags" | jq -r '.models[].name' 2>/dev/null || echo "Could not fetch models"
}

# Check for required tools
if ! command -v curl &> /dev/null; then
    echo "Error: curl is required but not installed"
    exit 1
fi

if ! command -v jq &> /dev/null; then
    echo "Warning: jq is not installed. Model listing may not work properly."
    echo "Install with: brew install jq (macOS) or apt-get install jq (Linux)"
fi

# Get server details
echo "Enter Ollama server details:"
echo
read -p "Server IP address [172.16.0.125]: " SERVER_IP
SERVER_IP=${SERVER_IP:-172.16.0.125}

read -p "Server port [11434]: " SERVER_PORT
SERVER_PORT=${SERVER_PORT:-11434}

read -p "Server name [Remote Ollama]: " SERVER_NAME
SERVER_NAME=${SERVER_NAME:-"Remote Ollama"}

ENDPOINT="http://$SERVER_IP:$SERVER_PORT"

# Test connection
if check_server "$ENDPOINT"; then
    echo
    list_models "$ENDPOINT"
    echo
else
    echo
    echo "Warning: Could not connect to server at $ENDPOINT"
    read -p "Continue anyway? (y/n): " CONTINUE
    if [[ $CONTINUE != "y" ]]; then
        echo "Setup cancelled."
        exit 1
    fi
fi

# Create configuration directory
CONFIG_DIR="$HOME/.sam/coding_agent"
mkdir -p "$CONFIG_DIR"

CONFIG_FILE="$CONFIG_DIR/ollama_config.json"

# Check if config file exists
if [ -f "$CONFIG_FILE" ]; then
    echo "Existing configuration found at $CONFIG_FILE"
    echo "Adding new server to existing configuration..."

    # Backup existing config
    cp "$CONFIG_FILE" "$CONFIG_FILE.backup"
    echo "Backup saved to $CONFIG_FILE.backup"
else
    echo "Creating new configuration file..."
    # Create base configuration
    cat > "$CONFIG_FILE" << EOF
{
  "servers": [],
  "selected_server": null,
  "selected_model": null,
  "auto_discover_local": true,
  "fallback_enabled": true,
  "model_preferences": {}
}
EOF
fi

# Add the new server using a temporary Python script
python3 << EOF
import json
import sys

config_file = "$CONFIG_FILE"
with open(config_file, 'r') as f:
    config = json.load(f)

# Check if server already exists
server_exists = any(s['name'] == "$SERVER_NAME" for s in config['servers'])
if server_exists:
    print(f"Server '{SERVER_NAME}' already exists in configuration")
    sys.exit(1)

# Add new server
new_server = {
    "name": "$SERVER_NAME",
    "endpoint": "$ENDPOINT",
    "models": [],
    "is_default": len(config['servers']) == 0,
    "is_local": "$SERVER_IP" in ["localhost", "127.0.0.1", "0.0.0.0"],
    "gpu_provider": None,
    "tags": ["remote", "custom"],
    "max_concurrent_requests": 4,
    "timeout_seconds": 600
}

config['servers'].append(new_server)

# Set as selected if first server
if len(config['servers']) == 1:
    config['selected_server'] = "$SERVER_NAME"

with open(config_file, 'w') as f:
    json.dump(config, f, indent=2)

print(f"✓ Server '{SERVER_NAME}' added successfully!")
EOF

if [ $? -eq 0 ]; then
    echo
    echo "==================================="
    echo "Setup Complete!"
    echo "==================================="
    echo
    echo "Server configured:"
    echo "  Name: $SERVER_NAME"
    echo "  Endpoint: $ENDPOINT"
    echo "  Config file: $CONFIG_FILE"
    echo
    echo "To use this server in SAM:"
    echo "1. Start SAM: cargo run"
    echo "2. The coding agent will automatically use the configured server"
    echo
    echo "To test with a specific model:"
    echo "  Example: 'create a rust project using gpt-oss:20b'"
    echo

    # Quick test option
    read -p "Would you like to test the connection now? (y/n): " TEST_NOW
    if [[ $TEST_NOW == "y" ]]; then
        echo
        echo "Testing generation with a simple prompt..."
        curl -s -X POST "$ENDPOINT/api/generate" \
            -H "Content-Type: application/json" \
            -d '{
                "model": "gpt-oss:20b",
                "prompt": "Hello, respond with OK if you are working",
                "stream": false
            }' | jq -r '.response' 2>/dev/null || echo "Test failed. Please check the server and model availability."
    fi
else
    echo "Error: Failed to update configuration"
    exit 1
fi