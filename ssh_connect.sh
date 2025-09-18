#!/bin/bash

# SAM SSH Connection Helper
# This script enables SSH-style access to SAM's remote interface

# Default values
SAM_HOST="${SAM_HOST:-localhost}"
SAM_PORT="${SAM_PORT:-2222}"
SAM_USER="${SAM_USER:-sam}"

echo "SAM SSH Connection Helper"
echo "========================="
echo ""

# Method 1: Direct SSH with command execution
echo "Method 1: Direct SSH connection"
echo "ssh $SAM_USER@$SAM_HOST -p 22 \"nc localhost $SAM_PORT\""
echo ""

# Method 2: SSH port forwarding
echo "Method 2: SSH port forwarding (recommended)"
echo "ssh -L $SAM_PORT:localhost:$SAM_PORT $SAM_USER@$SAM_HOST"
echo "Then in another terminal: telnet localhost $SAM_PORT"
echo ""

# Method 3: Direct telnet (if on same network)
echo "Method 3: Direct connection (same network)"
echo "telnet $SAM_HOST $SAM_PORT"
echo "# or: nc $SAM_HOST $SAM_PORT"
echo ""

echo "Default credentials: username=sam, password=sam"
echo "Configure with SSH_USERNAME and SSH_PASSWORD environment variables"
echo ""

# Interactive connection
read -p "Connect now using method 3 (telnet)? [y/N]: " -n 1 -r
echo
if [[ $REPLY =~ ^[Yy]$ ]]; then
    echo "Connecting to SAM at $SAM_HOST:$SAM_PORT..."
    exec telnet $SAM_HOST $SAM_PORT
fi