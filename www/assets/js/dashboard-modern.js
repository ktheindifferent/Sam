// ███████     █████     ███    ███    
// ██         ██   ██    ████  ████    
// ███████    ███████    ██ ████ ██    
//      ██    ██   ██    ██  ██  ██    
// ███████ ██ ██   ██ ██ ██      ██ ██ 
// Copyright 2021-2026 The Open Sam Foundation (OSF)
// Developed by Caleb Mitchell Smith (ktheindifferent, PixelCoda, p0indexter)
// Licensed under GPLv3....see LICENSE file.

/**
 * Enhanced S.A.M. Dashboard JavaScript - Modern Vanilla JS Version
 * Provides real-time monitoring, interactive controls, and comprehensive system overview
 * No jQuery dependencies
 */

class SAMDashboard {
    constructor() {
        this.updateInterval = 5000; // 5 seconds
        this.maxActivityItems = 50;
        this.activityData = [];
        this.chartData = {
            cpu: [],
            memory: [],
            disk: [],
            timestamps: []
        };
        this.charts = {};
        this.services = new Map();
        
        this.init();
    }

    init() {
        console.log('Initializing S.A.M. Dashboard (Modern Version)...');
        
        // Wait for DOM to be ready
        if (document.readyState === 'loading') {
            document.addEventListener('DOMContentLoaded', () => {
                this.setupDashboard();
            });
        } else {
            this.setupDashboard();
        }
    }

    setupDashboard() {
        this.setupEventListeners();
        this.createServiceCards();
        this.startRealTimeUpdates();
        this.loadInitialData();
        this.initializeCharts();
    }

    // Utility method to safely query elements
    $(selector, context = document) {
        const elements = context.querySelectorAll(selector);
        return elements.length === 1 ? elements[0] : elements;
    }

    // Utility method for AJAX requests
    async fetch(url, options = {}) {
        try {
            const response = await fetch(url, {
                headers: {
                    'Content-Type': 'application/json',
                    ...options.headers
                },
                ...options
            });
            
            if (!response.ok) {
                throw new Error(`HTTP error! status: ${response.status}`);
            }
            
            return await response.json();
        } catch (error) {
            console.error('Fetch error:', error);
            this.showNotification('Network error occurred', 'error');
            throw error;
        }
    }

    setupEventListeners() {
        // Sidebar navigation
        document.querySelectorAll('.nav-link').forEach(link => {
            link.addEventListener('click', (e) => {
                e.preventDefault();
                const target = e.currentTarget.getAttribute('href');
                this.navigateToSection(target);
            });
        });

        // Auto-refresh with Ctrl+R
        document.addEventListener('keydown', (e) => {
            if (e.key === 'r' && e.ctrlKey) {
                e.preventDefault();
                this.refreshAllData();
            }
        });

        // Service card interactions using event delegation
        document.addEventListener('click', (e) => {
            if (e.target.closest('.service-card')) {
                const card = e.target.closest('.service-card');
                const serviceId = card.dataset.serviceId;
                this.showServiceDetails(serviceId);
            }

            if (e.target.closest('.service-toggle-btn')) {
                const btn = e.target.closest('.service-toggle-btn');
                const serviceId = btn.dataset.serviceId;
                this.toggleService(serviceId);
            }

            if (e.target.closest('.service-logs-btn')) {
                const btn = e.target.closest('.service-logs-btn');
                const serviceId = btn.dataset.serviceId;
                this.viewServiceLogs(serviceId);
            }

            if (e.target.closest('.service-restart-btn')) {
                const btn = e.target.closest('.service-restart-btn');
                const serviceId = btn.dataset.serviceId;
                this.restartService(serviceId);
            }
        });

        // Responsive sidebar toggle
        const navbarToggler = document.querySelector('.navbar-toggler');
        if (navbarToggler) {
            navbarToggler.addEventListener('click', () => {
                const sidebar = document.querySelector('.sidebar');
                if (sidebar) {
                    sidebar.classList.toggle('show');
                }
            });
        }

        // Search functionality
        const searchInput = document.querySelector('#dashboard-search');
        if (searchInput) {
            searchInput.addEventListener('input', (e) => {
                this.filterServices(e.target.value);
            });
        }

        // Theme toggle
        const themeToggle = document.querySelector('#theme-toggle');
        if (themeToggle) {
            themeToggle.addEventListener('click', () => {
                this.toggleTheme();
            });
        }
    }

    createServiceCards() {
        const services = [
            { id: 'crawler', name: 'Web Crawler', icon: 'fas fa-spider', description: 'Content crawling and indexing' },
            { id: 'redis', name: 'Redis Cache', icon: 'fas fa-database', description: 'In-memory data structure store' },
            { id: 'postgres', name: 'PostgreSQL', icon: 'fas fa-server', description: 'Primary database server' },
            { id: 'docker', name: 'Docker', icon: 'fab fa-docker', description: 'Containerization platform' },
            { id: 'voice', name: 'Voice Services', icon: 'fas fa-microphone', description: 'STT/TTS processing' },
            { id: 'lifx', name: 'LIFX Control', icon: 'fas fa-lightbulb', description: 'Smart lighting management' },
            { id: 'p2p', name: 'P2P Network', icon: 'fas fa-network-wired', description: 'Peer-to-peer communication' },
            { id: 'security', name: 'Security', icon: 'fas fa-shield-alt', description: 'Security monitoring' }
        ];

        const serviceGrid = document.querySelector('.service-grid');
        if (!serviceGrid) return;

        serviceGrid.innerHTML = '';

        services.forEach(service => {
            this.services.set(service.id, service);
            
            const card = document.createElement('div');
            card.className = 'dashboard-card service-card p-3';
            card.dataset.serviceId = service.id;
            
            card.innerHTML = `
                <div class="d-flex align-items-center mb-2">
                    <i class="${service.icon} fa-2x text-primary mr-3"></i>
                    <div class="flex-grow-1">
                        <h6 class="mb-1">${service.name}</h6>
                        <small class="text-muted">${service.description}</small>
                    </div>
                    <div class="ml-auto">
                        <span class="status-indicator status-unknown" id="${service.id}-status"></span>
                    </div>
                </div>
                <div class="d-flex justify-content-between align-items-center">
                    <small class="text-muted" id="${service.id}-info">Checking status...</small>
                    <div class="btn-group btn-group-sm">
                        <button class="btn btn-outline-primary btn-sm service-toggle-btn" data-service-id="${service.id}" title="Toggle Service">
                            <i class="fas fa-power-off"></i>
                        </button>
                        <button class="btn btn-outline-secondary btn-sm service-logs-btn" data-service-id="${service.id}" title="View Logs">
                            <i class="fas fa-terminal"></i>
                        </button>
                        <button class="btn btn-outline-info btn-sm service-restart-btn" data-service-id="${service.id}" title="Restart Service">
                            <i class="fas fa-sync"></i>
                        </button>
                    </div>
                </div>
                <div class="mt-2">
                    <div class="progress" style="height: 4px;">
                        <div class="progress-bar" id="${service.id}-progress" role="progressbar" style="width: 0%"></div>
                    </div>
                </div>
            `;
            
            serviceGrid.appendChild(card);
        });

        // Add stats cards
        this.createStatsCards();
    }

    createStatsCards() {
        const statsContainer = document.querySelector('.stats-container');
        if (!statsContainer) return;

        const stats = [
            { id: 'cpu', label: 'CPU Usage', icon: 'fas fa-microchip', unit: '%' },
            { id: 'memory', label: 'Memory', icon: 'fas fa-memory', unit: 'GB' },
            { id: 'disk', label: 'Disk Space', icon: 'fas fa-hdd', unit: 'GB' },
            { id: 'network', label: 'Network', icon: 'fas fa-wifi', unit: 'Mbps' }
        ];

        statsContainer.innerHTML = '';

        stats.forEach(stat => {
            const card = document.createElement('div');
            card.className = 'col-md-3 mb-3';
            
            card.innerHTML = `
                <div class="dashboard-card stat-card p-3">
                    <div class="d-flex align-items-center">
                        <i class="${stat.icon} fa-2x text-info mr-3"></i>
                        <div class="flex-grow-1">
                            <small class="text-muted d-block">${stat.label}</small>
                            <h4 class="mb-0">
                                <span id="${stat.id}-value">--</span>
                                <small>${stat.unit}</small>
                            </h4>
                        </div>
                    </div>
                    <canvas id="${stat.id}-chart" height="50"></canvas>
                </div>
            `;
            
            statsContainer.appendChild(card);
        });
    }

    async startRealTimeUpdates() {
        // Initial update
        await this.updateDashboardData();
        
        // Set up periodic updates
        this.updateTimer = setInterval(() => {
            this.updateDashboardData();
        }, this.updateInterval);

        // Set up WebSocket for real-time events if available
        this.setupWebSocket();
    }

    setupWebSocket() {
        try {
            const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
            const wsUrl = `${protocol}//${window.location.host}/ws/dashboard`;
            
            this.ws = new WebSocket(wsUrl);
            
            this.ws.onopen = () => {
                console.log('WebSocket connected');
                this.showNotification('Real-time updates connected', 'success');
            };
            
            this.ws.onmessage = (event) => {
                const data = JSON.parse(event.data);
                this.handleRealtimeUpdate(data);
            };
            
            this.ws.onerror = (error) => {
                console.error('WebSocket error:', error);
            };
            
            this.ws.onclose = () => {
                console.log('WebSocket disconnected, falling back to polling');
                // Reconnect after 5 seconds
                setTimeout(() => this.setupWebSocket(), 5000);
            };
        } catch (error) {
            console.log('WebSocket not available, using polling only');
        }
    }

    handleRealtimeUpdate(data) {
        switch (data.type) {
            case 'service_status':
                this.updateServiceStatus(data.service, data.status);
                break;
            case 'system_stats':
                this.updateSystemStats(data.stats);
                break;
            case 'activity':
                this.addActivityItem(data.activity);
                break;
            case 'alert':
                this.showNotification(data.message, data.severity);
                break;
        }
    }

    async updateDashboardData() {
        try {
            // Fetch system stats
            const stats = await this.fetch('/api/system/stats');
            this.updateSystemStats(stats);
            
            // Fetch service statuses
            const services = await this.fetch('/api/services/status');
            this.updateAllServiceStatuses(services);
            
            // Fetch recent activity
            const activity = await this.fetch('/api/activity/recent');
            this.updateActivityFeed(activity);
            
        } catch (error) {
            console.error('Failed to update dashboard:', error);
        }
    }

    updateSystemStats(stats) {
        // Update CPU
        const cpuElement = document.getElementById('cpu-value');
        if (cpuElement) {
            cpuElement.textContent = stats.cpu?.toFixed(1) || '--';
        }
        
        // Update Memory
        const memoryElement = document.getElementById('memory-value');
        if (memoryElement) {
            const memoryGB = (stats.memory_used / 1024 / 1024 / 1024).toFixed(1);
            memoryElement.textContent = memoryGB;
        }
        
        // Update Disk
        const diskElement = document.getElementById('disk-value');
        if (diskElement) {
            const diskGB = (stats.disk_used / 1024 / 1024 / 1024).toFixed(1);
            diskElement.textContent = diskGB;
        }
        
        // Update Network
        const networkElement = document.getElementById('network-value');
        if (networkElement) {
            networkElement.textContent = stats.network_speed?.toFixed(1) || '--';
        }
        
        // Update charts
        this.updateCharts(stats);
    }

    updateAllServiceStatuses(services) {
        Object.entries(services).forEach(([serviceId, status]) => {
            this.updateServiceStatus(serviceId, status);
        });
    }

    updateServiceStatus(serviceId, status) {
        const statusElement = document.getElementById(`${serviceId}-status`);
        const infoElement = document.getElementById(`${serviceId}-info`);
        const progressElement = document.getElementById(`${serviceId}-progress`);
        
        if (statusElement) {
            statusElement.className = `status-indicator status-${status.state}`;
        }
        
        if (infoElement) {
            infoElement.textContent = status.message || `Status: ${status.state}`;
        }
        
        if (progressElement && status.progress !== undefined) {
            progressElement.style.width = `${status.progress}%`;
        }
    }

    updateActivityFeed(activities) {
        const activityFeed = document.querySelector('.activity-feed');
        if (!activityFeed) return;
        
        activityFeed.innerHTML = '';
        
        activities.slice(0, this.maxActivityItems).forEach(activity => {
            const item = document.createElement('div');
            item.className = 'activity-item p-2 border-bottom';
            
            const time = new Date(activity.timestamp).toLocaleTimeString();
            
            item.innerHTML = `
                <div class="d-flex justify-content-between">
                    <div>
                        <i class="${this.getActivityIcon(activity.type)} mr-2"></i>
                        <span>${activity.message}</span>
                    </div>
                    <small class="text-muted">${time}</small>
                </div>
            `;
            
            activityFeed.appendChild(item);
        });
    }

    getActivityIcon(type) {
        const icons = {
            'info': 'fas fa-info-circle text-info',
            'warning': 'fas fa-exclamation-triangle text-warning',
            'error': 'fas fa-times-circle text-danger',
            'success': 'fas fa-check-circle text-success',
            'system': 'fas fa-cog text-secondary'
        };
        return icons[type] || 'fas fa-circle text-muted';
    }

    initializeCharts() {
        // Initialize mini charts for stats if Chart.js is available
        if (typeof Chart !== 'undefined') {
            ['cpu', 'memory', 'disk', 'network'].forEach(stat => {
                const canvas = document.getElementById(`${stat}-chart`);
                if (canvas) {
                    const ctx = canvas.getContext('2d');
                    this.charts[stat] = new Chart(ctx, {
                        type: 'line',
                        data: {
                            labels: [],
                            datasets: [{
                                data: [],
                                borderColor: '#007bff',
                                borderWidth: 2,
                                fill: false,
                                tension: 0.4,
                                pointRadius: 0
                            }]
                        },
                        options: {
                            responsive: true,
                            maintainAspectRatio: false,
                            plugins: {
                                legend: { display: false }
                            },
                            scales: {
                                x: { display: false },
                                y: { display: false }
                            }
                        }
                    });
                }
            });
        }
    }

    updateCharts(stats) {
        const timestamp = new Date().toLocaleTimeString();
        
        // Keep only last 20 data points
        if (this.chartData.timestamps.length > 20) {
            this.chartData.timestamps.shift();
            this.chartData.cpu.shift();
            this.chartData.memory.shift();
            this.chartData.disk.shift();
        }
        
        this.chartData.timestamps.push(timestamp);
        this.chartData.cpu.push(stats.cpu || 0);
        this.chartData.memory.push(stats.memory_percent || 0);
        this.chartData.disk.push(stats.disk_percent || 0);
        
        // Update chart displays
        Object.keys(this.charts).forEach(key => {
            if (this.charts[key] && this.chartData[key]) {
                this.charts[key].data.labels = this.chartData.timestamps;
                this.charts[key].data.datasets[0].data = this.chartData[key];
                this.charts[key].update('none'); // Update without animation
            }
        });
    }

    async toggleService(serviceId) {
        try {
            const response = await this.fetch(`/api/services/${serviceId}/toggle`, {
                method: 'POST'
            });
            
            this.showNotification(`Service ${serviceId} toggled successfully`, 'success');
            this.updateServiceStatus(serviceId, response.status);
        } catch (error) {
            this.showNotification(`Failed to toggle service ${serviceId}`, 'error');
        }
    }

    async restartService(serviceId) {
        try {
            const response = await this.fetch(`/api/services/${serviceId}/restart`, {
                method: 'POST'
            });
            
            this.showNotification(`Service ${serviceId} restarting...`, 'info');
            this.updateServiceStatus(serviceId, response.status);
        } catch (error) {
            this.showNotification(`Failed to restart service ${serviceId}`, 'error');
        }
    }

    async viewServiceLogs(serviceId) {
        try {
            const logs = await this.fetch(`/api/services/${serviceId}/logs`);
            this.showLogsModal(serviceId, logs);
        } catch (error) {
            this.showNotification(`Failed to fetch logs for ${serviceId}`, 'error');
        }
    }

    showLogsModal(serviceId, logs) {
        // Create modal if it doesn't exist
        let modal = document.getElementById('logs-modal');
        if (!modal) {
            modal = document.createElement('div');
            modal.id = 'logs-modal';
            modal.className = 'modal fade';
            modal.innerHTML = `
                <div class="modal-dialog modal-lg">
                    <div class="modal-content">
                        <div class="modal-header">
                            <h5 class="modal-title">Service Logs</h5>
                            <button type="button" class="close" data-dismiss="modal">
                                <span>&times;</span>
                            </button>
                        </div>
                        <div class="modal-body">
                            <pre class="logs-content"></pre>
                        </div>
                    </div>
                </div>
            `;
            document.body.appendChild(modal);
        }
        
        const service = this.services.get(serviceId);
        modal.querySelector('.modal-title').textContent = `${service?.name || serviceId} Logs`;
        modal.querySelector('.logs-content').textContent = logs.join('\n');
        
        // Show modal (using Bootstrap if available, otherwise manual)
        if (typeof bootstrap !== 'undefined') {
            new bootstrap.Modal(modal).show();
        } else {
            modal.style.display = 'block';
            modal.classList.add('show');
        }
    }

    showServiceDetails(serviceId) {
        const service = this.services.get(serviceId);
        if (!service) return;
        
        // Navigate to service detail page or show detailed modal
        window.location.href = `/services/${serviceId}`;
    }

    filterServices(searchTerm) {
        const cards = document.querySelectorAll('.service-card');
        const term = searchTerm.toLowerCase();
        
        cards.forEach(card => {
            const serviceId = card.dataset.serviceId;
            const service = this.services.get(serviceId);
            
            if (service) {
                const matches = service.name.toLowerCase().includes(term) ||
                               service.description.toLowerCase().includes(term);
                
                card.style.display = matches ? 'block' : 'none';
            }
        });
    }

    navigateToSection(target) {
        // Hide all sections
        document.querySelectorAll('.dashboard-section').forEach(section => {
            section.style.display = 'none';
        });
        
        // Show target section
        const targetSection = document.querySelector(target);
        if (targetSection) {
            targetSection.style.display = 'block';
        }
        
        // Update active nav
        document.querySelectorAll('.nav-link').forEach(link => {
            link.classList.remove('active');
            if (link.getAttribute('href') === target) {
                link.classList.add('active');
            }
        });
    }

    toggleTheme() {
        const body = document.body;
        const currentTheme = body.dataset.theme || 'light';
        const newTheme = currentTheme === 'light' ? 'dark' : 'light';
        
        body.dataset.theme = newTheme;
        localStorage.setItem('dashboard-theme', newTheme);
        
        this.showNotification(`Switched to ${newTheme} theme`, 'info');
    }

    showNotification(message, type = 'info') {
        // Create notification element
        const notification = document.createElement('div');
        notification.className = `alert alert-${type} notification fade show`;
        notification.innerHTML = `
            ${message}
            <button type="button" class="close" data-dismiss="alert">
                <span>&times;</span>
            </button>
        `;
        
        // Add to notification container
        let container = document.getElementById('notification-container');
        if (!container) {
            container = document.createElement('div');
            container.id = 'notification-container';
            container.style.cssText = 'position: fixed; top: 20px; right: 20px; z-index: 9999;';
            document.body.appendChild(container);
        }
        
        container.appendChild(notification);
        
        // Auto-remove after 5 seconds
        setTimeout(() => {
            notification.classList.remove('show');
            setTimeout(() => notification.remove(), 300);
        }, 5000);
    }

    async loadInitialData() {
        try {
            // Load saved preferences
            const savedTheme = localStorage.getItem('dashboard-theme');
            if (savedTheme) {
                document.body.dataset.theme = savedTheme;
            }
            
            // Load dashboard configuration
            const config = await this.fetch('/api/dashboard/config');
            if (config.updateInterval) {
                this.updateInterval = config.updateInterval;
            }
            
            console.log('Dashboard initialized successfully');
        } catch (error) {
            console.error('Failed to load initial data:', error);
        }
    }

    refreshAllData() {
        this.showNotification('Refreshing all data...', 'info');
        this.updateDashboardData();
    }

    destroy() {
        // Clean up timers and connections
        if (this.updateTimer) {
            clearInterval(this.updateTimer);
        }
        
        if (this.ws) {
            this.ws.close();
        }
        
        // Remove event listeners
        document.removeEventListener('click', this.clickHandler);
        document.removeEventListener('keydown', this.keyHandler);
    }
}

// Initialize dashboard when ready
const dashboard = new SAMDashboard();

// Export for global access if needed
window.SAMDashboard = SAMDashboard;
window.dashboard = dashboard;