// ███████     █████     ███    ███
// ██         ██   ██    ████  ████
// ███████    ███████    ██ ████ ██
//      ██    ██   ██    ██  ██  ██
// ███████ ██ ██   ██ ██ ██      ██ ██
// Copyright 2021-2026 The Open Sam Foundation (OSF)
// Developed by Caleb Mitchell Smith (ktheindifferent, PixelCoda, p0indexter)
// Licensed under GPLv3....see LICENSE file.

// NextCloud Media Integration for SAM Media Center

// Global variables for NextCloud media functionality
let nextcloudMediaCredentials = null;
let currentMediaPath = '';
let currentMediaFilter = 'all';
let mediaFiles = [];
let currentMediaFile = null;
let websocketConnection = null;
const nextcloudWsPendingCallbacks = {};

function normalizeStorageCommandResponse(message) {
    const payload = message && typeof message.data === 'object' && message.data !== null
        ? message.data
        : {};
    const normalized = Object.assign({}, payload);
    normalized.id = message.id;
    normalized.command = payload.command || message.command;
    normalized.envelopeSuccess = message.success;
    if (typeof normalized.success !== 'boolean') {
        normalized.success = !!message.success;
    }
    normalized.data = payload.data || payload;
    return normalized;
}

// Initialize WebSocket connection for NextCloud commands
function initializeWebSocket() {
    const wsProtocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
    const isLocalHost = window.location.hostname === 'localhost' ||
        window.location.hostname === '127.0.0.1' ||
        window.location.hostname === '::1';
    const wsUrl = isLocalHost
        ? `${wsProtocol}//${window.location.hostname}:8080/ws`
        : `${wsProtocol}//${window.location.host}/ws`;

    websocketConnection = new WebSocket(wsUrl);
    window.ws = websocketConnection;

    websocketConnection.onopen = function(event) {
        console.log('WebSocket connected for NextCloud media');
        // Re-initialize Dropbox/SeaweedFS to use the new socket
        if (typeof initializeDropboxMediaWS === 'function') {
            initializeDropboxMediaWS();
        }
        if (typeof initializeSeaweedFSMediaWS === 'function') {
            initializeSeaweedFSMediaWS();
        }
    };

    websocketConnection.onmessage = function(event) {
        let data;
        try {
            data = JSON.parse(event.data);
        } catch (e) {
            console.error('Malformed WebSocket message:', e);
            return;
        }
        if (data.type === 'command_response' && nextcloudWsPendingCallbacks[data.id]) {
            const cb = nextcloudWsPendingCallbacks[data.id];
            delete nextcloudWsPendingCallbacks[data.id];
            cb(normalizeStorageCommandResponse(data));
            return;
        }
        handleNextCloudMediaResponse(data);
    };

    websocketConnection.onclose = function(event) {
        console.log('WebSocket disconnected, attempting to reconnect...');
        setTimeout(initializeWebSocket, 3000);
    };

    websocketConnection.onerror = function(error) {
        console.error('WebSocket error:', error);
    };
}

function sendNextCloudCommand(command, args, callback) {
    if (!websocketConnection || websocketConnection.readyState !== WebSocket.OPEN) {
        showMediaToast('WebSocket connection required for NextCloud operations', 'error');
        initializeWebSocket();
        return false;
    }

    const message = {
        type: 'command',
        id: generateMediaId(),
        command,
        args
    };

    if (callback) {
        nextcloudWsPendingCallbacks[message.id] = function(response) {
            response.command = response.command || command;
            callback(response);
        };
    }
    websocketConnection.send(JSON.stringify(message));
    return true;
}

// Connect to NextCloud server
async function connectToNextCloud() {
    const serverUrl = document.getElementById('nc-server').value;
    const username = document.getElementById('nc-username').value;
    const password = document.getElementById('nc-password').value;

    if (!serverUrl || !username || !password) {
        showMediaToast('Please fill in all NextCloud credentials', 'error');
        return;
    }

    showMediaToast('Connecting to NextCloud...', 'info');

    // Store credentials
    nextcloudMediaCredentials = { serverUrl, username, password };

    sendNextCloudCommand(
        'nextcloud_test_connection',
        {
            server_url: serverUrl,
            username: username,
            password: password
        },
        handleNextCloudOperationResponse
    );
}

function handleNextCloudOperationResponse(response) {
    switch (response.command) {
        case 'nextcloud_test_connection':
            if (response.success) {
                showMediaToast('NextCloud connection successful!', 'success');
                document.getElementById('nextcloud-connection-panel').style.display = 'none';
                document.getElementById('nextcloud-media-browser').style.display = 'block';
                loadNextCloudMediaFiles();
            } else {
                showMediaToast('NextCloud connection failed: ' + (response.error || 'Unknown error'), 'error');
            }
            break;

        case 'nextcloud_list_files':
            if (response.success && response.files) {
                mediaFiles = response.files || [];
                renderMediaFiles();
            } else {
                showMediaToast('Failed to load files: ' + (response.error || 'Unknown error'), 'error');
            }
            break;

        case 'nextcloud_upload_file':
            if (response.success) {
                showMediaToast('File uploaded successfully', 'success');
                loadNextCloudMediaFiles();
            } else {
                showMediaToast('Upload failed: ' + (response.error || 'Unknown error'), 'error');
            }
            break;

        case 'nextcloud_download_file':
            if (response.success && response.content) {
                downloadMediaFileFromResponse(response);
            } else {
                showMediaToast('Download failed: ' + (response.error || 'Unknown error'), 'error');
            }
            break;

        case 'nextcloud_delete_file':
            if (response.success) {
                showMediaToast('File deleted successfully', 'success');
                loadNextCloudMediaFiles();
            } else {
                showMediaToast('Delete failed: ' + (response.error || 'Unknown error'), 'error');
            }
            break;

        case 'nextcloud_create_directory':
            if (response.success) {
                showMediaToast('Folder created successfully', 'success');
                loadNextCloudMediaFiles();
            } else {
                showMediaToast('Create folder failed: ' + (response.error || 'Unknown error'), 'error');
            }
            break;
    }
}

function arrayBufferToBase64(buffer) {
    const bytes = new Uint8Array(buffer);
    const chunkSize = 0x8000;
    let binary = '';
    for (let i = 0; i < bytes.length; i += chunkSize) {
        binary += String.fromCharCode.apply(null, bytes.subarray(i, i + chunkSize));
    }
    return btoa(binary);
}

// Handle NextCloud WebSocket responses
function handleNextCloudMediaResponse(data) {
    if (data.type === 'command_response') {
        const response = normalizeStorageCommandResponse(data);
        handleNextCloudOperationResponse(response);
    }
}

// Load media files from NextCloud
function loadNextCloudMediaFiles() {
    if (!nextcloudMediaCredentials) {
        showMediaToast('Please connect to NextCloud first', 'error');
        return;
    }

    sendNextCloudCommand(
        'nextcloud_list_files',
        {
            server_url: nextcloudMediaCredentials.serverUrl,
            username: nextcloudMediaCredentials.username,
            password: nextcloudMediaCredentials.password,
            path: currentMediaPath
        },
        handleNextCloudOperationResponse
    );
}

// Render media files in grid
function renderMediaFiles() {
    const mediaGrid = document.getElementById('nextcloud-media-grid');
    const currentPath = document.getElementById('nc-current-path');

    // Update current path display
    currentPath.textContent = '/' + currentMediaPath;

    // Filter files based on current filter
    let filteredFiles = mediaFiles;
    if (currentMediaFilter !== 'all') {
        filteredFiles = mediaFiles.filter(file => getMediaType(file.mime_type) === currentMediaFilter);
    }

    if (!filteredFiles || filteredFiles.length === 0) {
        mediaGrid.innerHTML = '<div class="text-center" style="color: #adb5bd; grid-column: 1 / -1;">No media files found</div>';
        return;
    }

    let gridHTML = '';

    // Add back navigation if not at root
    if (currentMediaPath) {
        gridHTML += `
            <div class="media-item back-nav controller-btn" onclick="navigateBack()" style="background: rgba(42, 42, 58, 0.8); border: 2px solid #6c757d; border-radius: 8px; padding: 20px; text-align: center; cursor: pointer; transition: all 0.3s;">
                <div class="media-icon" style="font-size: 48px; color: #6c757d; margin-bottom: 10px;">
                    <i class="fas fa-arrow-left"></i>
                </div>
                <div class="media-name" style="color: white; font-size: 14px; word-break: break-word;">
                    .. (Back)
                </div>
            </div>
        `;
    }

    // Render files
    filteredFiles.forEach(file => {
        const mediaType = getMediaType(file.mime_type);
        const icon = getMediaIcon(file.name, file.is_directory, mediaType);
        const iconColor = getMediaIconColor(mediaType, file.is_directory);
        const safeName = escapeHtml(file.name);
        const safePath = escapeHtml(file.path);
        const safeMime = escapeHtml(file.mime_type || '');

        gridHTML += `
            <div class="media-item controller-btn"
                 onclick="${file.is_directory ? `navigateToMediaFolder('${safePath}')` : `openMediaFile('${safePath}', '${safeName}', '${safeMime}', ${file.size})`}"
                 style="background: rgba(42, 42, 58, 0.8); border: 2px solid ${iconColor}; border-radius: 8px; padding: 15px; text-align: center; cursor: pointer; transition: all 0.3s; position: relative;">

                <div class="media-icon" style="font-size: 48px; color: ${iconColor}; margin-bottom: 10px;">
                    <i class="${icon}"></i>
                </div>

                <div class="media-name" style="color: white; font-size: 12px; word-break: break-word; margin-bottom: 5px;">
                    ${safeName}
                </div>

                <div class="media-info" style="color: #adb5bd; font-size: 10px;">
                    ${file.is_directory ? 'Folder' : formatFileSize(file.size)}
                </div>

                ${!file.is_directory ? `
                    <div class="media-actions" style="position: absolute; top: 5px; right: 5px;">
                        <button class="btn btn-sm btn-outline-danger" onclick="event.stopPropagation(); deleteMediaFile('${safePath}')" title="Delete">
                            <i class="fas fa-trash" style="font-size: 10px;"></i>
                        </button>
                    </div>
                ` : ''}
            </div>
        `;
    });

    mediaGrid.innerHTML = gridHTML;
}

// Get media type from MIME type
function getMediaType(mimeType) {
    if (!mimeType) return 'other';

    if (mimeType.startsWith('image/')) return 'image';
    if (mimeType.startsWith('video/')) return 'video';
    if (mimeType.startsWith('audio/')) return 'audio';
    if (mimeType.includes('pdf') || mimeType.includes('document') || mimeType.includes('text')) return 'document';

    return 'other';
}

// Get icon for media file
function getMediaIcon(filename, isDirectory, mediaType) {
    if (isDirectory) return 'fas fa-folder';

    switch (mediaType) {
        case 'image': return 'fas fa-image';
        case 'video': return 'fas fa-video';
        case 'audio': return 'fas fa-music';
        case 'document': return 'fas fa-file-alt';
        default: return 'fas fa-file';
    }
}

// Get icon color for media type
function getMediaIconColor(mediaType, isDirectory) {
    if (isDirectory) return '#ffc107';

    switch (mediaType) {
        case 'image': return '#28a745';
        case 'video': return '#dc3545';
        case 'audio': return '#17a2b8';
        case 'document': return '#6f42c1';
        default: return '#6c757d';
    }
}

// Navigate to media folder
function navigateToMediaFolder(path) {
    currentMediaPath = path;
    loadNextCloudMediaFiles();
}

// Navigate back
function navigateBack() {
    const parts = currentMediaPath.split('/').filter(p => p);
    parts.pop();
    currentMediaPath = parts.join('/');
    loadNextCloudMediaFiles();
}

// Open media file
function openMediaFile(path, name, mimeType, size) {
    currentMediaFile = { path, name, mimeType, size };
    dropboxCurrentMediaFile = null;
    seaweedfsCurrentMediaFile = null;
    const mediaType = getMediaType(mimeType);

    // Hide all player types first
    document.getElementById('nextcloudVideoPlayer').style.display = 'none';
    document.getElementById('nextcloudAudioPlayer').style.display = 'none';
    document.getElementById('nextcloudImageViewer').style.display = 'none';
    document.getElementById('nextcloudDocumentViewer').style.display = 'none';

    // Set modal title
    document.getElementById('mediaPlayerTitle').innerHTML = `<i class="${getMediaIcon(name, false, mediaType)}"></i> ${name}`;

    // Create streaming URL for the file
    const streamUrl = createStreamingUrl(path);

    switch (mediaType) {
        case 'video':
            showVideoPlayer(streamUrl, name);
            break;
        case 'audio':
            showAudioPlayer(streamUrl, name, formatFileSize(size));
            break;
        case 'image':
            showImageViewer(streamUrl, name, formatFileSize(size));
            break;
        default:
            showDocumentViewer(name, formatFileSize(size));
            break;
    }

    // Show the modal
    $('#mediaPlayerModal').modal('show');
}

// Create streaming URL for NextCloud file
function createStreamingUrl(filePath) {
    if (!nextcloudMediaCredentials) return '#';

    const serverUrl = nextcloudMediaCredentials.serverUrl.replace(/\/$/, '');
    const username = encodeURIComponent(nextcloudMediaCredentials.username);
    const password = encodeURIComponent(nextcloudMediaCredentials.password);

    // Parse the server URL to embed credentials for Basic auth
    const url = new URL(serverUrl);
    const authedBase = `${url.protocol}//${username}:${password}@${url.host}${url.pathname}`;

    return authedBase + `/remote.php/dav/files/${nextcloudMediaCredentials.username}/` +
           filePath.replace(/^\//, '');
}

// Show video player
function showVideoPlayer(videoUrl, title) {
    // Dispose existing player if present (this removes the DOM element)
    const existingPlayer = videojs.getPlayer('nextcloudVideoPlayer');
    if (existingPlayer) {
        existingPlayer.dispose();
    }

    // Re-create the video element if it was removed by dispose()
    if (!document.getElementById('nextcloudVideoPlayer')) {
        const modalBody = document.querySelector('#mediaPlayerModal .modal-body');
        const video = document.createElement('video');
        video.id = 'nextcloudVideoPlayer';
        video.className = 'video-js vjs-default-skin';
        video.setAttribute('controls', '');
        video.setAttribute('preload', 'auto');
        video.setAttribute('width', '100%');
        video.setAttribute('height', '400');
        video.style.background = 'black';
        modalBody.insertBefore(video, modalBody.firstChild);
    }

    const videoPlayer = document.getElementById('nextcloudVideoPlayer');
    videoPlayer.style.display = 'block';

    // Determine MIME type from whichever integration is active
    const mimeType = (currentMediaFile && currentMediaFile.mimeType) ||
                     (dropboxCurrentMediaFile && dropboxCurrentMediaFile.mimeType) ||
                     (seaweedfsCurrentMediaFile && seaweedfsCurrentMediaFile.mimeType) ||
                     'video/mp4';

    const player = videojs('nextcloudVideoPlayer', {
        sources: [{
            src: videoUrl,
            type: mimeType
        }],
        controls: true,
        responsive: true,
        fluid: true
    });

    player.ready(() => {
        console.log('Video player ready for:', title);
    });
}

// Get active media file from whichever integration is open
function getActiveMediaFile() {
    return currentMediaFile || dropboxCurrentMediaFile || seaweedfsCurrentMediaFile || null;
}

// Show audio player
function showAudioPlayer(audioUrl, title, info) {
    document.getElementById('nextcloudAudioPlayer').style.display = 'block';
    document.getElementById('audioTitle').textContent = title;
    document.getElementById('audioInfo').textContent = info;
    document.getElementById('audioSource').src = audioUrl;
    const active = getActiveMediaFile();
    document.getElementById('audioSource').type = (active && active.mimeType) || 'audio/mpeg';

    const audioElement = document.querySelector('#nextcloudAudioPlayer audio');
    audioElement.load();
}

// Show image viewer
function showImageViewer(imageUrl, title, info) {
    document.getElementById('nextcloudImageViewer').style.display = 'block';
    document.getElementById('viewerImage').src = imageUrl;
    document.getElementById('imageTitle').textContent = title;
    document.getElementById('imageDetails').textContent = info;
}

// Show document viewer
function showDocumentViewer(title, info) {
    document.getElementById('nextcloudDocumentViewer').style.display = 'block';
    document.getElementById('documentTitle').textContent = title;
    document.getElementById('documentInfo').textContent = info;
}

// Filter media by type
function filterMediaType(type) {
    currentMediaFilter = type;

    // Update active button
    document.querySelectorAll('.media-filter-tabs .btn').forEach(btn => {
        btn.classList.remove('active');
    });
    document.querySelector(`.media-filter-tabs .btn[data-filter="${type}"]`).classList.add('active');

    renderMediaFiles();
}

// Upload media files
function uploadMediaFiles() {
    const input = document.createElement('input');
    input.type = 'file';
    input.multiple = true;
    input.accept = 'image/*,video/*,audio/*,.pdf,.doc,.docx,.txt';

    input.onchange = async (event) => {
        const files = event.target.files;
        for (let file of files) {
            await uploadMediaFile(file);
        }
    };

    input.click();
}

// Upload single media file
async function uploadMediaFile(file) {
    if (!nextcloudMediaCredentials) {
        showMediaToast('Please connect to NextCloud first', 'error');
        return;
    }

    showMediaToast(`Uploading ${file.name}...`, 'info');

    try {
        const arrayBuffer = await file.arrayBuffer();
        const base64 = arrayBufferToBase64(arrayBuffer);
        const remotePath = currentMediaPath ? `${currentMediaPath}/${file.name}` : file.name;

        sendNextCloudCommand(
            'nextcloud_upload_file',
            {
                server_url: nextcloudMediaCredentials.serverUrl,
                username: nextcloudMediaCredentials.username,
                password: nextcloudMediaCredentials.password,
                remote_path: remotePath,
                content: base64
            },
            handleNextCloudOperationResponse
        );
    } catch (error) {
        showMediaToast(`Error uploading ${file.name}: ` + error.message, 'error');
    }
}

// Create media folder
function createMediaFolder() {
    const folderName = prompt('Enter folder name:');
    if (!folderName) return;

    if (!nextcloudMediaCredentials) {
        showMediaToast('Please connect to NextCloud first', 'error');
        return;
    }

    const remotePath = currentMediaPath ? `${currentMediaPath}/${folderName}` : folderName;

    sendNextCloudCommand(
        'nextcloud_create_directory',
        {
            server_url: nextcloudMediaCredentials.serverUrl,
            username: nextcloudMediaCredentials.username,
            password: nextcloudMediaCredentials.password,
            remote_path: remotePath
        },
        handleNextCloudOperationResponse
    );
}

// Refresh NextCloud media
function refreshNextCloudMedia() {
    loadNextCloudMediaFiles();
}

// Delete media file
function deleteMediaFile(path) {
    if (!confirm(`Are you sure you want to delete this file?`)) return;

    if (!nextcloudMediaCredentials) {
        showMediaToast('Please connect to NextCloud first', 'error');
        return;
    }

    sendNextCloudCommand(
        'nextcloud_delete_file',
        {
            server_url: nextcloudMediaCredentials.serverUrl,
            username: nextcloudMediaCredentials.username,
            password: nextcloudMediaCredentials.password,
            remote_path: path
        },
        handleNextCloudOperationResponse
    );
}

// Delete current media (from modal) — unified dispatcher
function deleteCurrentMedia() {
    if (dropboxCurrentMediaFile) {
        deleteCurrentDropboxMedia();
    } else if (seaweedfsCurrentMediaFile) {
        deleteCurrentSeaweedFSMedia();
    } else if (currentMediaFile) {
        $('#mediaPlayerModal').modal('hide');
        deleteMediaFile(currentMediaFile.path);
    }
}

// Download current media — unified dispatcher
function downloadCurrentMedia() {
    if (dropboxCurrentMediaFile) {
        downloadCurrentDropboxMedia();
    } else if (seaweedfsCurrentMediaFile) {
        downloadCurrentSeaweedFSMedia();
    } else if (currentMediaFile) {
        sendNextCloudCommand(
            'nextcloud_download_file',
            {
                server_url: nextcloudMediaCredentials.serverUrl,
                username: nextcloudMediaCredentials.username,
                password: nextcloudMediaCredentials.password,
                remote_path: currentMediaFile.path
            },
            handleNextCloudOperationResponse
        );
    }
}

// Download media file from response
function downloadMediaFileFromResponse(response) {
    if (!response.content) return;

    const active = getActiveMediaFile();
    if (!active) {
        showMediaToast('No file selected for download', 'error');
        return;
    }

    try {
        const byteCharacters = atob(response.content);
        const byteNumbers = new Array(byteCharacters.length);
        for (let i = 0; i < byteCharacters.length; i++) {
            byteNumbers[i] = byteCharacters.charCodeAt(i);
        }
        const byteArray = new Uint8Array(byteNumbers);
        const blob = new Blob([byteArray], { type: active.mimeType || 'application/octet-stream' });

        const url = URL.createObjectURL(blob);
        const a = document.createElement('a');
        a.href = url;
        a.download = active.name;
        document.body.appendChild(a);
        a.click();
        document.body.removeChild(a);
        URL.revokeObjectURL(url);

        showMediaToast('File downloaded successfully', 'success');
    } catch (error) {
        showMediaToast('Error downloading file', 'error');
    }
}

// Open in new tab — unified dispatcher
function openInNewTab() {
    if (currentMediaFile) {
        window.open(createStreamingUrl(currentMediaFile.path), '_blank');
    } else if (dropboxCurrentMediaFile) {
        getDropboxDownloadUrl(dropboxCurrentMediaFile.path, function(url) {
            if (url) window.open(url, '_blank');
        });
    } else if (seaweedfsCurrentMediaFile) {
        window.open(createSeaweedFSStreamingUrl(seaweedfsCurrentMediaFile.path), '_blank');
    }
}

// HTML escape to prevent XSS from filenames
function escapeHtml(str) {
    const div = document.createElement('div');
    div.appendChild(document.createTextNode(str));
    return div.innerHTML;
}

// Utility functions
function generateMediaId() {
    return Math.random().toString(36).substr(2, 9);
}

function formatFileSize(bytes) {
    if (bytes === 0) return '0 B';
    const k = 1024;
    const sizes = ['B', 'KB', 'MB', 'GB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i];
}

function showMediaToast(message, type) {
    // Use toastr if available, otherwise console log
    if (typeof toastr !== 'undefined') {
        toastr[type](message);
    } else {
        console.log(`[${type.toUpperCase()}] ${message}`);
    }
}

// Initialize when document is ready
$(document).ready(function() {
    initializeWebSocket();

    // Initialize tooltips
    $('[title]').tooltip();

    // Stop audio/video playback when modal is closed
    $('#mediaPlayerModal').on('hidden.bs.modal', function() {
        // Pause and reset audio
        var audio = document.querySelector('#nextcloudAudioPlayer audio');
        if (audio) {
            audio.pause();
            audio.currentTime = 0;
        }
        // Dispose video player if active
        var player = videojs.getPlayer('nextcloudVideoPlayer');
        if (player) {
            player.pause();
        }
    });

    console.log('NextCloud Media integration initialized');
});
