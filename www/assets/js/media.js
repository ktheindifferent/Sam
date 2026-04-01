// ███████     █████     ███    ███
// ██         ██   ██    ████  ████
// ███████    ███████    ██ ████ ██
//      ██    ██   ██    ██  ██  ██
// ███████ ██ ██   ██ ██ ██      ██ ██
// Copyright 2021-2026 The Open Sam Foundation (OSF)
// Developed by Caleb Mitchell Smith (ktheindifferent, PixelCoda, p0indexter)
// Licensed under GPLv3....see LICENSE file.

/**
 * Enhanced Media Center Controls
 * Supports gamepad, touch gestures, and keyboard navigation
 */

const rAF = window.mozRequestAnimationFrame || window.requestAnimationFrame;
let current_menu_item = 0;
let current_app_item = 0;
var focusable_app_area = document.getElementsByClassName('tab-pane active')[0].getElementsByClassName('controller-btn');
var focusable_menu_area = document.getElementsByClassName('tab-btn');

// Media player state
const MediaPlayer = {
    isPlaying: false,
    currentTrack: null,
    volume: 50,
    isMuted: false,
    queue: [],
    snapcastClients: [],
    activeSource: null,
    repeatMode: 'off', // off, all, one
    isShuffling: false,
    mediaSessionActive: false,
    ambientLightEnabled: false,
    bassBoostEnabled: false
};

// Initialize when DOM is ready
document.addEventListener('DOMContentLoaded', function() {
    initMediaCenter();
});

function initMediaCenter() {
    initGamepadSupport();
    initTouchControls();
    initKeyboardShortcuts();
    initSnapcastStatus();
    initMediaSession();
    initVoiceCommands();
    initAmbientLightSync();
    console.log('Media Center initialized');
}

// Gamepad support
window.addEventListener('gamepadconnected', function (e) {
    console.log('Gamepad connected:', e.gamepad.id);
    updateLoop();
});

window.addEventListener('gamepaddisconnected', function(e) {
    console.log('Gamepad disconnected:', e.gamepad.id);
});

function initGamepadSupport() {
    // Vibration button
    const btnVibration = document.querySelector('#btn-vibration');
    if (btnVibration) {
        btnVibration.addEventListener('click', function (e) {
            hapticFeedback();
        });
    }
}

function hapticFeedback() {
    var gp = navigator.getGamepads()[0];
    if (!gp || !gp.vibrationActuator) {
        // Fallback to navigator.vibrate for mobile
        if (navigator.vibrate) {
            navigator.vibrate(200);
        }
        return;
    }
    gp.vibrationActuator.playEffect('dual-rumble', {
        startDelay: 0,
        duration: 1500,
        weakMagnitude: 1,
        strongMagnitude: 1
    });
}

// Touch controls for media center
function initTouchControls() {
    // Add touch feedback to all media controls
    document.querySelectorAll('.media-control-btn, .snapcast-control-btn').forEach(btn => {
        btn.classList.add('touch-feedback');

        // Double-tap for play/pause
        let lastTap = 0;
        btn.addEventListener('touchend', function(e) {
            const currentTime = new Date().getTime();
            const tapLength = currentTime - lastTap;
            if (tapLength < 300 && tapLength > 0) {
                e.preventDefault();
                togglePlayPause();
                lastTap = currentTime;
            } else {
                lastTap = currentTime;
            }
        });
    });

    // Volume slider with touch
    const volumeSlider = document.querySelector('#volume-slider');
    if (volumeSlider) {
        volumeSlider.addEventListener('touchstart', function() {
            this.classList.add('active');
        }, { passive: true });

        volumeSlider.addEventListener('touchend', function() {
            this.classList.remove('active');
        }, { passive: true });

        volumeSlider.addEventListener('input', function() {
            setVolume(this.value);
        });
    }

    // Swipe gestures on media player
    const mediaPlayer = document.querySelector('#media-player, #snapcast-player');
    if (mediaPlayer) {
        let touchStartX = 0;
        let touchStartY = 0;
        
        mediaPlayer.addEventListener('touchstart', (e) => {
            touchStartX = e.touches[0].clientX;
            touchStartY = e.touches[0].clientY;
        }, { passive: true });
        
        mediaPlayer.addEventListener('touchend', (e) => {
            const touchEndX = e.changedTouches[0].clientX;
            const touchEndY = e.changedTouches[0].clientY;
            const deltaX = touchEndX - touchStartX;
            const deltaY = touchEndY - touchStartY;
            
            // Detect circular motion for volume quick control
            if (Math.abs(deltaX) < 10 && Math.abs(deltaY) < 10) {
                // Tap - toggle play/pause
                togglePlayPause();
                showSwipeHint('Play/Pause ⏯️');
            } else if (Math.abs(deltaX) > Math.abs(deltaY) * 2) {
                // Horizontal swipe
                if (deltaX > 50) {
                    previousTrack();
                    showSwipeHint('Previous Track ⬅️');
                } else if (deltaX < -50) {
                    nextTrack();
                    showSwipeHint('Next Track ➡️');
                }
            } else if (Math.abs(deltaY) > Math.abs(deltaX) * 2) {
                // Vertical swipe for volume
                if (deltaY > 50) {
                    increaseVolume();
                    showSwipeHint('Volume Up 🔊↑');
                } else if (deltaY < -50) {
                    decreaseVolume();
                    showSwipeHint('Volume Down 🔊↓');
                }
            }
        }, { passive: true });
        
        // Pinch gesture for brightness/ambient light
        let initialPinchDistance = null;
        mediaPlayer.addEventListener('touchmove', (e) => {
            if (e.touches.length === 2) {
                const distance = Math.hypot(
                    e.touches[0].clientX - e.touches[1].clientX,
                    e.touches[0].clientY - e.touches[1].clientY
                );
                
                if (initialPinchDistance === null) {
                    initialPinchDistance = distance;
                } else {
                    const delta = distance - initialPinchDistance;
                    if (Math.abs(delta) > 30) {
                        // Pinch gesture detected
                        if (MediaPlayer.ambientLightEnabled) {
                            const brightness = delta > 0 ? 
                                Math.min(100, MediaPlayer.volume + 10) : 
                                Math.max(0, MediaPlayer.volume - 10);
                            setVolume(brightness);
                            showSwipeHint(`Volume: ${brightness}%`);
                        }
                        initialPinchDistance = distance;
                    }
                }
            }
        }, { passive: true });
        
        mediaPlayer.addEventListener('touchstart', () => {
            initialPinchDistance = null;
        }, { passive: true });
    }
    
    // Initialize media browser touch controls
    initMediaBrowserTouch();
}

// Media browser touch interactions
function initMediaBrowserTouch() {
    const mediaGrids = document.querySelectorAll('.media-grid, .nextcloud-media-grid, .dropbox-media-grid, .seaweedfs-media-grid');
    
    mediaGrids.forEach(grid => {
        // Horizontal scroll with swipe
        let isDown = false;
        let startX;
        let scrollLeft;
        
        grid.addEventListener('touchstart', (e) => {
            isDown = true;
            startX = e.touches[0].pageX - grid.offsetLeft;
            scrollLeft = grid.scrollLeft;
            grid.style.transition = 'none';
        }, { passive: true });
        
        grid.addEventListener('touchmove', (e) => {
            if (!isDown) return;
            const x = e.touches[0].pageX - grid.offsetLeft;
            const walk = (x - startX) * 2;
            grid.scrollLeft = scrollLeft - walk;
        }, { passive: true });
        
        grid.addEventListener('touchend', () => {
            isDown = false;
            grid.style.transition = '';
        });
    });
}

// Keyboard shortcuts
function initKeyboardShortcuts() {
    document.addEventListener('keydown', function(e) {
        // Only handle shortcuts when not typing in an input
        if (e.target.tagName === 'INPUT' || e.target.tagName === 'TEXTAREA') {
            return;
        }

        // Media control shortcuts (work globally)
        switch(e.code) {
            case 'Space':
                if (e.target.tagName !== 'BUTTON') {
                    e.preventDefault();
                    togglePlayPause();
                }
                break;
            case 'ArrowRight':
                e.preventDefault();
                nextTrack();
                break;
            case 'ArrowLeft':
                e.preventDefault();
                previousTrack();
                break;
            case 'ArrowUp':
                e.preventDefault();
                increaseVolume();
                break;
            case 'ArrowDown':
                e.preventDefault();
                decreaseVolume();
                break;
            case 'KeyM':
                e.preventDefault();
                toggleMute();
                break;
            case 'KeyR':
                if (e.ctrlKey) {
                    e.preventDefault();
                    toggleRepeatMode();
                }
                break;
            case 'KeyS':
                if (e.ctrlKey) {
                    e.preventDefault();
                    toggleShuffle();
                }
                break;
            case 'KeyN':
                if (e.ctrlKey) {
                    e.preventDefault();
                    nextTrack();
                }
                break;
            case 'KeyP':
                if (e.ctrlKey) {
                    e.preventDefault();
                    previousTrack();
                }
                break;
        }
    });
}

// Toggle repeat mode
function toggleRepeatMode() {
    const modes = ['off', 'all', 'one'];
    const currentIndex = modes.indexOf(MediaPlayer.repeatMode);
    MediaPlayer.repeatMode = modes[(currentIndex + 1) % modes.length];
    
    const repeatBtn = document.querySelector('#repeat-btn');
    if (repeatBtn) {
        repeatBtn.innerHTML = MediaPlayer.repeatMode === 'off' 
            ? '<i class="fas fa-redo"></i>' 
            : MediaPlayer.repeatMode === 'all'
            ? '<i class="fas fa-redo"></i><span style="font-size:8px;position:absolute;">ALL</span>'
            : '<i class="fas fa-redo"></i><span style="font-size:8px;position:absolute;">1</span>';
    }
    
    showNotification(`Repeat: ${MediaPlayer.repeatMode}`, 'info');
}

// Toggle shuffle
function toggleShuffle() {
    MediaPlayer.isShuffling = !MediaPlayer.isShuffling;
    
    const shuffleBtn = document.querySelector('#shuffle-btn');
    if (shuffleBtn) {
        shuffleBtn.classList.toggle('active', MediaPlayer.isShuffling);
        shuffleBtn.innerHTML = MediaPlayer.isShuffling
            ? '<i class="fas fa-random"></i><span style="font-size:8px;position:absolute;">ON</span>'
            : '<i class="fas fa-random"></i>';
    }
    
    showNotification(`Shuffle: ${MediaPlayer.isShuffling ? 'ON' : 'OFF'}`, 'info');
}

// Media control functions - Integrated with Snapcast API
function togglePlayPause() {
    const action = MediaPlayer.isPlaying ? 'pause' : 'play';
    
    fetch(`/api/services/media/snapcast/${action}`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' }
    })
    .then(response => response.json())
    .then(data => {
        if (data.success) {
            MediaPlayer.isPlaying = action === 'play';
            updatePlayPauseButton();
            console.log('Snapcast:', data.message);
        } else {
            console.error('Failed to', action + ':', data.error || data.message);
            // Fallback to local state
            MediaPlayer.isPlaying = !MediaPlayer.isPlaying;
            updatePlayPauseButton();
        }
    })
    .catch(err => {
        console.warn('Snapcast API unavailable, using local state:', err);
        MediaPlayer.isPlaying = !MediaPlayer.isPlaying;
        updatePlayPauseButton();
    });
}

function updatePlayPauseButton() {
    const playBtn = document.querySelector('#play-pause-btn');
    if (playBtn) {
        playBtn.innerHTML = MediaPlayer.isPlaying
            ? '<i class="fas fa-pause"></i>'
            : '<i class="fas fa-play"></i>';
    }
    
    // Update mobile play icon too
    const mobilePlayIcon = document.querySelector('#mobile-play-icon');
    if (mobilePlayIcon) {
        mobilePlayIcon.className = MediaPlayer.isPlaying ? 'fas fa-pause' : 'fas fa-play';
    }
}

function nextTrack() {
    fetch('/api/services/media/snapcast/next', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' }
    })
    .then(response => response.json())
    .then(data => {
        if (data.success) {
            console.log('Snapcast:', data.message);
            showNotification('Next track', 'info');
        } else {
            console.warn('Next track:', data.message);
            showNotification('Next track (source-dependent)', 'info');
        }
    })
    .catch(err => {
        console.warn('Snapcast API unavailable:', err);
        showNotification('Next track', 'info');
    });
}

function previousTrack() {
    fetch('/api/services/media/snapcast/previous', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' }
    })
    .then(response => response.json())
    .then(data => {
        if (data.success) {
            console.log('Snapcast:', data.message);
            showNotification('Previous track', 'info');
        } else {
            console.warn('Previous track:', data.message);
            showNotification('Previous track (source-dependent)', 'info');
        }
    })
    .catch(err => {
        console.warn('Snapcast API unavailable:', err);
        showNotification('Previous track', 'info');
    });
}

function setVolume(level) {
    const volumeLevel = parseInt(level);
    MediaPlayer.volume = volumeLevel;
    
    fetch('/api/services/media/snapcast/volume', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ level: volumeLevel })
    })
    .then(response => response.json())
    .then(data => {
        if (data.success) {
            console.log('Snapcast volume:', data.message);
            updateVolumeUI(volumeLevel);
        } else {
            console.warn('Volume control:', data.message);
            updateVolumeUI(volumeLevel);
        }
    })
    .catch(err => {
        console.warn('Snapcast API unavailable, local volume only:', err);
        updateVolumeUI(volumeLevel);
    });
}

function updateVolumeUI(level) {
    const volumeSlider = document.querySelector('#volume-slider');
    if (volumeSlider) {
        volumeSlider.value = level;
    }
    
    // Update any volume display elements
    document.querySelectorAll('.volume-display').forEach(el => {
        el.textContent = level + '%';
    });
}

function increaseVolume() {
    setVolume(Math.min(100, MediaPlayer.volume + 10));
}

function decreaseVolume() {
    setVolume(Math.max(0, MediaPlayer.volume - 10));
}

function toggleMute() {
    fetch('/api/services/media/snapcast/mute', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' }
    })
    .then(response => response.json())
    .then(data => {
        if (data.success) {
            MediaPlayer.isMuted = data.muted;
            updateMuteButton();
            console.log('Snapcast:', data.message);
        } else {
            console.warn('Mute toggle:', data.message);
            // Fallback to local state
            MediaPlayer.isMuted = !MediaPlayer.isMuted;
            updateMuteButton();
        }
    })
    .catch(err => {
        console.warn('Snapcast API unavailable, using local state:', err);
        MediaPlayer.isMuted = !MediaPlayer.isMuted;
        updateMuteButton();
    });
}

function updateMuteButton() {
    const muteBtn = document.querySelector('#mute-btn');
    if (muteBtn) {
        muteBtn.innerHTML = MediaPlayer.isMuted
            ? '<i class="fas fa-volume-mute"></i>'
            : '<i class="fas fa-volume-up"></i>';
    }
}

// Snapcast integration
function initSnapcastStatus() {
    // Check Snapcast server status
    fetch('/api/services/snapcast/status')
        .then(response => response.json())
        .then(data => {
            updateSnapcastStatus(data);
            if (data && data.running) {
                // Initialize WebSocket for real-time updates if available
                initSnapcastWebSocket();
            }
        })
        .catch(err => {
            console.warn('Snapcast status unavailable:', err);
        });

    // Poll for updates every 5 seconds
    setInterval(() => {
        fetch('/api/services/snapcast/status')
            .then(response => response.json())
            .then(data => {
                updateSnapcastStatus(data);
            })
            .catch(() => {});
    }, 5000);
}

// WebSocket for real-time Snapcast updates
function initSnapcastWebSocket() {
    if (typeof WebSocket !== 'undefined') {
        try {
            const ws = new WebSocket(`ws://${window.location.host}/ws`);
            
            ws.onopen = () => {
                console.log('Snapcast WebSocket connected');
                ws.send(JSON.stringify({
                    type: 'subscribe',
                    channels: ['snapcast', 'media']
                }));
            };
            
            ws.onmessage = (event) => {
                try {
                    const data = JSON.parse(event.data);
                    if (data.type === 'service_status' && data.service === 'snapcast') {
                        updateSnapcastStatus(data.status);
                    }
                } catch (e) {
                    console.warn('WebSocket message parse error:', e);
                }
            };
            
            ws.onerror = (err) => {
                console.warn('Snapcast WebSocket error:', err);
            };
        } catch (e) {
            console.warn('Snapcast WebSocket init failed:', e);
        }
    }
}

function updateSnapcastStatus(data) {
    const statusEl = document.querySelector('#snapcast-status');
    if (statusEl) {
        if (data && data.running) {
            statusEl.innerHTML = '<span class="status-indicator status-running"></span> Snapcast Active';
            // Fetch connected clients
            fetch('/api/services/snapcast/clients')
                .then(response => response.json())
                .then(clients => {
                    MediaPlayer.snapcastClients = clients;
                    updateClientList(clients);
                })
                .catch(() => {});
        } else {
            statusEl.innerHTML = '<span class="status-indicator status-stopped"></span> Snapcast Offline';
        }
    }
}

function updateClientList(clients) {
    const clientList = document.querySelector('#snapcast-clients');
    if (clientList && clients) {
        clientList.innerHTML = clients.map(client => `
            <div class="snapcast-client" data-client-id="${client.id || ''}">
                <div class="client-header">
                    <i class="fas fa-${client.connected ? 'wifi' : 'wifi-off'} ${client.connected ? 'text-success' : 'text-danger'}"></i>
                    <span class="client-name">${client.name || 'Unknown'}</span>
                </div>
                <div class="client-controls">
                    <input type="range" class="client-volume" 
                           value="${client.volume?.percent ?? 100}" 
                           min="0" max="100"
                           onchange="setClientVolume('${client.id || ''}', this.value)">
                    <button class="btn btn-sm ${client.volume?.muted ? 'btn-warning' : 'btn-secondary'}"
                            onclick="toggleClientMute('${client.id || ''}')">
                        <i class="fas fa-${client.volume?.muted ? 'volume-mute' : 'volume-up'}"></i>
                    </button>
                </div>
                <div class="client-info">
                    <small>${client.ip || ''}</small>
                </div>
            </div>
        `).join('');
    }
}

// Individual client volume control
function setClientVolume(clientId, level) {
    if (!clientId) {
        setVolume(level);
        return;
    }
    
    fetch('/api/services/snapcast/volume', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ level: parseInt(level), client_id: clientId })
    })
    .then(response => response.json())
    .then(data => {
        if (data.success) {
            console.log('Client volume updated:', data.message);
        }
    })
    .catch(err => {
        console.warn('Failed to set client volume:', err);
    });
}

function toggleClientMute(clientId) {
    if (!clientId) {
        toggleMute();
        return;
    }
    
    fetch('/api/services/snapcast/mute', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ client_id: clientId })
    })
    .then(response => response.json())
    .then(data => {
        if (data.success) {
            console.log('Client muted:', data.muted);
            // Refresh client list
            setTimeout(() => {
                fetch('/api/services/snapcast/clients')
                    .then(response => response.json())
                    .then(clients => updateClientList(clients));
            }, 500);
        }
    })
    .catch(err => {
        console.warn('Failed to toggle client mute:', err);
    });
}

// Media Session API for browser integration
function initMediaSession() {
    if ('mediaSession' in navigator) {
        MediaPlayer.mediaSessionActive = true;
        
        navigator.mediaSession.setActionHandler('play', () => {
            if (!MediaPlayer.isPlaying) togglePlayPause();
        });
        
        navigator.mediaSession.setActionHandler('pause', () => {
            if (MediaPlayer.isPlaying) togglePlayPause();
        });
        
        navigator.mediaSession.setActionHandler('previoustrack', () => {
            previousTrack();
        });
        
        navigator.mediaSession.setActionHandler('nexttrack', () => {
            nextTrack();
        });
        
        navigator.mediaSession.setActionHandler('seekbackward', (details) => {
            // Optional: implement seek backward
        });
        
        navigator.mediaSession.setActionHandler('seekforward', (details) => {
            // Optional: implement seek forward
        });
        
        console.log('Media Session API initialized');
    }
}

function updateMediaSessionMetadata(track) {
    if (!MediaPlayer.mediaSessionActive || !('mediaSession' in navigator)) return;
    
    navigator.mediaSession.metadata = new MediaMetadata({
        title: track.title || 'Unknown',
        artist: track.artist || 'Unknown',
        album: track.album || 'Unknown',
        artwork: [
            { src: track.artwork || '/assets/img/music-placeholder.png', sizes: '512x512', type: 'image/png' }
        ]
    });
}

// Voice commands for media control
function initVoiceCommands() {
    if ('webkitSpeechRecognition' in window || 'SpeechRecognition' in window) {
        const SpeechRecognition = window.SpeechRecognition || window.webkitSpeechRecognition;
        const recognition = new SpeechRecognition();
        
        recognition.continuous = false;
        recognition.interimResults = false;
        recognition.lang = 'en-US';
        
        recognition.onresult = (event) => {
            const command = event.results[0][0].transcript.toLowerCase();
            console.log('Voice command:', command);
            
            if (command.includes('play') || command.includes('resume')) {
                if (!MediaPlayer.isPlaying) togglePlayPause();
            } else if (command.includes('pause') || command.includes('stop')) {
                if (MediaPlayer.isPlaying) togglePlayPause();
            } else if (command.includes('next') || command.includes('skip')) {
                nextTrack();
            } else if (command.includes('previous') || command.includes('back')) {
                previousTrack();
            } else if (command.includes('louder') || command.includes('volume up')) {
                increaseVolume();
            } else if (command.includes('quieter') || command.includes('volume down')) {
                decreaseVolume();
            } else if (command.includes('mute') || command.includes('unmute')) {
                toggleMute();
            } else if (command.includes('party mode') || command.includes('all zones')) {
                enablePartyMode();
            } else if (command.includes('zone') && command.includes('volume')) {
                handleZoneVolumeCommand(command);
            }
        };
        
        recognition.onerror = (event) => {
            console.warn('Speech recognition error:', event.error);
        };
        
        window.startVoiceCommand = () => {
            recognition.start();
            showNotification('Listening...', 'info');
        };
        
        console.log('Voice commands initialized');
    }
}

// Ambient light sync with LIFX
function initAmbientLightSync() {
    const ambientBtn = document.querySelector('#ambient-light-btn');
    if (ambientBtn) {
        ambientBtn.addEventListener('click', () => {
            MediaPlayer.ambientLightEnabled = !MediaPlayer.ambientLightEnabled;
            ambientBtn.classList.toggle('active', MediaPlayer.ambientLightEnabled);
            ambientBtn.classList.toggle('syncing', MediaPlayer.ambientLightEnabled);
            
            if (MediaPlayer.ambientLightEnabled) {
                syncLightsToMusic(true);
                showNotification('Ambient light sync enabled', 'success');
            } else {
                syncLightsToMusic(false);
                showNotification('Ambient light sync disabled', 'info');
            }
        });
    }
    
    const bassBtn = document.querySelector('#bass-boost-btn');
    if (bassBtn) {
        bassBtn.addEventListener('click', () => {
            MediaPlayer.bassBoostEnabled = !MediaPlayer.bassBoostEnabled;
            bassBtn.classList.toggle('active', MediaPlayer.bassBoostEnabled);
            showNotification(`Bass boost ${MediaPlayer.bassBoostEnabled ? 'enabled' : 'disabled'}`, 'info');
        });
    }
    
    // Show mobile controls on touch devices
    if (typeof is_touch_enabled === 'function' && is_touch_enabled()) {
        const mobileControls = document.querySelector('#media-center-mobile-controls');
        if (mobileControls) {
            mobileControls.classList.add('show');
        }
    }
}

// Toggle ambient light for mobile
function toggleAmbientLight() {
    const ambientBtn = document.querySelector('#ambient-light-btn');
    if (ambientBtn) {
        ambientBtn.click();
    }
}

// Sync LIFX lights to music
function syncLightsToMusic(enable) {
    if (enable) {
        // Pulse lights to beat detection (simulated with interval)
        if (!MediaPlayer.lightSyncInterval) {
            let hue = 0;
            MediaPlayer.lightSyncInterval = setInterval(() => {
                if (MediaPlayer.isPlaying && MediaPlayer.ambientLightEnabled) {
                    hue = (hue + 30) % 360;
                    $.ajax({
                        url: '/api/services/lifx/set_color',
                        method: 'POST',
                        contentType: 'application/json',
                        data: JSON.stringify({
                            selector: 'all',
                            color: `hue:${hue * 182}`,
                            duration: 0.5
                        })
                    });
                }
            }, 500);
        }
    } else {
        if (MediaPlayer.lightSyncInterval) {
            clearInterval(MediaPlayer.lightSyncInterval);
            MediaPlayer.lightSyncInterval = null;
        }
    }
}

// Party mode - sync audio to all Snapcast clients
function enablePartyMode() {
    fetch('/api/services/snapcast/party_mode', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' }
    })
    .then(response => response.json())
    .then(data => {
        if (data.success) {
            showNotification('Party Mode enabled! All zones playing', 'success');
            MediaPlayer.partyMode = true;
        } else {
            showNotification('Party Mode: Synchronized playback', 'info');
        }
    })
    .catch(err => {
        console.warn('Party mode API unavailable:', err);
        showNotification('Party Mode enabled locally', 'info');
    });
}

// Handle zone-specific voice commands
function handleZoneVolumeCommand(command) {
    const zoneMatch = command.match(/zone\s+(\w+)\s+(?:set\s+)?volume\s+(\d+)/);
    if (zoneMatch) {
        const zoneName = zoneMatch[1].toLowerCase();
        const volume = parseInt(zoneMatch[2]);
        
        const client = MediaPlayer.snapcastClients.find(c => 
            c.name.toLowerCase().includes(zoneName)
        );
        
        if (client) {
            setClientVolume(client.id, volume);
            showNotification(`Set ${zoneName} volume to ${volume}%`, 'info');
        } else {
            showNotification(`Zone "${zoneName}" not found`, 'warning');
        }
    }
}

// Original gamepad navigation loop
function nextAppItem(index) {
    index++;
    current_app_item = index % focusable_app_area.length;
    if (focusable_app_area[current_app_item]) {
        focusable_app_area[current_app_item].focus();
    }
}

function prevAppItem(index) {
    if(index > 0){
        index--;
        current_app_item = index % focusable_app_area.length;
    } else {
        current_app_item = focusable_app_area.length - 1;
    }
    if (focusable_app_area[current_app_item]) {
        focusable_app_area[current_app_item].focus();
    }
}

function nextMenuItem(index) {
    index++;
    current_menu_item = index % focusable_menu_area.length;
    if (focusable_menu_area[current_menu_item]) {
        focusable_menu_area[current_menu_item].focus();
    }
}

function prevMenuItem(index) {
    if(index > 0){
        index--;
        current_menu_item = index % focusable_menu_area.length;
    } else {
        current_menu_item = focusable_menu_area.length - 1;
    }
    if (focusable_menu_area[current_menu_item]) {
        focusable_menu_area[current_menu_item].focus();
    }
}

function updateLoop() {
    let gp = navigator.getGamepads()[0];
    if (!gp) {
        setTimeout(function () { rAF(updateLoop); }, 160);
        return;
    }
    focusable_app_area = document.getElementsByClassName('tab-pane active')[0].getElementsByClassName('controller-btn');

    // Debounce button presses
    const now = Date.now();
    if (!this.lastButtonPress) this.lastButtonPress = 0;
    const debounceTime = 200;

    if (now - this.lastButtonPress < debounceTime) {
        setTimeout(function () { rAF(updateLoop); }, 160);
        return;
    }

    switch (true) {
        case gp.buttons[0].pressed: // A button
            this.lastButtonPress = now;
            var element = document.activeElement;
            console.log(element);
            element.click();
            break;
        case gp.buttons[13].pressed || gp.axes[1] == 1: // Down D-pad or right stick
            this.lastButtonPress = now;
            nextMenuItem(current_menu_item);
            break;
        case gp.buttons[12].pressed || gp.axes[1] == -1: // Up D-pad or right stick
            this.lastButtonPress = now;
            prevMenuItem(current_menu_item);
            break;
        case gp.buttons[15].pressed || gp.axes[0] == 1: // Right D-pad or right stick
            this.lastButtonPress = now;
            nextAppItem(current_app_item);
            break;
        case gp.buttons[14].pressed || gp.axes[0] == -1: // Left D-pad or right stick
            this.lastButtonPress = now;
            prevAppItem(current_app_item);
            break;
        case gp.buttons[1].pressed: // B button - Back
            this.lastButtonPress = now;
            console.log('Back button pressed');
            break;
        default:
            break;
    }

    setTimeout(function () {
        rAF(updateLoop);
    }, 160);
}
