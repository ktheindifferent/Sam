#[cfg(test)]
mod integration_tests {
    use super::command_parser::extract_commands;
    use super::action_executor::ActionExecutor;

    #[test]
    fn test_clear_command_integration() {
        // Simulate RiveScript response with clear command
        let rivescript_response = "Clearing the screen for you. ::::: clear :::::";
        
        // Extract commands
        let commands = extract_commands(rivescript_response);
        assert_eq!(commands, vec!["clear"]);
        
        // Create action executor
        let executor = ActionExecutor::new();
        
        // Execute the clear command
        let result = executor.execute_command("clear");
        
        // Verify it returns our special CLEAR_SCREEN response
        assert_eq!(result, "CLEAR_SCREEN");
        
        println!("Integration test passed! Clear command pipeline working correctly.");
    }

    #[test]
    fn test_regular_command_integration() {
        // Test a regular command that should go through CLI router
        let rivescript_response = "Here's your directory listing: ::::: ls -la :::::";
        
        // Extract commands
        let commands = extract_commands(rivescript_response);
        assert_eq!(commands, vec!["ls -la"]);
        
        // Create action executor
        let executor = ActionExecutor::new();
        
        // Execute the ls command (this will fail since we don't have a real CLI router in tests)
        // But we can verify it doesn't return CLEAR_SCREEN
        let result = executor.execute_command("ls -la");
        
        // Should not be CLEAR_SCREEN (will be error message from CLI router)
        assert_ne!(result, "CLEAR_SCREEN");
        
        println!("Integration test passed! Regular command pipeline working correctly.");
    }

    #[test]
    fn test_response_processing() {
        // Test the full response processing logic
        let mut response = "Clearing the screen for you. ::::: clear :::::".to_string();
        
        // Extract commands
        let commands = extract_commands(&response);
        
        if !commands.is_empty() {
            let executor = ActionExecutor::new();
            
            for command in commands {
                let result = executor.execute_command(&command);
                
                // If it's a clear command, replace the response
                if result == "CLEAR_SCREEN" {
                    response = "Screen cleared.".to_string();
                    break;
                }
            }
        }
        
        assert_eq!(response, "Screen cleared.");
        println!("Integration test passed! Full response processing working correctly.");
    }
}
