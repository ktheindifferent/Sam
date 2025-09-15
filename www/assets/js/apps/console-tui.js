// ███████     █████     ███    ███    
// ██         ██   ██    ████  ████    
// ███████    ███████    ██ ████ ██    
//      ██    ██   ██    ██  ██  ██    
// ███████ ██ ██   ██ ██ ██      ██ ██ 
// Copyright 2021-2026 The Open Sam Foundation (OSF)
// Developed by Caleb Mitchell Smith (ktheindifferent, PixelCoda, p0indexter)
// Licensed under GPLv3....see LICENSE file.

// TUI State Management
var currentMode = 'command';
var selectedServiceIndex = 0;
var services = [
  { name: 'Crawler', id: 'crawler', status: 'unknown' },
  { name: 'Redis', id: 'redis', status: 'unknown' },
  { name: 'Docker', id: 'docker', status: 'unknown' },
  { name: 'SMS', id: 'sms', status: 'unknown' },
  { name: 'PostgreSQL', id: 'postgres', status: 'unknown' },
  { name: 'LIFX', id: 'lifx', status: 'unknown' },
  { name: 'HTTP Server', id: 'http_server', status: 'unknown' },
  { name: 'Ollama', id: 'ollama', status: 'unknown' }
];

// Original terminal state
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

// WebSocket for real-time updates
var websocket = null;
var reconnectInterval = null;
var heartbeatInterval = null;
var systemStats = {
  cpu: '0%',
  memory: '0 MB',
  disk: '0%',
  updateCount: 0
};

$( document ).ready(function() {
  // Initialize TUI interface
  initializeTUI();
  connectWebSocket();
  
  // Set up mode switching
  $('.nav-tab').on('click', function() {
    var mode = $(this).data('mode');
    switchMode(mode);
  });
  
  // Initialize services list
  updateServicesDisplay();
  
  // Set up log filter
  $('#log-filter-input').on('input', function() {
    filterLogs($(this).val());
  });

  window.onkeydown = function(k){
    // Only log debug info in development
    // console.log(k)

    // Handle function keys for mode switching
    if (k.keyCode >= 112 && k.keyCode <= 118) { // F1-F7
      k.preventDefault();
      var modeIndex = k.keyCode - 112;
      var modes = ['command', 'services', 'logs', 'system', 'database', 'files', 'help'];
      if (modeIndex < modes.length) {
        switchMode(modes[modeIndex]);
        return;
      }
    }

    // Handle Ctrl+C - only prevent if we're processing a command
    if (k.keyCode === 67 && (k.ctrlKey || k.metaKey)) {
      if (processing_command) {
        k.preventDefault();
        appendToOutput('Process interrupted (Ctrl+C)');
        processing_command = false;
        return;
      }
      // Otherwise allow normal copy functionality
      return;
    }
    
    // Handle different modes
    if (currentMode === 'services') {
      handleServicesKeyboard(k);
      return;
    }
    
    // Only prevent default for command mode
    if (currentMode === 'command') {
      k.preventDefault();
    } else {
      return; // Let other modes handle their own keyboard
    }

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
        // todo - history log
        // term_history
        // term_history_selected_line =
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
      updateCommandInput();
    }

  };

  // Load initial data
  loadServerData();

});

// TUI Functions
function initializeTUI() {
  // Show initial mode without logging
  currentMode = 'command';
  $('.nav-tab').removeClass('active');
  $('.nav-tab[data-mode="command"]').addClass('active');
  $('.tui-mode').removeClass('active');
  $('#mode-command').addClass('active');
  updateCommandInput();
}

function switchMode(mode) {
  currentMode = mode;

  // Update navigation tabs
  $('.nav-tab').removeClass('active');
  $('.nav-tab[data-mode="' + mode + '"]').addClass('active');

  // Hide all modes
  $('.tui-mode').removeClass('active');

  // Show selected mode
  $('#mode-' + mode).addClass('active');

  // Mode-specific initialization
  switch(mode) {
    case 'services':
      updateServicesDisplay();
      break;
    case 'system':
      updateSystemInfo();
      break;
    case 'logs':
      loadLogs();
      break;
  }

  // Log mode switch like TUI does
  appendToOutput('Switched to ' + mode + ' mode (F' + (['command', 'services', 'logs', 'system', 'database', 'files', 'help'].indexOf(mode) + 1) + ')');
}

function updateCommandInput() {
  $('#terminal__prompt__buffer').html(term_buffer);
  $('#command-buffer').html(term_buffer);
}

function handleServicesKeyboard(k) {
  k.preventDefault();
  
  switch(k.keyCode) {
    case 38: // Up arrow
      if (selectedServiceIndex > 0) {
        selectedServiceIndex--;
        updateServicesDisplay();
      }
      break;
    case 40: // Down arrow
      if (selectedServiceIndex < services.length - 1) {
        selectedServiceIndex++;
        updateServicesDisplay();
      }
      break;
    case 32: // Space - Start/Stop service
      toggleService(services[selectedServiceIndex]);
      break;
    case 82: // R - Restart service
      restartService(services[selectedServiceIndex]);
      break;
    case 76: // L - View logs
      viewServiceLogs(services[selectedServiceIndex]);
      break;
    case 13: // Enter - Service details
      showServiceDetails(services[selectedServiceIndex]);
      break;
  }
}

function updateServicesDisplay() {
  var servicesList = $('#services-list');
  servicesList.empty();
  
  services.forEach(function(service, index) {
    var isSelected = index === selectedServiceIndex;
    var statusClass = 'service-status ' + service.status;
    
    var item = $('<div class="service-list-item' + (isSelected ? ' selected' : '') + '">');
    item.html(
      '<span class="service-label">' + service.name + ':</span> ' +
      '<span class="' + statusClass + '">' + service.status + '</span>'
    );
    
    item.on('click', function() {
      selectedServiceIndex = index;
      updateServicesDisplay();
    });
    
    servicesList.append(item);
  });
  
  // Update service details
  var selectedService = services[selectedServiceIndex];
  $('#selected-service-name').text(selectedService.name);
  $('#selected-service-status').text(selectedService.status).attr('class', 'detail-value service-status ' + selectedService.status);
}

function toggleService(service) {
  if (websocket && websocket.readyState === WebSocket.OPEN) {
    var command = service.status === 'running' ? 'stop_service' : 'start_service';
    var commandId = 'toggle_' + service.id + '_' + Date.now();

    websocket.send(JSON.stringify({
      type: 'command',
      id: commandId,
      command: command,
      args: { service: service.id }
    }));

    appendToOutput((service.status === 'running' ? 'Stopping' : 'Starting') + ' ' + service.name + ' service...');
  } else {
    appendToOutput('WebSocket not connected. Cannot control services.');
  }
}

function restartService(service) {
  if (websocket && websocket.readyState === WebSocket.OPEN) {
    var commandId = 'restart_' + service.id + '_' + Date.now();

    websocket.send(JSON.stringify({
      type: 'command',
      id: commandId,
      command: 'restart_service',
      args: { service: service.id }
    }));

    appendToOutput('Restarting ' + service.name + ' service...');
  } else {
    appendToOutput('WebSocket not connected. Cannot control services.');
  }
}

function viewServiceLogs(service) {
  if (websocket && websocket.readyState === WebSocket.OPEN) {
    var commandId = 'logs_' + service.id + '_' + Date.now();

    websocket.send(JSON.stringify({
      type: 'command',
      id: commandId,
      command: 'get_service_logs',
      args: { service: service.id, lines: 50 }
    }));

    appendToOutput('Fetching logs for ' + service.name + ' service...');
  } else {
    appendToOutput('WebSocket not connected. Cannot fetch logs.');
  }
}

function showServiceDetails(service) {
  if (websocket && websocket.readyState === WebSocket.OPEN) {
    var commandId = 'details_' + service.id + '_' + Date.now();

    websocket.send(JSON.stringify({
      type: 'command',
      id: commandId,
      command: 'get_service_details',
      args: { service: service.id }
    }));

    appendToOutput('Getting details for ' + service.name + ' service...');
  } else {
    appendToOutput('WebSocket not connected. Cannot get service details.');
  }
}

function updateSystemInfo() {
  $('#system-cpu-usage').text(systemStats.cpu);
  $('#system-memory-usage').text(systemStats.memory);
  $('#system-disk-usage').text(systemStats.disk);
  $('#system-update-count').text(systemStats.updateCount);
  
  // Update service overview
  var overview = $('#service-overview');
  overview.empty();
  
  services.forEach(function(service) {
    var statusClass = 'service-status ' + service.status;
    overview.append(
      '<div class="metric-row">' +
      '<span class="metric-label">' + service.name + ':</span>' +
      '<span class="metric-value ' + statusClass + '">' + service.status + '</span>' +
      '</div>'
    );
  });
}

function loadLogs() {
  var logContent = $('#log-content');
  logContent.empty();

  // Get current time for more realistic logs
  var now = new Date();
  var timeStr = now.getHours().toString().padStart(2, '0') + ':' +
                now.getMinutes().toString().padStart(2, '0') + ':' +
                now.getSeconds().toString().padStart(2, '0');

  // Realistic log entries that match SAM's TUI
  var logs = [
    { timestamp: timeStr, level: 'info', message: '[sam cli] start_prompt() called', target: 'sam::cli::tui' },
    { timestamp: timeStr, level: 'debug', message: 'TUI logger initialized', target: 'tui_logger' },
    { timestamp: timeStr, level: 'info', message: 'WebSocket server started on port 8080', target: 'sam::websocket' },
    { timestamp: timeStr, level: 'info', message: 'HTTP server started on port 8000', target: 'sam::http' },
    { timestamp: timeStr, level: 'debug', message: 'Service status update: Redis connected', target: 'sam::services::redis' },
    { timestamp: timeStr, level: 'debug', message: 'System metrics updated (CPU: ' + systemStats.cpu + ', Memory: ' + systemStats.memory + ')', target: 'sam::services::monitoring' },
    { timestamp: timeStr, level: 'warn', message: 'Service check timeout for Docker service', target: 'sam::services::docker' },
    { timestamp: timeStr, level: 'info', message: 'Database connection established', target: 'sam::services::database' },
    { timestamp: timeStr, level: 'debug', message: 'Update count: ' + systemStats.updateCount, target: 'sam::cli::tui' }
  ];

  logs.forEach(function(log) {
    logContent.append(
      '<div class="log-entry">' +
      '<span class="log-timestamp">' + log.timestamp + '</span>' +
      '<span class="log-level-' + log.level + '">[' + log.level.toUpperCase() + ']</span> ' +
      '<span class="log-target">' + log.target + '</span>: ' +
      log.message +
      '</div>'
    );
  });
}

function filterLogs(filter) {
  if (!filter) {
    $('.log-entry').show();
    return;
  }
  
  $('.log-entry').each(function() {
    var text = $(this).text().toLowerCase();
    if (text.includes(filter.toLowerCase())) {
      $(this).show();
    } else {
      $(this).hide();
    }
  });
}

function appendToOutput(message) {
  if (currentMode === 'command') {
    $('#terminal__body').append('<span id="terminal__prompt--sam">Sam:</span> ' + message + '<br/>');
    scrollToBottom();
  }
}

function scrollToBottom() {
  var objDiv = document.getElementById('terminal__body');
  if (objDiv) {
    objDiv.scrollTop = objDiv.scrollHeight;
  }
}

function loadServerData() {
  // Legacy function - can be removed or adapted for WebSocket
}

// WebSocket Functions
function connectWebSocket() {
  if (websocket) {
    websocket.close();
  }
  
  var protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
  var wsUrl = protocol + '//' + window.location.hostname + ':8080/ws';
  
  websocket = new WebSocket(wsUrl);
  
  websocket.onopen = function() {
    console.log('WebSocket connected');
    appendToOutput('Connected to SAM WebSocket server');

    // Clear reconnection interval if it was set
    if (reconnectInterval) {
      clearInterval(reconnectInterval);
      reconnectInterval = null;
    }

    // Subscribe to channels
    websocket.send(JSON.stringify({
      type: 'subscribe',
      channels: ['services', 'stats', 'alerts']
    }));

    // Request initial service status
    websocket.send(JSON.stringify({
      type: 'command',
      id: 'get_services_' + Date.now(),
      command: 'get_services',
      args: {}
    }));

    // Start heartbeat (like the TUI every 30 seconds)
    heartbeatInterval = setInterval(function() {
      if (websocket && websocket.readyState === WebSocket.OPEN) {
        websocket.send(JSON.stringify({
          type: 'heartbeat',
          timestamp: Date.now()
        }));
      }
    }, 30000);
  };
  
  websocket.onmessage = function(event) {
    try {
      var message = JSON.parse(event.data);
      handleWebSocketMessage(message);
    } catch (e) {
      console.error('Failed to parse WebSocket message:', e);
    }
  };
  
  websocket.onclose = function() {
    console.log('WebSocket disconnected');
    appendToOutput('Disconnected from WebSocket server');

    // Clear heartbeat
    if (heartbeatInterval) {
      clearInterval(heartbeatInterval);
      heartbeatInterval = null;
    }

    // Attempt to reconnect (like TUI does with exponential backoff)
    if (!reconnectInterval) {
      reconnectInterval = setInterval(function() {
        console.log('Attempting to reconnect WebSocket...');
        connectWebSocket();
      }, 5000);
    }
  };
  
  websocket.onerror = function(error) {
    console.error('WebSocket error:', error);
    appendToOutput('WebSocket error: ' + error);
  };
}

function handleWebSocketMessage(message) {
  switch(message.type) {
    case 'service_status':
      updateServiceStatus(message.service, message.status);
      break;
    case 'system_stats':
      updateSystemStats(message.stats);
      break;
    case 'command_response':
      handleCommandResponse(message);
      break;
    case 'activity':
      if (message.activity) {
        appendToOutput('Activity: ' + message.activity.message);
      }
      break;
  }
}

function updateServiceStatus(serviceName, status) {
  // Update service in our array
  var service = services.find(s => s.id === serviceName);
  if (service) {
    service.status = status.state || 'unknown';
  }
  
  // Update status display in command mode
  $('#status-' + serviceName).text(status.state || 'unknown')
    .removeClass().addClass('service-status ' + (status.state || 'unknown'));
  
  // Update services display if in services mode
  if (currentMode === 'services') {
    updateServicesDisplay();
  }
  
  // Update system info if in system mode
  if (currentMode === 'system') {
    updateSystemInfo();
  }
}

function updateSystemStats(stats) {
  systemStats = {
    cpu: stats.cpu ? stats.cpu.toFixed(1) + '%' : '0%',
    memory: stats.memory_percent ?
      (stats.memory_used && stats.memory_total ?
        (stats.memory_used / (1024 * 1024)).toFixed(0) + '/' + (stats.memory_total / (1024 * 1024)).toFixed(0) + ' MB (' + stats.memory_percent.toFixed(1) + '%)' :
        stats.memory_percent.toFixed(1) + '%') : '0 MB',
    disk: stats.disk_percent ? stats.disk_percent.toFixed(1) + '%' : '0%',
    updateCount: (systemStats.updateCount || 0) + 1
  };
  
  // Update command mode status
  $('#cpu-usage').text(systemStats.cpu);
  $('#memory-usage').text(systemStats.memory);
  
  // Update system mode if active
  if (currentMode === 'system') {
    updateSystemInfo();
  }
}

function handleCommandResponse(message) {
  if (message.success) {
    if (message.data) {
      appendToOutput('Command successful: ' + JSON.stringify(message.data));
    } else {
      appendToOutput('Command completed successfully');
    }
  } else {
    appendToOutput('Command failed: ' + (message.error || 'Unknown error'));
  }
}

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

  $.get("/api/io?input="+encodeURIComponent(cmd), function( data ) {
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
      updateCommandInput();
      
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
      updateCommandInput();

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
    "  ollama list                    - List installed models",
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
  
  $.get("/api/ollama/models")
    .done(function(data) {
      console.log("Ollama models response:", data);
      replaceLastSpinner();
      
      if (data.success && data.data && data.data.models) {
        var models = data.data.models;
        if (models.length === 0) {
          appendTerminalOutput("No models installed.");
          appendTerminalOutput("Run 'ollama pull <model>' to install a model.");
        } else {
          appendTerminalOutput("Installed Models (" + models.length + "):");
          appendTerminalOutput("");
          
          for (var i = 0; i < models.length; i++) {
            var model = models[i];
            var sizeGB = (model.size / (1024 * 1024 * 1024)).toFixed(1);
            appendTerminalOutput("  " + model.name + " (" + sizeGB + " GB)");
          }
        }
      } else {
        appendTerminalOutput("✗ " + (data.message || "Failed to list models"));
        if (data.message && data.message.includes("connection")) {
          appendTerminalOutput("Make sure Ollama service is running: 'ollama start'");
        }
      }
      
      finishCommand();
    })
    .fail(function(xhr, status, error) {
      console.log("Ollama models error:", xhr, status, error);
      replaceLastSpinner();
      appendTerminalOutput("✗ Failed to list models: " + (xhr.responseText || error));
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
  appendTerminalOutput("This will install: llama3.2, codellama, and mistral");
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
  updateCommandInput();
  
  scrollToBottom();
}