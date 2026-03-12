// ███████     █████     ███    ███
// ██         ██   ██    ████  ████
// ███████    ███████    ██ ████ ██
//      ██    ██   ██    ██  ██  ██
// ███████ ██ ██   ██ ██ ██      ██ ██
// Copyright 2021-2026 The Open Sam Foundation (OSF)
// Developed by Caleb Mitchell Smith (ktheindifferent, PixelCoda, p0indexter)
// Licensed under GPLv3....see LICENSE file.

// SeaweedFS Media Integration
let seaweedfsMediaCredentials = null;
let seaweedfsMediaFiles = [];
let seaweedfsCurrentFilter = 'all';
let seaweedfsCurrentMediaFile = null;

// Initialize WebSocket connection for SeaweedFS commands
let seaweedfsWsConnection = null;
const seaweedfsWsPendingCallbacks = {};

function initializeSeaweedFSMediaWS() {
    if (!window.ws || window.ws.readyState !== WebSocket.OPEN) {
        console.error('WebSocket connection not available for SeaweedFS media');
        return;
    }
    seaweedfsWsConnection = window.ws;
}

// Connect to SeaweedFS
function connectToSeaweedFS() {
    const masterUrl = document.getElementById('seaweedfs-master').value || 'http://localhost:9333';
    const filerUrl = document.getElementById('seaweedfs-filer').value || 'http://localhost:8888';
    const collection = document.getElementById('seaweedfs-collection').value || 'sam';
    const replication = document.getElementById('seaweedfs-replication').value || '000';

    seaweedfsMediaCredentials = {
        masterUrl: masterUrl,
        filerUrl: filerUrl,
        collection: collection,
        replication: replication
    };

    // Test connection and load files
    testSeaweedFSConnection();
}

function testSeaweedFSConnection() {
    if (!seaweedfsMediaCredentials) {
        showSeaweedFSToast('Please configure SeaweedFS connection first', 'error');
        return;
    }

    if (!seaweedfsWsConnection) {
        initializeSeaweedFSMediaWS();
    }

    // Send test connection command via WebSocket
    const testCommand = {
        type: 'command',
        id: generateSeaweedFSId(),
        command: 'seaweedfs_test_connection',
        args: {
            master_url: seaweedfsMediaCredentials.masterUrl,
            filer_url: seaweedfsMediaCredentials.filerUrl
        }
    };

    seaweedfsWsPendingCallbacks[testCommand.id] = function(data) {
        if (data.success) {
            showSeaweedFSToast('Connected to SeaweedFS successfully!', 'success');
            document.getElementById('seaweedfs-connection-panel').style.display = 'none';
            document.getElementById('seaweedfs-media-browser').style.display = 'block';
            loadSeaweedFSFiles();
        } else {
            showSeaweedFSToast('Failed to connect to SeaweedFS: ' + data.error, 'error');
        }
    };
    seaweedfsWsConnection.send(JSON.stringify(testCommand));
}

function loadSeaweedFSFiles(path = '/') {
    if (!seaweedfsMediaCredentials || !seaweedfsWsConnection) {
        showSeaweedFSToast('Please connect to SeaweedFS first', 'error');
        return;
    }

    const listCommand = {
        type: 'command',
        id: generateSeaweedFSId(),
        command: 'seaweedfs_list_files',
        args: {
            filer_url: seaweedfsMediaCredentials.filerUrl,
            path: path,
            limit: 100
        }
    };

    seaweedfsWsPendingCallbacks[listCommand.id] = function(data) {
        if (data.success) {
            seaweedfsMediaFiles = data.data.files || [];
            displaySeaweedFSFiles(seaweedfsMediaFiles);
            showSeaweedFSToast(`Loaded ${seaweedfsMediaFiles.length} files`, 'success');
        } else {
            showSeaweedFSToast('Failed to load SeaweedFS files: ' + data.error, 'error');
        }
    };
    seaweedfsWsConnection.send(JSON.stringify(listCommand));
}

function refreshSeaweedFSFiles() {
    loadSeaweedFSFiles();
}

function displaySeaweedFSFiles(files) {
    const grid = document.getElementById('seaweedfs-media-grid');
    grid.innerHTML = '';

    const filteredFiles = files.filter(file => {
        if (seaweedfsCurrentFilter === 'all') return true;
        return getSeaweedFSMediaType(file.mime_type) === seaweedfsCurrentFilter;
    });

    filteredFiles.forEach(file => {
        const fileCard = createSeaweedFSFileCard(file);
        grid.appendChild(fileCard);
    });
}

function createSeaweedFSFileCard(file) {
    const div = document.createElement('div');
    div.className = 'media-file-card controller-btn';
    div.style.cssText = 'background: rgba(255, 193, 7, 0.1); border: 1px solid #ffc107; border-radius: 10px; padding: 15px; text-align: center; cursor: pointer; transition: all 0.3s ease;';

    const mediaType = getSeaweedFSMediaType(file.mime_type);
    const icon = getSeaweedFSMediaIcon(file.name, file.is_folder, mediaType);

    div.innerHTML = `
        <div style="font-size: 48px; margin-bottom: 10px; color: #ffc107;">
            <i class="${icon}"></i>
        </div>
        <div style="color: white; font-weight: bold; margin-bottom: 5px; font-size: 14px; word-break: break-word;">
            ${escapeHtml(file.name)}
        </div>
        <div style="color: #bbb; font-size: 12px;">
            ${formatSeaweedFSFileSize(file.size)}
        </div>
        <div style="color: #bbb; font-size: 11px; margin-top: 5px;">
            ${new Date(file.modified).toLocaleDateString()}
        </div>
    `;

    // Add click handler
    div.onclick = function() {
        if (file.is_folder) {
            loadSeaweedFSFiles(file.path);
        } else {
            openSeaweedFSMediaFile(file.path, file.name, file.mime_type, file.size);
        }
    };

    // Add hover effects
    div.onmouseenter = function() {
        this.style.background = 'rgba(255, 193, 7, 0.2)';
        this.style.transform = 'scale(1.05)';
    };

    div.onmouseleave = function() {
        this.style.background = 'rgba(255, 193, 7, 0.1)';
        this.style.transform = 'scale(1)';
    };

    return div;
}

function filterSeaweedFSMedia(type) {
    seaweedfsCurrentFilter = type;

    // Update filter buttons
    document.querySelectorAll('[id^="seaweedfs-filter-"]').forEach(btn => {
        btn.classList.remove('active');
    });
    document.getElementById(`seaweedfs-filter-${type}`).classList.add('active');

    // Re-display files with new filter
    displaySeaweedFSFiles(seaweedfsMediaFiles);
}

function uploadSeaweedFSFiles() {
    const input = document.createElement('input');
    input.type = 'file';
    input.multiple = true;
    input.accept = 'image/*,video/*,audio/*,.pdf,.doc,.docx,.txt';
    input.onchange = async (event) => {
        const files = event.target.files;
        for (let file of files) {
            await uploadSeaweedFSFile(file);
        }
        // Refresh file list after uploads
        refreshSeaweedFSFiles();
    };
    input.click();
}

async function uploadSeaweedFSFile(file) {
    if (!seaweedfsMediaCredentials || !seaweedfsWsConnection) {
        showSeaweedFSToast('Please connect to SeaweedFS first', 'error');
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
        id: generateSeaweedFSId(),
        command: 'seaweedfs_upload_file',
        args: {
            master_url: seaweedfsMediaCredentials.masterUrl,
            filer_url: seaweedfsMediaCredentials.filerUrl,
            remote_path: `/${file.name}`,
            content: base64Content,
            filename: file.name,
            collection: seaweedfsMediaCredentials.collection,
            replication: seaweedfsMediaCredentials.replication
        }
    };

    return new Promise((resolve) => {
        seaweedfsWsPendingCallbacks[uploadCommand.id] = function(data) {
            if (data.success) {
                showSeaweedFSToast(`${file.name} uploaded successfully!`, 'success');
            } else {
                showSeaweedFSToast(`Failed to upload ${file.name}: ${data.error}`, 'error');
            }
            resolve();
        };
        seaweedfsWsConnection.send(JSON.stringify(uploadCommand));
    });
}

function openSeaweedFSMediaFile(path, name, mimeType, size) {
    const mediaType = getSeaweedFSMediaType(mimeType);
    seaweedfsCurrentMediaFile = { path, name, mimeType, size, mediaType };
    currentMediaFile = null;
    dropboxCurrentMediaFile = null;

    // Hide all player types first
    document.getElementById('nextcloudVideoPlayer').style.display = 'none';
    document.getElementById('nextcloudAudioPlayer').style.display = 'none';
    document.getElementById('nextcloudImageViewer').style.display = 'none';
    document.getElementById('nextcloudDocumentViewer').style.display = 'none';

    // Set modal title
    document.getElementById('mediaPlayerTitle').innerHTML = `<i class="${getSeaweedFSMediaIcon(name, false, mediaType)}"></i> ${name}`;

    // Create streaming URL (SeaweedFS provides direct access)
    const streamUrl = createSeaweedFSStreamingUrl(path);

    switch (mediaType) {
        case 'video':
            showVideoPlayer(streamUrl, name);
            break;
        case 'audio':
            showAudioPlayer(streamUrl, name, formatSeaweedFSFileSize(size));
            break;
        case 'image':
            showImageViewer(streamUrl, name, formatSeaweedFSFileSize(size));
            break;
        default:
            showDocumentViewer(name, formatSeaweedFSFileSize(size));
            break;
    }

    // Show the modal
    $('#mediaPlayerModal').modal('show');
}

function createSeaweedFSStreamingUrl(filePath) {
    if (!seaweedfsMediaCredentials) return '#';
    return `${seaweedfsMediaCredentials.filerUrl}${filePath}`;
}

// Utility functions for SeaweedFS media
function getSeaweedFSMediaType(mimeType) {
    if (!mimeType) return 'document';

    if (mimeType.startsWith('image/')) return 'image';
    if (mimeType.startsWith('video/')) return 'video';
    if (mimeType.startsWith('audio/')) return 'audio';
    return 'document';
}

function getSeaweedFSMediaIcon(filename, isFolder, mediaType) {
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

function formatSeaweedFSFileSize(bytes) {
    if (bytes === 0) return '0 B';
    const k = 1024;
    const sizes = ['B', 'KB', 'MB', 'GB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i];
}

function generateSeaweedFSId() {
    return 'seaweedfs_' + Math.random().toString(36).substr(2, 9);
}

function showSeaweedFSToast(message, type) {
    if (typeof toastr !== 'undefined') {
        toastr[type](message);
    } else {
        console.log(`[${type.toUpperCase()}] ${message}`);
    }
}

// Download current media file
function downloadCurrentSeaweedFSMedia() {
    if (!seaweedfsCurrentMediaFile) return;

    const downloadUrl = createSeaweedFSStreamingUrl(seaweedfsCurrentMediaFile.path);
    const a = document.createElement('a');
    a.href = downloadUrl;
    a.download = seaweedfsCurrentMediaFile.name;
    document.body.appendChild(a);
    a.click();
    document.body.removeChild(a);
    showSeaweedFSToast('Download started', 'success');
}

// Delete current media file
function deleteCurrentSeaweedFSMedia() {
    if (!seaweedfsCurrentMediaFile) return;

    if (confirm(`Are you sure you want to delete "${seaweedfsCurrentMediaFile.name}"?`)) {
        const deleteCommand = {
            type: 'command',
            id: generateSeaweedFSId(),
            command: 'seaweedfs_delete_file',
            args: {
                filer_url: seaweedfsMediaCredentials.filerUrl,
                path: seaweedfsCurrentMediaFile.path
            }
        };

        seaweedfsWsPendingCallbacks[deleteCommand.id] = function(data) {
            if (data.success) {
                showSeaweedFSToast('File deleted successfully', 'success');
                $('#mediaPlayerModal').modal('hide');
                refreshSeaweedFSFiles();
            } else {
                showSeaweedFSToast('Failed to delete file: ' + data.error, 'error');
            }
        };
        seaweedfsWsConnection.send(JSON.stringify(deleteCommand));
    }
}

// Create folder
function createSeaweedFSFolder() {
    const folderName = prompt('Enter folder name:');
    if (!folderName) return;

    const createCommand = {
        type: 'command',
        id: generateSeaweedFSId(),
        command: 'seaweedfs_create_folder',
        args: {
            filer_url: seaweedfsMediaCredentials.filerUrl,
            path: `/${folderName}`
        }
    };

    seaweedfsWsPendingCallbacks[createCommand.id] = function(data) {
        if (data.success) {
            showSeaweedFSToast('Folder created successfully', 'success');
            refreshSeaweedFSFiles();
        } else {
            showSeaweedFSToast('Failed to create folder: ' + data.error, 'error');
        }
    };
    seaweedfsWsConnection.send(JSON.stringify(createCommand));
}

// Initialize when page loads
$(document).ready(function() {
    initializeSeaweedFSMediaWS();
});