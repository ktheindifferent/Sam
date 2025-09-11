use crate::http::api::io::command_parser::extract_commands;
use crate::http::api::io::action_executor::ActionExecutor;

#[test]
fn test_clear_command_full_integration() {
    // Test the complete pipeline from RiveScript response to action execution
    
    // 1. Start with RiveScript response containing clear command
    let rivescript_response = "Clearing the screen for you. ::::: clear :::::";
    println!("Original RiveScript response: {}", rivescript_response);
    
    // 2. Extract commands using our parser
    let commands = extract_commands(rivescript_response);
    println!("Extracted commands: {:?}", commands);
    
    // Verify we extracted the clear command correctly
    assert_eq!(commands.len(), 1);
    assert_eq!(commands[0].trim(), "clear");
    
    // 3. Create action executor and process the command
    let executor = ActionExecutor::new();
    let result = executor.execute_command("clear");
    
    // 4. Verify the clear command returns our special CLEAR_SCREEN token
    println!("Action executor result: {}", result);
    assert_eq!(result, "CLEAR_SCREEN");
    
    // 5. Simulate IO module processing (what would happen in production)
    let final_response = if result == "CLEAR_SCREEN" {
        "Screen cleared.".to_string()
    } else {
        rivescript_response.to_string()
    };
    
    println!("Final response to user: {}", final_response);
    assert_eq!(final_response, "Screen cleared.");
    
    println!("✅ Complete clear command integration test passed!");
}

#[test] 
fn test_regular_command_integration() {
    // Test that regular commands don't get the CLEAR_SCREEN treatment
    
    let rivescript_response = "Here's your directory listing: ::::: ls -la :::::";
    println!("Original RiveScript response: {}", rivescript_response);
    
    let commands = extract_commands(rivescript_response);
    println!("Extracted commands: {:?}", commands);
    
    assert_eq!(commands.len(), 1);
    assert_eq!(commands[0].trim(), "ls -la");
    
    let executor = ActionExecutor::new();
    let result = executor.execute_command("ls -la");
    
    println!("Action executor result: {}", result);
    // Should NOT be CLEAR_SCREEN for regular commands
    assert_ne!(result, "CLEAR_SCREEN");
    
    println!("✅ Regular command integration test passed!");
}

#[test]
fn test_multiple_commands_with_clear() {
    // Test handling multiple commands where one is clear
    
    let rivescript_response = "I'll list files then clear: ::::: ls ::::: ::::: clear :::::";
    println!("Original RiveScript response: {}", rivescript_response);
    
    let commands = extract_commands(rivescript_response);
    println!("Extracted commands: {:?}", commands);
    
    assert_eq!(commands.len(), 2);
    
    let executor = ActionExecutor::new();
    
    // Process each command
    let mut found_clear = false;
    for command in &commands {
        let result = executor.execute_command(command.trim());
        println!("Command '{}' result: {}", command.trim(), result);
        
        if result == "CLEAR_SCREEN" {
            found_clear = true;
        }
    }
    
    assert!(found_clear, "Should have found CLEAR_SCREEN for clear command");
    
    println!("✅ Multiple commands with clear test passed!");
}
