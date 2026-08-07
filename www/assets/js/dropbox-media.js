// ███████     █████     ███    ███
// ██         ██   ██    ████  ████
// ███████    ███████    ██ ████ ██
//      ██    ██   ██    ██  ██  ██
// ███████ ██ ██   ██ ██ ██      ██ ██
// Copyright 2021-2026 The Open Sam Foundation (OSF)
// Developed by Caleb Mitchell Smith (ktheindifferent, PixelCoda, p0indexter)
// Licensed under GPLv3....see LICENSE file.

// Dropbox Media Integration
let dropboxMediaCredentials = null;
let dropboxMediaFiles = [];
let dropboxCurrentFilter = 'all';
let dropboxCurrentMediaFile = null;

// Initialize WebSocket connection for Dropbox commands
let dropboxWsConnection = null;
const dropboxWsPendingCallbacks = {};
let dropboxWsInitRetries = 0;
const dropboxWsMaxInitRetries = 20;

function normalizeDropboxCommandResponse(message) {
    if (typeof normalizeStorageCommandResponse === 'function') {
        return normalizeStorageCommandResponse(message);
    }
    const payload = message && typeof message.data === 'object' && message.data !== null
        ? message.data
        : {};
    const normalized = Object.assign({}, payload);
    normalized.id = message.id;
    normalized.envelopeSuccess = message.success;
    if (typeof normalized.success !== 'boolean') {
        normalized.success = !!message.success;
    }
    normalized.data = payload.data || payload;
    return normalized;
}

function initializeDropboxMediaWS() {
    if (!window.ws) {
        if (dropboxWsInitRetries < dropboxWsMaxInitRetries) {
            dropboxWsInitRetries++;
            setTimeout(initializeDropboxMediaWS, 250);
        } else {
            console.error('WebSocket connection not available for Dropbox media');
        }
        return;
    }

    dropboxWsConnection = window.ws;
    dropboxWsInitRetries = 0;

    if (dropboxWsConnection._samStorageDispatcherInstalled) {
        return;
    }
    dropboxWsConnection._samStorageDispatcherInstalled = true;

    // Add a listener that dispatches by message ID
    const existingHandler = dropboxWsConnection.onmessage;
    dropboxWsConnection.onmessage = function(event) {
        let data;
        try {
            data = JSON.parse(event.data);
        } catch (e) {
            console.error('Malformed WebSocket message:', e);
            return;
        }
        if (data.type === 'command_response' && dropboxWsPendingCallbacks[data.id]) {
            const cb = dropboxWsPendingCallbacks[data.id];
            delete dropboxWsPendingCallbacks[data.id];
            cb(normalizeDropboxCommandResponse(data));
            return;
        }
        if (typeof seaweedfsWsPendingCallbacks !== 'undefined' &&
            data.type === 'command_response' &&
            seaweedfsWsPendingCallbacks[data.id]) {
            const cb = seaweedfsWsPendingCallbacks[data.id];
            delete seaweedfsWsPendingCallbacks[data.id];
            cb(normalizeDropboxCommandResponse(data));
            return;
        }
        // Fall through to the original handler (e.g. NextCloud's)
        if (existingHandler) existingHandler(event);
    };
}

function sendDropboxCommand(command, args, callback) {
    if (!dropboxWsConnection || dropboxWsConnection.readyState !== WebSocket.OPEN) {
        initializeDropboxMediaWS();
    }

    if (!dropboxWsConnection || dropboxWsConnection.readyState !== WebSocket.OPEN) {
        showDropboxToast('WebSocket connection required for Dropbox operations', 'error');
        return false;
    }

    const message = {
        type: 'command',
        id: generateDropboxId(),
        command,
        args
    };

    if (callback) {
        dropboxWsPendingCallbacks[message.id] = callback;
    }

    dropboxWsConnection.send(JSON.stringify(message));
    return true;
}

// Connect to Dropbox
function connectToDropbox() {
    const accessToken = document.getElementById('dropbox-access-token').value;

    if (!accessToken) {
        showDropboxToast('Please enter your Dropbox access token', 'error');
        return;
    }

    dropboxMediaCredentials = {
        accessToken: accessToken
    };

    // Test connection and load files
    testDropboxConnection();
}

function authenticateDropbox() {
    showDropboxToast('Opening Dropbox authentication...', 'info');

    // Remove existing modal if present to avoid DOM leaks
    const existing = document.getElementById('dropboxAuthModal');
    if (existing) {
        existing.parentNode.removeChild(existing);
    }

    // This would normally open Dropbox OAuth flow
    // For now, we'll show instructions
    const modal = document.createElement('div');
    modal.innerHTML = `
        <div class="modal fade" id="dropboxAuthModal" tabindex="-1" role="dialog">
            <div class="modal-dialog" role="document">
                <div class="modal-content" style="background: #2a2a3a; color: white;">
                    <div class="modal-header">
                        <h5 class="modal-title">Dropbox Authentication</h5>
                        <button type="button" class="close" data-dismiss="modal">
                            <span style="color: white;">&times;</span>
                        </button>
                    </div>
                    <div class="modal-body">
                        <p>To get your Dropbox access token:</p>
                        <ol>
                            <li>Go to <a href="https://www.dropbox.com/developers/apps" target="_blank">Dropbox Developers</a></li>
                            <li>Create a new app or select an existing one</li>
                            <li>Generate an access token</li>
                            <li>Copy the access token and paste it in the field above</li>
                        </ol>
                    </div>
                    <div class="modal-footer">
                        <button type="button" class="btn btn-secondary" data-dismiss="modal">Close</button>
                    </div>
                </div>
            </div>
        </div>
    `;
    document.body.appendChild(modal);
    // Clean up DOM when modal is hidden
    $('#dropboxAuthModal').on('hidden.bs.modal', function() {
        $(this).parent().remove();
    });
    $('#dropboxAuthModal').modal('show');
}

function testDropboxConnection() {
    if (!dropboxMediaCredentials) {
        showDropboxToast('Please connect to Dropbox first', 'error');
        return;
    }

    if (!dropboxWsConnection) {
        initializeDropboxMediaWS();
    }

    sendDropboxCommand('dropbox_test_connection', {
        access_token: dropboxMediaCredentials.accessToken
    }, function(data) {
        if (data.success) {
            showDropboxToast('Connected to Dropbox successfully!', 'success');
            document.getElementById('dropbox-connection-panel').style.display = 'none';
            document.getElementById('dropbox-media-browser').style.display = 'block';
            loadDropboxFiles();
        } else {
            showDropboxToast('Failed to connect to Dropbox: ' + data.error, 'error');
        }
    });
}

function loadDropboxFiles(path = '') {
    if (!dropboxMediaCredentials) {
        showDropboxToast('Please connect to Dropbox first', 'error');
        return;
    }

    sendDropboxCommand('dropbox_list_files', {
        access_token: dropboxMediaCredentials.accessToken,
        path: path,
        limit: 100
    }, function(data) {
        if (data.success) {
            dropboxMediaFiles = data.data.files || [];
            displayDropboxFiles(dropboxMediaFiles);
            showDropboxToast(`Loaded ${dropboxMediaFiles.length} files`, 'success');
        } else {
            showDropboxToast('Failed to load Dropbox files: ' + data.error, 'error');
        }
    });
}

function refreshDropboxFiles() {
    loadDropboxFiles();
}

function displayDropboxFiles(files) {
    const grid = document.getElementById('dropbox-media-grid');
    grid.innerHTML = '';

    const filteredFiles = files.filter(file => {
        if (dropboxCurrentFilter === 'all') return true;
        return getDropboxMediaType(file.mime_type) === dropboxCurrentFilter;
    });

    filteredFiles.forEach(file => {
        const fileCard = createDropboxFileCard(file);
        grid.appendChild(fileCard);
    });
}

function createDropboxFileCard(file) {
    const div = document.createElement('div');
    div.className = 'media-file-card controller-btn';
    div.style.cssText = 'background: rgba(0, 123, 255, 0.1); border: 1px solid #007bff; border-radius: 10px; padding: 15px; text-align: center; cursor: pointer; transition: all 0.3s ease;';

    const mediaType = getDropboxMediaType(file.mime_type);
    const icon = getDropboxMediaIcon(file.name, file.is_folder, mediaType);

    div.innerHTML = `
        <div style="font-size: 48px; margin-bottom: 10px; color: #007bff;">
            <i class="${icon}"></i>
        </div>
        <div style="color: white; font-weight: bold; margin-bottom: 5px; font-size: 14px; word-break: break-word;">
            ${escapeHtml(file.name)}
        </div>
        <div style="color: #bbb; font-size: 12px;">
            ${formatDropboxFileSize(file.size)}
        </div>
        <div style="color: #bbb; font-size: 11px; margin-top: 5px;">
            ${new Date(file.modified).toLocaleDateString()}
        </div>
    `;

    // Add click handler
    div.onclick = function() {
        if (file.is_folder) {
            loadDropboxFiles(file.path);
        } else {
            openDropboxMediaFile(file.path, file.name, file.mime_type, file.size);
        }
    };

    // Add hover effects
    div.onmouseenter = function() {
        this.style.background = 'rgba(0, 123, 255, 0.2)';
        this.style.transform = 'scale(1.05)';
    };

    div.onmouseleave = function() {
        this.style.background = 'rgba(0, 123, 255, 0.1)';
        this.style.transform = 'scale(1)';
    };

    return div;
}

function filterDropboxMedia(type) {
    dropboxCurrentFilter = type;

    // Update filter buttons
    document.querySelectorAll('[id^="dropbox-filter-"]').forEach(btn => {
        btn.classList.remove('active');
    });
    document.getElementById(`dropbox-filter-${type}`).classList.add('active');

    // Re-display files with new filter
    displayDropboxFiles(dropboxMediaFiles);
}

function uploadDropboxFiles() {
    const input = document.createElement('input');
    input.type = 'file';
    input.multiple = true;
    input.accept = 'image/*,video/*,audio/*,.pdf,.doc,.docx,.txt';
    input.onchange = async (event) => {
        const files = event.target.files;
        for (let file of files) {
            await uploadDropboxFile(file);
        }
        // Refresh file list after uploads
        refreshDropboxFiles();
    };
    input.click();
}

async function uploadDropboxFile(file) {
    if (!dropboxMediaCredentials) {
        showDropboxToast('Please connect to Dropbox first', 'error');
        return;
    }

    // Convert file to base64 for WebSocket transmission
    const base64Content = await new Promise((resolve, reject) => {
        const reader = new FileReader();
        reader.onload = (e) => resolve(e.target.result.split(',')[1]);
        reader.onerror = reject;
        reader.readAsDataURL(file);
    });

    const uploadCommand = {
        type: 'command',
        id: generateDropboxId(),
        command: 'dropbox_upload_file',
        args: {
            access_token: dropboxMediaCredentials.accessToken,
            remote_path: `/${file.name}`,
            content: base64Content,
            filename: file.name
        }
    };

    return new Promise((resolve) => {
        const sent = sendDropboxCommand(uploadCommand.command, uploadCommand.args, function(data) {
            if (data.success) {
                showDropboxToast(`${file.name} uploaded successfully!`, 'success');
            } else {
                showDropboxToast(`Failed to upload ${file.name}: ${data.error}`, 'error');
            }
            resolve();
        });
        if (!sent) resolve();
    });
}

function openDropboxMediaFile(path, name, mimeType, size) {
    const mediaType = getDropboxMediaType(mimeType);
    dropboxCurrentMediaFile = { path, name, mimeType, size, mediaType };
    currentMediaFile = null;
    seaweedfsCurrentMediaFile = null;

    // Hide all player types first
    document.getElementById('nextcloudVideoPlayer').style.display = 'none';
    document.getElementById('nextcloudAudioPlayer').style.display = 'none';
    document.getElementById('nextcloudImageViewer').style.display = 'none';
    document.getElementById('nextcloudDocumentViewer').style.display = 'none';

    // Set modal title
    document.getElementById('mediaPlayerTitle').innerHTML = `<i class="${getDropboxMediaIcon(name, false, mediaType)}"></i> ${name}`;

    // Get download URL for the file
    getDropboxDownloadUrl(path, function(downloadUrl) {
        if (!downloadUrl) {
            showDropboxToast('Failed to get download URL', 'error');
            return;
        }

        switch (mediaType) {
            case 'video':
                showVideoPlayer(downloadUrl, name);
                break;
            case 'audio':
                showAudioPlayer(downloadUrl, name, formatDropboxFileSize(size));
                break;
            case 'image':
                showImageViewer(downloadUrl, name, formatDropboxFileSize(size));
                break;
            default:
                showDocumentViewer(name, formatDropboxFileSize(size));
                break;
        }

        // Show the modal
        $('#mediaPlayerModal').modal('show');
    });
}

function getDropboxDownloadUrl(path, callback) {
    if (!dropboxMediaCredentials) {
        callback(null);
        return;
    }

    const downloadCommand = {
        type: 'command',
        id: generateDropboxId(),
        command: 'dropbox_get_download_url',
        args: {
            access_token: dropboxMediaCredentials.accessToken,
            path: path
        }
    };

    sendDropboxCommand(downloadCommand.command, downloadCommand.args, function(data) {
        if (data.success) {
            callback(data.data.download_url);
        } else {
            callback(null);
        }
    });
}

// Utility functions for Dropbox media
function getDropboxMediaType(mimeType) {
    if (!mimeType) return 'document';

    if (mimeType.startsWith('image/')) return 'image';
    if (mimeType.startsWith('video/')) return 'video';
    if (mimeType.startsWith('audio/')) return 'audio';
    return 'document';
}

function getDropboxMediaIcon(filename, isFolder, mediaType) {
    if (isFolder) return 'fas fa-folder';

    switch (mediaType) {
        case 'image': return 'fas fa-image';
        case 'video': return 'fas fa-video';
        case 'audio': return 'fas fa-music';
        case 'document':
            if (filename.toLowerCase().endsWith('.pdf')) return 'fas fa-file-pdf';
            if (filename.toLowerCase().match(/\.(doc|docx)$/)) return 'fas fa-file-word';
            if (filename.toLowerCase().match(/\.(xls|xlsx)$/)) return 'fas fa-file-excel';
            if (filename.toLowerCase().match(/\.(ppt|pptx)$/)) return 'fas fa-file-powerpoint';
            return 'fas fa-file';
        default: return 'fas fa-file';
    }
}

function formatDropboxFileSize(bytes) {
    if (bytes === 0) return '0 B';
    const k = 1024;
    const sizes = ['B', 'KB', 'MB', 'GB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i];
}

function generateDropboxId() {
    return 'dropbox_' + Math.random().toString(36).substr(2, 9);
}

function showDropboxToast(message, type) {
    if (typeof toastr !== 'undefined') {
        toastr[type](message);
    } else {
        console.log(`[${type.toUpperCase()}] ${message}`);
    }
}

// Download current media file
function downloadCurrentDropboxMedia() {
    if (!dropboxCurrentMediaFile) return;

    getDropboxDownloadUrl(dropboxCurrentMediaFile.path, function(downloadUrl) {
        if (downloadUrl) {
            const a = document.createElement('a');
            a.href = downloadUrl;
            a.download = dropboxCurrentMediaFile.name;
            document.body.appendChild(a);
            a.click();
            document.body.removeChild(a);
            showDropboxToast('Download started', 'success');
        } else {
            showDropboxToast('Failed to get download URL', 'error');
        }
    });
}

// Delete current media file
function deleteCurrentDropboxMedia() {
    if (!dropboxCurrentMediaFile) return;

    if (confirm(`Are you sure you want to delete "${dropboxCurrentMediaFile.name}"?`)) {
        const deleteCommand = {
            type: 'command',
            id: generateDropboxId(),
            command: 'dropbox_delete_file',
            args: {
                access_token: dropboxMediaCredentials.accessToken,
                path: dropboxCurrentMediaFile.path
            }
        };

        sendDropboxCommand(deleteCommand.command, deleteCommand.args, function(data) {
            if (data.success) {
                showDropboxToast('File deleted successfully', 'success');
                $('#mediaPlayerModal').modal('hide');
                refreshDropboxFiles();
            } else {
                showDropboxToast('Failed to delete file: ' + data.error, 'error');
            }
        });
    }
}

// Initialize when page loads
$(document).ready(function() {
    initializeDropboxMediaWS();
});
