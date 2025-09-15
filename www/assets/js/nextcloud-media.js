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

// Initialize WebSocket connection for NextCloud commands
function initializeWebSocket() {
    const wsProtocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
    const wsUrl = `${wsProtocol}//${window.location.hostname}:8080/ws`;

    websocketConnection = new WebSocket(wsUrl);

    websocketConnection.onopen = function(event) {
        console.log('WebSocket connected for NextCloud media');
    };

    websocketConnection.onmessage = function(event) {
        const data = JSON.parse(event.data);
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

    // Test connection first
    const testMessage = {
        type: 'command',
        id: generateMediaId(),
        command: 'nextcloud_test_connection',
        args: {
            server_url: serverUrl,
            username: username,
            password: password
        }
    };

    if (websocketConnection && websocketConnection.readyState === WebSocket.OPEN) {
        websocketConnection.send(JSON.stringify(testMessage));
    } else {
        showMediaToast('WebSocket connection required for NextCloud operations', 'error');
        initializeWebSocket();
    }
}

// Handle NextCloud WebSocket responses
function handleNextCloudMediaResponse(data) {
    if (data.type === 'command_response') {
        const response = data;

        switch (response.command) {
            case 'nextcloud_test_connection':
                if (response.success) {
                    showMediaToast('NextCloud connection successful!', 'success');
                    // Hide connection panel and show browser
                    document.getElementById('nextcloud-connection-panel').style.display = 'none';
                    document.getElementById('nextcloud-media-browser').style.display = 'block';
                    // Load initial media files
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
}

// Load media files from NextCloud
function loadNextCloudMediaFiles() {
    if (!nextcloudMediaCredentials) {
        showMediaToast('Please connect to NextCloud first', 'error');
        return;
    }

    const message = {
        type: 'command',
        id: generateMediaId(),
        command: 'nextcloud_list_files',
        args: {
            server_url: nextcloudMediaCredentials.serverUrl,
            username: nextcloudMediaCredentials.username,
            password: nextcloudMediaCredentials.password,
            path: currentMediaPath
        }
    };

    if (websocketConnection && websocketConnection.readyState === WebSocket.OPEN) {
        websocketConnection.send(JSON.stringify(message));
    }
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

        gridHTML += `
            <div class="media-item controller-btn"
                 onclick="${file.is_directory ? `navigateToMediaFolder('${file.path}')` : `openMediaFile('${file.path}', '${file.name}', '${file.mime_type}', ${file.size})`}"
                 style="background: rgba(42, 42, 58, 0.8); border: 2px solid ${iconColor}; border-radius: 8px; padding: 15px; text-align: center; cursor: pointer; transition: all 0.3s; position: relative;">

                <div class="media-icon" style="font-size: 48px; color: ${iconColor}; margin-bottom: 10px;">
                    <i class="${icon}"></i>
                </div>

                <div class="media-name" style="color: white; font-size: 12px; word-break: break-word; margin-bottom: 5px;">
                    ${file.name}
                </div>

                <div class="media-info" style="color: #adb5bd; font-size: 10px;">
                    ${file.is_directory ? 'Folder' : formatFileSize(file.size)}
                </div>

                ${!file.is_directory ? `
                    <div class="media-actions" style="position: absolute; top: 5px; right: 5px;">
                        <button class="btn btn-sm btn-outline-danger" onclick="event.stopPropagation(); deleteMediaFile('${file.path}')" title="Delete">
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

    const credentials = btoa(`${nextcloudMediaCredentials.username}:${nextcloudMediaCredentials.password}`);
    const webdavUrl = nextcloudMediaCredentials.serverUrl.replace(/\/$/, '') +
                     `/remote.php/dav/files/${nextcloudMediaCredentials.username}/` +
                     filePath.replace(/^\//, '');

    return webdavUrl;
}

// Show video player
function showVideoPlayer(videoUrl, title) {
    const videoPlayer = document.getElementById('nextcloudVideoPlayer');
    videoPlayer.style.display = 'block';

    if (videojs.getPlayer('nextcloudVideoPlayer')) {
        videojs.getPlayer('nextcloudVideoPlayer').dispose();
    }

    const player = videojs('nextcloudVideoPlayer', {
        sources: [{
            src: videoUrl,
            type: currentMediaFile.mimeType
        }],
        controls: true,
        responsive: true,
        fluid: true
    });

    player.ready(() => {
        console.log('Video player ready for:', title);
    });
}

// Show audio player
function showAudioPlayer(audioUrl, title, info) {
    document.getElementById('nextcloudAudioPlayer').style.display = 'block';
    document.getElementById('audioTitle').textContent = title;
    document.getElementById('audioInfo').textContent = info;
    document.getElementById('audioSource').src = audioUrl;
    document.getElementById('audioSource').type = currentMediaFile.mimeType;

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
        const base64 = btoa(String.fromCharCode(...new Uint8Array(arrayBuffer)));
        const remotePath = currentMediaPath ? `${currentMediaPath}/${file.name}` : file.name;

        const message = {
            type: 'command',
            id: generateMediaId(),
            command: 'nextcloud_upload_file',
            args: {
                server_url: nextcloudMediaCredentials.serverUrl,
                username: nextcloudMediaCredentials.username,
                password: nextcloudMediaCredentials.password,
                remote_path: remotePath,
                content: base64
            }
        };

        if (websocketConnection && websocketConnection.readyState === WebSocket.OPEN) {
            websocketConnection.send(JSON.stringify(message));
        }
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

    const message = {
        type: 'command',
        id: generateMediaId(),
        command: 'nextcloud_create_directory',
        args: {
            server_url: nextcloudMediaCredentials.serverUrl,
            username: nextcloudMediaCredentials.username,
            password: nextcloudMediaCredentials.password,
            remote_path: remotePath
        }
    };

    if (websocketConnection && websocketConnection.readyState === WebSocket.OPEN) {
        websocketConnection.send(JSON.stringify(message));
    }
}

// Refresh NextCloud media
function refreshNextCloudMedia() {
    loadNextCloudMediaFiles();
}

// Delete media file
function deleteMediaFile(path) {
    if (!confirm(`Are you sure you want to delete this file?`)) return;

    const message = {
        type: 'command',
        id: generateMediaId(),
        command: 'nextcloud_delete_file',
        args: {
            server_url: nextcloudMediaCredentials.serverUrl,
            username: nextcloudMediaCredentials.username,
            password: nextcloudMediaCredentials.password,
            remote_path: path
        }
    };

    if (websocketConnection && websocketConnection.readyState === WebSocket.OPEN) {
        websocketConnection.send(JSON.stringify(message));
    }
}

// Delete current media (from modal)
function deleteCurrentMedia() {
    if (!currentMediaFile) return;
    $('#mediaPlayerModal').modal('hide');
    deleteMediaFile(currentMediaFile.path);
}

// Download current media
function downloadCurrentMedia() {
    if (!currentMediaFile) return;

    const message = {
        type: 'command',
        id: generateMediaId(),
        command: 'nextcloud_download_file',
        args: {
            server_url: nextcloudMediaCredentials.serverUrl,
            username: nextcloudMediaCredentials.username,
            password: nextcloudMediaCredentials.password,
            remote_path: currentMediaFile.path
        }
    };

    if (websocketConnection && websocketConnection.readyState === WebSocket.OPEN) {
        websocketConnection.send(JSON.stringify(message));
    }
}

// Download media file from response
function downloadMediaFileFromResponse(response) {
    if (!response.content) return;

    try {
        const byteCharacters = atob(response.content);
        const byteNumbers = new Array(byteCharacters.length);
        for (let i = 0; i < byteCharacters.length; i++) {
            byteNumbers[i] = byteCharacters.charCodeAt(i);
        }
        const byteArray = new Uint8Array(byteNumbers);
        const blob = new Blob([byteArray], { type: currentMediaFile.mimeType });

        const url = URL.createObjectURL(blob);
        const a = document.createElement('a');
        a.href = url;
        a.download = currentMediaFile.name;
        document.body.appendChild(a);
        a.click();
        document.body.removeChild(a);
        URL.revokeObjectURL(url);

        showMediaToast('File downloaded successfully', 'success');
    } catch (error) {
        showMediaToast('Error downloading file', 'error');
    }
}

// Open in new tab
function openInNewTab() {
    if (!currentMediaFile) return;
    const streamUrl = createStreamingUrl(currentMediaFile.path);
    window.open(streamUrl, '_blank');
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

    console.log('NextCloud Media integration initialized');
});