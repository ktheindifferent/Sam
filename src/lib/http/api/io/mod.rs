// SAM IO Module
// Copyright 2021-2026 The Open Sam Foundation (OSF)
// Developed by Caleb Mitchell Smith (ktheindifferent, PixelCoda, p0indexter)
// Licensed under GPLv3....see LICENSE file.

pub mod command_parser;
pub mod action_executor;
pub mod context_manager;
pub mod responses;

use rouille::Request;
use rouille::Response;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct IOReply {
    pub text: String,
    pub timestamp: i64,
    pub response_type: String,
    pub executed_actions: Vec<ExecutedAction>,
    pub context_updates: Option<serde_json::Value>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ExecutedAction {
    pub command: String,
    pub result: String,
    pub success: bool,
}

pub fn handle(
    _current_session: crate::memory::cache::WebSessions,
    request: &Request,
) -> Result<Response, crate::http::Error> {
    let input = request.get_param("input");
    let user_id = request.get_param("user_id").unwrap_or_else(|| "localuser".to_string());

    match input {
        Some(iput) => {
            // Use blocking runtime for async operations
            let rt = tokio::runtime::Runtime::new().unwrap();
            
            rt.block_on(async {
                // Load user context
                let mut context = context_manager::load_user_context(&user_id).await;
                
                // Update brain.py to include context
                let context_str = context_manager::serialize_context(&context);

                // Use the centralized RiveScript service (same as TUI)
                match crate::services::rivescript::query(&iput) {
                    Ok(rivescript_response) => {
                        let rs = rivescript_response.text;
                        let mut executed_actions = Vec::new();
                        let mut modified_response = rs.clone();

                        // Check for embedded commands
                        if rs.contains(":::::") {
                            let commands = command_parser::extract_commands(&rs);
                            
                            for command in commands {
                                let result = action_executor::execute_action(&command).await;
                                
                                // Handle web-specific actions
                                if result.output.starts_with("WEB_ACTION:") {
                                    let action_type = &result.output[11..]; // Remove "WEB_ACTION:" prefix
                                    
                                    match action_type {
                                        "CLEAR_SCREEN" => {
                                            // Replace the response with a clear screen instruction
                                            modified_response = "Screen cleared.".to_string();
                                            executed_actions.push(ExecutedAction {
                                                command: command.clone(),
                                                result: "CLEAR_SCREEN".to_string(),
                                                success: true,
                                            });
                                        }
                                        // Future web actions can be handled here:
                                        // "SCROLL_TO_TOP" => { ... }
                                        // "SCROLL_TO_BOTTOM" => { ... }
                                        // "REFRESH_PAGE" => { ... }
                                        _ => {
                                            executed_actions.push(ExecutedAction {
                                                command: command.clone(),
                                                result: result.output.clone(),
                                                success: result.success,
                                            });
                                        }
                                    }
                                } else {
                                    executed_actions.push(ExecutedAction {
                                        command: command.clone(),
                                        result: result.output.clone(),
                                        success: result.success,
                                    });
                                }
                                
                                // Remove the command marker from response
                                modified_response = command_parser::remove_command_markers(&modified_response, &command);
                            }
                            
                            // Update user context based on conversation
                            context_manager::update_context(&mut context, &iput, &modified_response, &executed_actions);
                        }

                        // Save updated context
                        context_manager::save_user_context(&user_id, &context).await;

                        let io = IOReply {
                            text: modified_response,
                            timestamp: chrono::Utc::now().timestamp(),
                            response_type: "io".to_string(),
                            executed_actions,
                            context_updates: Some(serde_json::to_value(&context).unwrap_or_default()),
                        };

                        Ok(Response::json(&io))
                    }
                    Err(_e) => {
                        let response = Response::text("RiveScript error").with_status_code(500);
                        Ok(response)
                    }
                }
            })
        }
        None => {
            let response = Response::text("IO input malformed").with_status_code(500);
            Ok(response)
        }
    }
}