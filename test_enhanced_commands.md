# Enhanced Commands Test Guide

## Overview
The SAM command system now supports comprehensive natural language processing for all system commands. Both TUI and web interfaces should recognize these patterns.

## Clear Screen (Working ✅)
- "clear screen" / "cls" / "clear my screen" / "clean screen"
- Should clear the output buffer in TUI and display "Screen cleared." message

## Basic File Operations
Try these natural language commands:

### Directory Navigation
- "show me files" / "list files" / "what files are here" / "directory contents"
- "where am i" / "current location" / "working directory"
- "go home" / "home directory" / "navigate home"
- "go to downloads" / "downloads folder" / "switch to downloads"
- "go back" / "up one level" / "parent directory"

### File Management
- "create file test.txt" / "new file test.txt" / "make file test.txt"
- "show file README.md" / "display file README.md" / "read file README.md"
- "edit test.txt" / "modify test.txt" / "open editor test.txt"
- "copy test.txt to backup.txt" / "duplicate test.txt to backup.txt"
- "move test.txt to temp.txt" / "rename test.txt to temp.txt"
- "delete test.txt" / "remove test.txt" / "remove file test.txt"

### System Information
- "system info" / "operating system" / "what system" / "computer info"
- "current time" / "what time is it" / "time now"
- "who am i" / "current user" / "my username"
- "disk space" / "storage space" / "available space" / "free space"
- "system performance" / "cpu usage" / "resource usage"
- "running processes" / "what's running" / "active applications"

## Service Management
All services support natural language:

### Web Crawler
- "start crawler" / "begin crawling" / "enable crawler"
- "stop crawler" / "halt crawling" / "disable crawler"
- "crawler status" / "check crawler" / "crawling status"
- "search crawled pages python" / "find in crawled rust"

### LIFX Lights
- "turn on lights" / "lights on" / "enable lights" / "start lights"
- "turn off lights" / "lights off" / "disable lights" / "stop lights"
- "light status" / "check lights" / "lights status"

### Redis Database
- "start redis" / "redis on" / "enable redis" / "start redis server"
- "stop redis" / "redis off" / "disable redis" / "stop redis server"
- "redis status" / "check redis" / "redis server status"

### PostgreSQL Database
- "start postgres" / "postgres on" / "postgresql start"
- "stop postgres" / "postgres off" / "postgresql stop"  
- "postgres status" / "check postgres" / "postgresql status"

### Spotify Music
- "start spotify" / "music start" / "enable spotify"
- "play music" / "start playing" / "music play"
- "pause music" / "stop playing" / "music pause"
- "shuffle music" / "random music" / "music shuffle"

### Docker
- "start docker" / "docker on" / "enable docker"
- "stop docker" / "docker off" / "disable docker"
- "docker status" / "check docker" / "docker daemon status"

## AI & Advanced Features

### Text-to-Speech
- "say hello world" / "speak hello world" / "voice hello world"
- "text to speech test message" / "talk test message"

### LLama AI
- "ask llama what is rust" / "llama query explain python"
- "llama what is the meaning of life"

### Networking
- "ssh user@server.com" / "connect to server.com" / "remote login server"
- "scan network" / "find devices" / "network discovery"

## Shortcuts & Variations
The system supports common shortcuts:
- "l" = "ls" (list files)
- "la" = "ls -la" (detailed list)
- "h" = "cd ~" (home)
- "b" = "cd .." (back)
- "c" = "clear" (clear screen)
- "q" = "exit" (quit)

## Error Handling & Help
- "i need help with files" - provides contextual help
- "how do i list files" - guides through operations
- "something not working" - triggers troubleshooting
- "redis error" - offers specific help

## Conversational Patterns
- "thanks" / "awesome" / "perfect" - acknowledges success
- "hi" / "hello" / "good morning" - friendly greetings
- "what can you do" - explains capabilities
- "nice work" / "cool" - positive feedback

## Testing Instructions

1. **TUI Testing**: Run SAM in TUI mode and try these commands
2. **Web Testing**: Use the web console interface
3. **Mixed Testing**: Try combinations like "clear screen then list files"
4. **Natural Variations**: Use your own natural language variations

## Expected Behavior

- All commands should execute and show results
- Clear screen should actually clear the display
- Service commands should start/stop/check services properly  
- File operations should work with actual files
- Natural language should be recognized and converted to proper commands
- Error messages should be helpful and contextual

## Notes

- Commands are processed through RiveScript with ::::: command ::::: embedding
- TUI and web interfaces both support the enhanced command processing
- All commands go through safety validation to prevent dangerous operations
- The system learns and adapts to natural language variations
