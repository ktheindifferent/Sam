#!/usr/bin/env python3

import re

def extract_commands(response):
    """Extract commands from RiveScript response"""
    pattern = r":::::(.+?):::::"
    matches = re.findall(pattern, response)
    return [match.strip() for match in matches if match.strip()]

def remove_command_markers(response, command):
    """Remove command markers from response"""
    marker_pattern = f":::::{re.escape(command)}:::::"
    return re.sub(marker_pattern, "", response)

# Test the pipeline
test_response = "Clearing the screen for you. ::::: clear :::::"

print("Original response:", test_response)
print()

# Extract commands
commands = extract_commands(test_response)
print("Extracted commands:", commands)
print()

# Process each command
for command in commands:
    print(f"Processing command: '{command}'")
    
    # Remove the command marker
    cleaned_response = remove_command_markers(test_response, command)
    print(f"Cleaned response: '{cleaned_response}'")
    
    # Simulate the action executor result
    if command == "clear":
        print("Action executor would return: CLEAR_SCREEN")
        final_response = "Screen cleared."
        print(f"Final response: '{final_response}'")
    else:
        print(f"Regular command execution for: {command}")

print()
print("Pipeline test completed!")
