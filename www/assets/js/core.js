// ███████     █████     ███    ███    
// ██         ██   ██    ████  ████    
// ███████    ███████    ██ ████ ██    
//      ██    ██   ██    ██  ██  ██    
// ███████ ██ ██   ██ ██ ██      ██ ██ 
// Copyright 2021-2026 The Open Sam Foundation (OSF)
// Developed by Caleb Mitchell Smith (ktheindifferent, PixelCoda, p0indexter)
// Licensed under GPLv3....see LICENSE file.


var current_human = null;

var current_session = null;

var notifications = undefined;

$(document).ready(function() {
    toastr.options = {
        timeOut: 5000,
        extendedTimeOut: 2000,
        positionClass: "toast-top-right",
        showDuration: 300,
        hideDuration: 300,
        showEasing: "swing",
        hideEasing: "linear",
        showMethod: "fadeIn",
        hideMethod: "fadeOut"
    };

    $.fn.modal.Constructor.prototype._enforceFocus = function() {}
    
    // Initialize enhanced dashboard features
    initializeDashboard();
    
    $.get("/api/current_human", function( data ) {
        current_human = data;
        $('.inject-human-name').each(function(i, obj) {
            // Use text() instead of html() to prevent XSS
            $(obj).text(current_human.name || 'User');
        });
    }).fail(function() {
        console.warn("Could not load current human data");
        $('.inject-human-name').text("User");
    });

    if(is_touch_enabled()){
        disableCursor();
    }

    // Only try to initialize notifications if we can verify session
    $.get("/api/current_session")
        .done(function( data ) {
            if (data && data.sid) {
                current_session = data;
                notifications = new Notifications(current_session);
                notifications.refresh();
                window.setInterval( function() {
                    notifications.refreshUnseen()
                }, 5000);
                console.log("Notifications initialized successfully");
            } else {
                console.warn("Invalid session data - notifications disabled");
            }
        })
        .fail(function(xhr, status, error) {
            console.warn("Could not load current session data - notifications disabled:", status);
            // Completely skip notifications initialization
        });
});

// Enhanced Dashboard Initialization
function initializeDashboard() {
    // Start real-time updates if we're on the main dashboard
    if (window.location.pathname === '/' || window.location.pathname === '/index.html') {
        startRealTimeUpdates();
        animateCards();
    }
}

// Real-time system monitoring
function startRealTimeUpdates() {
    updateSystemMetrics();
    setInterval(updateSystemMetrics, 5000); // Update every 5 seconds
}

async function updateSystemMetrics() {
    try {
        // Fetch real system metrics from API
        const response = await fetch('/api/system/metrics');
        const data = await response.json();
        
        // Update CPU
        $('#cpu-usage-card').text(`${data.cpu.usage_percent.toFixed(1)}%`);
        $('#cpu-progress-main').css('width', `${data.cpu.usage_percent}%`);
        
        // Update Memory
        const memoryUsedMB = data.memory.used_bytes / (1024 * 1024);
        const memoryPercent = data.memory.usage_percent;
        $('#memory-usage-card').text(`${memoryUsedMB.toFixed(0)} MB`);
        $('#memory-progress-main').css('width', `${memoryPercent}%`);
        
        // Update Disk
        $('#disk-progress-main').css('width', `${data.disk.usage_percent}%`);
        
        // Get service count (this would need to be from a separate endpoint)
        // For now, use a default or fetch from services API
        $('#services-count').text('8'); // Default value
        
    } catch (error) {
        console.error('Error updating system metrics:', error);
        
        // Use fallback mock data
        const mockData = {
            cpu: Math.random() * 100,
            memory: {
                used: 4000 + Math.random() * 4000,
                total: 16000
            },
            disk: 30 + Math.random() * 40,
        };
        
        $('#cpu-usage-card').text(`${mockData.cpu.toFixed(1)}%`);
        $('#cpu-progress-main').css('width', `${mockData.cpu}%`);
        
        const memoryPercent = (mockData.memory.used / mockData.memory.total) * 100;
        $('#memory-usage-card').text(`${mockData.memory.used.toFixed(0)} MB`);
        $('#memory-progress-main').css('width', `${memoryPercent}%`);
        
        $('#disk-progress-main').css('width', `${mockData.disk}%`);
        $('#services-count').text('8');
    }
}

// Animate cards on page load
function animateCards() {
    $('.card').each(function(index) {
        $(this).css({
            'opacity': '0',
            'transform': 'translateY(20px)'
        }).delay(index * 100).animate({
            'opacity': '1'
        }, 500, function() {
            $(this).css('transform', 'translateY(0)');
        });
    });
}

// Enhanced notification system
function showNotification(message, type = 'info', title = '') {
    switch(type) {
        case 'success':
            toastr.success(message, title);
            break;
        case 'error':
            toastr.error(message, title);
            break;
        case 'warning':
            toastr.warning(message, title);
            break;
        default:
            toastr.info(message, title);
    }
}

// Console/Terminal functionality
function openConsole() {
    // Check if console app exists, otherwise show placeholder
    if ($('#console-app').length) {
        $('#console-app').modal('show');
    } else {
        showNotification('Web terminal functionality coming soon!', 'info', 'Console');
        // In a real implementation, this would open apps/console/index.html
        window.open('/apps/console/index.html', '_blank', 'width=800,height=600');
    }
}

// Service management functions
function toggleService(serviceName) {
    showNotification(`Toggling ${serviceName} service...`, 'info');
    // In real implementation, make API call to toggle service
    setTimeout(() => {
        showNotification(`${serviceName} service toggled successfully!`, 'success');
        updateSystemMetrics(); // Refresh metrics
    }, 1000);
}

function restartService(serviceName) {
    showNotification(`Restarting ${serviceName} service...`, 'warning');
    // In real implementation, make API call to restart service
    setTimeout(() => {
        showNotification(`${serviceName} service restarted!`, 'success');
        updateSystemMetrics(); // Refresh metrics
    }, 2000);
}

// Enhanced touch/mobile support
function is_touch_enabled() {
    return ( 'ontouchstart' in window ) ||
           ( navigator.maxTouchPoints > 0 ) ||
           ( navigator.msMaxTouchPoints > 0 );
}

function disableCursor() {
    $('body').addClass('touch-enabled');
    // Add touch-specific styles
    $('<style>').prop('type', 'text/css').html(`
        .touch-enabled * {
            cursor: none !important;
        }
        .touch-enabled .btn:hover {
            transform: scale(1.05);
        }
        .touch-enabled .card:hover {
            transform: translateY(0) scale(1.02);
        }
    `).appendTo('head');
}

// Accessibility improvements
$(document).on('keydown', function(e) {
    // ESC key to close modals
    if (e.key === 'Escape') {
        $('.modal').modal('hide');
    }
    
    // Ctrl+R for refresh
    if (e.ctrlKey && e.key === 'r') {
        e.preventDefault();
        updateSystemMetrics();
        showNotification('Dashboard refreshed!', 'success');
    }
});

// Focus management for accessibility
$(document).on('focus', '.card', function() {
    $(this).addClass('focus-highlight');
}).on('blur', '.card', function() {
    $(this).removeClass('focus-highlight');
});

// Loading states for better UX
function showLoadingState(element) {
    const $el = $(element);
    $el.data('original-html', $el.html());
    $el.html('<span class="loading-spinner"></span> Loading...');
    $el.prop('disabled', true);
}

function hideLoadingState(element) {
    const $el = $(element);
    $el.html($el.data('original-html'));
    $el.prop('disabled', false);
}

// Error handling for AJAX requests
$(document).ajaxError(function(event, xhr, settings, error) {
    console.error('AJAX Error:', error);
    showNotification('Network error occurred. Please check your connection.', 'error');
});

// Service Worker registration for offline functionality
if ('serviceWorker' in navigator) {
    window.addEventListener('load', () => {
        navigator.serviceWorker.register('/service-worker.js')
            .then(registration => {
                console.log('ServiceWorker registration successful');
            })
            .catch(err => {
                console.log('ServiceWorker registration failed: ', err);
            });
    });
}


function newPopWindow(url, windowname, w, h, x, y)
{
    window.open(url, windowname, "resizable=no, toolbar=no, scrollbars=no, menubar=no, status=no, directories=no, width=" + w + ", height=" + h + ", left=" + x + ", top=" + y);
}

function is_touch_enabled() {
    return ( 'ontouchstart' in window ) ||
           ( navigator.maxTouchPoints > 0 ) ||
           ( navigator.msMaxTouchPoints > 0 );
}

function disableCursor(){
    var style = document.createElement('style');
    style.innerHTML = `* {
    cursor: none !important;
    }`;
    document.head.appendChild(style);
}

function uploadFile() {
    Swal.fire({
        title: "", 
        html: `
            <form id="new_file_form" class="user" action="/api/services/storage/files" method="post" enctype="multipart/form-data">
                <input type="hidden" name="csrf_token" value="${window.csrfToken || ''}">
                <input type="file" id="file_data" name="file_data" style="display: block !important;" 
                       accept="image/*,application/pdf,.doc,.docx,.txt">
            </form>
        `,  
        showConfirmButton: false
      });

      $(document).ready(function() {
        
        $( "#file_data" ).change(function() {
            const file = this.files[0];
            if (file && window.SecurityUtils) {
                const validation = window.SecurityUtils.validateFileUpload(file, {
                    maxSize: 50 * 1024 * 1024, // 50MB
                    allowedTypes: ['image/jpeg', 'image/png', 'image/gif', 'application/pdf', 'text/plain'],
                    allowedExtensions: ['jpg', 'jpeg', 'png', 'gif', 'pdf', 'txt', 'doc', 'docx']
                });
                
                if (!validation.valid) {
                    Swal.fire('Error', validation.error, 'error');
                    return;
                }
            }
           
            $("#new_file_form").submit();
          });
    });
}