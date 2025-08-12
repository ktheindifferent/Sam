// ███████     █████     ███    ███    
// ██         ██   ██    ████  ████    
// ███████    ███████    ██ ████ ██    
//      ██    ██   ██    ██  ██  ██    
// ███████ ██ ██   ██ ██ ██      ██ ██ 
// Copyright 2021-2026 The Open Sam Foundation (OSF)
// Developed by Caleb Mitchell Smith (ktheindifferent, PixelCoda, p0indexter)
// Licensed under GPLv3....see LICENSE file.

// Security utility functions for input sanitization and validation

// HTML entity encoding for safe output
function escapeHtml(str) {
    if (typeof str !== 'string') return '';
    const div = document.createElement('div');
    div.textContent = str;
    return div.innerHTML;
}

// JavaScript context encoding
function escapeJs(str) {
    if (typeof str !== 'string') return '';
    return str
        .replace(/\\/g, '\\\\')
        .replace(/'/g, "\\'")
        .replace(/"/g, '\\"')
        .replace(/\n/g, '\\n')
        .replace(/\r/g, '\\r')
        .replace(/\t/g, '\\t')
        .replace(/\//g, '\\/');
}

// URL encoding
function escapeUrl(str) {
    if (typeof str !== 'string') return '';
    return encodeURIComponent(str);
}

// CSS context encoding
function escapeCss(str) {
    if (typeof str !== 'string') return '';
    return str.replace(/[<>"'&]/g, function(match) {
        return '\\' + match.charCodeAt(0).toString(16) + ' ';
    });
}

// Validate and sanitize user input based on expected type
function sanitizeInput(input, type = 'text') {
    if (input === null || input === undefined) return '';
    
    // Convert to string
    input = String(input);
    
    switch(type) {
        case 'text':
            // Remove any HTML tags and encode special characters
            return escapeHtml(input.replace(/<[^>]*>/g, ''));
            
        case 'name':
            // Allow only alphanumeric, spaces, hyphens, apostrophes
            return input.replace(/[^a-zA-Z0-9\s\-']/g, '').slice(0, 100);
            
        case 'email':
            // Basic email validation and sanitization
            const emailRegex = /^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$/;
            const cleaned = input.toLowerCase().trim().slice(0, 254);
            return emailRegex.test(cleaned) ? cleaned : '';
            
        case 'url':
            // Validate URL and prevent javascript: protocol
            try {
                const url = new URL(input);
                if (!['http:', 'https:'].includes(url.protocol)) {
                    return '';
                }
                return url.href;
            } catch {
                return '';
            }
            
        case 'number':
            // Allow only numbers
            const num = parseFloat(input);
            return isNaN(num) ? 0 : num;
            
        case 'alphanumeric':
            // Allow only letters and numbers
            return input.replace(/[^a-zA-Z0-9]/g, '').slice(0, 100);
            
        case 'filename':
            // Sanitize filename to prevent path traversal
            return input
                .replace(/[^a-zA-Z0-9._-]/g, '')
                .replace(/\.{2,}/g, '.')
                .slice(0, 255);
                
        default:
            return escapeHtml(input);
    }
}

// Validate file upload
function validateFileUpload(file, options = {}) {
    const defaults = {
        maxSize: 10 * 1024 * 1024, // 10MB
        allowedTypes: ['image/jpeg', 'image/png', 'image/gif', 'application/pdf'],
        allowedExtensions: ['jpg', 'jpeg', 'png', 'gif', 'pdf']
    };
    
    const settings = Object.assign({}, defaults, options);
    
    // Check file size
    if (file.size > settings.maxSize) {
        return {
            valid: false,
            error: `File size exceeds maximum allowed size of ${settings.maxSize / (1024 * 1024)}MB`
        };
    }
    
    // Check MIME type
    if (!settings.allowedTypes.includes(file.type)) {
        return {
            valid: false,
            error: 'File type not allowed'
        };
    }
    
    // Check file extension
    const extension = file.name.split('.').pop().toLowerCase();
    if (!settings.allowedExtensions.includes(extension)) {
        return {
            valid: false,
            error: 'File extension not allowed'
        };
    }
    
    // Sanitize filename
    const sanitizedName = sanitizeInput(file.name, 'filename');
    
    return {
        valid: true,
        sanitizedName: sanitizedName
    };
}

// Create safe DOM element with sanitized content
function createSafeElement(tag, content, attributes = {}) {
    const element = document.createElement(tag);
    
    // Set text content (automatically escaped)
    if (content) {
        element.textContent = content;
    }
    
    // Set attributes safely
    for (const [key, value] of Object.entries(attributes)) {
        // Skip dangerous attributes
        if (['onclick', 'onload', 'onerror', 'onmouseover'].includes(key.toLowerCase())) {
            continue;
        }
        
        // Sanitize attribute value
        if (key === 'href' || key === 'src') {
            const sanitized = sanitizeInput(value, 'url');
            if (sanitized) {
                element.setAttribute(key, sanitized);
            }
        } else {
            element.setAttribute(key, escapeHtml(value));
        }
    }
    
    return element;
}

// Safe jQuery plugin for setting HTML content
if (typeof jQuery !== 'undefined') {
    jQuery.fn.safeHtml = function(content) {
        return this.each(function() {
            // Use text() instead of html() for safety
            jQuery(this).text(content);
        });
    };
    
    jQuery.fn.safeName = function(content) {
        return this.each(function() {
            const sanitized = sanitizeInput(content, 'name');
            jQuery(this).text(sanitized);
        });
    };
}

// Content Security Policy nonce generator (for server-side implementation)
function generateCSPNonce() {
    const array = new Uint8Array(16);
    crypto.getRandomValues(array);
    return btoa(String.fromCharCode.apply(null, array));
}

// Validate JSON response from API
function validateApiResponse(response) {
    // Check if response is valid JSON
    try {
        if (typeof response === 'string') {
            response = JSON.parse(response);
        }
        
        // Recursively sanitize string values in response
        function sanitizeObject(obj) {
            if (typeof obj === 'string') {
                return escapeHtml(obj);
            } else if (Array.isArray(obj)) {
                return obj.map(sanitizeObject);
            } else if (obj !== null && typeof obj === 'object') {
                const sanitized = {};
                for (const [key, value] of Object.entries(obj)) {
                    sanitized[key] = sanitizeObject(value);
                }
                return sanitized;
            }
            return obj;
        }
        
        return sanitizeObject(response);
    } catch (error) {
        console.error('Invalid API response:', error);
        return null;
    }
}

// Export functions for use in other scripts
window.SecurityUtils = {
    escapeHtml,
    escapeJs,
    escapeUrl,
    escapeCss,
    sanitizeInput,
    validateFileUpload,
    createSafeElement,
    generateCSPNonce,
    validateApiResponse
};