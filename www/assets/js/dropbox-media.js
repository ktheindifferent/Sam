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

function initializeDropboxMediaWS() {
    if (!window.ws || window.ws.readyState !== WebSocket.OPEN) {
        console.error('WebSocket connection not available for Dropbox media');
        return;
    }
    dropboxWsConnection = window.ws;
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

    // Send test connection command via WebSocket
    const testCommand = {
        type: 'command',
        id: generateDropboxId(),
        command: 'dropbox_test_connection',
        args: {
            access_token: dropboxMediaCredentials.accessToken
        }
    };

    dropboxWsConnection.send(JSON.stringify(testCommand));

    // Listen for response
    const originalHandler = dropboxWsConnection.onmessage;
    dropboxWsConnection.onmessage = function(event) {
        const data = JSON.parse(event.data);

        if (data.type === 'command_response' && data.id === testCommand.id) {
            if (data.success) {
                showDropboxToast('Connected to Dropbox successfully!', 'success');
                document.getElementById('dropbox-connection-panel').style.display = 'none';
                document.getElementById('dropbox-media-browser').style.display = 'block';
                loadDropboxFiles();
            } else {
                showDropboxToast('Failed to connect to Dropbox: ' + data.error, 'error');
            }
            // Restore original handler
            dropboxWsConnection.onmessage = originalHandler;
        }
    };
}

function loadDropboxFiles(path = '') {
    if (!dropboxMediaCredentials || !dropboxWsConnection) {
        showDropboxToast('Please connect to Dropbox first', 'error');
        return;
    }

    const listCommand = {
        type: 'command',
        id: generateDropboxId(),
        command: 'dropbox_list_files',
        args: {
            access_token: dropboxMediaCredentials.accessToken,
            path: path,
            limit: 100
        }
    };

    dropboxWsConnection.send(JSON.stringify(listCommand));

    // Listen for response
    const originalHandler = dropboxWsConnection.onmessage;
    dropboxWsConnection.onmessage = function(event) {
        const data = JSON.parse(event.data);

        if (data.type === 'command_response' && data.id === listCommand.id) {
            if (data.success) {
                dropboxMediaFiles = data.data.files || [];
                displayDropboxFiles(dropboxMediaFiles);
                showDropboxToast(`Loaded ${dropboxMediaFiles.length} files`, 'success');
            } else {
                showDropboxToast('Failed to load Dropbox files: ' + data.error, 'error');
            }
            // Restore original handler
            dropboxWsConnection.onmessage = originalHandler;
        }
    };
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
            ${icon}
        </div>
        <div style="color: white; font-weight: bold; margin-bottom: 5px; font-size: 14px; word-break: break-word;">
            ${file.name}
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
    if (!dropboxMediaCredentials || !dropboxWsConnection) {
        showDropboxToast('Please connect to Dropbox first', 'error');
        return;
    }

    // Convert file to base64 for WebSocket transmission
    const reader = new FileReader();
    reader.onload = function(e) {
        const base64Content = e.target.result.split(',')[1];

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

        dropboxWsConnection.send(JSON.stringify(uploadCommand));

        // Listen for response
        const originalHandler = dropboxWsConnection.onmessage;
        dropboxWsConnection.onmessage = function(event) {
            const data = JSON.parse(event.data);

            if (data.type === 'command_response' && data.id === uploadCommand.id) {
                if (data.success) {
                    showDropboxToast(`${file.name} uploaded successfully!`, 'success');
                } else {
                    showDropboxToast(`Failed to upload ${file.name}: ${data.error}`, 'error');
                }
                // Restore original handler
                dropboxWsConnection.onmessage = originalHandler;
            }
        };
    };

    reader.readAsDataURL(file);
}

function openDropboxMediaFile(path, name, mimeType, size) {
    const mediaType = getDropboxMediaType(mimeType);
    dropboxCurrentMediaFile = { path, name, mimeType, size, mediaType };

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
    if (!dropboxMediaCredentials || !dropboxWsConnection) {
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

    dropboxWsConnection.send(JSON.stringify(downloadCommand));

    // Listen for response
    const originalHandler = dropboxWsConnection.onmessage;
    dropboxWsConnection.onmessage = function(event) {
        const data = JSON.parse(event.data);

        if (data.type === 'command_response' && data.id === downloadCommand.id) {
            if (data.success) {
                callback(data.data.download_url);
            } else {
                callback(null);
            }
            // Restore original handler
            dropboxWsConnection.onmessage = originalHandler;
        }
    };
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
    if (isFolder) return '<i class="fas fa-folder"></i>';

    switch (mediaType) {
        case 'image': return '<i class="fas fa-image"></i>';
        case 'video': return '<i class="fas fa-video"></i>';
        case 'audio': return '<i class="fas fa-music"></i>';
        case 'document':
            if (filename.toLowerCase().endsWith('.pdf')) return '<i class="fas fa-file-pdf"></i>';
            if (filename.toLowerCase().match(/\.(doc|docx)$/)) return '<i class="fas fa-file-word"></i>';
            if (filename.toLowerCase().match(/\.(xls|xlsx)$/)) return '<i class="fas fa-file-excel"></i>';
            if (filename.toLowerCase().match(/\.(ppt|pptx)$/)) return '<i class="fas fa-file-powerpoint"></i>';
            return '<i class="fas fa-file"></i>';
        default: return '<i class="fas fa-file"></i>';
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

        dropboxWsConnection.send(JSON.stringify(deleteCommand));

        // Listen for response
        const originalHandler = dropboxWsConnection.onmessage;
        dropboxWsConnection.onmessage = function(event) {
            const data = JSON.parse(event.data);

            if (data.type === 'command_response' && data.id === deleteCommand.id) {
                if (data.success) {
                    showDropboxToast('File deleted successfully', 'success');
                    $('#mediaPlayerModal').modal('hide');
                    refreshDropboxFiles();
                } else {
                    showDropboxToast('Failed to delete file: ' + data.error, 'error');
                }
                // Restore original handler
                dropboxWsConnection.onmessage = originalHandler;
            }
        };
    }
}

// Initialize when page loads
$(document).ready(function() {
    initializeDropboxMediaWS();
});