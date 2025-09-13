use rouille::{Request, Response, input::json::JsonError};
use serde::{Deserialize, Serialize};
use log::{info, error};
use std::collections::HashMap;
use crate::services::ollama::OllamaService;

#[derive(Debug, Serialize, Deserialize)]
pub struct OllamaStatus {
    pub installed: bool,
    pub running: bool,
    pub version: Option<String>,
    pub models: Vec<String>,
    pub status_text: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GenerateRequest {
    pub model: String,
    pub prompt: String,
    pub options: Option<HashMap<String, serde_json::Value>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GenerateStreamRequest {
    pub model: String,
    pub prompt: String,
    pub options: Option<HashMap<String, serde_json::Value>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ModelAction {
    pub model: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct InstallResponse {
    pub success: bool,
    pub message: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ApiResponse {
    pub success: bool,
    pub message: String,
    pub data: Option<serde_json::Value>,
}

/// Handle Ollama service API endpoints
pub fn handle(request: &Request) -> Result<Response, crate::http::Error> {
    let url = request.url();
    
    match request.method() {
        "GET" => handle_get_request(&url),
        "POST" => handle_post_request(request, &url),
        "DELETE" => handle_delete_request(request, &url),
        _ => Ok(Response::empty_404().with_status_code(405)),
    }
}

fn handle_get_request(url: &str) -> Result<Response, crate::http::Error> {
    let rt = tokio::runtime::Runtime::new()?;
    
    match url {
        "/api/ollama/status" => rt.block_on(get_ollama_status()),
        "/api/ollama/models" => rt.block_on(list_models()),
        "/api/ollama/models/available" => rt.block_on(search_available_models("")),
        _ if url.starts_with("/api/ollama/models/available/") => {
            let query = url.trim_start_matches("/api/ollama/models/available/");
            rt.block_on(search_available_models(query))
        },
        _ if url.starts_with("/api/ollama/models/") && url.ends_with("/info") => {
            let model_name = url.trim_start_matches("/api/ollama/models/")
                               .trim_end_matches("/info");
            rt.block_on(get_model_info(model_name))
        },
        _ => Ok(Response::empty_404()),
    }
}

fn handle_post_request(request: &Request, url: &str) -> Result<Response, crate::http::Error> {
    let rt = tokio::runtime::Runtime::new()?;
    
    match url {
        "/api/ollama/install" => rt.block_on(install_ollama()),
        "/api/ollama/start" => rt.block_on(start_service()),
        "/api/ollama/stop" => rt.block_on(stop_service()),
        "/api/ollama/generate" => rt.block_on(generate_text(request)),
        "/api/ollama/generate/stream" => rt.block_on(generate_text_stream(request)),
        "/api/ollama/models/pull" => rt.block_on(pull_model(request)),
        "/api/ollama/models/install-recommended" => rt.block_on(install_recommended_models()),
        _ => Ok(Response::empty_404()),
    }
}

fn handle_delete_request(_request: &Request, url: &str) -> Result<Response, crate::http::Error> {
    let rt = tokio::runtime::Runtime::new()?;
    
    if url.starts_with("/api/ollama/models/") {
        let model_name = url.trim_start_matches("/api/ollama/models/");
        return rt.block_on(remove_model(model_name));
    }
    
    Ok(Response::empty_404())
}

async fn get_ollama_status() -> Result<Response, crate::http::Error> {
    info!("Getting Ollama status");
    
    let service = OllamaService::new_with_defaults();
    let installed = service.is_installed().await;
    let running = if installed { service.is_running().await } else { false };
    
    let version = if running {
        service.get_version().await.ok()
    } else {
        None
    };
    
    let models = if running {
        service.get_installed_model_names().await.unwrap_or_default()
    } else {
        Vec::new()
    };
    
    let status_text = if !installed {
        "Ollama not installed".to_string()
    } else if !running {
        "Ollama installed but not running".to_string()
    } else {
        format!("Ollama running with {} models", models.len())
    };
    
    let status = OllamaStatus {
        installed,
        running,
        version,
        models,
        status_text,
    };
    
    Ok(Response::json(&status))
}

async fn install_ollama() -> Result<Response, crate::http::Error> {
    info!("Installing Ollama");
    
    let service = OllamaService::new_with_defaults();
    
    if service.is_installed().await {
        let response = InstallResponse {
            success: true,
            message: "Ollama is already installed".to_string(),
        };
        return Ok(Response::json(&response));
    }
    
    match service.install().await {
        Ok(message) => {
            info!("Ollama installation successful: {}", message);
            let response = InstallResponse {
                success: true,
                message,
            };
            Ok(Response::json(&response))
        },
        Err(e) => {
            error!("Ollama installation failed: {}", e);
            let response = InstallResponse {
                success: false,
                message: format!("Installation failed: {}", e),
            };
            Ok(Response::json(&response).with_status_code(500))
        }
    }
}

async fn start_service() -> Result<Response, crate::http::Error> {
    info!("Starting Ollama service");
    
    let service = OllamaService::new_with_defaults();
    
    match service.start_service().await {
        Ok(message) => {
            info!("Ollama service start: {}", message);
            let response = ApiResponse {
                success: true,
                message,
                data: None,
            };
            Ok(Response::json(&response))
        },
        Err(e) => {
            error!("Failed to start Ollama service: {}", e);
            let response = ApiResponse {
                success: false,
                message: format!("Failed to start service: {}", e),
                data: None,
            };
            Ok(Response::json(&response).with_status_code(500))
        }
    }
}

async fn stop_service() -> Result<Response, crate::http::Error> {
    info!("Stopping Ollama service");
    
    let service = OllamaService::new_with_defaults();
    
    match service.stop_service().await {
        Ok(message) => {
            info!("Ollama service stop: {}", message);
            let response = ApiResponse {
                success: true,
                message,
                data: None,
            };
            Ok(Response::json(&response))
        },
        Err(e) => {
            error!("Failed to stop Ollama service: {}", e);
            let response = ApiResponse {
                success: false,
                message: format!("Failed to stop service: {}", e),
                data: None,
            };
            Ok(Response::json(&response).with_status_code(500))
        }
    }
}

async fn list_models() -> Result<Response, crate::http::Error> {
    info!("Listing Ollama models");
    
    let service = OllamaService::new_with_defaults();
    
    if !service.is_running().await {
        let response = ApiResponse {
            success: false,
            message: "Ollama service is not running".to_string(),
            data: None,
        };
        return Ok(Response::json(&response).with_status_code(503));
    }
    
    match service.list_models().await {
        Ok(models) => {
            let response = ApiResponse {
                success: true,
                message: format!("Found {} models", models.models.len()),
                data: Some(serde_json::to_value(models).map_err(|e| crate::http::Error::InternalServerError(e.to_string()))?),
            };
            Ok(Response::json(&response))
        },
        Err(e) => {
            error!("Failed to list models: {}", e);
            let response = ApiResponse {
                success: false,
                message: format!("Failed to list models: {}", e),
                data: None,
            };
            Ok(Response::json(&response).with_status_code(500))
        }
    }
}

async fn search_available_models(query: &str) -> Result<Response, crate::http::Error> {
    info!("Searching available models with query: '{}'", query);
    
    let service = OllamaService::new_with_defaults();
    
    match service.search_models(query).await {
        Ok(models) => {
            let response = ApiResponse {
                success: true,
                message: format!("Found {} models matching '{}'", models.len(), query),
                data: Some(serde_json::to_value(models).map_err(|e| crate::http::Error::InternalServerError(e.to_string()))?),
            };
            Ok(Response::json(&response))
        },
        Err(e) => {
            error!("Failed to search models: {}", e);
            let response = ApiResponse {
                success: false,
                message: format!("Failed to search models: {}", e),
                data: None,
            };
            Ok(Response::json(&response).with_status_code(500))
        }
    }
}

async fn get_model_info(model_name: &str) -> Result<Response, crate::http::Error> {
    info!("Getting model info for: {}", model_name);
    
    let service = OllamaService::new_with_defaults();
    
    if !service.is_running().await {
        let response = ApiResponse {
            success: false,
            message: "Ollama service is not running".to_string(),
            data: None,
        };
        return Ok(Response::json(&response).with_status_code(503));
    }
    
    match service.show_model(model_name).await {
        Ok(model_info) => {
            let response = ApiResponse {
                success: true,
                message: format!("Model info for {}", model_name),
                data: Some(model_info),
            };
            Ok(Response::json(&response))
        },
        Err(e) => {
            error!("Failed to get model info: {}", e);
            let response = ApiResponse {
                success: false,
                message: format!("Failed to get model info: {}", e),
                data: None,
            };
            Ok(Response::json(&response).with_status_code(500))
        }
    }
}

async fn generate_text(request: &Request) -> Result<Response, crate::http::Error> {
    let generate_req: GenerateRequest = match rouille::input::json_input(request) {
        Ok(req) => req,
        Err(JsonError::WrongContentType) => {
            let response = ApiResponse {
                success: false,
                message: "Content-Type must be application/json".to_string(),
                data: None,
            };
            return Ok(Response::json(&response).with_status_code(400));
        },
        Err(e) => {
            error!("Invalid JSON in generate request: {}", e);
            let response = ApiResponse {
                success: false,
                message: format!("Invalid JSON: {}", e),
                data: None,
            };
            return Ok(Response::json(&response).with_status_code(400));
        }
    };
    
    info!("Generating text with model: {}", generate_req.model);
    
    let service = OllamaService::new_with_defaults();
    
    if !service.is_running().await {
        let response = ApiResponse {
            success: false,
            message: "Ollama service is not running".to_string(),
            data: None,
        };
        return Ok(Response::json(&response).with_status_code(503));
    }
    
    match service.generate(&generate_req.model, &generate_req.prompt, generate_req.options).await {
        Ok(generation) => {
            let response = ApiResponse {
                success: true,
                message: "Text generated successfully".to_string(),
                data: Some(serde_json::to_value(generation).map_err(|e| crate::http::Error::InternalServerError(e.to_string()))?),
            };
            Ok(Response::json(&response))
        },
        Err(e) => {
            error!("Failed to generate text: {}", e);
            let response = ApiResponse {
                success: false,
                message: format!("Failed to generate text: {}", e),
                data: None,
            };
            Ok(Response::json(&response).with_status_code(500))
        }
    }
}

async fn generate_text_stream(request: &Request) -> Result<Response, crate::http::Error> {
    let generate_req: GenerateStreamRequest = match rouille::input::json_input(request) {
        Ok(req) => req,
        Err(JsonError::WrongContentType) => {
            let response = ApiResponse {
                success: false,
                message: "Content-Type must be application/json".to_string(),
                data: None,
            };
            return Ok(Response::json(&response).with_status_code(400));
        },
        Err(e) => {
            error!("Invalid JSON in stream generate request: {}", e);
            let response = ApiResponse {
                success: false,
                message: format!("Invalid JSON: {}", e),
                data: None,
            };
            return Ok(Response::json(&response).with_status_code(400));
        }
    };
    
    info!("Starting streaming generation with model: {}", generate_req.model);
    
    let service = OllamaService::new_with_defaults();
    
    if !service.is_running().await {
        let response = ApiResponse {
            success: false,
            message: "Ollama service is not running".to_string(),
            data: None,
        };
        return Ok(Response::json(&response).with_status_code(503));
    }
    
    // For now, we'll return a message indicating streaming is not fully implemented
    // In a production system, you'd want to implement Server-Sent Events or WebSocket streaming
    let response = ApiResponse {
        success: false,
        message: "Streaming generation not yet implemented in HTTP API. Use WebSocket for real-time streaming.".to_string(),
        data: None,
    };
    Ok(Response::json(&response).with_status_code(501))
}

async fn pull_model(request: &Request) -> Result<Response, crate::http::Error> {
    let model_req: ModelAction = match rouille::input::json_input(request) {
        Ok(req) => req,
        Err(JsonError::WrongContentType) => {
            let response = ApiResponse {
                success: false,
                message: "Content-Type must be application/json".to_string(),
                data: None,
            };
            return Ok(Response::json(&response).with_status_code(400));
        },
        Err(e) => {
            error!("Invalid JSON in pull model request: {}", e);
            let response = ApiResponse {
                success: false,
                message: format!("Invalid JSON: {}", e),
                data: None,
            };
            return Ok(Response::json(&response).with_status_code(400));
        }
    };
    
    info!("Pulling model: {}", model_req.model);
    
    let service = OllamaService::new_with_defaults();
    
    if !service.is_running().await {
        let response = ApiResponse {
            success: false,
            message: "Ollama service is not running".to_string(),
            data: None,
        };
        return Ok(Response::json(&response).with_status_code(503));
    }
    
    match service.pull_model(&model_req.model).await {
        Ok(message) => {
            info!("Model pull successful: {}", message);
            let response = ApiResponse {
                success: true,
                message,
                data: None,
            };
            Ok(Response::json(&response))
        },
        Err(e) => {
            error!("Failed to pull model: {}", e);
            let response = ApiResponse {
                success: false,
                message: format!("Failed to pull model: {}", e),
                data: None,
            };
            Ok(Response::json(&response).with_status_code(500))
        }
    }
}

async fn remove_model(model_name: &str) -> Result<Response, crate::http::Error> {
    info!("Removing model: {}", model_name);
    
    let service = OllamaService::new_with_defaults();
    
    if !service.is_running().await {
        let response = ApiResponse {
            success: false,
            message: "Ollama service is not running".to_string(),
            data: None,
        };
        return Ok(Response::json(&response).with_status_code(503));
    }
    
    match service.remove_model(model_name).await {
        Ok(message) => {
            info!("Model removal successful: {}", message);
            let response = ApiResponse {
                success: true,
                message,
                data: None,
            };
            Ok(Response::json(&response))
        },
        Err(e) => {
            error!("Failed to remove model: {}", e);
            let response = ApiResponse {
                success: false,
                message: format!("Failed to remove model: {}", e),
                data: None,
            };
            Ok(Response::json(&response).with_status_code(500))
        }
    }
}

async fn install_recommended_models() -> Result<Response, crate::http::Error> {
    info!("Installing recommended models");
    
    let service = OllamaService::new_with_defaults();
    
    if !service.is_running().await {
        let response = ApiResponse {
            success: false,
            message: "Ollama service is not running".to_string(),
            data: None,
        };
        return Ok(Response::json(&response).with_status_code(503));
    }
    
    match service.install_recommended_models().await {
        Ok(message) => {
            info!("Recommended models installation: {}", message);
            let response = ApiResponse {
                success: true,
                message,
                data: None,
            };
            Ok(Response::json(&response))
        },
        Err(e) => {
            error!("Failed to install recommended models: {}", e);
            let response = ApiResponse {
                success: false,
                message: format!("Failed to install recommended models: {}", e),
                data: None,
            };
            Ok(Response::json(&response).with_status_code(500))
        }
    }
}