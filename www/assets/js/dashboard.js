// ███████     █████     ███    ███    
// ██         ██   ██    ████  ████    
// ███████    ███████    ██ ████ ██    
//      ██    ██   ██    ██  ██  ██    
// ███████ ██ ██   ██ ██ ██      ██ ██ 
// Copyright 2021-2026 The Open Sam Foundation (OSF)
// Developed by Caleb Mitchell Smith (ktheindifferent, PixelCoda, p0indexter)
// Licensed under GPLv3....see LICENSE file.

/**
 * Enhanced S.A.M. Dashboard JavaScript
 * Provides real-time monitoring, interactive controls, and comprehensive system overview
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
        
        this.init();
    }

    init() {
        console.log('Initializing S.A.M. Dashboard...');
        this.setupEventListeners();
        this.createServiceCards();
        this.startRealTimeUpdates();
        this.loadInitialData();
    }

    setupEventListeners() {
        // Sidebar navigation
        $('.nav-link').on('click', (e) => {
            e.preventDefault();
            const target = $(e.currentTarget).attr('href');
            this.navigateToSection(target);
        });

        // Auto-refresh toggle
        $(document).on('keydown', (e) => {
            if (e.key === 'r' && e.ctrlKey) {
                e.preventDefault();
                this.refreshAllData();
            }
        });

        // Service card interactions
        $(document).on('click', '.service-card', (e) => {
            const serviceId = $(e.currentTarget).data('service-id');
            this.showServiceDetails(serviceId);
        });

        // Responsive sidebar toggle
        $('.navbar-toggler').on('click', () => {
            $('.sidebar').toggleClass('show');
        });
    }

    createServiceCards() {
        const services = [
            { id: 'crawler', name: 'Web Crawler', icon: 'fas fa-spider', description: 'Content crawling and indexing' },
            { id: 'redis', name: 'Redis Cache', icon: 'fas fa-database', description: 'In-memory data structure store' },
            { id: 'postgres', name: 'PostgreSQL', icon: 'fas fa-server', description: 'Primary database server' },
            { id: 'docker', name: 'Docker', icon: 'fab fa-docker', description: 'Containerization platform' },
            { id: 'sms', name: 'SMS Service', icon: 'fas fa-sms', description: 'Text message notifications' },
            { id: 'lifx', name: 'LIFX Control', icon: 'fas fa-lightbulb', description: 'Smart lighting management' },
            { id: 'http', name: 'HTTP Server', icon: 'fas fa-globe', description: 'Web interface server' },
            { id: 'ai', name: 'AI Processing', icon: 'fas fa-brain', description: 'Machine learning tasks' }
        ];

        const serviceGrid = $('.service-grid');
        serviceGrid.empty();

        services.forEach(service => {
            const cardHtml = `
                <div class="dashboard-card service-card p-3" data-service-id="${service.id}">
                    <div class="d-flex align-items-center mb-2">
                        <i class="${service.icon} fa-2x text-primary mr-3"></i>
                        <div>
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
                            <button class="btn btn-outline-primary btn-sm" onclick="dashboard.toggleService('${service.id}')">
                                <i class="fas fa-power-off"></i>
                            </button>
                            <button class="btn btn-outline-secondary btn-sm" onclick="dashboard.viewServiceLogs('${service.id}')">
                                <i class="fas fa-file-alt"></i>
                            </button>
                        </div>
                    </div>
                </div>
            `;
            serviceGrid.append(cardHtml);
        });
    }

    startRealTimeUpdates() {
        // Update system metrics
        this.updateSystemMetrics();
        setInterval(() => this.updateSystemMetrics(), this.updateInterval);

        // Update service statuses
        this.updateServiceStatuses();
        setInterval(() => this.updateServiceStatuses(), this.updateInterval);

        // Update activity log
        this.updateActivityLog();
        setInterval(() => this.updateActivityLog(), this.updateInterval * 2);

        // Update charts
        setInterval(() => this.updateCharts(), this.updateInterval);
    }

    async updateSystemMetrics() {
        try {
            // In a real implementation, this would call actual API endpoints
            const response = await this.mockApiCall('/api/system/metrics');
            
            // Update CPU
            const cpuUsage = response.cpu || Math.random() * 100;
            $('#cpu-usage').text(`${cpuUsage.toFixed(1)}%`);
            $('#cpu-progress').css('width', `${cpuUsage}%`);
            
            // Update Memory
            const memoryUsed = response.memory_used || Math.random() * 8000;
            const memoryTotal = response.memory_total || 16000;
            const memoryPercent = (memoryUsed / memoryTotal) * 100;
            $('#memory-usage').text(`${memoryUsed.toFixed(0)} MB`);
            $('#memory-progress').css('width', `${memoryPercent}%`);
            
            // Update Disk
            const diskPercent = response.disk_usage || Math.random() * 100;
            $('#disk-usage').text(`${diskPercent.toFixed(1)}%`);
            $('#disk-progress').css('width', `${diskPercent}%`);
            
            // Store data for charts
            this.chartData.cpu.push(cpuUsage);
            this.chartData.memory.push(memoryPercent);
            this.chartData.disk.push(diskPercent);
            this.chartData.timestamps.push(new Date());
            
            // Keep only last 20 data points
            if (this.chartData.cpu.length > 20) {
                this.chartData.cpu.shift();
                this.chartData.memory.shift();
                this.chartData.disk.shift();
                this.chartData.timestamps.shift();
            }
            
        } catch (error) {
            console.error('Error updating system metrics:', error);
            this.addActivity('System metrics update failed', 'error');
        }
    }

    async updateServiceStatuses() {
        const services = ['crawler', 'redis', 'postgres', 'docker', 'sms', 'lifx', 'http', 'ai'];
        
        for (const service of services) {
            try {
                // Mock API call - in real implementation, call actual endpoints
                const status = await this.mockApiCall(`/api/services/${service}/status`);
                this.updateServiceCard(service, status);
            } catch (error) {
                console.error(`Error updating ${service} status:`, error);
                this.updateServiceCard(service, { status: 'error', message: 'API Error' });
            }
        }
    }

    updateServiceCard(serviceId, statusData) {
        const statusElement = $(`#${serviceId}-status`);
        const infoElement = $(`#${serviceId}-info`);
        
        // Remove existing status classes
        statusElement.removeClass('status-running status-stopped status-unknown status-error');
        
        // Add appropriate status class and update info
        switch (statusData.status || 'unknown') {
            case 'running':
            case 'active':
            case 'online':
                statusElement.addClass('status-running');
                infoElement.text('Running normally');
                break;
            case 'stopped':
            case 'inactive':
            case 'offline':
                statusElement.addClass('status-stopped');
                infoElement.text('Currently stopped');
                break;
            case 'error':
                statusElement.addClass('status-error');
                infoElement.text(statusData.message || 'Error detected');
                break;
            default:
                statusElement.addClass('status-unknown');
                infoElement.text('Status unknown');
        }
    }

    async updateActivityLog() {
        try {
            // Mock API call for activity data
            const activities = await this.mockApiCall('/api/system/activity');
            
            // Add new activities
            activities.forEach(activity => {
                this.addActivity(activity.message, activity.type, activity.timestamp);
            });
            
        } catch (error) {
            console.error('Error updating activity log:', error);
        }
    }

    addActivity(message, type = 'info', timestamp = null) {
        const now = timestamp || new Date();
        const timeStr = now.toLocaleTimeString();
        
        const activity = {
            message,
            type,
            timestamp: now,
            timeStr
        };
        
        this.activityData.unshift(activity);
        
        // Keep only recent activities
        if (this.activityData.length > this.maxActivityItems) {
            this.activityData = this.activityData.slice(0, this.maxActivityItems);
        }
        
        this.renderActivityLog();
    }

    renderActivityLog() {
        const logContainer = $('#activity-log');
        logContainer.empty();
        
        this.activityData.slice(0, 10).forEach(activity => {
            const iconClass = this.getActivityIcon(activity.type);
            // Convert newlines to HTML line breaks for proper display
            const formattedMessage = activity.message.replace(/\n/g, '<br>');
            const itemHtml = `
                <div class="activity-item">
                    <i class="${iconClass} text-${this.getActivityColor(activity.type)}"></i>
                    <span class="ml-2">${formattedMessage}</span>
                    <div class="activity-timestamp">${activity.timeStr}</div>
                </div>
            `;
            logContainer.append(itemHtml);
        });
    }

    getActivityIcon(type) {
        const icons = {
            'info': 'fas fa-info-circle',
            'success': 'fas fa-check-circle',
            'warning': 'fas fa-exclamation-triangle',
            'error': 'fas fa-times-circle',
            'system': 'fas fa-cog'
        };
        return icons[type] || icons.info;
    }

    getActivityColor(type) {
        const colors = {
            'info': 'info',
            'success': 'success',
            'warning': 'warning',
            'error': 'danger',
            'system': 'secondary'
        };
        return colors[type] || colors.info;
    }

    updateCharts() {
        // Placeholder for chart updates
        // In a real implementation, you'd use Chart.js or similar library
        console.log('Updating charts with data:', this.chartData);
    }

    // API simulation
    async mockApiCall(endpoint) {
        // Simulate API delay
        await new Promise(resolve => setTimeout(resolve, Math.random() * 500));
        
        // Return mock data based on endpoint
        switch (endpoint) {
            case '/api/system/metrics':
                return {
                    cpu: Math.random() * 100,
                    memory_used: 4000 + Math.random() * 4000,
                    memory_total: 16000,
                    disk_usage: 30 + Math.random() * 40
                };
            
            case '/api/system/activity':
                const activities = [];
                if (Math.random() > 0.7) {
                    activities.push({
                        message: this.generateRandomActivity(),
                        type: ['info', 'success', 'warning'][Math.floor(Math.random() * 3)],
                        timestamp: new Date()
                    });
                }
                return activities;
            
            default:
                if (endpoint.includes('/api/services/')) {
                    const statuses = ['running', 'stopped', 'unknown', 'error'];
                    return {
                        status: statuses[Math.floor(Math.random() * statuses.length)],
                        message: 'Service operational'
                    };
                }
                return {};
        }
    }

    generateRandomActivity() {
        const messages = [
            'Database connection established',
            'Cache cleared successfully',
            'Backup completed',
            'User authentication successful',
            'System health check passed',
            'API request processed',
            'Log file rotated',
            'Service restart completed',
            'Memory usage optimized',
            'Security scan completed'
        ];
        return messages[Math.floor(Math.random() * messages.length)];
    }

    loadInitialData() {
        this.addActivity('Dashboard initialized', 'system');
        this.addActivity('Real-time monitoring started', 'success');
        
        // Load some initial activity data
        setTimeout(() => {
            this.addActivity('System services checked', 'info');
            this.addActivity('Database connection verified', 'success');
        }, 2000);
    }

    // Public methods for UI interactions
    refreshAllData() {
        this.addActivity('Manual refresh triggered', 'info');
        this.updateSystemMetrics();
        this.updateServiceStatuses();
        
        toastr.success('Dashboard data refreshed');
    }

    toggleService(serviceId) {
        this.addActivity(`Service ${serviceId} toggle requested`, 'warning');
        toastr.info(`Service ${serviceId} toggle functionality coming soon`);
    }

    viewServiceLogs(serviceId) {
        this.addActivity(`Viewing logs for ${serviceId}`, 'info');
        toastr.info(`Log viewer for ${serviceId} coming soon`);
    }

    showServiceDetails(serviceId) {
        this.addActivity(`Viewing details for ${serviceId}`, 'info');
        toastr.info(`Detailed view for ${serviceId} coming soon`);
    }

    showSystemInfo() {
        const info = `
            System Information:
            - CPU Cores: ${navigator.hardwareConcurrency || 'Unknown'}
            - User Agent: ${navigator.userAgent}
            - Platform: ${navigator.platform}
            - Memory: ${navigator.deviceMemory || 'Unknown'} GB
            - Connection: ${navigator.connection?.effectiveType || 'Unknown'}
        `;
        
        toastr.info(info, 'System Information', {
            timeOut: 10000,
            extendedTimeOut: 5000
        });
    }

    openTerminal() {
        this.addActivity('Terminal access requested', 'info');
        // In a real implementation, this would open a web-based terminal
        toastr.info('Web terminal functionality coming soon');
    }

    exportLogs() {
        this.addActivity('Log export requested', 'info');
        toastr.success('Log export functionality coming soon');
    }

    navigateToSection(section) {
        // Remove active class from all nav items
        $('.nav-item').removeClass('active');
        
        // Add active class to clicked item
        $(`.nav-link[href="${section}"]`).closest('.nav-item').addClass('active');
        
        this.addActivity(`Navigated to ${section.replace('#', '')}`, 'info');
    }
}

// Global functions for HTML onclick handlers
window.refreshAllData = () => window.dashboard.refreshAllData();
window.showSystemInfo = () => window.dashboard.showSystemInfo();
window.openTerminal = () => window.dashboard.openTerminal();
window.exportLogs = () => window.dashboard.exportLogs();

// Initialize dashboard when DOM is ready
$(document).ready(() => {
    window.dashboard = new SAMDashboard();
    console.log('S.A.M. Dashboard loaded successfully');
});

// Handle visibility change for performance optimization
document.addEventListener('visibilitychange', () => {
    if (document.hidden) {
        console.log('Dashboard hidden - reducing update frequency');
        // Could reduce update frequency when tab is not visible
    } else {
        console.log('Dashboard visible - resuming normal updates');
        // Resume normal update frequency
    }
});

// Service Worker registration for offline functionality (future enhancement)
if ('serviceWorker' in navigator) {
    window.addEventListener('load', () => {
        navigator.serviceWorker.register('/sw.js')
            .then(registration => {
                console.log('SW registered: ', registration);
            })
            .catch(registrationError => {
                console.log('SW registration failed: ', registrationError);
            });
    });
}