// SAM Dashboard Service Control
// WebSocket connection for real-time updates
let ws = null;
let reconnectAttempts = 0;
const maxReconnectAttempts = 5;
let serviceStatuses = {};
let startTime = Date.now();
let heartbeatInterval = null;

// Initialize WebSocket connection
function initWebSocket() {
    // Skip WebSocket if we know backend is offline
    const systemStatus = document.getElementById('system-status');
    if (systemStatus && systemStatus.textContent === 'Backend Offline') {
        console.log('Skipping WebSocket - backend is offline');
        return;
    }
    
    // WebSocket configuration
    const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
    const hostname = window.location.hostname;
    
    // In production, WebSocket might be proxied through the same port
    // In development, use port 8080
    let wsUrl;
    if (hostname === 'localhost' || hostname === '127.0.0.1') {
        // Local development - use port 8080
        wsUrl = `${protocol}//${hostname}:8080/ws`;
    } else {
        // Production - try same port first (assumes reverse proxy)
        wsUrl = `${protocol}//${window.location.host}/ws`;
    }
    
    console.log('Attempting WebSocket connection to:', wsUrl);
    
    try {
        ws = new WebSocket(wsUrl);
        window.ws = ws;
        
        ws.onopen = () => {
            console.log('WebSocket connected');
            reconnectAttempts = 0;
            updateConnectionStatus(true);
            addLog('WebSocket connection established', 'success');
            requestServiceStatus();
            
            // Start heartbeat to keep connection alive
            startHeartbeat();
        };
        
        ws.onmessage = (event) => {
            try {
                console.log('WebSocket message received:', event.data);
                const data = JSON.parse(event.data);
                handleWebSocketMessage(data);
            } catch (e) {
                console.error('Failed to parse WebSocket message:', e, 'Raw message:', event.data);
            }
        };
        
        ws.onerror = (error) => {
            console.error('WebSocket error:', error);
            addLog('WebSocket error occurred', 'error');
        };
        
        ws.onclose = () => {
            console.log('WebSocket disconnected');
            updateConnectionStatus(false);
            addLog('WebSocket connection lost', 'warning');
            stopHeartbeat();
            attemptReconnect();
        };
    } catch (error) {
        console.error('Failed to create WebSocket:', error);
        updateConnectionStatus(false);
        // Fall back to polling
        startPolling();
    }
}

// Handle incoming WebSocket messages
function handleWebSocketMessage(data) {
    console.log('Handling WebSocket message type:', data.type, data);
    
    if (data.type === 'service_status') {
        // Convert WebSocket status format to UI format
        const uiStatus = {
            running: data.status.state === 'healthy' || data.status.state === 'running',
            status_text: data.status.message || data.status.state,
            ...data.status
        };
        updateServiceStatus(data.service, uiStatus);
    } else if (data.type === 'system_stats') {
        updateSystemStats(data.stats);
    } else if (data.type === 'command_response') {
        console.log('Received command response:', data);
        handleCommandResponse(data);
    } else if (data.type === 'error') {
        addLog(data.message, 'error');
    } else if (data.type === 'activity') {
        addLog(data.activity.message, 'info');
    } else if (data.type === 'alert') {
        addLog(data.message, data.severity);
        // Show toast for warning and above
        if (data.severity === 'warning' || data.severity === 'error' || data.severity === 'critical') {
            showToast(data.message, data.severity === 'critical' ? 'error' : data.severity);
        }
    } else if (data.type === 'heartbeat_ack') {
        console.log('Heartbeat acknowledged');
    }
}

// Start heartbeat to keep connection alive
function startHeartbeat() {
    stopHeartbeat(); // Clear any existing interval
    heartbeatInterval = setInterval(() => {
        if (ws && ws.readyState === WebSocket.OPEN) {
            const heartbeat = {
                type: 'heartbeat',
                timestamp: Date.now()
            };
            ws.send(JSON.stringify(heartbeat));
            console.log('Sent heartbeat');
        }
    }, 30000); // Send heartbeat every 30 seconds
}

// Stop heartbeat
function stopHeartbeat() {
    if (heartbeatInterval) {
        clearInterval(heartbeatInterval);
        heartbeatInterval = null;
    }
}

// Attempt to reconnect WebSocket
function attemptReconnect() {
    if (reconnectAttempts < maxReconnectAttempts) {
        reconnectAttempts++;
        setTimeout(() => {
            console.log(`Attempting to reconnect... (${reconnectAttempts}/${maxReconnectAttempts})`);
            initWebSocket();
        }, 3000 * reconnectAttempts);
    } else {
        addLog('Max reconnection attempts reached. Falling back to polling.', 'error');
        startPolling();
    }
}

// Update connection status indicator
function updateConnectionStatus(connected) {
    const indicator = document.getElementById('connection-status');
    if (connected) {
        indicator.innerHTML = `
            <span class="status-indicator running"></span>
            <span style="color: var(--text-secondary);">Connected</span>
        `;
    } else {
        indicator.innerHTML = `
            <span class="status-indicator stopped"></span>
            <span style="color: var(--text-secondary);">Disconnected</span>
        `;
    }
}

// Request service status via WebSocket or HTTP
function requestServiceStatus() {
    if (ws && ws.readyState === WebSocket.OPEN) {
        // Subscribe to service updates
        ws.send(JSON.stringify({ 
            type: 'subscribe', 
            channels: ['services', 'stats', 'alerts', 'activity'] 
        }));
        // Request current service statuses
        const getServicesCmd = { 
            type: 'command', 
            id: generateCommandId(),
            command: 'get_services',
            args: {}
        };
        console.log('Requesting service statuses:', getServicesCmd);
        ws.send(JSON.stringify(getServicesCmd));
    } else {
        // Fall back to HTTP polling
        fetchServiceStatus();
    }
}

// Generate unique command ID
function generateCommandId() {
    return `cmd_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`;
}

// Handle command responses from WebSocket
function handleCommandResponse(data) {
    console.log('Processing command response:', data);
    
    if (data.success) {
        if (data.data) {
            // Handle service control responses
            if (data.data.message) {
                showToast(data.data.message, 'success');
                addLog(data.data.message, 'success');
                // Request updated status after service control
                setTimeout(() => requestServiceStatus(), 1000);
            }
            // Handle service status response (get_services command)
            else if (typeof data.data === 'object') {
                console.log('Received service statuses:', data.data);
                for (const [service, status] of Object.entries(data.data)) {
                    if (typeof status === 'object') {
                        // Convert WebSocket status format to UI format
                        const uiStatus = {
                            running: status.state === 'healthy' || status.state === 'running',
                            status_text: status.message || status.state,
                            ...status
                        };
                        updateServiceStatus(service, uiStatus);
                    }
                }
            }
        }
    } else {
        console.error('Command failed:', data.data);
        const errorMsg = data.data?.error || data.data?.message || 'Unknown error';
        showToast(`Command failed: ${errorMsg}`, 'error');
        addLog(`Command failed: ${errorMsg}`, 'error');
    }
}

// Update system statistics (both service-control and core.js dashboard elements)
function updateSystemStats(stats) {
    if (stats.cpu !== undefined) {
        const cpuElement = document.getElementById('cpu-usage');
        if (cpuElement) cpuElement.textContent = `${stats.cpu.toFixed(1)}%`;
        // Also update core.js dashboard elements
        const cpuCard = document.getElementById('cpu-usage-card');
        if (cpuCard) cpuCard.textContent = `${stats.cpu.toFixed(1)}%`;
        const cpuProgress = document.getElementById('cpu-progress-main');
        if (cpuProgress) cpuProgress.style.width = `${stats.cpu}%`;
    }
    if (stats.memory_percent !== undefined) {
        const memElement = document.getElementById('memory-usage');
        if (memElement) memElement.textContent = `${stats.memory_percent.toFixed(1)}%`;
        const memProgress = document.getElementById('memory-progress-main');
        if (memProgress) memProgress.style.width = `${stats.memory_percent}%`;
    }
    if (stats.memory_used !== undefined) {
        const memCard = document.getElementById('memory-usage-card');
        if (memCard) {
            const memMB = (stats.memory_used / (1024 * 1024)).toFixed(0);
            memCard.textContent = `${memMB} MB`;
        }
    }
    if (stats.disk_percent !== undefined) {
        const diskElement = document.getElementById('disk-usage');
        if (diskElement) diskElement.textContent = `${stats.disk_percent.toFixed(1)}%`;
        const diskProgress = document.getElementById('disk-progress-main');
        if (diskProgress) diskProgress.style.width = `${stats.disk_percent}%`;
    }
}

// Fetch service status via HTTP
async function fetchServiceStatus() {
    let hasBackendError = false;
    
    try {
        // Fetch individual service statuses
        const services = ['redis', 'crawler', 'postgres', 'docker', 'voice', 'websocket', 'lifx'];
        
        for (const service of services) {
            try {
                let statusUrl;
                let status = { running: false };
                
                // Service-specific status endpoints
                switch(service) {
                    case 'redis':
                        statusUrl = '/api/services/redis/status';
                        const redisResponse = await fetch(statusUrl);
                        if (redisResponse.status === 502) {
                            hasBackendError = true;
                            break;
                        }
                        if (redisResponse.ok) {
                            const contentType = redisResponse.headers.get('content-type');
                            if (contentType && contentType.includes('application/json')) {
                                status = await redisResponse.json();
                            } else {
                                const text = await redisResponse.text();
                                status.running = text.includes('running');
                            }
                        }
                        break;
                        
                    case 'crawler':
                        statusUrl = '/api/services/crawler/status';
                        const crawlerResponse = await fetch(statusUrl);
                        if (crawlerResponse.ok) {
                            const crawlerData = await crawlerResponse.json();
                            status = crawlerData;
                        }
                        break;
                        
                    case 'docker':
                        statusUrl = '/api/services/docker/status';
                        const dockerResponse = await fetch(statusUrl);
                        if (dockerResponse.ok) {
                            const text = await dockerResponse.text();
                            status.running = text.includes('running');
                        }
                        break;

                    case 'lifx':
                        statusUrl = '/api/services/lifx/status';
                        try {
                            const lifxResponse = await fetch(statusUrl);
                            if (lifxResponse.ok) {
                                const lifxData = await lifxResponse.json();
                                status.running = lifxData.running === true;
                                status.bulb_count = lifxData.bulb_count || 0;
                                status.message = lifxData.status || 'Unknown';
                            }
                        } catch (e) {
                            console.warn('LIFX status not available:', e);
                            status.running = false;
                        }
                        break;

                    default:
                        // Generic service check
                        status.running = false;
                }
                
                updateServiceStatus(service, status);
            } catch (error) {
                console.error(`Failed to fetch ${service} status:`, error);
                updateServiceStatus(service, { running: false });
            }
        }
        
        if (hasBackendError) {
            showBackendError();
        }
    } catch (error) {
        console.error('Failed to fetch service status:', error);
        addLog('Failed to fetch service status', 'error');
    }
}

// Update service status in UI
function updateServiceStatus(service, status) {
    console.log(`Updating service ${service} status:`, status);
    serviceStatuses[service] = status;
    
    const card = document.getElementById(`${service}-card`);
    if (!card) {
        console.warn(`No card found for service: ${service}`);
        return;
    }
    
    const statusIndicator = document.getElementById(`${service}-status`);
    const statusText = document.getElementById(`${service}-status-text`);
    const startBtn = document.getElementById(`${service}-start`);
    const stopBtn = document.getElementById(`${service}-stop`);
    
    if (statusIndicator) {
        statusIndicator.className = `status-indicator ${status.running ? 'running' : 'stopped'}`;
    }
    
    if (statusText) {
        statusText.textContent = status.running ? 'Running' : 'Stopped';
        statusText.style.color = status.running ? 'var(--success)' : 'var(--error)';
    }
    
    if (startBtn) {
        startBtn.disabled = status.running;
    }
    
    if (stopBtn) {
        stopBtn.disabled = !status.running;
    }
    
    // Update service-specific metrics
    updateServiceMetrics(service, status);
    
    // Update active services count
    updateActiveServicesCount();
}

// Update service-specific metrics
function updateServiceMetrics(service, status) {
    if (service === 'redis' && status.metrics) {
        updateElement('redis-connections', status.metrics.connections || '-');
        updateElement('redis-memory', formatBytes(status.metrics.memory) || '-');
        updateElement('redis-keys', status.metrics.keys || '-');
    } else if (service === 'crawler' && status.metrics) {
        updateElement('crawler-pages', status.metrics.pages_crawled || '0');
        updateElement('crawler-queue', status.metrics.queue_size || '0');
        updateElement('crawler-last-run', formatTime(status.metrics.last_run) || 'Never');
    } else if (service === 'postgres' && status.metrics) {
        updateElement('postgres-connections', status.metrics.connections || '-');
        updateElement('postgres-size', formatBytes(status.metrics.db_size) || '-');
        updateElement('postgres-version', status.metrics.version || '-');
    } else if (service === 'docker' && status.metrics) {
        updateElement('docker-containers', status.metrics.containers || '-');
        updateElement('docker-images', status.metrics.images || '-');
        updateElement('docker-version', status.metrics.version || '-');
    } else if (service === 'websocket' && status.metrics) {
        updateElement('websocket-connections', status.metrics.connections || '0');
        updateElement('websocket-messages', status.metrics.messages_per_sec || '0');
    } else if (service === 'voice' && status.metrics) {
        updateElement('voice-sessions', status.metrics.sessions || '0');
    } else if (service === 'ollama' && status.metrics) {
        updateElement('ollama-models', status.metrics.models_count || '0');
        updateElement('ollama-version', status.metrics.version || '-');
    } else if (service === 'lifx') {
        // LIFX lighting service
        updateElement('lifx-bulbs', status.bulb_count || '0');
        updateElement('lifx-status', status.message || (status.running ? 'Running' : 'Stopped'));
    }
}

// Update active services count
function updateActiveServicesCount() {
    const total = Object.keys(serviceStatuses).length || 6;
    const active = Object.values(serviceStatuses).filter(s => s.running).length;
    updateElement('active-services', `${active}/${total}`);
}

// Service control functions
async function startService(service) {
    showToast(`Starting ${service}...`, 'info');
    disableServiceButtons(service, true);
    
    try {
        // Use WebSocket if available
        if (ws && ws.readyState === WebSocket.OPEN) {
            const commandId = generateCommandId();
            const command = {
                type: 'command',
                id: commandId,
                command: 'start_service',
                args: { service: service }
            };
            console.log('Sending WebSocket command:', command);
            ws.send(JSON.stringify(command));
            
            // Wait for response via WebSocket (handled by handleCommandResponse)
            showToast(`${service} start command sent`, 'info');
            addLog(`Sent start command for ${service} service`, 'info');
        } else {
            // Fall back to HTTP
            let response;
            
            // Service-specific start endpoints
            switch(service) {
                case 'redis':
                    response = await fetch('/api/services/redis/start', { method: 'POST' });
                    break;
                case 'crawler':
                    response = await fetch('/api/services/crawler/start', { method: 'POST' });
                    break;
                case 'docker':
                    response = await fetch('/api/services/docker/start', { method: 'POST' });
                    break;
                default:
                    response = await fetch(`/api/services/${service}/start`, { method: 'POST' });
            }
            
            if (response && response.ok) {
                showToast(`${service} started successfully`, 'success');
                addLog(`Started ${service} service`, 'success');
            } else {
                throw new Error(`Failed to start ${service}`);
            }
        }
    } catch (error) {
        showToast(`Failed to start ${service}`, 'error');
        addLog(`Failed to start ${service}: ${error.message}`, 'error');
    } finally {
        disableServiceButtons(service, false);
        setTimeout(requestServiceStatus, 1000);
    }
}

async function stopService(service) {
    showToast(`Stopping ${service}...`, 'info');
    disableServiceButtons(service, true);
    
    try {
        // Use WebSocket if available
        if (ws && ws.readyState === WebSocket.OPEN) {
            const commandId = generateCommandId();
            const command = {
                type: 'command',
                id: commandId,
                command: 'stop_service',
                args: { service: service }
            };
            console.log('Sending WebSocket stop command:', command);
            ws.send(JSON.stringify(command));
            
            // Wait for response via WebSocket (handled by handleCommandResponse)
            showToast(`${service} stop command sent`, 'info');
            addLog(`Sent stop command for ${service} service`, 'info');
        } else {
            // Fall back to HTTP
            let response;
            
            // Service-specific stop endpoints
            switch(service) {
                case 'redis':
                    response = await fetch('/api/services/redis/stop', { method: 'POST' });
                    break;
                case 'crawler':
                    response = await fetch('/api/services/crawler/stop', { method: 'POST' });
                    break;
                case 'docker':
                    response = await fetch('/api/services/docker/stop', { method: 'POST' });
                    break;
                default:
                    response = await fetch(`/api/services/${service}/stop`, { method: 'POST' });
            }
            
            if (response && response.ok) {
                showToast(`${service} stopped successfully`, 'success');
                addLog(`Stopped ${service} service`, 'success');
            } else {
                throw new Error(`Failed to stop ${service}`);
            }
        }
    } catch (error) {
        showToast(`Failed to stop ${service}`, 'error');
        addLog(`Failed to stop ${service}: ${error.message}`, 'error');
    } finally {
        disableServiceButtons(service, false);
        setTimeout(requestServiceStatus, 1000);
    }
}

async function restartService(service) {
    showToast(`Restarting ${service}...`, 'info');
    disableServiceButtons(service, true);
    
    try {
        // Use WebSocket if available
        if (ws && ws.readyState === WebSocket.OPEN) {
            const commandId = generateCommandId();
            const command = {
                type: 'command',
                id: commandId,
                command: 'restart_service',
                args: { service: service }
            };
            console.log('Sending WebSocket restart command:', command);
            ws.send(JSON.stringify(command));
            
            // Wait for response via WebSocket (handled by handleCommandResponse)
            showToast(`${service} restart command sent`, 'info');
            addLog(`Sent restart command for ${service} service`, 'info');
        } else {
            // Fall back to HTTP (stop then start)
            await stopService(service);
            setTimeout(async () => {
                await startService(service);
            }, 2000);
        }
    } catch (error) {
        showToast(`Failed to restart ${service}`, 'error');
        addLog(`Failed to restart ${service}: ${error.message}`, 'error');
    } finally {
        disableServiceButtons(service, false);
        setTimeout(requestServiceStatus, 3000);
    }
}

// Disable/enable service buttons
function disableServiceButtons(service, disabled) {
    const buttons = [`${service}-start`, `${service}-stop`, `${service}-restart`];
    buttons.forEach(id => {
        const btn = document.getElementById(id);
        if (btn) btn.disabled = disabled;
    });
}

// Add log entry
function addLog(message, level = 'info') {
    const container = document.getElementById('log-container');
    const entry = document.createElement('div');
    entry.className = `log-entry ${level}`;
    
    // Convert newlines to HTML line breaks for proper display
    const formattedMessage = message.replace(/\n/g, '<br>');
    
    const timestamp = new Date().toLocaleTimeString();
    entry.innerHTML = `
        <span class="log-timestamp">${timestamp}</span>
        <span class="log-message">${formattedMessage}</span>
    `;
    
    container.appendChild(entry);
    container.scrollTop = container.scrollHeight;
    
    // Keep only last 100 entries
    while (container.children.length > 100) {
        container.removeChild(container.firstChild);
    }
}

// Clear logs
function clearLogs() {
    const container = document.getElementById('log-container');
    container.innerHTML = '';
    addLog('Logs cleared');
}

// Show toast notification
function showToast(message, type = 'info') {
    const toast = document.getElementById('toast');
    toast.textContent = message;
    toast.className = `toast ${type} show`;
    
    setTimeout(() => {
        toast.classList.remove('show');
    }, 3000);
}

// Update system metrics
async function updateSystemMetrics() {
    try {
        // Try to get system metrics from health endpoint
        const response = await fetch('/health/detailed');
        if (response.ok) {
            const contentType = response.headers.get('content-type');
            if (contentType && contentType.includes('application/json')) {
                const data = await response.json();
                if (data.metrics) {
                    updateMetrics(data.metrics);
                }
            } else {
                console.warn('Health endpoint returned non-JSON response');
            }
        } else if (response.status === 502) {
            console.error('Backend server is not running (502 Bad Gateway)');
            showBackendError();
        }
    } catch (error) {
        console.error('Failed to fetch system metrics:', error);
    }
}

// Update metrics in UI
function updateMetrics(data) {
    if (data.cpu_usage !== undefined) {
        updateElement('cpu-usage', `${Math.round(data.cpu_usage)}%`);
    }
    if (data.memory_usage !== undefined) {
        updateElement('memory-usage', `${Math.round(data.memory_usage)}%`);
    }
    updateUptime();
}

// Update uptime
function updateUptime() {
    const uptime = Date.now() - startTime;
    const days = Math.floor(uptime / (1000 * 60 * 60 * 24));
    const hours = Math.floor((uptime % (1000 * 60 * 60 * 24)) / (1000 * 60 * 60));
    const minutes = Math.floor((uptime % (1000 * 60 * 60)) / (1000 * 60));
    
    updateElement('uptime', `${days}d ${hours}h ${minutes}m`);
}

// Utility functions
function updateElement(id, value) {
    const element = document.getElementById(id);
    if (element) {
        element.textContent = value;
    }
}

function formatBytes(bytes) {
    if (!bytes || bytes === 0) return '0 B';
    const k = 1024;
    const sizes = ['B', 'KB', 'MB', 'GB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return Math.round(bytes / Math.pow(k, i) * 100) / 100 + ' ' + sizes[i];
}

function formatTime(timestamp) {
    if (!timestamp) return 'Never';
    const date = new Date(timestamp);
    const now = new Date();
    const diff = now - date;
    
    if (diff < 60000) return 'Just now';
    if (diff < 3600000) return `${Math.floor(diff / 60000)} min ago`;
    if (diff < 86400000) return `${Math.floor(diff / 3600000)} hours ago`;
    return date.toLocaleDateString();
}

// Start polling as fallback (slower on touch devices to save battery)
function startPolling() {
    const isTouchDevice = ('ontouchstart' in window) || (navigator.maxTouchPoints > 0);
    const pollInterval = isTouchDevice ? 10000 : 5000;
    setInterval(() => {
        fetchServiceStatus();
        updateSystemMetrics();
    }, pollInterval);
}

// Check environment mode
async function checkEnvironment() {
    try {
        const response = await fetch('/api/environment');
        if (response.ok) {
            const data = await response.json();
            if (data.is_caprover) {
                addLog('Running in CapRover mode - external services enabled', 'info');
                document.getElementById('system-status').textContent = 'CapRover Mode';
                document.getElementById('system-status').className = 'status-badge online';
                
                // Hide Docker controls in CapRover mode
                const dockerCard = document.getElementById('docker-card');
                if (dockerCard) {
                    dockerCard.style.display = 'none';
                }
            }
        }
    } catch (error) {
        console.log('Environment check failed, assuming standard mode');
    }
}

// Show backend error message
function showBackendError() {
    const systemStatus = document.getElementById('system-status');
    if (systemStatus) {
        systemStatus.textContent = 'Backend Offline';
        systemStatus.className = 'status-badge offline';
    }
    
    // Update all service cards to show offline
    const services = ['redis', 'crawler', 'postgres', 'docker', 'voice', 'websocket'];
    services.forEach(service => {
        updateServiceStatus(service, { 
            running: false, 
            status_text: 'Backend offline' 
        });
    });
    
    // Show error in log
    addLog('Backend server is not responding. Please check if SAM is running.', 'error');
}

// Initialize dashboard
function init() {
    addLog('Initializing SAM Control Center...', 'info');
    
    // Check environment first
    checkEnvironment();
    
    // Try WebSocket first (but don't let it block other operations)
    setTimeout(() => initWebSocket(), 100);
    
    // Initial data fetch
    fetchServiceStatus();
    updateSystemMetrics();
    
    // Update uptime continuously
    setInterval(updateUptime, 1000);

    // Only poll REST endpoints when WebSocket is not connected
    setInterval(() => {
        if (!ws || ws.readyState !== WebSocket.OPEN) {
            updateSystemMetrics();
            fetchServiceStatus();
        }
    }, 5000);
}

// Start when DOM is ready
if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', init);
} else {
    init();
}
