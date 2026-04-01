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
    bassBoostEnabled: false,
    crossfadeEnabled: false,
    crossfadeDuration: 5,
    equalizerPreset: 'flat',
    lastFmScrobbling: false,
    lyricsEnabled: false,
    sleepTimer: null,
    wakeUpTimer: null,
    playbackHistory: [],
    favorites: [],
    lifxSyncEnabled: false,
    lifxSyncMode: 'pulse',
    touchHoldTimer: null,
    lastMediaInteraction: 0,
    lifxBeatDetection: {
        threshold: 0.3,
        lastBeat: 0,
        beatCooldown: 150,
        adaptiveThreshold: 0.25,
        beatHistory: [],
        bpmEstimate: 120,
        lastBpmUpdate: 0,
        userThreshold: 0.3,
        enabled: false,
        sensitivity: 'medium',
        lastBeatTime: 0,
        beatCount: 0,
        sensitivity: 0.5,
        bpmHistory: [],
        beatIntensity: 0,
        lowPassFilter: 0.8,
        highPassFilter: 0.2,
        consecutiveBeats: 0,
        missedBeats: 0,
        dynamicThresholdFactor: 0.15,
        peakDecay: 0.95,
        energyHistory: []
    },
    lifxSceneMode: 'ambient',
    lifxColorHistory: [],
    lifxScreenAnalyzer: null,
    zonePresets: {},
    lastZoneAdjustment: 0,
    audioContext: null,
    analyser: null,
    mediaElementSource: null,
    bassBoostGain: null,
    equalizer: null,
    visualizationData: null,
    touchGestures: {
        enabled: true,
        sensitivity: 'medium',
        lastSwipeTime: 0,
        doubleTapTimeout: 300,
        longPressDelay: 500,
        swipeThreshold: 50,
        pinchSensitivity: 30,
        velocityThreshold: 0.3,
        edgeSwipeThreshold: 20,
        holdProgressInterval: 50,
        customThresholds: {
            low: { swipe: 80, pinch: 50, velocity: 0.2 },
            medium: { swipe: 50, pinch: 30, velocity: 0.3 },
            high: { swipe: 25, pinch: 15, velocity: 0.5 }
        }
    },
    miniPlayerVisible: false,
    queuePosition: 0,
    playbackSpeed: 1.0,
    lyricsVisible: false,
    nowPlayingHistory: [],
    ambientLightMode: 'spectrum',
    lifxBeatHistory: [],
    visualizationActive: false,
    colorCycleActive: false,
    partyMode: false,
    lightSyncInterval: null,
    beatDetectionInterval: null,
    auroraInterval: null,
    pulseInterval: null,
    touchTrailEnabled: true,
    touchTrailMaxCount: 10,
    gestureHistory: [],
    lastGestureTime: 0,
    gestureCooldown: 100
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
    initZonePresets();
    initMediaTouchGestures();
    initMiniPlayer();
    initNowPlayingToast();
    initEqualizerVisualization();
    loadMediaSyncPreferences();
    console.log('Media Center initialized');
}

function loadMediaSyncPreferences() {
    const savedSyncMode = localStorage.getItem('lifx_sync_mode');
    if (savedSyncMode) {
        MediaPlayer.lifxSyncMode = savedSyncMode;
    }
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
                showSwipeHint('Play/Pause ⏯️');
                lastTap = currentTime;
            } else {
                lastTap = currentTime;
            }
        });
        
        // Long press for quick actions
        let longPressTimer;
        btn.addEventListener('touchstart', function(e) {
            longPressTimer = setTimeout(() => {
                showQuickActionMenu(btn);
            }, 500);
        });
        
        btn.addEventListener('touchend', function() {
            if (longPressTimer) clearTimeout(longPressTimer);
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
        
        // Pinch gesture for volume control
        let initialPinchDistance = null;
        let pinchStartTime = 0;
        
        mediaPlayer.addEventListener('touchmove', (e) => {
            if (e.touches.length === 2) {
                const distance = Math.hypot(
                    e.touches[0].clientX - e.touches[1].clientX,
                    e.touches[0].clientY - e.touches[1].clientY
                );
                
                if (initialPinchDistance === null) {
                    initialPinchDistance = distance;
                    pinchStartTime = Date.now();
                } else {
                    const delta = distance - initialPinchDistance;
                    if (Math.abs(delta) > 30) {
                        const brightness = delta > 0 ? 
                            Math.min(100, MediaPlayer.volume + 10) : 
                            Math.max(0, MediaPlayer.volume - 10);
                        setVolume(brightness);
                        showSwipeHint(`Volume: ${brightness}%`);
                        initialPinchDistance = distance;
                    }
                }
            }
        }, { passive: true });
        
        mediaPlayer.addEventListener('touchstart', () => {
            initialPinchDistance = null;
            pinchStartTime = 0;
        }, { passive: true });
        
        // Edge swipe gestures for global controls
        let edgeTouchStartX = null;
        let edgeTouchStartY = null;
        
        document.addEventListener('touchstart', (e) => {
            const touch = e.touches[0];
            edgeTouchStartX = touch.clientX;
            edgeTouchStartY = touch.clientY;
            
            if (touch.clientX > window.innerWidth - 20) {
                MediaPlayer.edgeTouchActive = true;
            }
        }, { passive: true });
        
        document.addEventListener('touchend', (e) => {
            if (!MediaPlayer.edgeTouchActive) return;
            
            const touch = e.changedTouches[0];
            const deltaX = touch.clientX - edgeTouchStartX;
            const deltaY = touch.clientY - edgeTouchStartY;
            
            if (deltaX < -50 && Math.abs(deltaY) < 50) {
                nextTrack();
                showSwipeHint('Next Track', '⏭️');
            } else if (deltaX > 50 && Math.abs(deltaY) < 50) {
                previousTrack();
                showSwipeHint('Previous Track', '⏮️');
            } else if (deltaY < -50 && Math.abs(deltaX) < 50) {
                increaseVolume();
                showSwipeHint('Volume Up', '🔊');
            } else if (deltaY > 50 && Math.abs(deltaX) < 50) {
                decreaseVolume();
                showSwipeHint('Volume Down', '🔉');
            }
            
            MediaPlayer.edgeTouchActive = false;
        });
    }
    
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
        let touchStartTime;
        
        grid.addEventListener('touchstart', (e) => {
            isDown = true;
            startX = e.touches[0].pageX - grid.offsetLeft;
            scrollLeft = grid.scrollLeft;
            grid.style.transition = 'none';
            touchStartTime = Date.now();
        }, { passive: true });
        
        grid.addEventListener('touchmove', (e) => {
            if (!isDown) return;
            const x = e.touches[0].pageX - grid.offsetLeft;
            const walk = (x - startX) * 2;
            grid.scrollLeft = scrollLeft - walk;
        }, { passive: true });
        
        grid.addEventListener('touchend', (e) => {
            isDown = false;
            grid.style.transition = '';
            
            // Detect quick swipe for fast scrolling
            const touchEndTime = Date.now();
            const deltaX = e.changedTouches[0].pageX - (startX + grid.offsetLeft);
            const touchDuration = touchEndTime - touchStartTime;
            
            if (Math.abs(deltaX) > 100 && touchDuration < 300) {
                // Quick swipe detected - scroll by full page
                const scrollAmount = grid.clientWidth * 0.8;
                grid.scrollLeft += deltaX > 0 ? -scrollAmount : scrollAmount;
            }
        });
    });
    
    // Add three-finger swipe for quick actions
    let touchCount = 0;
    document.addEventListener('touchstart', (e) => {
        touchCount = e.touches.length;
    }, { passive: true });
    
    document.addEventListener('touchend', (e) => {
        if (touchCount === 3) {
            const touch = e.changedTouches[0];
            const deltaX = touch.clientX - (e.touches[0]?.clientX || touch.clientX);
            
            if (deltaX < -50) {
                showSwipeHint('Next Track ⏭️');
                nextTrack();
            } else if (deltaX > 50) {
                showSwipeHint('Previous Track ⏮️');
                previousTrack();
            }
        }
        touchCount = 0;
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

// Sleep timer functionality
function setSleepTimer(minutes) {
    if (MediaPlayer.sleepTimer) {
        clearTimeout(MediaPlayer.sleepTimer);
    }
    
    if (minutes > 0) {
        MediaPlayer.sleepTimer = setTimeout(() => {
            if (MediaPlayer.isPlaying) {
                togglePlayPause();
                showNotification('Sleep timer expired - playback stopped', 'info');
            }
        }, minutes * 60 * 1000);
        
        showNotification(`Sleep timer set for ${minutes} minutes`, 'success');
    }
}

// Wake up timer
function setWakeUpTimer(hours, minutes) {
    const now = new Date();
    const wakeTime = new Date();
    wakeTime.setHours(hours, minutes, 0, 0);
    
    if (wakeTime < now) {
        wakeTime.setDate(wakeTime.getDate() + 1);
    }
    
    const delay = wakeTime - now;
    
    MediaPlayer.wakeUpTimer = setTimeout(() => {
        if (!MediaPlayer.isPlaying) {
            togglePlayPause();
            setVolume(30);
            showNotification('Wake up! Music starting your day', 'success');
        }
    }, delay);
    
    showNotification(`Wake up timer set for ${hours}:${minutes.toString().padStart(2, '0')}`, 'success');
}

// Quick action menu for long press
function showQuickActionMenu(btn) {
    const actionId = btn.id;
    const actions = {
        'play-pause-btn': ['Play', 'Pause', 'Add to Queue'],
        'next-btn': ['Next Track', 'Skip +10s', 'Add to Favorites'],
        'prev-btn': ['Previous Track', 'Restart Track', 'Remove from Queue'],
        'volume-slider': ['Set Volume', 'Mute', 'Max Volume'],
        'mute-btn': ['Mute All', 'Unmute All', 'Set 50%']
    };
    
    if (actions[actionId] && typeof Swal !== 'undefined') {
        Swal.fire({
            title: 'Quick Actions',
            html: `
                <div class="quick-action-menu">
                    ${actions[actionId].map(action => 
                        `<button class="btn btn-sm btn-outline-primary m-1" onclick="handleQuickAction('${actionId}', '${action}')">${action}</button>`
                    ).join('')}
                </div>
            `,
            showConfirmButton: false,
            showCloseButton: true
        });
    }
}

function handleQuickAction(controlId, action) {
    switch(action) {
        case 'Play':
            if (!MediaPlayer.isPlaying) togglePlayPause();
            break;
        case 'Pause':
            if (MediaPlayer.isPlaying) togglePlayPause();
            break;
        case 'Next Track':
            nextTrack();
            break;
        case 'Previous Track':
            previousTrack();
            break;
        case 'Mute All':
            MediaPlayer.isMuted = true;
            updateMuteButton();
            break;
        case 'Unmute All':
            MediaPlayer.isMuted = false;
            updateMuteButton();
            break;
        case 'Max Volume':
            setVolume(100);
            break;
        case 'Set 50%':
            setVolume(50);
            break;
    }
    if (typeof Swal !== 'undefined') Swal.close();
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

function showSwipeHint(text, icon) {
    let hint = document.querySelector('.media-center-touch-hint');
    if (!hint) {
        hint = document.createElement('div');
        hint.className = 'media-center-touch-hint';
        document.body.appendChild(hint);
    }
    hint.innerHTML = `<span style="font-size: 32px; display: block; margin-bottom: 10px;">${icon || ''}</span>${text}`;
    hint.classList.add('visible');
    setTimeout(() => {
        hint.classList.remove('visible');
    }, 1500);
}

function showMediaTouchHint(text, icon, duration = 1500, position = 'center') {
    let hint = document.querySelector('.media-center-touch-hint');
    if (!hint) {
        hint = document.createElement('div');
        hint.className = 'media-center-touch-hint';
        document.body.appendChild(hint);
    }
    
    const iconSpan = icon ? `<span style="font-size: 32px; display: block; margin-bottom: 10px; animation: hint-bounce 0.5s ease;">${icon}</span>` : '';
    hint.innerHTML = `${iconSpan}${text}`;
    
    if (position === 'top') {
        hint.style.top = '20%';
        hint.style.transform = 'translateX(-50%)';
    } else if (position === 'bottom') {
        hint.style.top = '80%';
        hint.style.transform = 'translateX(-50%)';
    } else {
        hint.style.top = '50%';
        hint.style.transform = 'translate(-50%, -50%)';
    }
    
    hint.classList.add('visible');
    setTimeout(() => {
        hint.classList.remove('visible');
    }, duration);
}

function toggleLifxMediaSync() {
    MediaPlayer.lifxSyncEnabled = !MediaPlayer.lifxSyncEnabled;
    
    const syncBtn = document.querySelector('#lifx-sync-btn');
    if (syncBtn) {
        syncBtn.classList.toggle('active', MediaPlayer.lifxSyncEnabled);
    }
    
    if (MediaPlayer.lifxSyncEnabled) {
        showMediaTouchHint('LIFX Media Sync ON', '🎵💡');
        showNotification('Lights will sync with music rhythm', 'success');
        initAudioContext();
        startBeatDetection();
    } else {
        showMediaTouchHint('LIFX Media Sync OFF', '💡');
        showNotification('LIFX media sync disabled', 'info');
        stopBeatDetection();
    }
}

function pulseLifxWithBeat(beatStrength = 1.0) {
    if (!MediaPlayer.lifxSyncEnabled || !MediaPlayer.isPlaying) return;
    
    const now = Date.now();
    const beatDetection = MediaPlayer.beatDetection;
    const bpm = beatDetection.bpmEstimate || 120;
    const intensity = beatDetection.beatIntensity || beatStrength;
    
    if (now - beatDetection.lastBeatTime < beatDetection.beatCooldown) {
        return;
    }
    
    beatDetection.lastBeatTime = now;
    beatDetection.beatHistory.push(now);
    if (beatDetection.beatHistory.length > 8) {
        beatDetection.beatHistory.shift();
    }
    
    const targets = LifXTouchControls && LifXTouchControls.multiBulbSelection && LifXTouchControls.multiBulbSelection.length > 0
        ? LifXTouchControls.multiBulbSelection.join(',')
        : 'all';
    
    const syncMode = MediaPlayer.lifxSyncMode || 'pulse';
    
    if (syncMode === 'rainbow') {
        pulseLifxRainbow(beatStrength, targets);
    } else if (syncMode === 'ambient') {
        pulseLifxAmbient(beatStrength, targets);
    } else if (syncMode === 'strobe') {
        pulseLifxStrobe(beatStrength, targets);
    } else if (syncMode === 'zone') {
        pulseLifxZone(beatStrength, targets);
    } else {
        pulseLifxStandard(beatStrength, targets, bpm, intensity);
    }
    
    updateBpmDisplay();
}

function pulseLifxStandard(beatStrength, targets, bpm, intensity) {
    const beatDetection = MediaPlayer.beatDetection;
    const hueStep = Math.max(5, 30 - (intensity * 20));
    const hue = beatDetection.lastHue || 0;
    const newHue = (hue + hueStep) % 360;
    beatDetection.lastHue = newHue;
    
    const colorTemp = MediaPlayer.lifxSceneMode === 'warm' ? 2700 : 
                      MediaPlayer.lifxSceneMode === 'cool' ? 6500 : 4000;
    
    const pulseBrightness = 0.3 + (intensity * 0.7);
    const saturation = 50 + (intensity * 50);
    const duration = Math.max(0.05, 0.15 - (intensity * 0.1));
    
    $.ajax({
        url: '/api/services/lifx/set_state',
        method: 'POST',
        contentType: 'application/json',
        data: JSON.stringify({
            selector: targets === 'all' ? 'all' : `id:${targets}`,
            power: 'on',
            brightness: pulseBrightness,
            duration: duration
        })
    });
    
    $.ajax({
        url: '/api/services/lifx/set_color',
        method: 'POST',
        contentType: 'application/json',
        data: JSON.stringify({
            selector: targets === 'all' ? 'all' : `id:${targets}`,
            color: `hue:${Math.round(newHue * 182)} saturation:${Math.round(saturation)}%`,
            kelvin: colorTemp,
            duration: duration * 2
        }),
        error: () => {}
    });
}

function pulseLifxRainbow(beatStrength, targets) {
    const beatDetection = MediaPlayer.beatDetection;
    beatDetection.rainbowHue = (beatDetection.rainbowHue || 0) + 20;
    if (beatDetection.rainbowHue >= 360) beatDetection.rainbowHue = 0;
    
    const brightness = 0.5 + (beatStrength * 0.5);
    
    $.ajax({
        url: '/api/services/lifx/set_color',
        method: 'POST',
        contentType: 'application/json',
        data: JSON.stringify({
            selector: targets === 'all' ? 'all' : `id:${targets}`,
            color: `hue:${Math.round(beatDetection.rainbowHue * 182)} saturation:100% brightness:${brightness * 100}%`,
            duration: 0.1
        })
    });
}

function pulseLifxAmbient(beatStrength, targets) {
    const video = document.querySelector('video');
    if (!video) return pulseLifxStandard(beatStrength, targets, 120, beatStrength);
    
    const canvas = document.createElement('canvas');
    canvas.width = 10;
    canvas.height = 10;
    const ctx = canvas.getContext('2d');
    ctx.drawImage(video, 0, 0, 10, 10);
    const imageData = ctx.getImageData(0, 0, 10, 10);
    const data = imageData.data;
    
    let r = 0, g = 0, b = 0;
    for (let i = 0; i < data.length; i += 4) {
        r += data[i];
        g += data[i + 1];
        b += data[i + 2];
    }
    const pixelCount = data.length / 4;
    r = Math.round(r / pixelCount);
    g = Math.round(g / pixelCount);
    b = Math.round(b / pixelCount);
    
    const rgb = { r, g, b };
    const hsv = rgbToHsv(rgb.r, rgb.g, rgb.b);
    const brightness = 0.3 + (beatStrength * 0.7);
    
    $.ajax({
        url: '/api/services/lifx/set_color',
        method: 'POST',
        contentType: 'application/json',
        data: JSON.stringify({
            selector: targets === 'all' ? 'all' : `id:${targets}`,
            color: `hue:${Math.round(hsv.h * 182)} saturation:${Math.round(hsv.s * 100)}% brightness:${brightness * 100}%`,
            duration: 0.15
        })
    });
}

function pulseLifxStrobe(beatStrength, targets) {
    const beatDetection = MediaPlayer.beatDetection;
    const flashIntensity = beatStrength > 0.7 ? 1.0 : 0.5;
    const colors = ['#ff0000', '#00ff00', '#0000ff', '#ffff00', '#00ffff', '#ff00ff', '#ffffff'];
    
    if (!beatDetection.strobeIndex) beatDetection.strobeIndex = 0;
    const color = colors[beatDetection.strobeIndex % colors.length];
    beatDetection.strobeIndex++;
    
    const rgb = hexToRgb(color);
    const hsv = rgbToHsv(rgb.r, rgb.g, rgb.b);
    
    $.ajax({
        url: '/api/services/lifx/set_color',
        method: 'POST',
        contentType: 'application/json',
        data: JSON.stringify({
            selector: targets === 'all' ? 'all' : `id:${targets}`,
            color: `hue:${Math.round(hsv.h * 182)} saturation:${Math.round(hsv.s * 100)}% brightness:${flashIntensity * 100}%`,
            duration: 0.05
        })
    });
}

function pulseLifxZone(beatStrength, targets) {
    const beatDetection = MediaPlayer.beatDetection;
    const zones = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9];
    const zoneIndex = (beatDetection.zoneIndex || 0) % zones.length;
    beatDetection.zoneIndex = zoneIndex + 1;
    
    const hue = (zoneIndex * 36) + (beatStrength * 20);
    const brightness = 0.4 + (beatStrength * 0.6);
    
    $.ajax({
        url: '/api/services/lifx/zones',
        method: 'POST',
        contentType: 'application/json',
        data: JSON.stringify({
            selector: targets === 'all' ? 'all' : `id:${targets}`,
            start_index: zoneIndex,
            end_index: zoneIndex,
            color: `hue:${Math.round(hue * 182)} saturation:80% brightness:${brightness * 100}%`,
            duration: 0.1
        })
    });
}

function setLifxSyncMode(mode) {
    MediaPlayer.lifxSyncMode = mode;
    localStorage.setItem('lifx_sync_mode', mode);
    showMediaTouchHint(`Sync Mode: ${mode}`, '🎵');
    showNotification(`LIFX sync mode set to ${mode}`, 'info');
}

function hexToRgb(hex) {
    const result = /^#?([a-f\d]{2})([a-f\d]{2})([a-f\d]{2})$/i.exec(hex);
    return result ? {
        r: parseInt(result[1], 16),
        g: parseInt(result[2], 16),
        b: parseInt(result[3], 16)
    } : { r: 255, g: 255, b: 255 };
}

function rgbToHsv(r, g, b) {
    r /= 255; g /= 255; b /= 255;
    const max = Math.max(r, g, b), min = Math.min(r, g, b);
    let h, s, v = max;
    const d = max - min;
    s = max === 0 ? 0 : d / max;
    if (max === min) {
        h = 0;
    } else {
        switch (max) {
            case r: h = (g - b) / d + (g < b ? 6 : 0); break;
            case g: h = (b - r) / d + 2; break;
            case b: h = (r - g) / d + 4; break;
        }
        h /= 6;
    }
    return { h, s, v };
}

function setLifxSceneForMedia(sceneName) {
    if (LifXTouchControls && typeof LifXTouchControls.applyScene === 'function') {
        LifXTouchControls.applyScene(sceneName);
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
        
        // Add double-tap for color cycle mode
        let lastTap = 0;
        ambientBtn.addEventListener('touchend', function(e) {
            const currentTime = Date.now();
            if (currentTime - lastTap < 300) {
                cycleAmbientLightMode();
            }
            lastTap = currentTime;
        });
    }
    
    const bassBtn = document.querySelector('#bass-boost-btn');
    if (bassBtn) {
        bassBtn.addEventListener('click', () => {
            MediaPlayer.bassBoostEnabled = !MediaPlayer.bassBoostEnabled;
            bassBtn.classList.toggle('active', MediaPlayer.bassBoostEnabled);
            initAudioContext();
            applyBassBoost(MediaPlayer.bassBoostEnabled);
            showNotification(`Bass boost ${MediaPlayer.bassBoostEnabled ? 'enabled' : 'disabled'}`, 'info');
        });
    }
    
    // Add crossfade toggle
    const crossfadeBtn = document.querySelector('#crossfade-btn');
    if (crossfadeBtn) {
        crossfadeBtn.addEventListener('click', () => {
            MediaPlayer.crossfadeEnabled = !MediaPlayer.crossfadeEnabled;
            crossfadeBtn.classList.toggle('active', MediaPlayer.crossfadeEnabled);
            showNotification(`Crossfade ${MediaPlayer.crossfadeEnabled ? 'enabled' : 'disabled'}`, 'info');
        });
    }
    
    // Show mobile controls on touch devices
    if (typeof is_touch_enabled === 'function' && is_touch_enabled()) {
        const mobileControls = document.querySelector('#media-center-mobile-controls');
        if (mobileControls) {
            mobileControls.classList.add('show');
            
            // Add sleep timer button to mobile controls
            const sleepTimerBtn = document.createElement('button');
            sleepTimerBtn.className = 'media-center-mobile-btn';
            sleepTimerBtn.id = 'sleep-timer-btn';
            sleepTimerBtn.innerHTML = '<i class="fas fa-clock"></i>';
            sleepTimerBtn.onclick = showSleepTimerDialog;
            mobileControls.appendChild(sleepTimerBtn);
            
            // Add equalizer button
            const eqBtn = document.createElement('button');
            eqBtn.className = 'media-center-mobile-btn';
            eqBtn.id = 'equalizer-btn';
            eqBtn.innerHTML = '<i class="fas fa-sliders-h"></i>';
            eqBtn.onclick = showEqualizerDialog;
            mobileControls.appendChild(eqBtn);
        }
    }
}

// Audio context for advanced audio processing
function initAudioContext() {
    if (MediaPlayer.audioContext) return;
    
    try {
        const AudioContext = window.AudioContext || window.webkitAudioContext;
        if (!AudioContext) return;
        
        MediaPlayer.audioContext = new AudioContext();
        MediaPlayer.analyser = MediaPlayer.audioContext.createAnalyser();
        MediaPlayer.analyser.fftSize = 512;
        MediaPlayer.analyser.smoothingTimeConstant = 0.8;
        MediaPlayer.visualizationData = new Uint8Array(MediaPlayer.analyser.frequencyBinCount);
        
        MediaPlayer.lifxBeatHistory = [];
        MediaPlayer.lifxBeatDetection.threshold = 0.25;
        MediaPlayer.lifxBeatDetection.lastHue = 0;
        
        // Create equalizer
        MediaPlayer.equalizer = {
            low: MediaPlayer.audioContext.createBiquadFilter(),
            mid: MediaPlayer.audioContext.createBiquadFilter(),
            high: MediaPlayer.audioContext.createBiquadFilter()
        };
        
        MediaPlayer.equalizer.low.type = 'lowshelf';
        MediaPlayer.equalizer.low.frequency.value = 200;
        
        MediaPlayer.equalizer.mid.type = 'peaking';
        MediaPlayer.equalizer.mid.frequency.value = 1000;
        MediaPlayer.equalizer.mid.Q.value = 1;
        
        MediaPlayer.equalizer.high.type = 'highshelf';
        MediaPlayer.equalizer.high.frequency.value = 3000;
        
        // Create bass boost
        MediaPlayer.bassBoostGain = MediaPlayer.audioContext.createGain();
        MediaPlayer.bassBoostGain.gain.value = 0;
        
        console.log('Audio context initialized');
    } catch (e) {
        console.warn('Audio context init failed:', e);
    }
}

function updateBpmEstimate() {
    const history = MediaPlayer.lifxBeatDetection.beatHistory;
    if (history.length < 3) return;
    
    const now = Date.now();
    const recentBeats = history.slice(-5);
    const intervals = [];
    
    for (let i = 1; i < recentBeats.length; i++) {
        intervals.push(recentBeats[i].time - recentBeats[i-1].time);
    }
    
    if (intervals.length > 0) {
        const avgInterval = intervals.reduce((a, b) => a + b, 0) / intervals.length;
        const estimatedBpm = Math.round(60000 / avgInterval);
        
        if (estimatedBpm > 60 && estimatedBpm < 200) {
            MediaPlayer.lifxBeatDetection.bpmEstimate = estimatedBpm;
            MediaPlayer.lifxBeatDetection.lastBpmUpdate = now;
            MediaPlayer.lifxBeatDetection.beatCooldown = Math.max(100, 60000 / estimatedBpm - 50);
        }
    }
}

function updateBpmDisplay() {
    const bpmEl = document.querySelector('#bpm-display');
    const bpmValueEl = document.querySelector('#bpm-value');
    
    if (MediaPlayer.lifxBeatDetection.bpmEstimate && MediaPlayer.lifxSyncEnabled) {
        const bpm = Math.round(MediaPlayer.lifxBeatDetection.bpmEstimate);
        
        if (bpmEl) {
            bpmEl.classList.add('active', 'lifx-sync');
            bpmEl.style.display = 'inline-flex';
            if (MediaPlayer.beatDetection.lastBeatDetected) {
                bpmEl.classList.add('beat-detected');
                setTimeout(() => bpmEl.classList.remove('beat-detected'), 150);
            }
        }
        
        if (bpmValueEl) {
            bpmValueEl.textContent = bpm;
            bpmValueEl.style.color = '#ff6b6b';
            bpmValueEl.style.animation = 'bpm-pulse 0.5s ease-in-out infinite';
        }
    } else if (MediaPlayer.beatDetection.bpmEstimate && MediaPlayer.beatDetection.enabled) {
        const bpm = Math.round(MediaPlayer.beatDetection.bpmEstimate);
        
        if (bpmEl) {
            bpmEl.classList.add('active');
            bpmEl.classList.remove('lifx-sync');
            bpmEl.style.display = 'inline-flex';
            if (MediaPlayer.beatDetection.lastBeatDetected) {
                bpmEl.classList.add('beat-detected');
                setTimeout(() => bpmEl.classList.remove('beat-detected'), 150);
            }
        }
        
        if (bpmValueEl) {
            bpmValueEl.textContent = bpm;
            bpmValueEl.style.color = '#00d4ff';
        }
    } else {
        if (bpmEl) {
            bpmEl.classList.remove('active', 'lifx-sync', 'beat-detected');
        }
        
        if (bpmValueEl) {
            bpmValueEl.textContent = '--';
            bpmValueEl.style.color = '';
            bpmValueEl.style.animation = '';
        }
    }
}

function detectAudioBeat() {
    if (!MediaPlayer.audioContext || !MediaPlayer.analyser || !MediaPlayer.isPlaying) {
        return 0;
    }
    
    const dataArray = MediaPlayer.visualizationData;
    MediaPlayer.analyser.getByteFrequencyData(dataArray);
    
    const bassRange = dataArray.slice(0, 10);
    const bassEnergy = bassRange.reduce((a, b) => a + b, 0) / bassRange.length / 255;
    
    const midRange = dataArray.slice(10, 50);
    const midEnergy = midRange.reduce((a, b) => a + b, 0) / midRange.length / 255;
    
    const trebleRange = dataArray.slice(50, 128);
    const trebleEnergy = trebleRange.reduce((a, b) => a + b, 0) / trebleRange.length / 255;
    
    const beatDetection = MediaPlayer.lifxBeatDetection;
    
    const lowPassFiltered = (beatDetection.lowPassFilter * bassEnergy) + ((1 - beatDetection.lowPassFilter) * (beatDetection.lastBassEnergy || 0));
    beatDetection.lastBassEnergy = lowPassFiltered;
    
    const highPassFiltered = (beatDetection.highPassFilter * trebleEnergy) + ((1 - beatDetection.highPassFilter) * (beatDetection.lastTrebleEnergy || 0));
    beatDetection.lastTrebleEnergy = highPassFiltered;
    
    const avgEnergy = (lowPassFiltered * 0.6) + (midEnergy * 0.25) + (highPassFiltered * 0.15);
    
    beatDetection.energyHistory.push(avgEnergy);
    if (beatDetection.energyHistory.length > 30) {
        beatDetection.energyHistory.shift();
    }
    
    const history = beatDetection.beatHistory;
    let recentAvgStrength = 0.3;
    if (history.length > 0) {
        recentAvgStrength = history.reduce((a, b) => a + b.strength, 0) / history.length;
    }
    
    const avgEnergyHistory = beatDetection.energyHistory.reduce((a, b) => a + b, 0) / beatDetection.energyHistory.length;
    const energyVariance = beatDetection.energyHistory.reduce((sum, e) => sum + Math.pow(e - avgEnergyHistory, 2), 0) / beatDetection.energyHistory.length;
    const stdDev = Math.sqrt(energyVariance);
    
    const sensitivityMultiplier = beatDetection.sensitivity === 'high' ? 0.7 : beatDetection.sensitivity === 'low' ? 1.3 : 1.0;
    const dynamicThreshold = Math.max(0.12, Math.min(0.5, 
        (avgEnergyHistory * (1 - beatDetection.dynamicThresholdFactor)) + 
        (recentAvgStrength * beatDetection.dynamicThresholdFactor) + 
        (stdDev * sensitivityMultiplier)));
    
    const beatStrength = (lowPassFiltered * 0.7) + (midEnergy * 0.2) + (highPassFiltered * 0.1);
    
    if (beatStrength > dynamicThreshold) {
        const now = Date.now();
        const timeSinceLastBeat = now - beatDetection.lastBeat;
        
        const minInterval = beatDetection.bpmEstimate > 0 ? 
            Math.max(80, (60000 / beatDetection.bpmEstimate) * 0.35) : 150;
        
        if (timeSinceLastBeat > minInterval) {
            const normalizedStrength = Math.min(1.0, 0.4 + ((beatStrength - dynamicThreshold) / 0.5));
            
            beatDetection.lastBeat = now;
            beatDetection.lastBeatTime = now;
            beatDetection.beatCount++;
            beatDetection.beatIntensity = normalizedStrength;
            beatDetection.consecutiveBeats = (beatDetection.consecutiveBeats || 0) + 1;
            beatDetection.missedBeats = 0;
            
            beatDetection.beatHistory.push({
                time: now,
                strength: normalizedStrength,
                energy: beatStrength
            });
            if (beatDetection.beatHistory.length > 8) {
                beatDetection.beatHistory.shift();
            }
            
            const bpm = estimateBPM(now);
            if (bpm && bpm > 60 && bpm < 200) {
                const smoothingFactor = 0.3;
                beatDetection.bpmEstimate = Math.round(
                    (beatDetection.bpmEstimate * (1 - smoothingFactor)) + (bpm * smoothingFactor)
                );
                beatDetection.bpmEstimate = Math.max(60, Math.min(200, beatDetection.bpmEstimate));
            }
            
            const estimatedBeatInterval = 60000 / (beatDetection.bpmEstimate || 120);
            beatDetection.beatCooldown = Math.max(100, estimatedBeatInterval * 0.4);
            
            beatDetection.peakEnergy = Math.max(beatDetection.peakEnergy || 0, beatStrength);
            beatDetection.peakDecay = 0.96;
            
            if (beatDetection.peakEnergy) {
                beatDetection.threshold = beatDetection.peakEnergy * beatDetection.dynamicThresholdFactor;
                beatDetection.peakEnergy *= beatDetection.peakDecay;
            }
            
            pulseLifxWithBeat(normalizedStrength);
            updateBpmDisplay();
            return normalizedStrength;
        }
    } else {
        beatDetection.missedBeats = (beatDetection.missedBeats || 0) + 1;
        if (beatDetection.missedBeats > 8) {
            beatDetection.consecutiveBeats = 0;
            beatDetection.peakEnergy = (beatDetection.peakEnergy || 0) * 0.9;
        }
    }
    
    if (beatDetection.peakEnergy) {
        beatDetection.peakEnergy *= beatDetection.peakDecay;
    }
    
    return 0;
}

function estimateBPM(currentTime) {
    const bpmHistory = MediaPlayer.lifxBeatDetection.bpmHistory;
    const now = currentTime || Date.now();
    
    if (MediaPlayer.lifxBeatDetection.lastBeat > 0) {
        const interval = now - MediaPlayer.lifxBeatDetection.lastBeat;
        if (interval > 200 && interval < 2000) {
            const instantBPM = 60000 / interval;
            bpmHistory.push(instantBPM);
            if (bpmHistory.length > 10) {
                bpmHistory.shift();
            }
            
            if (bpmHistory.length >= 3) {
                const avgBPM = bpmHistory.reduce((a, b) => a + b, 0) / bpmHistory.length;
                return Math.round(avgBPM);
            }
        }
    }
    
    return null;
}

function startBeatDetection() {
    if (!MediaPlayer.beatDetectionInterval && MediaPlayer.lifxSyncEnabled) {
        MediaPlayer.beatDetectionInterval = setInterval(() => {
            if (MediaPlayer.isPlaying && MediaPlayer.lifxSyncEnabled) {
                const beatStrength = detectAudioBeat();
                if (beatStrength > 0) {
                    MediaPlayer.lifxBeatDetection.consecutiveBeats = (MediaPlayer.lifxBeatDetection.consecutiveBeats || 0) + 1;
                } else {
                    MediaPlayer.lifxBeatDetection.consecutiveBeats = 0;
                }
            }
        }, 100);
        console.log('Beat detection started');
    }
}

function stopBeatDetection() {
    if (MediaPlayer.beatDetectionInterval) {
        clearInterval(MediaPlayer.beatDetectionInterval);
        MediaPlayer.beatDetectionInterval = null;
        console.log('Beat detection stopped');
    }
}

function applyBassBoost(enabled) {
    if (!MediaPlayer.audioContext) return;
    
    MediaPlayer.bassBoostGain.gain.value = enabled ? 0.3 : 0;
    showNotification(`Bass boost ${enabled ? 'active' : 'disabled'}`, 'info');
}

function setEqualizerBands(low, mid, high) {
    if (!MediaPlayer.equalizer) return;
    
    MediaPlayer.equalizer.low.gain.value = low;
    MediaPlayer.equalizer.mid.gain.value = mid;
    MediaPlayer.equalizer.high.gain.value = high;
}

function showEqualizerDialog() {
    if (typeof Swal === 'undefined') {
        alert('Equalizer requires SweetAlert2');
        return;
    }
    
    initAudioContext();
    
    Swal.fire({
        title: 'Equalizer',
        html: `
            <div style="padding: 20px;">
                <div style="margin-bottom: 20px;">
                    <label style="color: #adb5bd; display: block; margin-bottom: 10px;">Bass (200Hz)</label>
                    <input type="range" id="eq-low" min="-12" max="12" value="0" step="1" 
                           style="width: 100%;" oninput="updateEqualizerDisplay()">
                    <span id="eq-low-value" style="color: #00d4ff;">0 dB</span>
                </div>
                <div style="margin-bottom: 20px;">
                    <label style="color: #adb5bd; display: block; margin-bottom: 10px;">Mid (1kHz)</label>
                    <input type="range" id="eq-mid" min="-12" max="12" value="0" step="1" 
                           style="width: 100%;" oninput="updateEqualizerDisplay()">
                    <span id="eq-mid-value" style="color: #00d4ff;">0 dB</span>
                </div>
                <div style="margin-bottom: 20px;">
                    <label style="color: #adb5bd; display: block; margin-bottom: 10px;">Treble (3kHz)</label>
                    <input type="range" id="eq-high" min="-12" max="12" value="0" step="1" 
                           style="width: 100%;" oninput="updateEqualizerDisplay()">
                    <span id="eq-high-value" style="color: #00d4ff;">0 dB</span>
                </div>
                <div style="display: flex; gap: 10px; justify-content: center; margin-top: 20px;">
                    <button class="btn btn-sm btn-outline-primary" onclick="applyEqualizerPreset('flat')">Flat</button>
                    <button class="btn btn-sm btn-outline-primary" onclick="applyEqualizerPreset('bass')">Bass Boost</button>
                    <button class="btn btn-sm btn-outline-primary" onclick="applyEqualizerPreset('vocal')">Vocal</button>
                    <button class="btn btn-sm btn-outline-primary" onclick="applyEqualizerPreset('bright')">Bright</button>
                </div>
            </div>
        `,
        showConfirmButton: false,
        showCloseButton: true,
        width: '400px'
    });
}

function updateEqualizerDisplay() {
    const low = document.getElementById('eq-low');
    const mid = document.getElementById('eq-mid');
    const high = document.getElementById('eq-high');
    
    if (low) document.getElementById('eq-low-value').textContent = `${low.value > 0 ? '+' : ''}${low.value} dB`;
    if (mid) document.getElementById('eq-mid-value').textContent = `${mid.value > 0 ? '+' : ''}${mid.value} dB`;
    if (high) document.getElementById('eq-high-value').textContent = `${high.value > 0 ? '+' : ''}${high.value} dB`;
    
    setEqualizerBands(parseInt(low?.value || 0), parseInt(mid?.value || 0), parseInt(high?.value || 0));
}

function applyEqualizerPreset(preset) {
    const presets = {
        flat: [0, 0, 0],
        bass: [8, 2, 4],
        vocal: [-2, 4, 2],
        bright: [2, 0, 6],
        rock: [5, 3, 6],
        jazz: [4, 2, 5],
        classical: [6, 2, 4],
        pop: [3, 4, 5]
    };
    
    const [low, mid, high] = presets[preset] || [0, 0, 0];
    setEqualizerBands(low, mid, high);
    
    // Update sliders
    const lowSlider = document.getElementById('eq-low');
    const midSlider = document.getElementById('eq-mid');
    const highSlider = document.getElementById('eq-high');
    
    if (lowSlider) lowSlider.value = low;
    if (midSlider) midSlider.value = mid;
    if (highSlider) highSlider.value = high;
    
    updateEqualizerDisplay();
    showNotification(`Equalizer preset: ${preset}`, 'info');
}

// Cycle through ambient light modes
function cycleAmbientLightMode() {
    const modes = ['spectrum', 'warm', 'cool', 'beat', 'visualizer', 'aurora', 'pulse', 'fire', 'ocean', 'neon'];
    const currentIndex = modes.indexOf(MediaPlayer.ambientLightMode || 'spectrum');
    MediaPlayer.ambientLightMode = modes[(currentIndex + 1) % modes.length];
    showNotification(`Ambient mode: ${MediaPlayer.ambientLightMode}`, 'info');
    
    if (MediaPlayer.ambientLightMode === 'visualizer') {
        startAudioVisualization();
    } else if (MediaPlayer.ambientLightMode === 'aurora') {
        startAuroraEffect();
    } else if (MediaPlayer.ambientLightMode === 'pulse') {
        startPulseEffect();
    } else if (MediaPlayer.ambientLightMode === 'fire') {
        startFireEffect();
    } else if (MediaPlayer.ambientLightMode === 'ocean') {
        startOceanEffect();
    } else if (MediaPlayer.ambientLightMode === 'neon') {
        startNeonEffect();
    } else {
        stopAudioVisualization();
        stopAuroraEffect();
        stopPulseEffect();
        stopFireEffect();
        stopOceanEffect();
        stopNeonEffect();
    }
}

// Audio visualization for LIFX lights
function startAudioVisualization() {
    if (!MediaPlayer.audioContext) {
        initAudioContext();
    }
    
    if (!MediaPlayer.visualizationActive && MediaPlayer.audioContext && MediaPlayer.analyser) {
        MediaPlayer.visualizationActive = true;
        visualizeAudio();
        showMediaTouchHint('Audio Visualization ON', '🎨');
    }
}

function stopAudioVisualization() {
    MediaPlayer.visualizationActive = false;
    showMediaTouchHint('Audio Visualization OFF', '🎨');
}

// Aurora borealis effect
function startAuroraEffect() {
    if (!MediaPlayer.auroraInterval) {
        let hue1 = 120;
        let hue2 = 180;
        let hue3 = 240;
        
        MediaPlayer.auroraInterval = setInterval(() => {
            const targets = LifXTouchControls && LifXTouchControls.multiBulbSelection && LifXTouchControls.multiBulbSelection.length > 0
                ? LifXTouchControls.multiBulbSelection.join(',')
                : 'all';
            
            const brightness = 40 + Math.sin(Date.now() / 1000) * 20;
            
            $.ajax({
                url: '/api/services/lifx/set_color',
                method: 'POST',
                contentType: 'application/json',
                data: JSON.stringify({
                    selector: targets === 'all' ? 'all' : `id:${targets}`,
                    color: `hue:${Math.round(hue1 * 182)} saturation:80%`,
                    brightness: brightness / 100,
                    duration: 1.0
                })
            });
            
            hue1 = (hue1 + 0.5) % 360;
            hue2 = (hue2 + 0.3) % 360;
            hue3 = (hue3 + 0.7) % 360;
        }, 100);
        
        showNotification('Aurora effect started', 'info');
    }
}

function stopAuroraEffect() {
    if (MediaPlayer.auroraInterval) {
        clearInterval(MediaPlayer.auroraInterval);
        MediaPlayer.auroraInterval = null;
        showNotification('Aurora effect stopped', 'info');
    }
}

// Pulse effect
function startPulseEffect() {
    if (!MediaPlayer.pulseInterval) {
        MediaPlayer.pulseInterval = setInterval(() => {
            const targets = LifXTouchControls && LifXTouchControls.multiBulbSelection && LifXTouchControls.multiBulbSelection.length > 0
                ? LifXTouchControls.multiBulbSelection.join(',')
                : 'all';
            
            const brightness = 30 + Math.sin(Date.now() / 500) * 30;
            const hue = (Date.now() / 50) % 360;
            
            $.ajax({
                url: '/api/services/lifx/set_color',
                method: 'POST',
                contentType: 'application/json',
                data: JSON.stringify({
                    selector: targets === 'all' ? 'all' : `id:${targets}`,
                    color: `hue:${Math.round(hue * 182)} saturation:70%`,
                    brightness: brightness / 100,
                    duration: 0.5
                })
            });
        }, 100);
        
        showNotification('Pulse effect started', 'info');
    }
}

function stopPulseEffect() {
    if (MediaPlayer.pulseInterval) {
        clearInterval(MediaPlayer.pulseInterval);
        MediaPlayer.pulseInterval = null;
        showNotification('Pulse effect stopped', 'info');
    }
}

// Fire effect - warm flickering colors
function startFireEffect() {
    if (!MediaPlayer.fireInterval) {
        MediaPlayer.fireInterval = setInterval(() => {
            const targets = LifXTouchControls && LifXTouchControls.multiBulbSelection && LifXTouchControls.multiBulbSelection.length > 0
                ? LifXTouchControls.multiBulbSelection.join(',')
                : 'all';
            
            const brightness = 40 + Math.random() * 30;
            const kelvin = 1800 + Math.random() * 600;
            const hue = (20 + Math.random() * 20) * 182;
            
            $.ajax({
                url: '/api/services/lifx/set_color',
                method: 'POST',
                contentType: 'application/json',
                data: JSON.stringify({
                    selector: targets === 'all' ? 'all' : `id:${targets}`,
                    color: `hue:${Math.round(hue)} saturation:80%`,
                    brightness: brightness / 100,
                    duration: 0.3
                })
            });
        }, 200 + Math.random() * 300);
        
        showNotification('Fire effect started', 'info');
    }
}

function stopFireEffect() {
    if (MediaPlayer.fireInterval) {
        clearInterval(MediaPlayer.fireInterval);
        MediaPlayer.fireInterval = null;
        showNotification('Fire effect stopped', 'info');
    }
}

// Ocean effect - cool flowing blues and greens
function startOceanEffect() {
    if (!MediaPlayer.oceanInterval) {
        let baseHue = 180;
        MediaPlayer.oceanInterval = setInterval(() => {
            const targets = LifXTouchControls && LifXTouchControls.multiBulbSelection && LifXTouchControls.multiBulbSelection.length > 0
                ? LifXTouchControls.multiBulbSelection.join(',')
                : 'all';
            
            baseHue = (baseHue + 2) % 60;
            const hue = (180 + Math.sin(Date.now() / 2000) * 30 + baseHue) * 182;
            const brightness = 50 + Math.sin(Date.now() / 1500) * 20;
            const saturation = 60 + Math.sin(Date.now() / 1000) * 20;
            
            $.ajax({
                url: '/api/services/lifx/set_color',
                method: 'POST',
                contentType: 'application/json',
                data: JSON.stringify({
                    selector: targets === 'all' ? 'all' : `id:${targets}`,
                    color: `hue:${Math.round(hue)} saturation:${Math.round(saturation)}%`,
                    brightness: brightness / 100,
                    duration: 0.8
                })
            });
        }, 500);
        
        showNotification('Ocean effect started', 'info');
    }
}

function stopOceanEffect() {
    if (MediaPlayer.oceanInterval) {
        clearInterval(MediaPlayer.oceanInterval);
        MediaPlayer.oceanInterval = null;
        showNotification('Ocean effect stopped', 'info');
    }
}

// Neon effect - vibrant cycling colors
function startNeonEffect() {
    if (!MediaPlayer.neonInterval) {
        let hue = 0;
        MediaPlayer.neonInterval = setInterval(() => {
            const targets = LifXTouchControls && LifXTouchControls.multiBulbSelection && LifXTouchControls.multiBulbSelection.length > 0
                ? LifXTouchControls.multiBulbSelection.join(',')
                : 'all';
            
            hue = (hue + 5) % 360;
            const brightness = 70 + Math.sin(Date.now() / 300) * 20;
            
            $.ajax({
                url: '/api/services/lifx/set_color',
                method: 'POST',
                contentType: 'application/json',
                data: JSON.stringify({
                    selector: targets === 'all' ? 'all' : `id:${targets}`,
                    color: `hue:${Math.round(hue * 182)} saturation:100%`,
                    brightness: brightness / 100,
                    duration: 0.4
                })
            });
        }, 150);
        
        showNotification('Neon effect started', 'info');
    }
}

function stopNeonEffect() {
    if (MediaPlayer.neonInterval) {
        clearInterval(MediaPlayer.neonInterval);
        MediaPlayer.neonInterval = null;
        showNotification('Neon effect stopped', 'info');
    }
}

function visualizeAudio() {
    if (!MediaPlayer.visualizationActive || !MediaPlayer.isPlaying) {
        setTimeout(() => visualizeAudio(), 100);
        return;
    }
    
    const analyser = MediaPlayer.analyser;
    const dataArray = MediaPlayer.visualizationData;
    analyser.getByteFrequencyData(dataArray);
    
    // Calculate average energy in different frequency bands
    const bass = dataArray.slice(0, 10).reduce((a, b) => a + b, 0) / 10;
    const mid = dataArray.slice(10, 50).reduce((a, b) => a + b, 0) / 40;
    const treble = dataArray.slice(50, 128).reduce((a, b) => a + b, 0) / 78;
    
    // Map to colors
    const bassHue = (bass / 255) * 60; // Reds to yellows
    const midHue = ((mid / 255) * 120) + 120; // Greens to cyans
    const trebleHue = ((treble / 255) * 120) + 240; // Blues to magentas
    
    const dominantHue = bass > mid && bass > treble ? bassHue :
                        mid > treble ? midHue : trebleHue;
    
    const targets = LifXTouchControls && LifXTouchControls.multiBulbSelection && LifXTouchControls.multiBulbSelection.length > 0
        ? LifXTouchControls.multiBulbSelection.join(',')
        : 'all';
    
    $.ajax({
        url: '/api/services/lifx/set_color',
        method: 'POST',
        contentType: 'application/json',
        data: JSON.stringify({
            selector: targets === 'all' ? 'all' : `id:${targets}`,
            color: `hue:${Math.round(dominantHue * 182)} saturation:${Math.round((Math.max(bass, mid, treble) / 255) * 100)}%`,
            brightness: Math.round(Math.max(bass, mid, treble) / 255 * 100),
            duration: 0.1
        })
    });
    
    setTimeout(() => visualizeAudio(), 100);
}

// Sleep timer dialog
function showSleepTimerDialog() {
    if (typeof Swal !== 'undefined') {
        Swal.fire({
            title: 'Sleep Timer',
            html: `
                <div class="sleep-timer-options">
                    <button class="btn btn-sm btn-outline-primary m-1" onclick="setSleepTimer(15)">15 min</button>
                    <button class="btn btn-sm btn-outline-primary m-1" onclick="setSleepTimer(30)">30 min</button>
                    <button class="btn btn-sm btn-outline-primary m-1" onclick="setSleepTimer(45)">45 min</button>
                    <button class="btn btn-sm btn-outline-primary m-1" onclick="setSleepTimer(60)">1 hour</button>
                    <button class="btn btn-sm btn-outline-danger m-1" onclick="setSleepTimer(0)">Cancel</button>
                </div>
            `,
            showConfirmButton: false,
            showCloseButton: true
        });
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
        if (!MediaPlayer.lightSyncInterval) {
            let hue = 0;
            let brightness = 50;
            let direction = 1;
            
            MediaPlayer.lightSyncInterval = setInterval(() => {
                if (MediaPlayer.isPlaying && MediaPlayer.ambientLightEnabled) {
                    hue = (hue + 30 * direction) % 360;
                    
                    if (MediaPlayer.lifxSceneMode === 'spectrum') {
                        brightness = 50 + Math.sin(Date.now() / 500) * 30;
                    } else if (MediaPlayer.lifxSceneMode === 'warm') {
                        hue = 30;
                        brightness = 40;
                    } else if (MediaPlayer.lifxSceneMode === 'cool') {
                        hue = 200;
                        brightness = 60;
                    } else if (MediaPlayer.lifxSceneMode === 'beat') {
                        pulseLifxWithBeat();
                        return;
                    }
                    
                    $.ajax({
                        url: '/api/services/lifx/set_color',
                        method: 'POST',
                        contentType: 'application/json',
                        data: JSON.stringify({
                            selector: 'all',
                            color: `hue:${hue * 182}`,
                            brightness: brightness / 100,
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

// Toggle crossfade
function toggleCrossfade() {
    MediaPlayer.crossfadeEnabled = !MediaPlayer.crossfadeEnabled;
    
    const crossfadeBtn = document.querySelector('#crossfade-btn');
    if (crossfadeBtn) {
        crossfadeBtn.classList.toggle('active', MediaPlayer.crossfadeEnabled);
    }
    
    showNotification(`Crossfade ${MediaPlayer.crossfadeEnabled ? 'enabled' : 'disabled'}`, 'info');
}

// Zone presets for quick multi-zone configuration
function initZonePresets() {
    const saved = localStorage.getItem('sam_zone_presets');
    if (saved) {
        try {
            MediaPlayer.zonePresets = JSON.parse(saved);
        } catch (e) {
            console.warn('Failed to load zone presets:', e);
        }
    }
}

function saveZonePreset(name, config) {
    MediaPlayer.zonePresets[name] = config;
    localStorage.setItem('sam_zone_presets', JSON.stringify(MediaPlayer.zonePresets));
    showNotification(`Zone preset "${name}" saved`, 'success');
}

function loadZonePreset(name) {
    const preset = MediaPlayer.zonePresets[name];
    if (preset) {
        preset.forEach(zone => {
            setClientVolume(zone.id, zone.volume);
            if (zone.muted) toggleClientMute(zone.id);
        });
        showNotification(`Loaded preset "${name}"`, 'success');
    } else {
        showNotification(`Preset "${name}" not found`, 'warning');
    }
}

function createZonePresetFromCurrent(name) {
    const config = MediaPlayer.snapcastClients.map(client => ({
        id: client.id,
        name: client.name,
        volume: client.volume?.percent || 50,
        muted: client.volume?.muted || false
    }));
    saveZonePreset(name, config);
}

// Add track to favorites
function addToFavorites(trackId) {
    if (!MediaPlayer.favorites.includes(trackId)) {
        MediaPlayer.favorites.push(trackId);
        showNotification('Added to favorites', 'success');
        localStorage.setItem('sam_media_favorites', JSON.stringify(MediaPlayer.favorites));
    }
}

// Remove from favorites
function removeFromFavorites(trackId) {
    const index = MediaPlayer.favorites.indexOf(trackId);
    if (index > -1) {
        MediaPlayer.favorites.splice(index, 1);
        showNotification('Removed from favorites', 'info');
        localStorage.setItem('sam_media_favorites', JSON.stringify(MediaPlayer.favorites));
    }
}

// Load favorites from localStorage
function loadFavorites() {
    try {
        const stored = localStorage.getItem('sam_media_favorites');
        if (stored) {
            MediaPlayer.favorites = JSON.parse(stored);
        }
    } catch (e) {
        console.warn('Failed to load favorites:', e);
    }
}

// Media discovery - find similar content
const MediaDiscovery = {
    relatedContent: [],
    discoveryHistory: [],
    
    findSimilar: function(currentMedia) {
        if (!currentMedia || !currentMedia.genre) {
            return this.getPopularContent();
        }
        
        return this.relatedContent.filter(item => 
            item.genre === currentMedia.genre && 
            item.id !== currentMedia.id
        ).slice(0, 10);
    },
    
    getPopularContent: function() {
        return this.relatedContent.slice(0, 10);
    },
    
    addToHistory: function(media) {
        this.discoveryHistory.unshift(media);
        if (this.discoveryHistory.length > 50) {
            this.discoveryHistory.pop();
        }
        localStorage.setItem('sam_media_history', JSON.stringify(this.discoveryHistory));
    },
    
    getHistory: function() {
        try {
            const stored = localStorage.getItem('sam_media_history');
            if (stored) {
                this.discoveryHistory = JSON.parse(stored);
            }
        } catch (e) {
            console.warn('Failed to load history:', e);
        }
        return this.discoveryHistory;
    },
    
    showDiscoveryPanel: function() {
        if (typeof Swal === 'undefined') {
            alert('Discovery requires SweetAlert2');
            return;
        }
        
        const history = this.getHistory();
        const popular = this.getPopularContent();
        
        Swal.fire({
            title: 'Discover Content',
            html: `
                <div style="text-align: left; padding: 10px;">
                    <h4 style="color: #00d4ff; margin-bottom: 15px;">Recently Played</h4>
                    <div style="display: grid; grid-template-columns: repeat(3, 1fr); gap: 10px; margin-bottom: 20px;">
                        ${history.slice(0, 6).map(item => `
                            <div class="media-discovery-item" style="background: rgba(42,42,58,0.5); padding: 10px; border-radius: 8px; cursor: pointer;"
                                 onclick="MediaDiscovery.playDiscoveredItem('${item.id || ''}')">
                                <div style="font-size: 24px; text-align: center;">${item.icon || '🎵'}</div>
                                <div style="font-size: 11px; text-align: center; margin-top: 5px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap;">${item.title || 'Unknown'}</div>
                            </div>
                        `).join('')}
                    </div>
                    <h4 style="color: #00d4ff; margin-bottom: 15px;">Popular Now</h4>
                    <div style="display: grid; grid-template-columns: repeat(3, 1fr); gap: 10px;">
                        ${popular.slice(0, 6).map(item => `
                            <div class="media-discovery-item" style="background: rgba(42,42,58,0.5); padding: 10px; border-radius: 8px; cursor: pointer;"
                                 onclick="MediaDiscovery.playDiscoveredItem('${item.id || ''}')">
                                <div style="font-size: 24px; text-align: center;">${item.icon || '🎵'}</div>
                                <div style="font-size: 11px; text-align: center; margin-top: 5px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap;">${item.title || 'Unknown'}</div>
                            </div>
                        `).join('')}
                    </div>
                </div>
            `,
            showConfirmButton: false,
            showCloseButton: true,
            width: '600px'
        });
    },
    
    playDiscoveredItem: function(itemId) {
        if (itemId) {
            showNotification(`Playing: ${itemId}`, 'info');
        }
        if (typeof Swal !== 'undefined') Swal.close();
    }
};

// Spotify integration helpers
const SpotifyControls = {
    isConnected: false,
    deviceId: null,
    player: null,
    
    connect: function() {
        if (!window.Spotify) {
            const script = document.createElement('script');
            script.src = 'https://sdk.scdn.co/spotify-player.js';
            document.head.appendChild(script);
        }
        
        window.onSpotifyWebPlaybackSDKReady = () => {
            this.initializePlayer();
        };
    },
    
    initializePlayer: function() {
        const player = new Spotify.Player({
            name: 'SAM Media Center',
            getOAuthToken: cb => {
                this.getOAuthToken(cb);
            }
        });
        
        player.addListener('ready', ({ device_id }) => {
            console.log('Spotify player ready with ID:', device_id);
            this.deviceId = device_id;
            this.isConnected = true;
            showNotification('Spotify connected', 'success');
        });
        
        player.addListener('not_ready', () => {
            console.warn('Spotify player not ready');
            this.isConnected = false;
        });
        
        player.addListener('player_state_changed', state => {
            if (!state) return;
            
            MediaPlayer.isPlaying = !state.paused;
            MediaPlayer.currentTrack = {
                title: state.track_window.current_track.name,
                artist: state.track_window.current_track.artists[0].name,
                album: state.track_window.current_track.album.name,
                artwork: state.track_window.current_track.album.images[0]?.url
            };
            
            updatePlayPauseButton();
            updateMediaSessionMetadata(MediaPlayer.currentTrack);
        });
        
        player.connect();
        this.player = player;
    },
    
    getOAuthToken: function(cb) {
        fetch('/api/services/spotify/token')
            .then(response => response.json())
            .then(data => {
                if (data.access_token) {
                    cb(data.access_token);
                } else {
                    console.error('Failed to get Spotify token');
                    cb(null);
                }
            })
            .catch(err => {
                console.error('Spotify token fetch error:', err);
                cb(null);
            });
    },
    
    play: function() {
        if (this.player) this.player.togglePlay();
    },
    
    pause: function() {
        if (this.player) this.player.togglePlay();
    },
    
    nextTrack: function() {
        if (this.player) this.player.nextTrack();
    },
    
    previousTrack: function() {
        if (this.player) this.player.previousTrack();
    },
    
    setVolume: function(volume) {
        if (this.player) this.player.setVolume(volume / 100);
    }
};

// Initialize Spotify when media center loads
function initSpotifyIntegration() {
    fetch('/api/services/spotify/status')
        .then(response => response.json())
        .then(data => {
            if (data && data.running && data.connected) {
                SpotifyControls.connect();
            }
        })
        .catch(() => {
            console.log('Spotify service not available');
        });
}

// Update initMediaCenter to include new init functions
const origInitMediaCenter = typeof initMediaCenter === 'function' ? initMediaCenter : null;
if (origInitMediaCenter) {
    window.initMediaCenter = function() {
        origInitMediaCenter();
        initSpotifyIntegration();
        loadFavorites();
        initMiniPlayer();
    };
}

// Mini player functionality
function initMiniPlayer() {
    const miniPlayer = document.getElementById('mini-player');
    if (miniPlayer) {
        miniPlayer.style.display = 'none';
        MediaPlayer.miniPlayerVisible = false;
        
        // Make mini player draggable
        makeDraggable(miniPlayer);
        
        // Auto-hide mini player when playback stops
        if (typeof MediaPlayer !== 'undefined') {
            MediaPlayer.autoHideMiniPlayer = true;
        }
    }
}

function makeDraggable(element) {
    let pos1 = 0, pos2 = 0, pos3 = 0, pos4 = 0;
    
    element.onmousedown = dragMouseDown;
    element.ontouchstart = dragTouchStart;
    
    function dragMouseDown(e) {
        e.preventDefault();
        pos3 = e.clientX;
        pos4 = e.clientY;
        document.onmouseup = closeDragElement;
        document.onmousemove = elementDrag;
    }
    
    function elementDrag(e) {
        e.preventDefault();
        pos1 = pos3 - e.clientX;
        pos2 = pos4 - e.clientY;
        pos3 = e.clientX;
        pos4 = e.clientY;
        element.style.top = (element.offsetTop - pos2) + 'px';
        element.style.left = (element.offsetLeft - pos1) + 'px';
    }
    
    function closeDragElement() {
        document.onmouseup = null;
        document.onmousemove = null;
    }
    
    function dragTouchStart(e) {
        const touch = e.touches[0];
        pos3 = touch.clientX;
        pos4 = touch.clientY;
        element.ontouchmove = elementTouchDrag;
        element.ontouchend = closeTouchDrag;
    }
    
    function elementTouchDrag(e) {
        const touch = e.touches[0];
        pos1 = pos3 - touch.clientX;
        pos2 = pos4 - touch.clientY;
        pos3 = touch.clientX;
        pos4 = touch.clientY;
        element.style.top = (element.offsetTop - pos2) + 'px';
        element.style.left = (element.offsetLeft - pos1) + 'px';
    }
    
    function closeTouchDrag() {
        element.ontouchmove = null;
        element.ontouchend = null;
    }
}

function toggleMiniPlayer() {
    const miniPlayer = document.getElementById('mini-player');
    if (!miniPlayer) return;
    
    if (MediaPlayer.miniPlayerVisible) {
        miniPlayer.style.display = 'none';
        MediaPlayer.miniPlayerVisible = false;
    } else {
        miniPlayer.style.display = 'block';
        MediaPlayer.miniPlayerVisible = true;
        updateMiniPlayerInfo();
    }
}

function updateMiniPlayerInfo() {
    const miniPlayer = document.getElementById('mini-player');
    if (!miniPlayer || !MediaPlayer.miniPlayerVisible) return;
    
    const titleEl = miniPlayer.querySelector('.mini-player-title');
    const artistEl = miniPlayer.querySelector('.mini-player-artist');
    const playIcon = document.getElementById('mini-play-icon');
    
    if (titleEl && MediaPlayer.currentTrack) {
        titleEl.textContent = MediaPlayer.currentTrack.title || 'Now Playing';
    }
    
    if (artistEl && MediaPlayer.currentTrack) {
        artistEl.textContent = MediaPlayer.currentTrack.artist || 'Unknown Artist';
    }
    
    if (playIcon) {
        playIcon.className = MediaPlayer.isPlaying ? 'fas fa-pause' : 'fas fa-play';
    }
}

function showNowPlaying(track) {
    const toast = document.getElementById('now-playing-toast');
    if (!toast) return;
    
    const titleEl = toast.querySelector('.now-playing-title');
    const artistEl = toast.querySelector('.now-playing-artist');
    
    if (titleEl) titleEl.textContent = track?.title || 'Now Playing';
    if (artistEl) artistEl.textContent = track?.artist || 'Unknown Artist';
    
    toast.classList.add('visible');
    
    setTimeout(() => {
        hideNowPlaying();
    }, 5000);
}

function hideNowPlaying() {
    const toast = document.getElementById('now-playing-toast');
    if (toast) {
        toast.classList.remove('visible');
    }
}

function createMediaVisualization(containerId) {
    const container = document.getElementById(containerId);
    if (!container) return;
    
    container.innerHTML = '';
    container.className = 'media-visualization';
    
    for (let i = 0; i < 16; i++) {
        const bar = document.createElement('div');
        bar.className = 'media-visualization-bar';
        bar.style.height = '10px';
        bar.style.transitionDelay = `${i * 0.02}s`;
        container.appendChild(bar);
    }
    
    return container;
}

function updateMediaVisualization() {
    if (!MediaPlayer.isPlaying || !MediaPlayer.audioContext) {
        return;
    }
    
    const analyser = MediaPlayer.analyser;
    const dataArray = MediaPlayer.visualizationData;
    
    if (!analyser || !dataArray) return;
    
    analyser.getByteFrequencyData(dataArray);
    
    const bars = document.querySelectorAll('.media-visualization-bar');
    const step = Math.floor(dataArray.length / bars.length);
    
    bars.forEach((bar, i) => {
        const value = dataArray[i * step];
        const height = Math.max(5, (value / 255) * 80);
        bar.style.height = `${height}px`;
    });
    
    requestAnimationFrame(updateMediaVisualization);
}

function initMediaTouchGestures() {
    const mediaContainer = document.querySelector('.media-center-container') || document.body;
    let touchStartX = 0;
    let touchStartY = 0;
    let lastTapTime = 0;
    let touchStartTime = 0;
    let longPressTimer = null;
    let touchHoldProgressTimer = null;
    let touchHoldProgress = 0;
    let touchTrail = [];
    let touchTrailElement = null;
    
    mediaContainer.addEventListener('touchstart', (e) => {
        touchStartX = e.touches[0].clientX;
        touchStartY = e.touches[0].clientY;
        touchStartTime = Date.now();
        touchHoldProgress = 0;
        touchTrail = [{ x: touchStartX, y: touchStartY, time: Date.now() }];
        
        const settings = MediaPlayer.touchGestures;
        if (settings.enabled && settings.longPressDelay > 0) {
            const thresholds = settings.customThresholds[settings.sensitivity] || settings.customThresholds.medium;
            
            touchHoldProgressTimer = setInterval(() => {
                touchHoldProgress += 10;
                updateTouchHoldProgress(touchHoldProgress);
                if (touchHoldProgress >= 100) {
                    showMediaTouchHint('Long Press Detected', '👆', 800, 'top');
                    if (touchHoldProgressTimer) clearInterval(touchHoldProgressTimer);
                }
            }, settings.longPressDelay / 10);
        }
    }, { passive: true });
    
    mediaContainer.addEventListener('touchmove', (e) => {
        if (longPressTimer) {
            clearTimeout(longPressTimer);
            longPressTimer = null;
        }
        if (touchHoldProgressTimer) {
            clearInterval(touchHoldProgressTimer);
            touchHoldProgressTimer = null;
        }
        
        const touch = e.touches[0];
        touchTrail.push({ x: touch.clientX, y: touch.clientY, time: Date.now() });
        if (touchTrail.length > 10) touchTrail.shift();
        
        updateGestureTrail(touch.clientX, touch.clientY);
    }, { passive: true });
    
    mediaContainer.addEventListener('touchend', (e) => {
        if (longPressTimer) {
            clearTimeout(longPressTimer);
            longPressTimer = null;
        }
        if (touchHoldProgressTimer) {
            clearInterval(touchHoldProgressTimer);
            touchHoldProgressTimer = null;
        }
        
        clearGestureTrail();
        
        const touchEndX = e.changedTouches[0].clientX;
        const touchEndY = e.changedTouches[0].clientY;
        const deltaX = touchEndX - touchStartX;
        const deltaY = touchEndY - touchStartY;
        const currentTime = Date.now();
        const touchDuration = currentTime - touchStartTime;
        
        const settings = MediaPlayer.touchGestures;
        const thresholds = settings.customThresholds[settings.sensitivity] || settings.customThresholds.medium;
        const distance = Math.sqrt(deltaX * deltaX + deltaY * deltaY);
        const velocity = distance / touchDuration;
        
        if (distance < thresholds.swipe * 0.5) {
            const tapLength = currentTime - lastTapTime;
            if (tapLength < settings.doubleTapTimeout && tapLength > 0) {
                togglePlayPause();
                if (navigator.vibrate) navigator.vibrate(50);
                showMediaTouchHint('Play/Pause', '⏯️', 1200, 'bottom');
            }
            lastTapTime = currentTime;
        } else if (Math.abs(deltaX) > Math.abs(deltaY) * 2) {
            if (deltaX > thresholds.swipe && velocity > thresholds.velocity) {
                previousTrack();
                if (navigator.vibrate) navigator.vibrate(30);
                showMediaTouchHint('Previous Track', '⏮️', 1000, 'bottom');
            } else if (deltaX < -thresholds.swipe && velocity > thresholds.velocity) {
                nextTrack();
                if (navigator.vibrate) navigator.vibrate(30);
                showMediaTouchHint('Next Track', '⏭️', 1000, 'bottom');
            }
        } else if (Math.abs(deltaY) > Math.abs(deltaX) * 2) {
            if (deltaY > thresholds.swipe && velocity > thresholds.velocity) {
                increaseVolume();
                if (navigator.vibrate) navigator.vibrate(20);
                showMediaTouchHint('Volume Up', '🔊', 1000, 'top');
            } else if (deltaY < -thresholds.swipe && velocity > thresholds.velocity) {
                decreaseVolume();
                if (navigator.vibrate) navigator.vibrate(20);
                showMediaTouchHint('Volume Down', '🔇', 1000, 'top');
            }
        }
    }, { passive: true });
}

function initNowPlayingToast() {
    const toast = document.getElementById('now-playing-toast');
    if (toast) {
        toast.style.display = 'none';
        toast.classList.remove('visible');
    }
}

function updateTouchHoldProgress(progress) {
    let progressEl = document.querySelector('.touch-hold-progress-value');
    if (!progressEl) {
        progressEl = document.createElement('div');
        progressEl.className = 'touch-hold-progress-value';
        progressEl.style.cssText = 'position:fixed;top:50%;left:50%;transform:translate(-50%,-50%);font-size:24px;color:#00d4ff;font-weight:bold;z-index:9999;pointer-events:none;';
        document.body.appendChild(progressEl);
    }
    progressEl.textContent = progress + '%';
    
    if (progress >= 100) {
        setTimeout(() => progressEl.remove(), 500);
    }
}

function updateGestureTrail(x, y) {
    if (!touchTrailElement) {
        touchTrailElement = document.createElement('div');
        touchTrailElement.className = 'gesture-trail';
        touchTrailElement.style.cssText = 'position:fixed;pointer-events:none;z-index:9998;opacity:0.6;';
        document.body.appendChild(touchTrailElement);
    }
    
    const gradientStops = touchTrail.map((point, i) => {
        const alpha = (i / touchTrail.length) * 0.8;
        return `rgba(0, 212, 255, ${alpha}) ${i * 10}%`;
    }).join(',');
    
    touchTrailElement.style.cssText = `
        position:fixed;
        left:${x - 20}px;
        top:${y - 20}px;
        width:40px;
        height:40px;
        border-radius:50%;
        background:radial-gradient(circle, ${gradientStops});
        pointer-events:none;
        z-index:9998;
        transition:opacity 0.1s ease;
    `;
}

function clearGestureTrail() {
    if (touchTrailElement) {
        touchTrailElement.style.opacity = '0';
        setTimeout(() => {
            if (touchTrailElement) touchTrailElement.remove();
            touchTrailElement = null;
        }, 100);
    }
    touchTrail = [];
}

function initEqualizerVisualization() {
    if (typeof MediaPlayer !== 'undefined') {
        MediaPlayer.showVisualization = true;
    }
}

function initBeatDetection() {
    const beatDetectionBtn = document.getElementById('beat-detection-btn');
    if (beatDetectionBtn) {
        beatDetectionBtn.addEventListener('click', () => toggleBeatDetection());
        
        let longPressTimer;
        beatDetectionBtn.addEventListener('touchstart', () => {
            longPressTimer = setTimeout(() => {
                showBeatDetectionSettings();
            }, 500);
        });
        
        beatDetectionBtn.addEventListener('touchend', () => {
            if (longPressTimer) clearTimeout(longPressTimer);
        });
    }
}

function toggleBeatDetection() {
    MediaPlayer.beatDetection.enabled = !MediaPlayer.beatDetection.enabled;
    
    const btn = document.getElementById('beat-detection-btn');
    if (btn) {
        btn.classList.toggle('active', MediaPlayer.beatDetection.enabled);
    }
    
    if (MediaPlayer.beatDetection.enabled) {
        startBeatDetection();
        showNotification('Beat detection enabled', 'success');
    } else {
        stopBeatDetection();
        showNotification('Beat detection disabled', 'info');
    }
}

function setBeatDetectionThreshold(threshold) {
    MediaPlayer.beatDetection.threshold = threshold;
    MediaPlayer.beatDetection.userThreshold = threshold;
    showNotification(`Beat detection threshold: ${threshold}`, 'info');
}

function setBeatDetectionSensitivity(level) {
    const thresholds = {
        'low': 0.6,
        'medium': 0.3,
        'high': 0.15
    };
    MediaPlayer.beatDetection.sensitivity = level;
    MediaPlayer.beatDetection.threshold = thresholds[level] || 0.3;
    showNotification(`Beat sensitivity: ${level}`, 'info');
}

function setTouchGestureSensitivity(level) {
    const thresholds = {
        'low': { swipe: 80, pinch: 50, velocity: 0.2 },
        'medium': { swipe: 50, pinch: 30, velocity: 0.3 },
        'high': { swipe: 25, pinch: 15, velocity: 0.5 }
    };
    
    MediaPlayer.touchGestures.sensitivity = level;
    const settings = thresholds[level] || thresholds.medium;
    MediaPlayer.touchGestures.swipeThreshold = settings.swipe;
    MediaPlayer.touchGestures.pinchSensitivity = settings.pinch;
    MediaPlayer.touchGestures.velocityThreshold = settings.velocity;
    
    showNotification(`Touch sensitivity: ${level}`, 'info');
    
    if (typeof Swal !== 'undefined') {
        Swal.close();
        showTouchSensitivitySettings();
    }
}

function showTouchSensitivitySettings() {
    if (typeof Swal === 'undefined') {
        alert('Touch Sensitivity Settings: low, medium, high');
        return;
    }
    
    const currentSensitivity = MediaPlayer.touchGestures.sensitivity || 'medium';
    const thresholds = MediaPlayer.touchGestures.customThresholds[currentSensitivity] || 
                       MediaPlayer.touchGestures.customThresholds.medium;
    
    Swal.fire({
        title: '<i class="fas fa-hand-pointer"></i> Touch Sensitivity',
        html: `
            <div class="touch-sensitivity-panel">
                <div class="sensitivity-option ${currentSensitivity === 'low' ? 'active' : ''}" onclick="setTouchGestureSensitivity('low')">
                    <div class="sensitivity-option-label">
                        <span class="sensitivity-option-icon">🎯</span>
                        <div>
                            <strong>Low Sensitivity</strong>
                            <div class="sensitivity-option-description">Requires larger gestures - fewer accidental triggers. Best for casual browsing.</div>
                        </div>
                    </div>
                    ${currentSensitivity === 'low' ? '<i class="fas fa-check-circle" style="color: #00d4ff;"></i>' : ''}
                </div>
                
                <div class="sensitivity-option ${currentSensitivity === 'medium' ? 'active' : ''}" onclick="setTouchGestureSensitivity('medium')">
                    <div class="sensitivity-option-label">
                        <span class="sensitivity-option-icon">⚡</span>
                        <div>
                            <strong>Medium Sensitivity</strong>
                            <div class="sensitivity-option-description">Balanced gesture detection - recommended for most users. Perfect for daily use.</div>
                        </div>
                    </div>
                    ${currentSensitivity === 'medium' ? '<i class="fas fa-check-circle" style="color: #00d4ff;"></i>' : ''}
                </div>
                
                <div class="sensitivity-option ${currentSensitivity === 'high' ? 'active' : ''}" onclick="setTouchGestureSensitivity('high')">
                    <div class="sensitivity-option-label">
                        <span class="sensitivity-option-icon">🚀</span>
                        <div>
                            <strong>High Sensitivity</strong>
                            <div class="sensitivity-option-description">Responds to subtle gestures - maximum responsiveness. Ideal for power users.</div>
                        </div>
                    </div>
                    ${currentSensitivity === 'high' ? '<i class="fas fa-check-circle" style="color: #00d4ff;"></i>' : ''}
                </div>
                
                <div class="gesture-sensitivity-visualizer" style="margin-top: 20px;">
                    <div class="gesture-sensitivity-threshold" style="top: ${100 - (thresholds.swipe / 80 * 100)}%;"></div>
                    <div class="gesture-sensitivity-level" id="gesture-sensitivity-level" style="height: ${thresholds.swipe / 80 * 100}%;"></div>
                    <div style="padding: 10px; color: #adb5bd; font-size: 11px; text-align: center;">
                        <i class="fas fa-chart-bar"></i> Real-time Gesture Threshold Visualization
                    </div>
                </div>
                
                <div style="margin-top: 20px; padding: 15px; background: rgba(42, 42, 58, 0.5); border-radius: 10px;">
                    <h5 style="color: #00d4ff; margin-bottom: 10px; font-size: 14px;">
                        <i class="fas fa-sliders-h"></i> Current Thresholds
                    </h5>
                    <div style="color: #adb5bd; font-size: 12px;">
                        <div style="display: flex; justify-content: space-between; margin-bottom: 8px;">
                            <span><i class="fas fa-arrows-alt-h"></i> Swipe Threshold:</span>
                            <span style="color: #00d4ff; font-weight: bold;">${thresholds.swipe}px</span>
                        </div>
                        <div style="display: flex; justify-content: space-between; margin-bottom: 8px;">
                            <span><i class="fas fa-compress-arrows-alt"></i> Pinch Sensitivity:</span>
                            <span style="color: #00d4ff; font-weight: bold;">${thresholds.pinch}px</span>
                        </div>
                        <div style="display: flex; justify-content: space-between;">
                            <span><i class="fas fa-tachometer-alt"></i> Velocity Threshold:</span>
                            <span style="color: #00d4ff; font-weight: bold;">${thresholds.velocity}</span>
                        </div>
                    </div>
                </div>
                
                <div style="margin-top: 15px; padding: 12px; background: rgba(0, 212, 255, 0.1); border-radius: 10px; border: 1px solid rgba(0, 212, 255, 0.3);">
                    <h5 style="color: #00d4ff; margin-bottom: 8px; font-size: 13px;">
                        <i class="fas fa-lightbulb"></i> Quick Tip
                    </h5>
                    <p style="color: #adb5bd; font-size: 11px; margin: 0;">
                        Test your sensitivity by swiping on the media player. If gestures aren't registered, try higher sensitivity. 
                        If accidental triggers occur, try lower sensitivity.
                    </p>
                </div>
            </div>
        `,
        showConfirmButton: false,
        showCloseButton: true,
        width: '550px',
        didOpen: () => {
            const levelElement = document.getElementById('gesture-sensitivity-level');
            if (levelElement) {
                const threshold = MediaPlayer.touchGestures.customThresholds[currentSensitivity].swipe;
                levelElement.style.height = `${(threshold / 80) * 100}%`;
            }
        }
    });
}

function showBeatDetectionSettings() {
    showMediaSyncSettings('beat');
}

function showTouchSensitivitySettings() {
    if (typeof Swal === 'undefined') {
        alert('Touch Sensitivity Settings: low, medium, high');
        return;
    }
    
    const currentSensitivity = MediaPlayer.touchGestures.sensitivity || 'medium';
    const thresholds = MediaPlayer.touchGestures.customThresholds[currentSensitivity] || 
                       MediaPlayer.touchGestures.customThresholds.medium;
    
    Swal.fire({
        title: '<i class="fas fa-hand-pointer"></i> Touch Sensitivity',
        html: `
            <div class="touch-sensitivity-panel">
                <div class="sensitivity-option ${currentSensitivity === 'low' ? 'active' : ''}" onclick="setTouchGestureSensitivity('low')">
                    <div class="sensitivity-option-label">
                        <span class="sensitivity-option-icon">🎯</span>
                        <div>
                            <strong>Low Sensitivity</strong>
                            <div class="sensitivity-option-description">Requires larger gestures - fewer accidental triggers. Best for casual browsing.</div>
                        </div>
                    </div>
                    ${currentSensitivity === 'low' ? '<i class="fas fa-check-circle" style="color: #00d4ff;"></i>' : ''}
                </div>
                
                <div class="sensitivity-option ${currentSensitivity === 'medium' ? 'active' : ''}" onclick="setTouchGestureSensitivity('medium')">
                    <div class="sensitivity-option-label">
                        <span class="sensitivity-option-icon">⚡</span>
                        <div>
                            <strong>Medium Sensitivity</strong>
                            <div class="sensitivity-option-description">Balanced gesture detection - recommended for most users. Perfect for daily use.</div>
                        </div>
                    </div>
                    ${currentSensitivity === 'medium' ? '<i class="fas fa-check-circle" style="color: #00d4ff;"></i>' : ''}
                </div>
                
                <div class="sensitivity-option ${currentSensitivity === 'high' ? 'active' : ''}" onclick="setTouchGestureSensitivity('high')">
                    <div class="sensitivity-option-label">
                        <span class="sensitivity-option-icon">🚀</span>
                        <div>
                            <strong>High Sensitivity</strong>
                            <div class="sensitivity-option-description">Responds to subtle gestures - maximum responsiveness. Ideal for power users.</div>
                        </div>
                    </div>
                    ${currentSensitivity === 'high' ? '<i class="fas fa-check-circle" style="color: #00d4ff;"></i>' : ''}
                </div>
                
                <div class="gesture-sensitivity-visualizer" style="margin-top: 20px;">
                    <div class="gesture-sensitivity-threshold" style="top: ${100 - (thresholds.swipe / 80 * 100)}%;"></div>
                    <div class="gesture-sensitivity-level" id="gesture-sensitivity-level" style="height: ${thresholds.swipe / 80 * 100}%;"></div>
                    <div style="padding: 10px; color: #adb5bd; font-size: 11px; text-align: center;">
                        <i class="fas fa-chart-bar"></i> Real-time Gesture Threshold Visualization
                    </div>
                </div>
                
                <div style="margin-top: 20px; padding: 15px; background: rgba(42, 42, 58, 0.5); border-radius: 10px;">
                    <h5 style="color: #00d4ff; margin-bottom: 10px; font-size: 14px;">
                        <i class="fas fa-sliders-h"></i> Current Thresholds
                    </h5>
                    <div style="color: #adb5bd; font-size: 12px;">
                        <div style="display: flex; justify-content: space-between; margin-bottom: 8px;">
                            <span><i class="fas fa-arrows-alt-h"></i> Swipe Threshold:</span>
                            <span style="color: #00d4ff; font-weight: bold;">${thresholds.swipe}px</span>
                        </div>
                        <div style="display: flex; justify-content: space-between; margin-bottom: 8px;">
                            <span><i class="fas fa-compress-arrows-alt"></i> Pinch Sensitivity:</span>
                            <span style="color: #00d4ff; font-weight: bold;">${thresholds.pinch}px</span>
                        </div>
                        <div style="display: flex; justify-content: space-between;">
                            <span><i class="fas fa-tachometer-alt"></i> Velocity Threshold:</span>
                            <span style="color: #00d4ff; font-weight: bold;">${thresholds.velocity}</span>
                        </div>
                    </div>
                </div>
                
                <div style="margin-top: 15px; padding: 12px; background: rgba(0, 212, 255, 0.1); border-radius: 10px; border: 1px solid rgba(0, 212, 255, 0.3);">
                    <h5 style="color: #00d4ff; margin-bottom: 8px; font-size: 13px;">
                        <i class="fas fa-lightbulb"></i> Quick Tip
                    </h5>
                    <p style="color: #adb5bd; font-size: 11px; margin: 0;">
                        Test your sensitivity by swiping on the media player. If gestures aren't registered, try higher sensitivity. 
                        If accidental triggers occur, try lower sensitivity.
                    </p>
                </div>
            </div>
        `,
        showConfirmButton: false,
        showCloseButton: true,
        width: '550px',
        didOpen: () => {
            const levelElement = document.getElementById('gesture-sensitivity-level');
            if (levelElement) {
                const threshold = MediaPlayer.touchGestures.customThresholds[currentSensitivity].swipe;
                levelElement.style.height = `${(threshold / 80) * 100}%`;
            }
        }
    });
}

function showMediaSyncSettings(activeTab = 'overview') {
    if (typeof Swal === 'undefined') {
        alert('Media Sync Settings: Beat Detection, LIFX Sync, Ambient Light');
        return;
    }
    
    const currentThreshold = MediaPlayer.beatDetection.threshold || 0.3;
    const currentSensitivity = MediaPlayer.beatDetection.sensitivity || 'medium';
    const lifxSyncEnabled = MediaPlayer.lifxSyncEnabled || false;
    const ambientLightEnabled = MediaPlayer.ambientLightEnabled || false;
    const ambientLightMode = MediaPlayer.ambientLightMode || 'spectrum';
    
    Swal.fire({
        title: '<i class="fas fa-sliders-h"></i> Media Sync Settings',
        html: `
            <div class="media-sync-settings">
                <div class="settings-tabs" style="display: flex; gap: 5px; margin-bottom: 20px; border-bottom: 2px solid rgba(0, 212, 255, 0.3); padding-bottom: 10px;">
                    <button class="btn btn-sm ${activeTab === 'overview' ? 'btn-primary' : 'btn-outline-secondary'}" 
                            onclick="showMediaSyncSettings('overview')" style="flex: 1;">
                        <i class="fas fa-home"></i> Overview
                    </button>
                    <button class="btn btn-sm ${activeTab === 'beat' ? 'btn-primary' : 'btn-outline-secondary'}" 
                            onclick="showMediaSyncSettings('beat')" style="flex: 1;">
                        <i class="fas fa-wave-square"></i> Beat
                    </button>
                    <button class="btn btn-sm ${activeTab === 'lifx' ? 'btn-primary' : 'btn-outline-secondary'}" 
                            onclick="showMediaSyncSettings('lifx')" style="flex: 1;">
                        <i class="fas fa-lightbulb"></i> LIFX
                    </button>
                    <button class="btn btn-sm ${activeTab === 'ambient' ? 'btn-primary' : 'btn-outline-secondary'}" 
                            onclick="showMediaSyncSettings('ambient')" style="flex: 1;">
                        <i class="fas fa-cloud-moon"></i> Ambient
                    </button>
                </div>
                
                ${activeTab === 'overview' ? `
                    <div class="settings-overview">
                        <h4 style="color: #00d4ff; margin-bottom: 15px;"><i class="fas fa-chart-pie"></i> Current Status</h4>
                        <div style="display: grid; grid-template-columns: 1fr 1fr; gap: 15px; margin-bottom: 20px;">
                            <div style="background: rgba(0, 212, 255, 0.1); padding: 15px; border-radius: 10px; text-align: center;">
                                <div style="font-size: 24px; margin-bottom: 5px;">${lifxSyncEnabled ? '✅' : '❌'}</div>
                                <div style="font-size: 12px; color: #adb5bd;">LIFX Sync</div>
                            </div>
                            <div style="background: rgba(0, 212, 255, 0.1); padding: 15px; border-radius: 10px; text-align: center;">
                                <div style="font-size: 24px; margin-bottom: 5px;">${ambientLightEnabled ? '✅' : '❌'}</div>
                                <div style="font-size: 12px; color: #adb5bd;">Ambient Light</div>
                            </div>
                            <div style="background: rgba(0, 212, 255, 0.1); padding: 15px; border-radius: 10px; text-align: center;">
                                <div style="font-size: 24px; margin-bottom: 5px;">${MediaPlayer.beatDetection.enabled ? '✅' : '❌'}</div>
                                <div style="font-size: 12px; color: #adb5bd;">Beat Detection</div>
                            </div>
                            <div style="background: rgba(0, 212, 255, 0.1); padding: 15px; border-radius: 10px; text-align: center;">
                                <div style="font-size: 24px; margin-bottom: 5px;">${MediaPlayer.isPlaying ? '🎵' : '⏸'}</div>
                                <div style="font-size: 12px; color: #adb5bd;">Playback</div>
                            </div>
                        </div>
                        
                        <h4 style="color: #00d4ff; margin-bottom: 15px;"><i class="fas fa-bolt"></i> Quick Actions</h4>
                        <div style="display: flex; gap: 10px; flex-wrap: wrap;">
                            <button class="btn btn-sm btn-outline-primary" onclick="toggleLifxMediaSync(); showMediaSyncSettings('overview')">
                                <i class="fas fa-${lifxSyncEnabled ? 'toggle-on' : 'toggle-off'}"></i> ${lifxSyncEnabled ? 'Disable' : 'Enable'} LIFX Sync
                            </button>
                            <button class="btn btn-sm btn-outline-primary" onclick="toggleAmbientLight(); showMediaSyncSettings('overview')">
                                <i class="fas fa-${ambientLightEnabled ? 'toggle-on' : 'toggle-off'}"></i> ${ambientLightEnabled ? 'Disable' : 'Enable'} Ambient
                            </button>
                            <button class="btn btn-sm btn-outline-primary" onclick="cycleAmbientLightMode(); showMediaSyncSettings('overview')">
                                <i class="fas fa-sync"></i> Cycle Mode
                            </button>
                        </div>
                    </div>
                ` : ''}
                
                ${activeTab === 'beat' ? `
                    <div class="beat-detection-settings">
                        <h4><i class="fas fa-sliders-h"></i> Detection Threshold</h4>
                        <div style="padding: 20px;">
                            <label style="color: #adb5bd; display: block; margin-bottom: 10px;">
                                Threshold: <span id="threshold-value" style="color: #00d4ff;">${currentThreshold}</span>
                            </label>
                            <input type="range" id="beat-threshold" min="0.1" max="0.9" step="0.05" 
                                   value="${currentThreshold}" 
                                   style="width: 100%;" 
                                   oninput="document.getElementById('threshold-value').textContent = this.value; setBeatDetectionThreshold(parseFloat(this.value))">
                            <div style="display: flex; justify-content: space-between; margin-top: 5px; font-size: 11px; color: #6c757d;">
                                <span>More Sensitive</span>
                                <span>Less Sensitive</span>
                            </div>
                        </div>
                        
                        <h4 style="margin-top: 20px;"><i class="fas fa-gauge"></i> Preset Sensitivity</h4>
                        <div style="display: flex; gap: 10px; justify-content: center; flex-wrap: wrap;">
                            <button class="btn btn-sm ${currentSensitivity === 'low' ? 'btn-primary' : 'btn-outline-secondary'}" 
                                    onclick="setBeatDetectionSensitivity('low'); showMediaSyncSettings('beat')">Low</button>
                            <button class="btn btn-sm ${currentSensitivity === 'medium' ? 'btn-primary' : 'btn-outline-secondary'}" 
                                    onclick="setBeatDetectionSensitivity('medium'); showMediaSyncSettings('beat')">Medium</button>
                            <button class="btn btn-sm ${currentSensitivity === 'high' ? 'btn-primary' : 'btn-outline-secondary'}" 
                                    onclick="setBeatDetectionSensitivity('high'); showMediaSyncSettings('beat')">High</button>
                        </div>
                        
                        <div style="margin-top: 20px; padding: 15px; background: rgba(42, 42, 58, 0.5); border-radius: 10px;">
                            <h5 style="color: #00d4ff; margin-bottom: 10px; font-size: 14px;"><i class="fas fa-info-circle"></i> Tips</h5>
                            <ul style="color: #adb5bd; font-size: 12px; margin: 0; padding-left: 20px;">
                                <li>Lower threshold = more responsive to quiet beats</li>
                                <li>Higher threshold = only detects strong beats</li>
                                <li>Adjust while music is playing for best results</li>
                            </ul>
                        </div>
                    </div>
                ` : ''}
                
                ${activeTab === 'lifx' ? `
                    <div class="lifx-sync-settings">
                        <h4><i class="fas fa-lightbulb"></i> LIFX Media Sync</h4>
                        <div style="padding: 20px; text-align: center;">
                            <button class="btn btn-lg ${lifxSyncEnabled ? 'btn-success' : 'btn-outline-secondary'}" 
                                    onclick="toggleLifxMediaSync(); showMediaSyncSettings('lifx')" 
                                    style="width: 200px; margin-bottom: 20px;">
                                <i class="fas fa-${lifxSyncEnabled ? 'check' : 'times'}"></i> ${lifxSyncEnabled ? 'Enabled' : 'Disabled'}
                            </button>
                            <p style="color: #adb5bd; font-size: 13px;">Sync LIFX lights to music rhythm and beats</p>
                        </div>
                        
                        <h4 style="margin-top: 20px;"><i class="fas fa-bolt"></i> Sync Mode</h4>
                        <div style="display: grid; grid-template-columns: repeat(2, 1fr); gap: 10px; margin-bottom: 20px;">
                            ${['pulse', 'rainbow', 'ambient', 'strobe', 'zone'].map(mode => `
                                <button class="btn btn-sm ${MediaPlayer.lifxSyncMode === mode ? 'btn-primary' : 'btn-outline-secondary'}" 
                                        onclick="setLifxSyncMode('${mode}'); showMediaSyncSettings('lifx')"
                                        style="padding: 12px;">
                                    <i class="fas fa-${mode === 'pulse' ? 'heart' : mode === 'rainbow' ? 'rainbow' : mode === 'ambient' ? 'cloud' : mode === 'strobe' ? 'bolt' : 'bars'}"></i> ${mode.charAt(0).toUpperCase() + mode.slice(1)}
                                </button>
                            `).join('')}
                        </div>
                        
                        <h4 style="margin-top: 20px;"><i class="fas fa-music"></i> Beat Detection Mode</h4>
                        <div style="display: flex; gap: 10px; justify-content: center; flex-wrap: wrap;">
                            <button class="btn btn-sm ${MediaPlayer.lifxBeatDetection.enabled ? 'btn-primary' : 'btn-outline-secondary'}" 
                                    onclick="MediaPlayer.lifxBeatDetection.enabled = !MediaPlayer.lifxBeatDetection.enabled; showNotification('Beat detection ' + (MediaPlayer.lifxBeatDetection.enabled ? 'enabled' : 'disabled'), 'info'); showMediaSyncSettings('lifx')">
                                <i class="fas fa-wave-square"></i> Beat Mode
                            </button>
                            <button class="btn btn-sm ${MediaPlayer.lifxSceneMode === 'ambient' ? 'btn-primary' : 'btn-outline-secondary'}" 
                                    onclick="MediaPlayer.lifxSceneMode = 'ambient'; showNotification('LIFX mode: ambient', 'info'); showMediaSyncSettings('lifx')">
                                <i class="fas fa-cloud"></i> Ambient
                            </button>
                            <button class="btn btn-sm ${MediaPlayer.lifxSceneMode === 'visualizer' ? 'btn-primary' : 'btn-outline-secondary'}" 
                                    onclick="MediaPlayer.lifxSceneMode = 'visualizer'; startAudioVisualization(); showNotification('LIFX mode: visualizer', 'info'); showMediaSyncSettings('lifx')">
                                <i class="fas fa-chart-bar"></i> Visualizer
                            </button>
                        </div>
                        
                        <div style="margin-top: 20px; padding: 15px; background: rgba(42, 42, 58, 0.5); border-radius: 10px;">
                            <h5 style="color: #00d4ff; margin-bottom: 10px; font-size: 14px;"><i class="fas fa-info-circle"></i> Sync Modes</h5>
                            <ul style="color: #adb5bd; font-size: 12px; margin: 0; padding-left: 20px;">
                                <li><strong>Pulse:</strong> Classic beat-synced pulsing</li>
                                <li><strong>Rainbow:</strong> Cycles colors on each beat</li>
                                <li><strong>Ambient:</strong> Matches screen colors</li>
                                <li><strong>Strobe:</strong> Flashing party effect</li>
                                <li><strong>Zone:</strong> Sequential zone lighting</li>
                            </ul>
                        </div>
                    </div>
                ` : ''}
                
                ${activeTab === 'ambient' ? `
                    <div class="ambient-light-settings">
                        <h4><i class="fas fa-cloud-moon"></i> Ambient Light Sync</h4>
                        <div style="padding: 20px; text-align: center;">
                            <button class="btn btn-lg ${ambientLightEnabled ? 'btn-success' : 'btn-outline-secondary'}" 
                                    onclick="toggleAmbientLight(); showMediaSyncSettings('ambient')" 
                                    style="width: 200px; margin-bottom: 20px;">
                                <i class="fas fa-${ambientLightEnabled ? 'check' : 'times'}"></i> ${ambientLightEnabled ? 'Enabled' : 'Disabled'}
                            </button>
                            <p style="color: #adb5bd; font-size: 13px;">Sync ambient lighting to media content</p>
                        </div>
                        
                        <h4 style="margin-top: 20px;"><i class="fas fa-palette"></i> Ambient Mode</h4>
                        <div style="display: grid; grid-template-columns: repeat(2, 1fr); gap: 10px;">
                            ${['spectrum', 'warm', 'cool', 'beat', 'visualizer', 'aurora', 'pulse', 'fire', 'ocean', 'neon'].map(mode => `
                                <button class="btn btn-sm ${ambientLightMode === mode ? 'btn-primary' : 'btn-outline-secondary'}" 
                                        onclick="MediaPlayer.ambientLightMode = '${mode}'; cycleAmbientLightMode(); showMediaSyncSettings('ambient')"
                                        style="padding: 10px;">
                                    <i class="fas fa-${mode === 'spectrum' ? 'bars' : mode === 'warm' ? 'sun' : mode === 'cool' ? 'snowflake' : mode === 'beat' ? 'wave-square' : mode === 'visualizer' ? 'chart-bar' : mode === 'aurora' ? 'cloud' : mode === 'pulse' ? 'circle' : mode === 'fire' ? 'fire' : mode === 'ocean' ? 'water' : 'bolt'}"></i> ${mode.charAt(0).toUpperCase() + mode.slice(1)}
                                </button>
                            `).join('')}
                        </div>
                        
                        <div style="margin-top: 20px; padding: 15px; background: rgba(42, 42, 58, 0.5); border-radius: 10px;">
                            <h5 style="color: #00d4ff; margin-bottom: 10px; font-size: 14px;"><i class="fas fa-info-circle"></i> Modes</h5>
                            <ul style="color: #adb5bd; font-size: 12px; margin: 0; padding-left: 20px;">
                                <li><strong>Spectrum:</strong> Full color range cycling</li>
                                <li><strong>Warm/Cool:</strong> Fixed color temperature</li>
                                <li><strong>Beat:</strong> Pulses to music rhythm</li>
                                <li><strong>Aurora/Pulse:</strong> Dynamic effects</li>
                            </ul>
                        </div>
                    </div>
                ` : ''}
            </div>
        `,
        showConfirmButton: false,
        showCloseButton: true,
        width: '650px'
    });
}

function startBeatDetection() {
    const video = document.querySelector('video');
    const audio = document.querySelector('audio');
    const mediaElement = video || audio;
    
    if (!mediaElement) return;
    
    try {
        if (!MediaPlayer.audioContext) {
            MediaPlayer.audioContext = new (window.AudioContext || window.webkitAudioContext)();
            MediaPlayer.analyser = MediaPlayer.audioContext.createAnalyser();
            MediaPlayer.analyser.fftSize = 256;
            MediaPlayer.mediaElementSource = MediaPlayer.audioContext.createMediaElementSource(mediaElement);
            MediaPlayer.mediaElementSource.connect(MediaPlayer.analyser);
            MediaPlayer.analyser.connect(MediaPlayer.audioContext.destination);
        }
        
        const bufferLength = MediaPlayer.analyser.frequencyBinCount;
        const dataArray = new Uint8Array(bufferLength);
        
        const detectBeat = () => {
            if (!MediaPlayer.beatDetection.enabled) return;
            
            MediaPlayer.analyser.getByteFrequencyData(dataArray);
            
            const bass = dataArray.slice(0, 10).reduce((a, b) => a + b, 0) / 10;
            const mid = dataArray.slice(10, 30).reduce((a, b) => a + b, 0) / 20;
            const now = Date.now();
            
            const adaptiveThreshold = MediaPlayer.beatDetection.threshold * 255;
            const energy = (bass * 0.7) + (mid * 0.3);
            const timeSinceLastBeat = now - MediaPlayer.beatDetection.lastBeatTime;
            
            const minInterval = Math.max(100, 60000 / (MediaPlayer.beatDetection.bpmEstimate + 20));
            
            if (energy > adaptiveThreshold && timeSinceLastBeat > minInterval) {
                MediaPlayer.beatDetection.lastBeatTime = now;
                MediaPlayer.beatDetection.beatCount++;
                MediaPlayer.beatDetection.beatIntensity = Math.min(1.0, energy / 255);
                
                MediaPlayer.beatDetection.peakEnergy = Math.max(MediaPlayer.beatDetection.peakEnergy || 0, energy);
                MediaPlayer.beatDetection.threshold = MediaPlayer.beatDetection.peakEnergy * 0.7 / 255;
                MediaPlayer.beatDetection.peakDecay = 0.98;
                
                if (MediaPlayer.beatDetection.beatHistory.length >= 4) {
                    const intervals = [];
                    for (let i = MediaPlayer.beatDetection.beatHistory.length - 1; i > 0; i--) {
                        intervals.push(MediaPlayer.beatDetection.beatHistory[i] - MediaPlayer.beatDetection.beatHistory[i-1]);
                    }
                    const avgInterval = intervals.reduce((a, b) => a + b, 0) / intervals.length;
                    if (avgInterval > 0) {
                        MediaPlayer.beatDetection.bpmEstimate = Math.round(60000 / avgInterval);
                        MediaPlayer.beatDetection.bpmEstimate = Math.max(60, Math.min(200, MediaPlayer.beatDetection.bpmEstimate));
                    }
                }
                MediaPlayer.beatDetection.beatHistory.push(now);
                if (MediaPlayer.beatDetection.beatHistory.length > 8) {
                    MediaPlayer.beatDetection.beatHistory.shift();
                }
                
                if (MediaPlayer.onLifxUpdate) {
                    MediaPlayer.onLifxUpdate({ 
                        type: 'beat', 
                        intensity: MediaPlayer.beatDetection.beatIntensity,
                        bpm: MediaPlayer.beatDetection.bpmEstimate
                    });
                }
                
                pulseVisualization();
                updateBpmDisplay();
            }
            
            if (MediaPlayer.beatDetection.peakEnergy) {
                MediaPlayer.beatDetection.peakEnergy *= MediaPlayer.beatDetection.peakDecay;
            }
            
            requestAnimationFrame(detectBeat);
        };
        
        detectBeat();
    } catch (err) {
        console.error('Beat detection error:', err);
    }
}

function stopBeatDetection() {
    if (MediaPlayer.audioContext) {
        MediaPlayer.audioContext.close();
        MediaPlayer.audioContext = null;
        MediaPlayer.analyser = null;
        MediaPlayer.mediaElementSource = null;
    }
}

function pulseVisualization() {
    const bars = document.querySelectorAll('.media-visualization-bar');
    const beatIntensity = MediaPlayer.beatDetection.beatIntensity || 1.0;
    
    bars.forEach((bar, i) => {
        const baseHeight = 10 + i * 10;
        const beatHeight = 20 + (Math.random() * 80 * beatIntensity);
        const targetHeight = Math.min(100, Math.max(baseHeight, beatHeight));
        
        bar.style.transition = 'height 0.08s cubic-bezier(0.25, 0.46, 0.45, 0.94)';
        bar.style.height = `${targetHeight}%`;
        bar.style.filter = `brightness(${1 + beatIntensity * 0.5})`;
        
        setTimeout(() => {
            bar.style.transition = 'height 0.15s ease-out';
            bar.style.height = `${baseHeight}%`;
            bar.style.filter = 'brightness(1)';
        }, 120);
    });
}

function initLifxMediaSync() {
    const lifxSyncBtn = document.getElementById('lifx-sync-btn');
    if (lifxSyncBtn) {
        lifxSyncBtn.addEventListener('click', () => toggleLifxMediaSync());
    }
}

function toggleLifxMediaSync() {
    if (typeof LifXTouchControls === 'undefined') {
        showNotification('LIFX controls not available', 'error');
        return;
    }
    
    MediaPlayer.lifxMediaSyncEnabled = !MediaPlayer.lifxMediaSyncEnabled;
    
    const btn = document.getElementById('lifx-sync-btn');
    if (btn) {
        btn.classList.toggle('active', MediaPlayer.lifxMediaSyncEnabled);
    }
    
    if (MediaPlayer.lifxMediaSyncEnabled) {
        if (typeof LifXTouchControls !== 'undefined') {
            LifXTouchControls.lifxMediaSyncEnabled = true;
        }
        startLightSync();
        showNotification('LIFX media sync enabled', 'success');
    } else {
        if (typeof LifXTouchControls !== 'undefined') {
            LifXTouchControls.lifxMediaSyncEnabled = false;
        }
        stopLightSync();
        showNotification('LIFX media sync disabled', 'info');
    }
}

function startLightSync() {
    const video = document.querySelector('video');
    if (!video) return;
    
    const syncLights = () => {
        if (!MediaPlayer.lifxMediaSyncEnabled || video.paused) {
            return;
        }
        
        try {
            const canvas = document.createElement('canvas');
            canvas.width = 1;
            canvas.height = 1;
            const ctx = canvas.getContext('2d');
            ctx.drawImage(video, 0, 0, 1, 1);
            const pixel = ctx.getImageData(0, 0, 1, 1).data;
            
            const rgb = { r: pixel[0], g: pixel[1], b: pixel[2] };
            const hsv = rgbToHsv(rgb.r, rgb.g, rgb.b);
            
            if (MediaPlayer.onLifxUpdate) {
                MediaPlayer.onLifxUpdate({
                    type: 'color',
                    hue: Math.round(hsv.h * 182),
                    saturation: Math.round(hsv.s * 100)
                });
            }
        } catch (err) {
            console.error('Light sync error:', err);
        }
        
        MediaPlayer.lightSyncInterval = setTimeout(syncLights, 500);
    };
    
    syncLights();
}

function stopLightSync() {
    if (MediaPlayer.lightSyncInterval) {
        clearTimeout(MediaPlayer.lightSyncInterval);
        MediaPlayer.lightSyncInterval = null;
    }
}

function rgbToHsv(r, g, b) {
    r /= 255; g /= 255; b /= 255;
    const max = Math.max(r, g, b), min = Math.min(r, g, b);
    let h, s, v = max;
    const d = max - min;
    s = max === 0 ? 0 : d / max;
    if (max === min) {
        h = 0;
    } else {
        switch (max) {
            case r: h = (g - b) / d + (g < b ? 6 : 0); break;
            case g: h = (b - r) / d + 2; break;
            case b: h = (r - g) / d + 4; break;
        }
        h /= 6;
    }
    return { h, s, v };
}

function showMediaTouchHint(text, icon) {
    const existingHint = document.querySelector('.media-center-touch-hint');
    if (existingHint) {
        existingHint.remove();
    }
    
    const hint = document.createElement('div');
    hint.className = 'media-center-touch-hint visible';
    hint.innerHTML = `<span style="font-size: 32px; display: block; margin-bottom: 10px;">${icon}</span>${text}`;
    document.body.appendChild(hint);
    
    setTimeout(() => {
        hint.classList.remove('visible');
        setTimeout(() => hint.remove(), 300);
    }, 1500);
}

function setPlaybackSpeed(speed) {
    MediaPlayer.playbackSpeed = speed;
    showNotification(`Playback speed: ${speed}x`, 'info');
}

function addToNowPlayingHistory(track) {
    if (!track) return;
    
    MediaPlayer.nowPlayingHistory.unshift({
        ...track,
        playedAt: Date.now()
    });
    
    if (MediaPlayer.nowPlayingHistory.length > 20) {
        MediaPlayer.nowPlayingHistory.pop();
    }
    
    localStorage.setItem('sam_now_playing_history', JSON.stringify(MediaPlayer.nowPlayingHistory));
}

// Enhanced Media Center Features
const MediaEnhancements = {
    watchPartyEnabled: false,
    watchPartyHost: null,
    watchPartyParticipants: [],
    synchronizedPlayback: false,
    playbackOffset: 0,
    
    initWatchParty: function() {
        if (!window.WebSocket) {
            showNotification('Watch Party requires WebSocket support', 'error');
            return;
        }
        
        this.watchPartyEnabled = true;
        console.log('Watch Party initialized');
    },
    
    createWatchParty: function(roomName) {
        if (typeof ws === 'undefined' || !ws || ws.readyState !== WebSocket.OPEN) {
            showNotification('Connecting to server...', 'info');
            setTimeout(() => this.createWatchParty(roomName), 1000);
            return;
        }
        
        ws.send(JSON.stringify({
            type: 'command',
            command: 'create_watch_party',
            args: { room: roomName }
        }));
        
        this.watchPartyHost = true;
        showNotification(`Watch Party "${roomName}" created`, 'success');
    },
    
    joinWatchParty: function(roomName) {
        if (typeof ws === 'undefined' || !ws || ws.readyState !== WebSocket.OPEN) {
            showNotification('Connecting to server...', 'info');
            setTimeout(() => this.joinWatchParty(roomName), 1000);
            return;
        }
        
        ws.send(JSON.stringify({
            type: 'command',
            command: 'join_watch_party',
            args: { room: roomName }
        }));
        
        showNotification(`Joined Watch Party "${roomName}"`, 'success');
    },
    
    syncPlaybackState: function(state) {
        if (!this.watchPartyEnabled || !ws || ws.readyState !== WebSocket.OPEN) return;
        
        ws.send(JSON.stringify({
            type: 'command',
            command: 'sync_playback',
            args: {
                playing: state.playing,
                currentTime: state.currentTime,
                speed: state.speed
            }
        }));
    },
    
    initScreenCasting: function() {
        if (!navigator.mediaDevices || !navigator.mediaDevices.getDisplayMedia) {
            showNotification('Screen casting not supported in this browser', 'warning');
            return;
        }
        
        window.castScreen = async () => {
            try {
                const stream = await navigator.mediaDevices.getDisplayMedia({
                    video: { cursor: 'always' },
                    audio: true
                });
                
                const videoEl = document.createElement('video');
                videoEl.srcObject = stream;
                videoEl.play();
                
                document.getElementById('media-player').appendChild(videoEl);
                showNotification('Screen casting started', 'success');
                
                stream.getVideoTracks()[0].onended = () => {
                    videoEl.remove();
                    showNotification('Screen casting stopped', 'info');
                };
            } catch (err) {
                console.error('Screen cast error:', err);
                showNotification('Screen casting failed', 'error');
            }
        };
        
        console.log('Screen casting initialized');
    },
    
    initAudioVisualization: function() {
        const visualizerContainer = document.getElementById('audio-visualizer');
        if (!visualizerContainer) return;
        
        if (!MediaPlayer.audioContext) {
            initAudioContext();
        }
        
        const canvas = document.createElement('canvas');
        canvas.id = 'visualizer-canvas';
        canvas.width = visualizerContainer.clientWidth;
        canvas.height = visualizerContainer.clientHeight;
        visualizerContainer.appendChild(canvas);
        
        const ctx = canvas.getContext('2d');
        const analyser = MediaPlayer.analyser;
        const dataArray = new Uint8Array(analyser.frequencyBinCount);
        
        const draw = () => {
            if (!MediaPlayer.isPlaying) {
                requestAnimationFrame(draw);
                return;
            }
            
            requestAnimationFrame(draw);
            analyser.getByteFrequencyData(dataArray);
            
            ctx.fillStyle = 'rgba(42, 42, 58, 0.2)';
            ctx.fillRect(0, 0, canvas.width, canvas.height);
            
            const barWidth = canvas.width / dataArray.length;
            let x = 0;
            
            for (let i = 0; i < dataArray.length; i++) {
                const barHeight = (dataArray[i] / 255) * canvas.height;
                
                const gradient = ctx.createLinearGradient(0, canvas.height, 0, 0);
                gradient.addColorStop(0, '#00d4ff');
                gradient.addColorStop(0.5, '#ff00ff');
                gradient.addColorStop(1, '#ff6b6b');
                
                ctx.fillStyle = gradient;
                ctx.fillRect(x, canvas.height - barHeight, barWidth - 1, barHeight);
                x += barWidth;
            }
        };
        
        draw();
        console.log('Audio visualization initialized');
    },
    
    initMediaKeyboardShortcuts: function() {
        document.addEventListener('keydown', (e) => {
            if (e.target.tagName === 'INPUT' || e.target.tagName === 'TEXTAREA') return;
            
            const handlers = {
                'KeyF': () => this.toggleFullscreen(),
                'KeyM': () => toggleMute(),
                'Space': () => { e.preventDefault(); togglePlayPause(); },
                'ArrowLeft': () => this.seek(-10),
                'ArrowRight': () => this.seek(10),
                'ArrowUp': () => increaseVolume(),
                'ArrowDown': () => decreaseVolume(),
                'Digit0': () => setPlaybackSpeed(0.5),
                'Digit1': () => setPlaybackSpeed(1.0),
                'Digit2': () => setPlaybackSpeed(1.5),
                'Digit3': () => setPlaybackSpeed(2.0)
            };
            
            if (handlers[e.code]) {
                handlers[e.code]();
            }
        });
        
        console.log('Media keyboard shortcuts initialized');
    },
    
    toggleFullscreen: function() {
        const player = document.getElementById('media-player');
        if (!player) return;
        
        if (document.fullscreenElement) {
            document.exitFullscreen();
        } else {
            player.requestFullscreen();
        }
    },
    
    seek: function(seconds) {
        const video = document.querySelector('video');
        if (!video) return;
        
        video.currentTime = Math.max(0, Math.min(video.duration, video.currentTime + seconds));
        showNotification(`Seeked ${seconds > 0 ? '+' : ''}${seconds}s`, 'info');
    },
    
    initPictureInPicture: function() {
        const video = document.querySelector('video');
        if (!video || !document.pictureInPictureEnabled) return;
        
        window.togglePiP = async () => {
            try {
                if (document.pictureInPictureElement) {
                    await document.exitPictureInPicture();
                } else {
                    await video.requestPictureInPicture();
                }
            } catch (err) {
                console.error('PiP error:', err);
                showNotification('Picture in Picture failed', 'error');
            }
        };
        
        console.log('Picture in Picture initialized');
    },
    
    initMediaSessionControls: function() {
        if ('mediaSession' in navigator) {
            navigator.mediaSession.setActionHandler('seekbackward', (details) => {
                this.seek(-10);
            });
            
            navigator.mediaSession.setActionHandler('seekforward', (details) => {
                this.seek(10);
            });
            
            navigator.mediaSession.setActionHandler('seekto', (details) => {
                const video = document.querySelector('video');
                if (video) {
                    video.currentTime = details.seekTime;
                }
            });
            
            console.log('Enhanced Media Session controls initialized');
        }
    },
    
    getWatchPartyState: function() {
        return {
            enabled: this.watchPartyEnabled,
            isHost: this.watchPartyHost,
            participants: this.watchPartyParticipants,
            synchronized: this.synchronizedPlayback
        };
    }
};

// Initialize enhanced media features
document.addEventListener('DOMContentLoaded', () => {
    MediaEnhancements.initScreenCasting();
    MediaEnhancements.initMediaKeyboardShortcuts();
    MediaEnhancements.initPictureInPicture();
    MediaEnhancements.initMediaSessionControls();
    
    if (typeof is_touch_enabled === 'function' && is_touch_enabled()) {
        console.log('Touch device detected - enhanced media controls enabled');
    }
});

function getNowPlayingHistory() {
    try {
        const stored = localStorage.getItem('sam_now_playing_history');
        if (stored) {
            MediaPlayer.nowPlayingHistory = JSON.parse(stored);
        }
    } catch (e) {
        console.warn('Failed to load now playing history:', e);
    }
    return MediaPlayer.nowPlayingHistory;
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

// Enhanced Beat Detection Calibration
const BeatDetectionCalibration = {
    isActive: false,
    calibrationData: [],
    thresholdHistory: [],
    sensitivityHistory: [],
    bpmHistory: [],
    visualizationFrame: null,
    
    start: function() {
        this.isActive = true;
        this.calibrationData = [];
        this.thresholdHistory = [];
        this.sensitivityHistory = [];
        this.bpmHistory = [];
        this.updateVisualization();
        this.startRealTimeMonitoring();
        showNotification('Beat detection calibration started', 'info');
    },
    
    stop: function() {
        this.isActive = false;
        this.stopRealTimeMonitoring();
        showNotification('Beat detection calibration stopped', 'info');
    },
    
    addDataPoint: function(energy, threshold, sensitivity, bpm) {
        if (!this.isActive) return;
        
        this.calibrationData.push({ energy, threshold, sensitivity, bpm, time: Date.now() });
        this.thresholdHistory.push(threshold);
        this.sensitivityHistory.push(sensitivity);
        this.bpmHistory.push(bpm || 0);
        
        if (this.calibrationData.length > 100) {
            this.calibrationData.shift();
        }
        if (this.thresholdHistory.length > 50) {
            this.thresholdHistory.shift();
            this.sensitivityHistory.shift();
            this.bpmHistory.shift();
        }
        
        this.updateVisualization();
        this.updateStatsPanel();
    },
    
    startRealTimeMonitoring: function() {
        const monitor = () => {
            if (!this.isActive) return;
            
            const energy = MediaPlayer.lifxBeatDetection.energyHistory.length > 0 
                ? MediaPlayer.lifxBeatDetection.energyHistory[MediaPlayer.lifxBeatDetection.energyHistory.length - 1] 
                : 0;
            const threshold = MediaPlayer.lifxBeatDetection.threshold || 0.3;
            const sensitivity = MediaPlayer.lifxBeatDetection.sensitivity || 'medium';
            const bpm = MediaPlayer.lifxBeatDetection.bpmEstimate || 0;
            
            this.addDataPoint(energy, threshold, sensitivity, bpm);
            this.visualizationFrame = requestAnimationFrame(monitor);
        };
        monitor();
    },
    
    stopRealTimeMonitoring: function() {
        if (this.visualizationFrame) {
            cancelAnimationFrame(this.visualizationFrame);
            this.visualizationFrame = null;
        }
    },
    
    updateVisualization: function() {
        const barsContainer = document.querySelector('.beat-energy-visualization');
        if (!barsContainer) return;
        
        barsContainer.innerHTML = '';
        const barCount = 32;
        
        for (let i = 0; i < barCount; i++) {
            const bar = document.createElement('div');
            bar.className = 'beat-energy-bar';
            
            if (this.calibrationData.length > 0) {
                const dataIndex = Math.floor((this.calibrationData.length / barCount) * i);
                const data = this.calibrationData[dataIndex] || { energy: 0, threshold: 0.3 };
                const height = Math.max(5, data.energy * 100);
                const thresholdHeight = data.threshold * 100;
                
                const isBeat = data.energy > data.threshold;
                const intensity = Math.min(1, data.energy / data.threshold);
                
                bar.style.cssText = `
                    height: ${height}%;
                    background: linear-gradient(to top, 
                        ${isBeat ? '#ff6b6b' : '#6c757d'} 0%, 
                        ${isBeat ? '#ff0080' : '#adb5bd'} ${50 + (intensity * 50)}%,
                        ${isBeat ? '#00d4ff' : '#495057'} 100%);
                    position: relative;
                    transition: height 0.05s ease, transform 0.1s ease;
                    transform: ${isBeat ? 'scaleY(1.1)' : 'scaleY(1)'};
                    box-shadow: ${isBeat ? `0 0 ${10 + (intensity * 20)}px rgba(255, 107, 107, ${0.3 + (intensity * 0.5)})` : 'none'};
                `;
                
                if (isBeat) {
                    bar.classList.add('peak');
                    bar.innerHTML = `<div style="position: absolute; top: -18px; left: 50%; transform: translateX(-50%); font-size: 10px; color: #ff6b6b; font-weight: bold;">${(data.energy * 100).toFixed(0)}%</div>`;
                }
                
                const thresholdMarker = document.createElement('div');
                thresholdMarker.className = 'threshold-marker';
                thresholdMarker.style.cssText = `
                    position: absolute;
                    bottom: ${thresholdHeight}%;
                    left: 0;
                    right: 0;
                    height: 2px;
                    background: #ff6b6b;
                    box-shadow: 0 0 5px rgba(255, 107, 107, 0.8);
                `;
                bar.appendChild(thresholdMarker);
            } else {
                bar.style.height = '5%';
                bar.style.background = 'linear-gradient(to top, #2c3e50 0%, #34495e 100%)';
            }
            
            barsContainer.appendChild(bar);
        }
    },
    
    updateStatsPanel: function() {
        const statsPanel = document.querySelector('.calibration-stats');
        if (!statsPanel) return;
        
        const recentData = this.calibrationData.slice(-20);
        const avgEnergy = recentData.length > 0 
            ? recentData.reduce((sum, d) => sum + d.energy, 0) / recentData.length 
            : 0;
        const maxEnergy = recentData.length > 0 
            ? Math.max(...recentData.map(d => d.energy)) 
            : 0;
        const beatCount = recentData.filter(d => d.energy > d.threshold).length;
        const currentBpm = this.bpmHistory.length > 0 
            ? this.bpmHistory[this.bpmHistory.length - 1] 
            : 0;
        
        statsPanel.innerHTML = `
            <div class="stat-item">
                <span class="stat-label">Avg Energy:</span>
                <span class="stat-value">${(avgEnergy * 100).toFixed(1)}%</span>
            </div>
            <div class="stat-item">
                <span class="stat-label">Max Energy:</span>
                <span class="stat-value">${(maxEnergy * 100).toFixed(1)}%</span>
            </div>
            <div class="stat-item">
                <span class="stat-label">Beats Detected:</span>
                <span class="stat-value" style="color: #00ff88;">${beatCount}</span>
            </div>
            <div class="stat-item">
                <span class="stat-label">Current BPM:</span>
                <span class="stat-value" style="color: #ff6b6b;">${currentBpm > 0 ? Math.round(currentBpm) : '--'}</span>
            </div>
            <div class="stat-item">
                <span class="stat-label">Samples:</span>
                <span class="stat-value">${this.calibrationData.length}</span>
            </div>
        `;
    },
    
    autoCalibrate: function() {
        if (this.calibrationData.length < 20) {
            showNotification('Need more data for calibration. Play music for a few seconds.', 'warning');
            return;
        }
        
        const energies = this.calibrationData.map(d => d.energy);
        const avgEnergy = energies.reduce((a, b) => a + b, 0) / energies.length;
        const maxEnergy = Math.max(...energies);
        const minEnergy = Math.min(...energies);
        const stdDev = Math.sqrt(energies.reduce((sum, e) => sum + Math.pow(e - avgEnergy, 2), 0) / energies.length);
        
        const optimalThreshold = avgEnergy + (stdDev * 0.5);
        const recommendedSensitivity = optimalThreshold < 0.2 ? 'high' : optimalThreshold > 0.4 ? 'low' : 'medium';
        
        if (MediaPlayer.beatDetection) {
            MediaPlayer.beatDetection.threshold = Math.max(0.1, Math.min(0.9, optimalThreshold));
            MediaPlayer.beatDetection.userThreshold = Math.max(0.1, Math.min(0.9, optimalThreshold));
            MediaPlayer.beatDetection.sensitivity = recommendedSensitivity;
        }
        
        showNotification(`Auto-calibrated: threshold=${optimalThreshold.toFixed(3)}, sensitivity=${recommendedSensitivity}`, 'success');
        this.stop();
    },
    
    reset: function() {
        this.calibrationData = [];
        this.thresholdHistory = [];
        this.sensitivityHistory = [];
        this.bpmHistory = [];
        this.updateVisualization();
        this.updateStatsPanel();
        showNotification('Calibration data reset', 'info');
    },
    
    exportCalibrationData: function() {
        if (this.calibrationData.length === 0) {
            showNotification('No calibration data to export', 'warning');
            return null;
        }
        
        const data = {
            timestamp: Date.now(),
            samples: this.calibrationData.length,
            avgThreshold: this.calibrationData.reduce((s, d) => s + d.threshold, 0) / this.calibrationData.length,
            avgEnergy: this.calibrationData.reduce((s, d) => s + d.energy, 0) / this.calibrationData.length,
            detectedBeats: this.calibrationData.filter(d => d.energy > d.threshold).length,
            data: this.calibrationData
        };
        
        const blob = new Blob([JSON.stringify(data, null, 2)], { type: 'application/json' });
        const url = URL.createObjectURL(blob);
        const a = document.createElement('a');
        a.href = url;
        a.download = `beat-calibration-${Date.now()}.json`;
        a.click();
        URL.revokeObjectURL(url);
        
        showNotification('Calibration data exported', 'success');
        return data;
    },
    
    importCalibrationData: function(jsonData) {
        try {
            const data = typeof jsonData === 'string' ? JSON.parse(jsonData) : jsonData;
            
            if (data && data.data && Array.isArray(data.data)) {
                this.calibrationData = data.data;
                this.thresholdHistory = data.data.map(d => d.threshold);
                this.sensitivityHistory = data.data.map(d => d.sensitivity);
                this.bpmHistory = data.data.map(d => d.bpm || 0);
                
                this.updateVisualization();
                this.updateStatsPanel();
                
                if (data.avgThreshold && MediaPlayer.beatDetection) {
                    MediaPlayer.beatDetection.threshold = data.avgThreshold;
                    MediaPlayer.beatDetection.userThreshold = data.avgThreshold;
                }
                
                showNotification(`Imported ${data.samples} calibration samples`, 'success');
                return true;
            }
        } catch (e) {
            showNotification('Failed to import calibration data', 'error');
        }
        return false;
    }
};

// Enhanced Scene Presets
const EnhancedScenePresets = {
    presets: {
        'meditation': { hue: 280, saturation: 60, brightness: 30, temperature: 4000, name: 'Meditation', emoji: '🧘' },
        'gaming': { hue: 300, saturation: 90, brightness: 80, temperature: 6500, name: 'Gaming', emoji: '🎮' },
        'cooking': { hue: 30, saturation: 70, brightness: 90, temperature: 4500, name: 'Cooking', emoji: '🍳' },
        'creative': { hue: 290, saturation: 75, brightness: 70, temperature: 5500, name: 'Creative', emoji: '🎨' },
        'yoga': { hue: 150, saturation: 50, brightness: 60, temperature: 4000, name: 'Yoga', emoji: '🧘‍♀️' },
        'workout': { hue: 0, saturation: 80, brightness: 90, temperature: 6500, name: 'Workout', emoji: '💪' },
        'movie': { hue: 25, saturation: 60, brightness: 40, temperature: 2700, name: 'Movie', emoji: '🎬' },
        'study': { hue: 210, saturation: 40, brightness: 80, temperature: 6000, name: 'Study', emoji: '📚' },
        'dinner': { hue: 35, saturation: 65, brightness: 50, temperature: 3000, name: 'Dinner', emoji: '🍽️' },
        'morning': { hue: 45, saturation: 50, brightness: 70, temperature: 5000, name: 'Morning', emoji: '🌅' },
        'goodnight': { hue: 220, saturation: 20, brightness: 15, temperature: 2700, name: 'Goodnight', emoji: '🌙' },
        'rainbow': { hue: 0, saturation: 100, brightness: 90, temperature: 5500, name: 'Rainbow', emoji: '🌈', special: 'rainbow' },
        'fireplace': { hue: 15, saturation: 80, brightness: 60, temperature: 2200, name: 'Fireplace', emoji: '🔥', special: 'flicker' },
        'ice': { hue: 170, saturation: 60, brightness: 80, temperature: 8000, name: 'Ice', emoji: '🧊' },
        'aurora': { hue: 140, saturation: 70, brightness: 65, temperature: 6000, name: 'Aurora', emoji: '🌌', special: 'aurora' },
        'nebula': { hue: 270, saturation: 85, brightness: 55, temperature: 7000, name: 'Nebula', emoji: '☄️', special: 'nebula' },
        'thunder': { hue: 50, saturation: 40, brightness: 85, temperature: 9000, name: 'Thunder', emoji: '⚡', special: 'thunder' },
        'cosmic': { hue: 260, saturation: 90, brightness: 50, temperature: 8000, name: 'Cosmic', emoji: '🌠', special: 'cosmic' },
        'dream': { hue: 190, saturation: 60, brightness: 45, temperature: 4500, name: 'Dream', emoji: '💭', special: 'dream' },
        'chill': { hue: 230, saturation: 45, brightness: 55, temperature: 3500, name: 'Chill', emoji: '😌' },
        'adventure': { hue: 35, saturation: 85, brightness: 75, temperature: 5000, name: 'Adventure', emoji: '🗺️', special: 'adventure' },
        'festival': { hue: 320, saturation: 95, brightness: 85, temperature: 6000, name: 'Festival', emoji: '🎉', special: 'festival' },
        'sunset': { hue: 20, saturation: 75, brightness: 65, temperature: 2800, name: 'Sunset', emoji: '🌇' },
        'ocean': { hue: 200, saturation: 70, brightness: 60, temperature: 6500, name: 'Ocean', emoji: '🌊', special: 'ocean' },
        'forest': { hue: 120, saturation: 65, brightness: 55, temperature: 4500, name: 'Forest', emoji: '🌲' },
        'candy': { hue: 330, saturation: 85, brightness: 75, temperature: 5500, name: 'Candy', emoji: '🍬' },
        'nightlight': { hue: 240, saturation: 30, brightness: 20, temperature: 2700, name: 'Nightlight', emoji: '🌃' },
        'focus': { hue: 200, saturation: 35, brightness: 85, temperature: 6500, name: 'Focus', emoji: '🎯' },
        'relax': { hue: 180, saturation: 40, brightness: 50, temperature: 4000, name: 'Relax', emoji: '😌' },
        'energize': { hue: 40, saturation: 80, brightness: 90, temperature: 6000, name: 'Energize', emoji: '⚡' },
        'galaxy': { hue: 290, saturation: 80, brightness: 60, temperature: 7000, name: 'Galaxy', emoji: '🌌', special: 'galaxy' },
        'lava': { hue: 10, saturation: 90, brightness: 70, temperature: 2500, name: 'Lava', emoji: '🌋', special: 'lava' }
    },
    
    currentScene: null,
    
    apply: function(sceneName) {
        const scene = this.presets[sceneName];
        if (!scene) return;
        
        this.currentScene = sceneName;
        
        const targets = LifXTouchControls && LifXTouchControls.multiBulbSelection && LifXTouchControls.multiBulbSelection.length > 0
            ? LifXTouchControls.multiBulbSelection.join(',')
            : 'all';
        
        if (scene.special) {
            this.applySpecialEffect(scene.special, targets);
        } else {
            $.ajax({
                url: '/api/services/lifx/set_color',
                method: 'POST',
                contentType: 'application/json',
                data: JSON.stringify({
                    selector: targets === 'all' ? 'all' : `id:${targets}`,
                    color: `hue:${scene.hue * 182} saturation:${scene.saturation}% brightness:${scene.brightness / 100}`,
                    kelvin: scene.temperature,
                    duration: 0.5
                })
            });
        }
        
        this.showSceneIndicator(scene);
        showNotification(`Scene: ${scene.name}`, 'info');
    },
    
    applySpecialEffect: function(effect, targets) {
        switch(effect) {
            case 'rainbow':
                MediaPlayer.lifxSyncMode = 'rainbow';
                break;
            case 'flicker':
                startFireEffect();
                break;
            case 'aurora':
                startAuroraEffect();
                break;
            case 'ocean':
                startOceanEffect();
                break;
            case 'nebula':
            case 'cosmic':
            case 'dream':
            case 'adventure':
            case 'festival':
            case 'thunder':
            case 'galaxy':
            case 'lava':
                this.startDynamicEffect(effect, targets);
                break;
        }
    },
    
    startDynamicEffect: function(effectName, targets) {
        const effects = {
            'nebula': { 
                hues: [270, 300, 330, 280, 310], 
                duration: 4000, 
                brightness: [40, 70],
                saturation: [70, 90],
                pattern: 'smooth',
                description: 'Swirling cosmic clouds'
            },
            'cosmic': { 
                hues: [260, 290, 320, 350, 280], 
                duration: 5000, 
                brightness: [30, 60],
                saturation: [80, 100],
                pattern: 'pulse',
                description: 'Deep space rhythms'
            },
            'dream': { 
                hues: [180, 200, 220, 190, 210], 
                duration: 6000, 
                brightness: [35, 55],
                saturation: [50, 70],
                pattern: 'fade',
                description: 'Ethereal dreamscapes'
            },
            'adventure': { 
                hues: [30, 40, 50, 35, 45], 
                duration: 3000, 
                brightness: [60, 85],
                saturation: [70, 90],
                pattern: 'energetic',
                description: 'Bold exploration'
            },
            'festival': { 
                hues: [320, 340, 0, 20, 200, 220], 
                duration: 2000, 
                brightness: [70, 95],
                saturation: [85, 100],
                pattern: 'party',
                description: 'Celebration mode'
            },
            'thunder': { 
                hues: [50, 0, 60], 
                duration: 2000,
                brightness: [40, 100],
                saturation: [30, 60],
                pattern: 'storm',
                flash: true,
                description: 'Dramatic lightning storms'
            },
            'galaxy': {
                hues: [280, 300, 320, 260, 290, 310],
                duration: 5000,
                brightness: [40, 70],
                saturation: [75, 95],
                pattern: 'smooth',
                description: 'Spiral galaxy rotation'
            },
            'lava': {
                hues: [10, 15, 20, 5, 25],
                duration: 3000,
                brightness: [50, 85],
                saturation: [80, 100],
                pattern: 'pulse',
                description: 'Molten lava flow'
            }
        };
        
        const effect = effects[effectName];
        if (!effect) return;
        
        let hueIndex = 0;
        let brightnessUp = true;
        let saturationUp = true;
        let currentBrightness = effect.brightness[0];
        let currentSaturation = effect.saturation[0];
        let phase = 0;
        
        const applyEffect = () => {
            if (this.currentScene !== effectName) {
                return;
            }
            
            phase += 0.1;
            
            switch(effect.pattern) {
                case 'smooth':
                    hueIndex = (hueIndex + 1) % effect.hues.length;
                    currentBrightness = effect.brightness[0] + 
                        (Math.sin(phase) * (effect.brightness[1] - effect.brightness[0]) / 2) + 
                        (effect.brightness[1] + effect.brightness[0]) / 2;
                    currentSaturation = effect.saturation[0] + 
                        (Math.cos(phase) * (effect.saturation[1] - effect.saturation[0]) / 2) + 
                        (effect.saturation[1] + effect.saturation[0]) / 2;
                    break;
                    
                case 'pulse':
                    hueIndex = (hueIndex + 1) % effect.hues.length;
                    currentBrightness = effect.brightness[0] + 
                        (Math.abs(Math.sin(phase)) * (effect.brightness[1] - effect.brightness[0]));
                    currentSaturation = effect.saturation[1];
                    break;
                    
                case 'fade':
                    hueIndex = Math.floor((phase / (Math.PI * 2)) * effect.hues.length) % effect.hues.length;
                    currentBrightness = effect.brightness[0] + 
                        ((Math.sin(phase) + 1) / 2 * (effect.brightness[1] - effect.brightness[0]));
                    currentSaturation = effect.saturation[0] + 
                        ((Math.cos(phase * 0.5) + 1) / 2 * (effect.saturation[1] - effect.saturation[0]));
                    break;
                    
                case 'energetic':
                    if (Math.random() > 0.7) hueIndex = (hueIndex + 1) % effect.hues.length;
                    currentBrightness = effect.brightness[1] - (Math.random() * 10);
                    currentSaturation = effect.saturation[1];
                    break;
                    
                case 'party':
                    hueIndex = (hueIndex + Math.floor(Math.random() * 2) + 1) % effect.hues.length;
                    currentBrightness = effect.brightness[0] + (Math.random() * (effect.brightness[1] - effect.brightness[0]));
                    currentSaturation = effect.saturation[1];
                    break;
                    
                case 'storm':
                    if (effect.flash && Math.random() > 0.85) {
                        currentBrightness = 100;
                        currentSaturation = 20;
                        hueIndex = Math.floor(Math.random() * effect.hues.length);
                    } else {
                        currentBrightness = effect.brightness[0] + (Math.random() * 20);
                        currentSaturation = effect.saturation[0] + (Math.random() * 20);
                        if (Math.random() > 0.6) hueIndex = (hueIndex + 1) % effect.hues.length;
                    }
                    break;
            }
            
            const hue = effect.hues[hueIndex];
            
            $.ajax({
                url: '/api/services/lifx/set_color',
                method: 'POST',
                contentType: 'application/json',
                data: JSON.stringify({
                    selector: targets === 'all' ? 'all' : `id:${targets}`,
                    color: `hue:${hue * 182} saturation:${Math.round(currentSaturation)}% brightness:${Math.round(currentBrightness) / 100}`,
                    duration: effect.duration / 1000
                })
            });
        };
        
        const intervalId = setInterval(applyEffect, effect.duration / 8);
        
        this.dynamicEffectIntervals = this.dynamicEffectIntervals || {};
        this.dynamicEffectIntervals[effectName] = intervalId;
    },
    
    stopDynamicEffect: function(effectName) {
        if (this.dynamicEffectIntervals && this.dynamicEffectIntervals[effectName]) {
            clearInterval(this.dynamicEffectIntervals[effectName]);
            delete this.dynamicEffectIntervals[effectName];
        }
    },
    
    stopAllDynamicEffects: function() {
        if (this.dynamicEffectIntervals) {
            for (const effectName in this.dynamicEffectIntervals) {
                clearInterval(this.dynamicEffectIntervals[effectName]);
            }
            this.dynamicEffectIntervals = {};
        }
    },
    
    showSceneIndicator: function(scene) {
        let indicator = document.querySelector('.scene-indicator');
        if (!indicator) {
            indicator = document.createElement('div');
            indicator.className = 'scene-indicator';
            document.body.appendChild(indicator);
        }
        
        indicator.className = `scene-indicator ${this.currentScene}`;
        indicator.innerHTML = `${scene.emoji} ${scene.name}`;
        indicator.style.display = 'block';
        
        setTimeout(() => {
            indicator.style.display = 'none';
        }, 3000);
    },
    
    showSceneSelector: function() {
        if (typeof Swal === 'undefined') {
            alert('Scene Selector requires SweetAlert2');
            return;
        }
        
        const sceneKeys = Object.keys(this.presets);
        const categorizedScenes = {
            'Ambient': ['meditation', 'yoga', 'chill', 'goodnight'],
            'Focus': ['study', 'creative', 'gaming', 'morning'],
            'Activity': ['cooking', 'workout', 'adventure', 'festival'],
            'Entertainment': ['movie', 'dinner', 'party', 'rainbow'],
            'Special Effects': ['fireplace', 'aurora', 'nebula', 'cosmic', 'dream', 'thunder', 'ice', 'ocean']
        };
        
        let categoriesHtml = '';
        for (const [category, scenes] of Object.entries(categorizedScenes)) {
            const sceneItems = scenes.filter(s => this.presets[s]).map(key => {
                const scene = this.presets[key];
                const gradient = this.getSceneGradient(key);
                const isSpecial = scene.special ? '<span class="special-badge"><i class="fas fa-star"></i></span>' : '';
                return `
                    <div class="scene-preview-item-enhanced ${this.currentScene === key ? 'active' : ''}" 
                         onclick="EnhancedScenePresets.apply('${key}'); if (typeof Swal !== 'undefined') Swal.close();">
                        <div class="scene-gradient-preview" style="${gradient}">
                            ${isSpecial}
                        </div>
                        <div class="scene-preview-emoji">${scene.emoji}</div>
                        <div class="scene-preview-label-enhanced">${scene.name}</div>
                    </div>
                `;
            }).join('');
            
            categoriesHtml += `
                <div class="scene-category">
                    <h4 style="color: #00d4ff; margin-bottom: 10px; font-size: 14px;">
                        <i class="fas fa-${this.getCategoryIcon(category)}"></i> ${category}
                    </h4>
                    <div class="scene-preview-grid-enhanced" style="margin-bottom: 20px;">
                        ${sceneItems}
                    </div>
                </div>
            `;
        }
        
        Swal.fire({
            title: '<i class="fas fa-palette"></i> Scene Presets',
            html: `
                <div class="scene-selector-categorized">
                    ${categoriesHtml}
                </div>
                <div style="margin-top: 20px; padding: 15px; background: rgba(42, 42, 58, 0.5); border-radius: 10px;">
                    <h5 style="color: #00d4ff; margin-bottom: 10px; font-size: 14px;">
                        <i class="fas fa-info-circle"></i> Tips
                    </h5>
                    <ul style="color: #adb5bd; font-size: 12px; margin: 0; padding-left: 20px;">
                        <li>Tap any scene to apply it instantly</li>
                        <li>Special effect scenes have dynamic animations</li>
                        <li>Double-tap ambient button to cycle scenes</li>
                        <li>Hold scene button for quick settings</li>
                    </ul>
                </div>
            `,
            showConfirmButton: false,
            showCloseButton: true,
            width: '700px',
            customClass: {
                html: 'scene-selector-html'
            }
        });
    },
    
    getCategoryIcon: function(category) {
        const icons = {
            'Ambient': 'cloud-moon',
            'Focus': 'brain',
            'Activity': 'running',
            'Entertainment': 'film',
            'Special Effects': 'magic'
        };
        return icons[category] || 'lightbulb';
    },
    
    getSceneGradient: function(sceneName) {
        const gradients = {
            'meditation': 'background: linear-gradient(135deg, #9b59b6, #8e44ad);',
            'gaming': 'background: linear-gradient(135deg, #9b59b6, #e91e63);',
            'cooking': 'background: linear-gradient(135deg, #f39c12, #e67e22);',
            'creative': 'background: linear-gradient(135deg, #8e44ad, #9b59b6);',
            'yoga': 'background: linear-gradient(135deg, #27ae60, #2ecc71);',
            'workout': 'background: linear-gradient(135deg, #e74c3c, #c0392b);',
            'movie': 'background: linear-gradient(135deg, #d35400, #e67e22);',
            'study': 'background: linear-gradient(135deg, #3498db, #2980b9);',
            'dinner': 'background: linear-gradient(135deg, #f39c12, #d35400);',
            'morning': 'background: linear-gradient(135deg, #f1c40f, #f39c12);',
            'goodnight': 'background: linear-gradient(135deg, #2c3e50, #34495e);',
            'rainbow': 'background: linear-gradient(135deg, #ff0080, #00ff80, #0080ff); background-size: 200% 200%;',
            'fireplace': 'background: linear-gradient(135deg, #ff4500, #ff8c00);',
            'ice': 'background: linear-gradient(135deg, #7fffd4, #00ced1);',
            'aurora': 'background: linear-gradient(135deg, #00ff88, #00ced1, #0080ff);',
            'nebula': 'background: linear-gradient(135deg, #9b59b6, #e91e63, #3498db);',
            'thunder': 'background: linear-gradient(135deg, #f1c40f, #fff);',
            'cosmic': 'background: linear-gradient(135deg, #4a0072, #7b1fa2, #e91e63);',
            'dream': 'background: linear-gradient(135deg, #4fc3f7, #4dd0e1, #80cbc4);',
            'chill': 'background: linear-gradient(135deg, #5c6bc0, #7986cb, #90a4ae);',
            'adventure': 'background: linear-gradient(135deg, #ff6f00, #ff8f00, #ffa000);',
            'festival': 'background: linear-gradient(135deg, #e91e63, #9c27b0, #ff4081);'
        };
        return gradients[sceneName] || 'background: linear-gradient(135deg, #27a0b9, #00d4ff);';
    }
};

// Real-time BPM Indicator
function showBpmIndicator() {
    let indicator = document.querySelector('.bpm-realtime-indicator');
    if (!indicator) {
        indicator = document.createElement('div');
        indicator.className = 'bpm-realtime-indicator';
        indicator.innerHTML = `
            <i class="fas fa-heartbeat bpm-icon"></i>
            <span class="bpm-value" id="realtime-bpm-value">--</span>
            <span class="bpm-label">BPM</span>
        `;
        document.body.appendChild(indicator);
    }
    
    indicator.classList.add('visible');
    updateBpmValue();
}

function hideBpmIndicator() {
    const indicator = document.querySelector('.bpm-realtime-indicator');
    if (indicator) {
        indicator.classList.remove('visible');
        setTimeout(() => indicator.remove(), 300);
    }
}

function updateBpmValue() {
    const valueEl = document.getElementById('realtime-bpm-value');
    const bpmDisplay = document.querySelector('#bpm-display');
    
    if (MediaPlayer.lifxBeatDetection && MediaPlayer.lifxBeatDetection.bpmEstimate) {
        const bpm = Math.round(MediaPlayer.lifxBeatDetection.bpmEstimate);
        if (valueEl) valueEl.textContent = bpm;
    } else if (MediaPlayer.beatDetection && MediaPlayer.beatDetection.bpmEstimate) {
        const bpm = Math.round(MediaPlayer.beatDetection.bpmEstimate);
        if (valueEl) valueEl.textContent = bpm;
    }
    
    setTimeout(() => updateBpmValue(), 500);
}

// Enhanced Touch Feedback
function showEnhancedTouchFeedback(x, y, type, message) {
    const feedback = document.createElement('div');
    feedback.className = 'touch-gesture-feedback';
    feedback.innerHTML = `<div class="swipe-indicator">${message}</div>`;
    document.body.appendChild(feedback);
    
    setTimeout(() => {
        feedback.querySelector('.swipe-indicator').classList.add('visible');
    }, 10);
    
    setTimeout(() => {
        feedback.remove();
    }, 1000);
}

// Media Sync Quick Settings
function showMediaSyncQuickSettings() {
    if (typeof Swal === 'undefined') {
        alert('Media Sync Quick Settings requires SweetAlert2');
        return;
    }
    
    const currentBpm = MediaPlayer.lifxBeatDetection.bpmEstimate || MediaPlayer.beatDetection?.bpmEstimate || 0;
    const currentThreshold = MediaPlayer.beatDetection?.threshold || 0.3;
    const currentSensitivity = MediaPlayer.beatDetection?.sensitivity || MediaPlayer.lifxBeatDetection?.sensitivity || 'medium';
    
    Swal.fire({
        title: '<i class="fas fa-sliders-h"></i> Media Sync Quick Settings',
        html: `
            <div class="media-sync-dashboard">
                <div class="sync-status-grid">
                    <div class="sync-status-card ${MediaPlayer.lifxSyncEnabled ? 'active' : ''}">
                        <div class="status-icon">${MediaPlayer.lifxSyncEnabled ? '🎵💡' : '💡'}</div>
                        <div class="status-label">LIFX Sync</div>
                        <div class="status-value">${MediaPlayer.lifxSyncEnabled ? 'ON' : 'OFF'}</div>
                        <button class="btn-toggle" onclick="toggleLifxMediaSync(); showMediaSyncQuickSettings();">
                            <i class="fas fa-toggle-${MediaPlayer.lifxSyncEnabled ? 'on' : 'off'}"></i>
                        </button>
                    </div>
                    <div class="sync-status-card ${MediaPlayer.ambientLightEnabled ? 'active' : ''}">
                        <div class="status-icon">${MediaPlayer.ambientLightEnabled ? '🌈' : '🌑'}</div>
                        <div class="status-label">Ambient Light</div>
                        <div class="status-value">${MediaPlayer.ambientLightEnabled ? 'ON' : 'OFF'}</div>
                        <button class="btn-toggle" onclick="toggleAmbientLight(); showMediaSyncQuickSettings();">
                            <i class="fas fa-toggle-${MediaPlayer.ambientLightEnabled ? 'on' : 'off'}"></i>
                        </button>
                    </div>
                    <div class="sync-status-card ${MediaPlayer.beatDetection?.enabled || MediaPlayer.lifxBeatDetection?.enabled ? 'active' : ''}">
                        <div class="status-icon">🎵</div>
                        <div class="status-label">Beat Detection</div>
                        <div class="status-value">${MediaPlayer.beatDetection?.enabled || MediaPlayer.lifxBeatDetection?.enabled ? 'ON' : 'OFF'}</div>
                        <button class="btn-toggle" onclick="toggleBeatDetection(); showMediaSyncQuickSettings();">
                            <i class="fas fa-toggle-${MediaPlayer.beatDetection?.enabled || MediaPlayer.lifxBeatDetection?.enabled ? 'on' : 'off'}"></i>
                        </button>
                    </div>
                    <div class="sync-status-card ${MediaPlayer.isPlaying ? 'active' : ''}">
                        <div class="status-icon">${MediaPlayer.isPlaying ? '▶️' : '⏸'}</div>
                        <div class="status-label">Playback</div>
                        <div class="status-value">${MediaPlayer.isPlaying ? 'PLAYING' : 'PAUSED'}</div>
                        <button class="btn-toggle" onclick="togglePlayPause(); showMediaSyncQuickSettings();">
                            <i class="fas fa-${MediaPlayer.isPlaying ? 'pause' : 'play'}"></i>
                        </button>
                    </div>
                </div>
                
                <div class="quick-actions-row">
                    <button class="btn-quick-action" onclick="cycleAmbientLightMode(); showMediaSyncQuickSettings();">
                        <i class="fas fa-sync"></i> Cycle Mode
                    </button>
                    <button class="btn-quick-action" onclick="showMediaSyncSettings('beat'); if (typeof Swal !== 'undefined') Swal.close();">
                        <i class="fas fa-wave-square"></i> Beat Settings
                    </button>
                    <button class="btn-quick-action" onclick="showMediaSyncSettings('lifx'); if (typeof Swal !== 'undefined') Swal.close();">
                        <i class="fas fa-lightbulb"></i> LIFX Settings
                    </button>
                    <button class="btn-quick-action" onclick="EnhancedScenePresets.showSceneSelector(); if (typeof Swal !== 'undefined') Swal.close();">
                        <i class="fas fa-palette"></i> Scenes
                    </button>
                </div>
                
                <div class="real-time-metrics">
                    <h4><i class="fas fa-chart-line"></i> Real-time Metrics</h4>
                    <div class="metrics-grid">
                        <div class="metric-item">
                            <span class="metric-label">BPM</span>
                            <span class="metric-value bpm-value">${currentBpm > 0 ? Math.round(currentBpm) : '--'}</span>
                        </div>
                        <div class="metric-item">
                            <span class="metric-label">Threshold</span>
                            <span class="metric-value">${currentThreshold.toFixed(2)}</span>
                        </div>
                        <div class="metric-item">
                            <span class="metric-label">Sensitivity</span>
                            <span class="metric-value">${currentSensitivity}</span>
                        </div>
                        <div class="metric-item">
                            <span class="metric-label">Sync Mode</span>
                            <span class="metric-value">${MediaPlayer.lifxSyncMode || 'pulse'}</span>
                        </div>
                    </div>
                </div>
                
                <div class="beat-detection-calibration">
                    <h4><i class="fas fa-chart-bar"></i> Real-time Energy Visualization</h4>
                    <div class="beat-energy-visualization"></div>
                    <div class="calibration-stats"></div>
                    <div class="calibration-controls">
                        <button class="btn btn-sm btn-primary" onclick="BeatDetectionCalibration.start()">
                            <i class="fas fa-play"></i> Start
                        </button>
                        <button class="btn btn-sm btn-success" onclick="BeatDetectionCalibration.autoCalibrate()">
                            <i class="fas fa-magic"></i> Auto
                        </button>
                        <button class="btn btn-sm btn-info" onclick="BeatDetectionCalibration.exportCalibrationData()">
                            <i class="fas fa-download"></i> Export
                        </button>
                        <button class="btn btn-sm btn-secondary" onclick="BeatDetectionCalibration.reset()">
                            <i class="fas fa-undo"></i> Reset
                        </button>
                        <button class="btn btn-sm btn-danger" onclick="BeatDetectionCalibration.stop(); showMediaSyncQuickSettings();">
                            <i class="fas fa-stop"></i> Stop
                        </button>
                    </div>
                </div>
            </div>
        `,
        showConfirmButton: false,
        showCloseButton: true,
        width: '750px',
        customClass: {
            html: 'media-sync-dashboard-html'
        }
    });
    
    setTimeout(() => {
        BeatDetectionCalibration.start();
    }, 100);
}

function showBeatDetectionCalibration() {
    BeatDetectionCalibration.start();
}

// Enhanced touch feedback with visual trail
function createTouchTrail(x, y) {
    if (!MediaPlayer.touchTrailEnabled) return;
    
    const trail = document.createElement('div');
    trail.className = 'touch-trail';
    trail.style.left = (x - 10) + 'px';
    trail.style.top = (y - 10) + 'px';
    document.body.appendChild(trail);
    
    setTimeout(() => {
        if (trail.parentNode) trail.remove();
    }, 400);
}

// Enhanced gesture feedback with animation
function showEnhancedGestureFeedback(gestureType, direction, x, y) {
    const feedback = document.createElement('div');
    feedback.className = 'touch-gesture-indicator visible';
    feedback.style.left = x + 'px';
    feedback.style.top = y + 'px';
    feedback.style.transform = 'translate(-50%, -50%)';
    
    const icons = {
        'swipe-left': '←',
        'swipe-right': '→',
        'swipe-up': '↑',
        'swipe-down': '↓',
        'tap': '👆',
        'double-tap': '👆👆',
        'long-press': '⏱',
        'pinch-in': '🤏',
        'pinch-out': '👐'
    };
    
    const icon = icons[`${gestureType}-${direction}`] || icons[gestureType] || '✨';
    feedback.innerHTML = `<span class="swipe-direction-arrow">${icon}</span>`;
    document.body.appendChild(feedback);
    
    setTimeout(() => {
        feedback.remove();
    }, 800);
}

// Gesture history for undo functionality
function recordGesture(gesture) {
    MediaPlayer.gestureHistory.push({
        ...gesture,
        timestamp: Date.now()
    });
    
    if (MediaPlayer.gestureHistory.length > 20) {
        MediaPlayer.gestureHistory.shift();
    }
}

function undoLastGesture() {
    const lastGesture = MediaPlayer.gestureHistory.pop();
    if (lastGesture) {
        console.log('Undoing gesture:', lastGesture);
        showNotification('Gesture undone', 'info');
        return lastGesture;
    }
    return null;
}

// Enhanced BPM indicator with animation
function updateBpmRealtime() {
    const indicator = document.querySelector('.bpm-realtime-indicator');
    const valueEl = document.getElementById('realtime-bpm-value');
    
    if (!indicator || !valueEl) return;
    
    const bpm = MediaPlayer.lifxBeatDetection?.bpmEstimate || 
                MediaPlayer.beatDetection?.bpmEstimate || 0;
    
    if (bpm > 0 && (MediaPlayer.lifxSyncEnabled || MediaPlayer.ambientLightEnabled)) {
        valueEl.textContent = Math.round(bpm);
        indicator.classList.add('visible');
        
        // Pulse the indicator with the beat
        const now = Date.now();
        const timeSinceLastBeat = now - (MediaPlayer.lifxBeatDetection?.lastBeatTime || 0);
        const beatInterval = 60000 / bpm;
        
        if (timeSinceLastBeat < beatInterval * 0.2) {
            indicator.style.transform = 'translateY(0) scale(1.05)';
            indicator.style.boxShadow = '0 0 30px rgba(255, 107, 107, 0.8)';
        } else {
            indicator.style.transform = 'translateY(0) scale(1)';
            indicator.style.boxShadow = '';
        }
    } else {
        indicator.classList.remove('visible');
    }
}

// Smooth color transition for LIFX sync
function smoothColorTransition(targetHue, targetSaturation, targetBrightness, duration) {
    const startHue = MediaPlayer.lastHue || 0;
    const startSat = MediaPlayer.lastSaturation || 100;
    const startBright = MediaPlayer.lastBrightness || 50;
    
    const startTime = Date.now();
    
    function animate() {
        const elapsed = Date.now() - startTime;
        const progress = Math.min(1, elapsed / duration);
        
        // Ease out cubic
        const eased = 1 - Math.pow(1 - progress, 3);
        
        const currentHue = startHue + (targetHue - startHue) * eased;
        const currentSat = startSat + (targetSaturation - startSat) * eased;
        const currentBright = startBright + (targetBrightness - startBright) * eased;
        
        MediaPlayer.lastHue = currentHue;
        MediaPlayer.lastSaturation = currentSat;
        MediaPlayer.lastBrightness = currentBright;
        
        if (progress < 1) {
            requestAnimationFrame(animate);
        }
    }
    
    animate();
}

// Improved beat detection with adaptive thresholding
function enhanceBeatDetection() {
    const detection = MediaPlayer.lifxBeatDetection;
    
    if (!detection) return;
    
    // Adaptive sensitivity based on recent beat history
    const recentBeats = detection.beatHistory?.slice(-8) || [];
    const beatCount = recentBeats.length;
    
    if (beatCount < 3) {
        // Not enough data, use default sensitivity
        detection.adaptiveThreshold = 0.25;
    } else {
        // Calculate beat consistency
        const intervals = [];
        for (let i = 1; i < recentBeats.length; i++) {
            intervals.push(recentBeats[i].time - recentBeats[i-1].time);
        }
        
        const avgInterval = intervals.reduce((a, b) => a + b, 0) / intervals.length;
        const variance = intervals.reduce((sum, i) => sum + Math.pow(i - avgInterval, 2), 0) / intervals.length;
        const stdDev = Math.sqrt(variance);
        const consistency = 1 - Math.min(1, stdDev / avgInterval);
        
        // Adjust threshold based on consistency
        if (consistency > 0.8) {
            // Very consistent beats, can be more sensitive
            detection.adaptiveThreshold = Math.max(0.15, detection.threshold * 0.8);
        } else if (consistency < 0.4) {
            // Inconsistent, raise threshold to avoid false positives
            detection.adaptiveThreshold = Math.min(0.5, detection.threshold * 1.2);
        }
    }
}

// Media playback quality indicator
function showPlaybackQualityIndicator(quality) {
    const indicator = document.createElement('div');
    indicator.className = 'playback-quality-indicator';
    indicator.style.cssText = `
        position: fixed;
        top: 20px;
        right: 20px;
        background: rgba(42, 42, 58, 0.9);
        border: 2px solid ${quality === 'high' ? '#00ff88' : quality === 'medium' ? '#ffaa00' : '#ff6b6b'};
        border-radius: 8px;
        padding: 8px 12px;
        z-index: 9999;
        color: #fff;
        font-size: 12px;
        font-weight: bold;
        opacity: 0;
        transition: opacity 0.3s ease;
    `;
    indicator.innerHTML = `
        <i class="fas fa-${quality === 'high' ? 'signal' : quality === 'medium' ? 'wifi' : 'wifi-off'}"></i>
        ${quality.toUpperCase()}
    `;
    document.body.appendChild(indicator);
    
    setTimeout(() => indicator.classList.add('visible'), 10);
    setTimeout(() => {
        indicator.classList.remove('visible');
        setTimeout(() => indicator.remove(), 300);
    }, 3000);
}

// Queue management enhancements
function showQueuePositionToast(position, total) {
    const toast = document.createElement('div');
    toast.className = 'queue-position-toast';
    toast.style.cssText = `
        position: fixed;
        bottom: 80px;
        left: 50%;
        transform: translateX(-50%) translateY(20px);
        background: rgba(42, 42, 58, 0.95);
        border: 1px solid rgba(0, 212, 255, 0.5);
        border-radius: 12px;
        padding: 12px 24px;
        z-index: 9999;
        color: #00d4ff;
        font-size: 14px;
        opacity: 0;
        transition: all 0.3s ease;
    `;
    toast.innerHTML = `
        <i class="fas fa-list-ul"></i>
        Track ${position} of ${total} in queue
    `;
    document.body.appendChild(toast);
    
    setTimeout(() => {
        toast.style.opacity = '1';
        toast.style.transform = 'translateX(-50%) translateY(0)';
    }, 10);
    
    setTimeout(() => {
        toast.style.opacity = '0';
        toast.style.transform = 'translateX(-50%) translateY(20px)';
        setTimeout(() => toast.remove(), 300);
    }, 2000);
}

// Initialize enhanced media features
function initEnhancedMediaFeatures() {
    console.log('Enhanced media features initialized');
    
    // Update BPM indicator periodically
    setInterval(updateBpmRealtime, 200);
    
    // Enhance beat detection periodically
    setInterval(enhanceBeatDetection, 1000);
    
    document.addEventListener('DOMContentLoaded', function() {
        showBpmIndicator();
        
        // Add touch trail to media player
        const mediaPlayer = document.querySelector('#media-player, #snapcast-player');
        if (mediaPlayer && MediaPlayer.touchTrailEnabled) {
            mediaPlayer.addEventListener('touchmove', (e) => {
                const touch = e.touches[0];
                createTouchTrail(touch.clientX, touch.clientY);
            }, { passive: true });
        }
    });
}

initEnhancedMediaFeatures();
