// ███████     █████     ███    ███    
// ██         ██   ██    ████  ████    
// ███████    ███████    ██ ████ ██    
//      ██    ██   ██    ██  ██  ██    
// ███████ ██ ██   ██ ██ ██      ██ ██ 
// Copyright 2021-2026 The Open Sam Foundation (OSF)
// Developed by Caleb Mitchell Smith (ktheindifferent, PixelCoda, p0indexter)
// Licensed under GPLv3....see LICENSE file.

var term_directory = "~";
var term_buffer = "";
var term_buffer_2 = "";
var term_cursor_location = 1;
var term_history = [];
var term_history_selected_line;
var term_count_spaceholders = 0;
var cursor_margin = 0;
var processing_command = false;
var computer_ip_addr = "";

$( document ).ready(function() {

	window.onkeydown = function(k){
		console.log(k)

		k.preventDefault();

		var banned_keys = [16, 17, 18, 19, 20, 33, 34, 35, 36, 45, 46, 93, 144, 145, 112, 113, 114, 115, 116, 117, 118, 119, 120, 121, 122, 123];


		if(!processing_command){
			if(k.keyCode == 8){

				if(term_cursor_location == 1){
					term_buffer = term_buffer.slice(0, -1);
				} else {
					term_buffer = term_buffer.substring(0, (term_buffer.length) - term_cursor_location) + term_buffer.substring((term_buffer.length) - term_cursor_location + 1, (term_buffer.length));
				}

				// if(term_buffer.slice(-1) == " "){
				// 	term_cursor_location = term_cursor_location - 1;
				// 	cursor_margin = cursor_margin + 9;
				// 	$("#terminal__prompt--cursor").css("margin-left", cursor_margin+"px");
				// }
			}
			else if(k.keyCode == 32){
				//space
				term_buffer += " ";
				term_cursor_location = term_cursor_location - 1;
				cursor_margin = cursor_margin + 9;
				$("#terminal__prompt--cursor").css("margin-left", cursor_margin+"px");
				term_count_spaceholders += 1;
			}
			else if(k.keyCode == 37){
				//arrow Left
				if(term_cursor_location < term_buffer.length){
					term_cursor_location = term_cursor_location + 1;
					cursor_margin = cursor_margin - 9;
					$("#terminal__prompt--cursor").css("margin-left", cursor_margin+"px");
				}
			}
			else if(k.keyCode == 38){
				//arrow up
				if(term_history_selected_line == undefined){
					term_history_selected_line = term_history.length;
				}

				if(term_history_selected_line > 0){
					term_history_selected_line = term_history_selected_line - 1;
				}

				if(term_history.length > 0){
					if((term_history_selected_line + 1) == term_history.length){
						term_buffer_2 = term_buffer;
					}
					term_buffer = term_history[term_history_selected_line]
				}

				// if(term_history_selected_line == undefined){
				// 	term_buffer = term_buffer_2;
				// }

			}
			else if(k.keyCode == 39){
				//arrow Right
				if(term_cursor_location > 1){
					term_cursor_location = term_cursor_location - 1;
					cursor_margin = cursor_margin + 9;
					$("#terminal__prompt--cursor").css("margin-left", cursor_margin+"px");
				}
			}
			else if(k.keyCode == 40){

				if((term_history_selected_line + 1) == term_history.length){
					term_history_selected_line = undefined;
					term_buffer = term_buffer_2;
				}


				if(term_history_selected_line < term_history.length){
					term_history_selected_line = term_history_selected_line + 1;
					term_buffer = term_history[term_history_selected_line];
				} else {
					term_history_selected_line = undefined;
					term_buffer = term_buffer_2;
				}


				// if(term_history_selected_line == undefined){
				//
				// }
			}

			else if(k.keyCode == 9){
				//tab
			} else if(k.keyCode == 13){
				//enter
				processing_command = true;
				var html = $("#terminal__body").html().replace("term_host_name_body","").replace("terminal__prompt__buffer","").replace("terminal__prompt--cursor","").replace("id=\"terminal__prompt--location\"", "");
	 			$("#terminal__body").html(html);
				term_history.push(term_buffer);
				sendCommand(term_buffer);


			} else if(k.keyCode == 27){
				//escape
			} else {
				if(banned_keys.includes(k.keyCode)){

				}else{
					if(term_count_spaceholders > 0){
						for (let step = 0; step < term_count_spaceholders; step++) {
							term_cursor_location = term_cursor_location + 1;
							cursor_margin = cursor_margin - 9;
							$("#terminal__prompt--cursor").css("margin-left", cursor_margin+"px");
							term_count_spaceholders = term_count_spaceholders - 1;
						}
					}
					var other_keys = ["/", "\\", "!", "@", "#", "$", "%", "^", "&", "*", "(", ")", "-", "_", "+", "=", "{", "}", "[", "]", ":", ";", "'", "\"", "<", ",", ">", ".", "?", "`", "~"]
					if(k.key.match(/^[a-zA-Z1-9 ]+$/) || other_keys.includes(k.key)){

						if(term_cursor_location == 1){
							term_buffer += k.key;
						} else {
							var yyy = (term_buffer.length) - term_cursor_location;
							yyy = yyy + 1;
							term_buffer = term_buffer.substring(0, yyy) + k.key + term_buffer.substring(yyy, (term_buffer.length));
						}

					}
				}
			}
			$("#terminal__prompt__buffer").html(term_buffer);
		}

	};



	$.get( "/api/server_data", function( data ) {

		// update page
		$.each(data.clients, function( index, value ) {
			if(value.hostname == getUrlParameter("hostname")){
				$("#term_host_name_header").text(value.hostname);
				$("#term_host_name_body").text(value.hostname);
				computer_ip_addr = value.local_ip_address;
				$.get('https://'+computer_ip_addr+':6789'+"/api/cmd?command=pwd&directory="+term_directory, function( data ) {
					console.log(data);
					$("#terminal__prompt--location").html(data);
				});
			}
		});



	});

});

function sendCommand(cmd){
	var cmd_parts = cmd.split(" ");
	if(cmd_parts[0] == "cd"){
		if (cmd_parts[1].length > 0) {
			if(cmd_parts[1].charAt(0) == "/"){
				term_directory = cmd_parts[1];
			} else {
				if(term_directory == "/"){
					term_directory = term_directory + cmd_parts[1];
				} else {
					term_directory = term_directory + "/" + cmd_parts[1];
				}
			}
		}
	}

	// Check if this is an Ollama command
	if (cmd_parts[0] === "ollama") {
		console.log("Detected Ollama command:", cmd);
		handleOllamaCommand(cmd, cmd_parts);
		return;
	}

	$.get("/api/io?input="+cmd, function( data ) {
		// Check for web actions first
		var shouldClearScreen = false;
		if (data.executed_actions) {
			for (var i = 0; i < data.executed_actions.length; i++) {
				var action = data.executed_actions[i];
				if (action.result === "CLEAR_SCREEN") {
					shouldClearScreen = true;
					break;
				}
			}
		}
		
		// Handle clear screen action
		if (shouldClearScreen) {
			$("#terminal__body").empty(); // Clear the terminal
			
			// Add a confirmation message
			$("#terminal__body").append("<span id='terminal__prompt--sam'>Sam<span></span>:</span> Screen cleared.");
			
			// Reset terminal state
			term_buffer = "";
			term_cursor_location = 1;
			cursor_margin = 0;
			processing_command = false;
			
			// Add new prompt
			var html = "<div id='terminal__prompt'>\
						  <span id='terminal__prompt--user'>Caleb@<span id='term_host_name_body'></span>:</span>\
						  <span class='terminal__prompt--location' id='terminal__prompt--location'>~</span>\
						  <span id='terminal__prompt--bling'>$<span id='terminal__prompt__buffer'></span></span>\
						  <span id='terminal__prompt--cursor'></span>\
						</div>";
			$("#terminal__body").append(html);
			
			var objDiv = document.getElementById("terminal__body");
			objDiv.scrollTop = objDiv.scrollHeight;
			return; // Skip normal processing
		}
		
		// Normal response processing
		speak(data.text).then(function () {
			$("#terminal__body").append("<span id='terminal__prompt--sam'>Sam<span></span>:</span> "+data.text.replaceAll("\n", "<br/>") + "<br/>");

			term_buffer = "";
			term_cursor_location = 1;
			cursor_margin = 0;
			processing_command = false;
	
			var html = "<div id='terminal__prompt'>\
						  <span id='terminal__prompt--user'>Caleb@<span id='term_host_name_body'></span>:</span>\
						  <span class='terminal__prompt--location' id='terminal__prompt--location'>~</span>\
						  <span id='terminal__prompt--bling'>$<span id='terminal__prompt__buffer'></span></span>\
						  <span id='terminal__prompt--cursor'></span>\
						</div>";
			$("#terminal__body").append(html);
	
	
			var objDiv = document.getElementById("terminal__body");
			objDiv.scrollTop = objDiv.scrollHeight;
		});


	
	});
}

function handleOllamaCommand(cmd, cmd_parts) {
	var subcommand = cmd_parts[1] || "";
	
	console.log("Handling Ollama command:", cmd, "parts:", cmd_parts);
	
	// Handle shorthand: "ollama <model> <prompt>" should be "ollama run <model> <prompt>"
	if (subcommand && !["help", "status", "install", "start", "stop", "list", "models", "pull", "remove", "run", "search", "info", "install-recommended"].includes(subcommand)) {
		// Treat as "ollama run <model> <prompt>"
		var model = cmd_parts[1];
		var prompt = cmd_parts.slice(2).join(" ");
		console.log("Treating as ollama run:", model, prompt);
		if (model && prompt) {
			runOllamaGeneration(model, prompt);
			return;
		}
	}
	
	switch(subcommand) {
		case "":
		case "help":
			showOllamaHelp();
			break;
		case "status":
			getOllamaStatus();
			break;
		case "install":
			installOllama();
			break;
		case "start":
			startOllama();
			break;
		case "stop":
			stopOllama();
			break;
		case "list":
		case "models":
			listOllamaModels();
			break;
		case "pull":
			var model = cmd_parts[2];
			if (!model) {
				appendTerminalOutput("Usage: ollama pull <model_name>");
				appendTerminalOutput("Example: ollama pull llama3.2");
				finishCommand();
			} else {
				pullOllamaModel(model);
			}
			break;
		case "remove":
			var model = cmd_parts[2];
			if (!model) {
				appendTerminalOutput("Usage: ollama remove <model_name>");
				appendTerminalOutput("Example: ollama remove llama3.2");
				finishCommand();
			} else {
				removeOllamaModel(model);
			}
			break;
		case "run":
			var model = cmd_parts[2];
			var prompt = cmd_parts.slice(3).join(" ");
			if (!model || !prompt) {
				appendTerminalOutput("Usage: ollama run <model_name> <prompt>");
				appendTerminalOutput("Example: ollama run llama3.2 \"Hello, how are you?\"");
				finishCommand();
			} else {
				runOllamaGeneration(model, prompt);
			}
			break;
		case "search":
			var query = cmd_parts.slice(2).join(" ");
			searchOllamaModels(query);
			break;
		case "info":
			var model = cmd_parts[2];
			if (!model) {
				appendTerminalOutput("Usage: ollama info <model_name>");
				finishCommand();
			} else {
				getOllamaModelInfo(model);
			}
			break;
		case "install-recommended":
			installRecommendedModels();
			break;
		default:
			appendTerminalOutput("Unknown ollama command: " + subcommand);
			appendTerminalOutput("Type 'ollama help' to see available commands");
			finishCommand();
			break;
	}
}

function showOllamaHelp() {
	var helpText = [
		"Ollama AI Commands:",
		"",
		"  ollama help                    - Show this help",
		"  ollama status                  - Check Ollama service status",
		"  ollama install                 - Install Ollama if not present",
		"  ollama start                   - Start Ollama service",
		"  ollama stop                    - Stop Ollama service",
		"",
		"Model Management:",
		"  ollama list                    - List installed and available models",
		"  ollama pull <model>            - Download a model (e.g., llama3.2)",
		"  ollama remove <model>          - Remove a model",
		"  ollama search [query]          - Search available models",
		"  ollama info <model>            - Show model information",
		"  ollama install-recommended     - Install recommended models",
		"",
		"AI Generation:",
		"  ollama run <model> <prompt>    - Generate text with a model",
		"",
		"Examples:",
		"  ollama pull llama3.2",
		"  ollama run llama3.2 \"Explain quantum computing\"",
		"  ollama search code"
	];
	
	for (var i = 0; i < helpText.length; i++) {
		appendTerminalOutput(helpText[i]);
	}
	finishCommand();
}

function getOllamaStatus() {
	appendTerminalOutput("⠋ Checking Ollama status...", true);
	
	$.get("/api/ollama/status")
		.done(function(data) {
			console.log("Ollama status response:", data);
			replaceLastSpinner();
			
			appendTerminalOutput("Ollama Status:");
			appendTerminalOutput("  Installed: " + (data.installed ? "✓ Yes" : "✗ No"));
			appendTerminalOutput("  Running:   " + (data.running ? "✓ Yes" : "✗ No"));
			
			if (data.version) {
				appendTerminalOutput("  Version:   " + data.version);
			}
			
			if (data.models && data.models.length > 0) {
				appendTerminalOutput("  Models:    " + data.models.length + " installed");
			}
			
			if (!data.installed) {
				appendTerminalOutput("");
				appendTerminalOutput("Run 'ollama install' to install Ollama.");
			} else if (!data.running) {
				appendTerminalOutput("");
				appendTerminalOutput("Run 'ollama start' to start the service.");
			}
			
			finishCommand();
		})
		.fail(function(xhr, status, error) {
			console.log("Ollama status error:", xhr, status, error);
			replaceLastSpinner();
			appendTerminalOutput("✗ Failed to get Ollama status: " + (xhr.responseText || error));
			finishCommand();
		});
}

function installOllama() {
	appendTerminalOutput("⠋ Installing Ollama...", true);
	
	$.ajax({
		url: "/api/ollama/install",
		type: "POST",
		contentType: "application/json",
		data: JSON.stringify({}),
		success: function(data) {
			replaceLastSpinner();
			
			if (data.success) {
				appendTerminalOutput("✓ " + data.message);
			} else {
				appendTerminalOutput("✗ " + data.message);
			}
			
			finishCommand();
		},
		error: function() {
			replaceLastSpinner();
			appendTerminalOutput("✗ Installation failed");
			finishCommand();
		}
	});
}

function startOllama() {
	appendTerminalOutput("⠋ Starting Ollama service...", true);
	
	$.ajax({
		url: "/api/ollama/start",
		type: "POST",
		contentType: "application/json",
		data: JSON.stringify({}),
		success: function(data) {
			replaceLastSpinner();
			
			if (data.success) {
				appendTerminalOutput("✓ " + data.message);
			} else {
				appendTerminalOutput("✗ " + data.message);
			}
			
			finishCommand();
		},
		error: function() {
			replaceLastSpinner();
			appendTerminalOutput("✗ Failed to start Ollama service");
			finishCommand();
		}
	});
}

function stopOllama() {
	appendTerminalOutput("⠋ Stopping Ollama service...", true);
	
	$.ajax({
		url: "/api/ollama/stop",
		type: "POST",
		contentType: "application/json",
		data: JSON.stringify({}),
		success: function(data) {
			replaceLastSpinner();
			
			if (data.success) {
				appendTerminalOutput("✓ " + data.message);
			} else {
				appendTerminalOutput("✗ " + data.message);
			}
			
			finishCommand();
		},
		error: function() {
			replaceLastSpinner();
			appendTerminalOutput("✗ Failed to stop Ollama service");
			finishCommand();
		}
	});
}

function listOllamaModels() {
	appendTerminalOutput("⠋ Loading models...", true);

	$.get("/api/ollama/models/all")
		.done(function(data) {
			console.log("Ollama all models response:", data);
			replaceLastSpinner();

			if (data.success && data.data) {
				var modelData = data.data;
				var installedModels = modelData.installed || [];
				var availableModels = modelData.available || [];

				// Show installed models first
				if (installedModels.length === 0) {
					appendTerminalOutput("Installed Models (0):");
					appendTerminalOutput("");
					appendTerminalOutput("  No models installed.");
				} else {
					appendTerminalOutput("Installed Models (" + installedModels.length + "):");
					appendTerminalOutput("");
					for (var i = 0; i < installedModels.length; i++) {
						var model = installedModels[i];
						var sizeGB = (model.size / (1024 * 1024 * 1024)).toFixed(1);
						appendTerminalOutput("  " + model.name + " (" + sizeGB + " GB)");
					}
				}

				appendTerminalOutput("");
				appendTerminalOutput("Available Models for Installation:");
				appendTerminalOutput("");

				// Group available models by category
				var groupedModels = {
					'Llama Models': [],
					'Code Models': [],
					'Mistral Models': [],
					'Gemma Models': [],
					'DeepSeek Models': [],
					'Other Models': []
				};

				for (var i = 0; i < availableModels.length; i++) {
					var model = availableModels[i];
					if (model.startsWith('llama')) {
						groupedModels['Llama Models'].push(model);
					} else if (model.startsWith('code') || model.includes('coder')) {
						groupedModels['Code Models'].push(model);
					} else if (model.startsWith('mistral') || model.includes('mixtral')) {
						groupedModels['Mistral Models'].push(model);
					} else if (model.startsWith('gemma')) {
						groupedModels['Gemma Models'].push(model);
					} else if (model.startsWith('deepseek')) {
						groupedModels['DeepSeek Models'].push(model);
					} else {
						groupedModels['Other Models'].push(model);
					}
				}

				// Display models by category
				for (var category in groupedModels) {
					if (groupedModels[category].length > 0) {
						appendTerminalOutput("  " + category + ":");
						for (var j = 0; j < groupedModels[category].length; j++) {
							appendTerminalOutput("    " + groupedModels[category][j]);
						}
						appendTerminalOutput("");
					}
				}

				appendTerminalOutput("Use 'ollama pull <model>' to install a model.");
				appendTerminalOutput("Example: ollama pull llama3.2:latest");
			} else {
				appendTerminalOutput("✗ " + (data.message || "Failed to list models"));
			}

			finishCommand();
		})
		.fail(function(xhr, status, error) {
			console.log("Ollama models error:", xhr, status, error);
			replaceLastSpinner();

			var errorMsg = "✗ Failed to list models";
			if (xhr.responseJSON && xhr.responseJSON.message) {
				errorMsg += ": " + xhr.responseJSON.message;
			} else if (xhr.status === 503) {
				errorMsg += ": Ollama service is not running";
			} else {
				errorMsg += ": " + (error || status);
			}

			appendTerminalOutput(errorMsg);
			if (xhr.status === 503) {
				appendTerminalOutput("Make sure Ollama service is running: 'ollama start'");
			}

			finishCommand();
		});
}

function pullOllamaModel(model) {
	appendTerminalOutput("This may take several minutes depending on model size.");
	appendTerminalOutput("⠋ Pulling model: " + model + "...", true);
	
	var requestData = { model: model };
	
	$.ajax({
		url: "/api/ollama/models/pull",
		type: "POST",
		contentType: "application/json",
		data: JSON.stringify(requestData),
		success: function(data) {
			replaceLastSpinner();
			
			if (data.success) {
				appendTerminalOutput("✓ " + data.message);
			} else {
				appendTerminalOutput("✗ " + data.message);
				if (data.message && data.message.includes("connection")) {
					appendTerminalOutput("Make sure Ollama service is running: 'ollama start'");
				}
			}
			
			finishCommand();
		},
		error: function() {
			replaceLastSpinner();
			appendTerminalOutput("✗ Failed to pull model: " + model);
			finishCommand();
		}
	});
}

function removeOllamaModel(model) {
	appendTerminalOutput("⠋ Removing model: " + model + "...", true);
	
	$.ajax({
		url: "/api/ollama/models/" + encodeURIComponent(model),
		type: "DELETE",
		success: function(data) {
			replaceLastSpinner();
			
			if (data.success) {
				appendTerminalOutput("✓ " + data.message);
			} else {
				appendTerminalOutput("✗ " + data.message);
			}
			
			finishCommand();
		},
		error: function() {
			replaceLastSpinner();
			appendTerminalOutput("✗ Failed to remove model: " + model);
			finishCommand();
		}
	});
}

function runOllamaGeneration(model, prompt) {
	appendTerminalOutput("Prompt: " + prompt);
	appendTerminalOutput("");
	appendTerminalOutput("⠋ Generating with model '" + model + "'...", true);
	
	var requestData = {
		model: model,
		prompt: prompt,
		options: null
	};
	
	console.log("Making generate request with data:", requestData);
	
	$.ajax({
		url: "/api/ollama/generate",
		type: "POST",
		contentType: "application/json",
		data: JSON.stringify(requestData),
		success: function(data) {
			console.log("Generate response:", data);
			replaceLastSpinner();
			
			if (data.success && data.data) {
				var response = data.data;
				
				appendTerminalOutput("Response:");
				appendTerminalOutput("");
				
				// Split response into lines for better display
				var responseLines = response.response.split('\n');
				for (var i = 0; i < responseLines.length; i++) {
					if (responseLines[i].trim().length > 0) {
						appendTerminalOutput("  " + responseLines[i]);
					} else if (i < responseLines.length - 1) {
						// Add empty line for spacing
						appendTerminalOutput("");
					}
				}
				
				appendTerminalOutput("");
				if (response.total_duration) {
					var durationSec = (response.total_duration / 1000000000).toFixed(2);
					appendTerminalOutput("Generated in " + durationSec + "s");
				}
			} else {
				appendTerminalOutput("✗ " + (data.message || "Failed to generate text"));
				if (data.message && data.message.includes("connection")) {
					appendTerminalOutput("Make sure Ollama service is running: 'ollama start'");
				} else if (data.message && data.message.includes("not found")) {
					appendTerminalOutput("Model '" + model + "' not found. Use 'ollama pull " + model + "' to install it.");
				}
			}
			
			finishCommand();
		},
		error: function(xhr, status, error) {
			console.log("Generate error:", xhr, status, error);
			replaceLastSpinner();
			appendTerminalOutput("✗ Failed to generate text: " + (xhr.responseText || error));
			finishCommand();
		}
	});
}

function searchOllamaModels(query) {
	var searchUrl = query ? "/api/ollama/models/available/" + encodeURIComponent(query) : "/api/ollama/models/available";
	appendTerminalOutput("⠋ Searching models...", true);
	
	$.get(searchUrl, function(data) {
		replaceLastSpinner();
		
		if (data.success && data.data) {
			var models = data.data;
			
			if (models.length === 0) {
				if (query) {
					appendTerminalOutput("No models found matching '" + query + "'");
				} else {
					appendTerminalOutput("No models available");
				}
			} else {
				if (query) {
					appendTerminalOutput("Models matching '" + query + "' (" + models.length + "):");
				} else {
					appendTerminalOutput("Popular models available:");
				}
				appendTerminalOutput("");
				
				for (var i = 0; i < models.length; i++) {
					appendTerminalOutput("  " + models[i]);
				}
				
				appendTerminalOutput("");
				appendTerminalOutput("Use 'ollama pull <model>' to install a model.");
			}
		} else {
			appendTerminalOutput("✗ " + (data.message || "Failed to search models"));
		}
		
		finishCommand();
	}).fail(function() {
		replaceLastSpinner();
		appendTerminalOutput("✗ Failed to search models");
		finishCommand();
	});
}

function getOllamaModelInfo(model) {
	appendTerminalOutput("⠋ Getting model information...", true);
	
	$.get("/api/ollama/models/" + encodeURIComponent(model) + "/info", function(data) {
		replaceLastSpinner();
		
		if (data.success && data.data) {
			appendTerminalOutput("Model Information: " + model);
			appendTerminalOutput("");
			
			// Pretty print the JSON information
			var jsonStr = JSON.stringify(data.data, null, 2);
			var lines = jsonStr.split('\n');
			for (var i = 0; i < lines.length; i++) {
				appendTerminalOutput("  " + lines[i]);
			}
		} else {
			appendTerminalOutput("✗ " + (data.message || "Failed to get model info"));
			if (data.message && data.message.includes("connection")) {
				appendTerminalOutput("Make sure Ollama service is running: 'ollama start'");
			}
		}
		
		finishCommand();
	}).fail(function() {
		replaceLastSpinner();
		appendTerminalOutput("✗ Failed to get model info for '" + model + "'");
		finishCommand();
	});
}

function installRecommendedModels() {
	appendTerminalOutput("This will install: llama3.2, codellama, mistral, gemma2:2b, and phi3:mini");
	appendTerminalOutput("This may take several minutes.");
	appendTerminalOutput("⠋ Installing recommended models...", true);
	
	$.ajax({
		url: "/api/ollama/models/install-recommended",
		type: "POST",
		contentType: "application/json",
		data: JSON.stringify({}),
		success: function(data) {
			replaceLastSpinner();
			
			if (data.success) {
				appendTerminalOutput("");
				appendTerminalOutput("Installation Results:");
				
				var lines = data.message.split('\n');
				for (var i = 0; i < lines.length; i++) {
					if (lines[i].trim()) {
						appendTerminalOutput("  " + lines[i]);
					}
				}
			} else {
				appendTerminalOutput("✗ " + data.message);
			}
			
			finishCommand();
		},
		error: function() {
			replaceLastSpinner();
			appendTerminalOutput("✗ Failed to install recommended models");
			finishCommand();
		}
	});
}

// Helper functions for terminal output management
var spinnerChars = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
var spinnerIndex = 0;
var spinnerInterval = null;

function appendTerminalOutput(text, isSpinner = false) {
	var outputElement;
	if (isSpinner) {
		// Start with first spinner character
		outputElement = "<span class='spinner-line' id='current-spinner'><span id='terminal__prompt--sam'>Sam:</span> <span class='spinner-char'>" + spinnerChars[0] + "</span> " + text.replace(/^⠋\s*/, "") + "</span>";
		$("#terminal__body").append(outputElement);
		
		// Start spinner animation
		startSpinner();
	} else {
		outputElement = "<span id='terminal__prompt--sam'>Sam:</span> " + text.replaceAll("\n", "<br/>");
		$("#terminal__body").append(outputElement + "<br/>");
	}
	
	scrollToBottom();
}

function startSpinner() {
	spinnerIndex = 0;
	
	if (spinnerInterval) {
		clearInterval(spinnerInterval);
	}
	
	spinnerInterval = setInterval(function() {
		spinnerIndex = (spinnerIndex + 1) % spinnerChars.length;
		$('#current-spinner .spinner-char').text(spinnerChars[spinnerIndex]);
	}, 80);
}

function replaceLastSpinner() {
	// Stop spinner animation
	if (spinnerInterval) {
		clearInterval(spinnerInterval);
		spinnerInterval = null;
	}
	
	// Remove the last spinner line
	$("#terminal__body").find('#current-spinner').remove();
}

function finishCommand() {
	term_buffer = "";
	term_cursor_location = 1;
	cursor_margin = 0;
	processing_command = false;

	var html = "<div id='terminal__prompt'>\
				  <span id='terminal__prompt--user'>Caleb@<span id='term_host_name_body'></span>:</span>\
				  <span class='terminal__prompt--location' id='terminal__prompt--location'>~</span>\
				  <span id='terminal__prompt--bling'>$<span id='terminal__prompt__buffer'></span></span>\
				  <span id='terminal__prompt--cursor'></span>\
				</div>";
	$("#terminal__body").append(html);
	
	scrollToBottom();
}

function scrollToBottom() {
	var objDiv = document.getElementById("terminal__body");
	objDiv.scrollTop = objDiv.scrollHeight;
}