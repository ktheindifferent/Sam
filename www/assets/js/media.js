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
        holdProgressInterval: 50
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
    pulseInterval: null
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

function showMediaTouchHint(text, icon, duration = 1500) {
    let hint = document.querySelector('.media-center-touch-hint');
    if (!hint) {
        hint = document.createElement('div');
        hint.className = 'media-center-touch-hint';
        document.body.appendChild(hint);
    }
    hint.innerHTML = `<span style="font-size: 32px; display: block; margin-bottom: 10px; animation: hint-bounce 0.5s ease;">${icon || ''}</span>${text}`;
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
    
    updateBpmDisplay();
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
    if (bpmEl && MediaPlayer.beatDetection.bpmEstimate) {
        bpmEl.textContent = `${MediaPlayer.beatDetection.bpmEstimate} BPM`;
    }
    
    const bpmValueEl = document.querySelector('#bpm-value');
    if (bpmValueEl) {
        bpmValueEl.textContent = MediaPlayer.beatDetection.bpmEstimate || '--';
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
    
    const avgEnergy = (bassEnergy * 0.5) + (midEnergy * 0.3) + (trebleEnergy * 0.2);
    
    const beatDetection = MediaPlayer.lifxBeatDetection;
    beatDetection.energyHistory.push(avgEnergy);
    if (beatDetection.energyHistory.length > 20) {
        beatDetection.energyHistory.shift();
    }
    
    const history = beatDetection.beatHistory;
    let recentAvgStrength = 0.3;
    if (history.length > 0) {
        recentAvgStrength = history.reduce((a, b) => a + b.strength, 0) / history.length;
    }
    
    const avgEnergyHistory = beatDetection.energyHistory.reduce((a, b) => a + b, 0) / beatDetection.energyHistory.length;
    const energyVariance = beatDetection.energyHistory.reduce((sum, e) => sum + Math.pow(e - avgEnergyHistory, 2), 0) / beatDetection.energyHistory.length;
    const varianceFactor = Math.min(0.2, Math.sqrt(energyVariance) * 0.5);
    
    const dynamicThreshold = Math.max(0.15, Math.min(0.45, 
        (avgEnergyHistory * (1 - beatDetection.dynamicThresholdFactor)) + 
        (recentAvgStrength * beatDetection.dynamicThresholdFactor) + 
        varianceFactor));
    
    const beatStrength = (bassEnergy * 0.6) + (midEnergy * 0.25) + (trebleEnergy * 0.15);
    
    if (beatStrength > dynamicThreshold) {
        const now = Date.now();
        const timeSinceLastBeat = now - beatDetection.lastBeat;
        
        if (timeSinceLastBeat > beatDetection.beatCooldown) {
            const normalizedStrength = Math.min(1.0, 0.5 + ((beatStrength - dynamicThreshold) / 0.4));
            
            const bpm = estimateBPM(now);
            if (bpm) {
                beatDetection.bpmEstimate = bpm;
            }
            
            beatDetection.beatIntensity = normalizedStrength;
            beatDetection.consecutiveBeats = (beatDetection.consecutiveBeats || 0) + 1;
            beatDetection.missedBeats = 0;
            
            const estimatedBeatInterval = 60000 / beatDetection.bpmEstimate;
            beatDetection.beatCooldown = Math.max(100, estimatedBeatInterval * 0.4);
            
            pulseLifxWithBeat(normalizedStrength);
            return normalizedStrength;
        }
    } else {
        beatDetection.missedBeats = (beatDetection.missedBeats || 0) + 1;
        if (beatDetection.missedBeats > 5) {
            beatDetection.consecutiveBeats = 0;
        }
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
    
    mediaContainer.addEventListener('touchstart', (e) => {
        touchStartX = e.touches[0].clientX;
        touchStartY = e.touches[0].clientY;
        touchStartTime = Date.now();
        
        const settings = MediaPlayer.touchGestures;
        if (settings.enabled && settings.longPressDelay > 0) {
            longPressTimer = setTimeout(() => {
                showMediaTouchHint('Long Press', '👆');
            }, settings.longPressDelay);
        }
    }, { passive: true });
    
    mediaContainer.addEventListener('touchmove', (e) => {
        if (longPressTimer) {
            clearTimeout(longPressTimer);
            longPressTimer = null;
        }
    }, { passive: true });
    
    mediaContainer.addEventListener('touchend', (e) => {
        if (longPressTimer) {
            clearTimeout(longPressTimer);
            longPressTimer = null;
        }
        
        const touchEndX = e.changedTouches[0].clientX;
        const touchEndY = e.changedTouches[0].clientY;
        const deltaX = touchEndX - touchStartX;
        const deltaY = touchEndY - touchStartY;
        const currentTime = Date.now();
        const touchDuration = currentTime - touchStartTime;
        
        const settings = MediaPlayer.touchGestures;
        const distance = Math.sqrt(deltaX * deltaX + deltaY * deltaY);
        const velocity = distance / touchDuration;
        
        if (distance < settings.swipeThreshold * 0.5) {
            const tapLength = currentTime - lastTapTime;
            if (tapLength < settings.doubleTapTimeout && tapLength > 0) {
                togglePlayPause();
                showMediaTouchHint('Play/Pause', '⏯️', 1200);
            }
            lastTapTime = currentTime;
        } else if (Math.abs(deltaX) > Math.abs(deltaY) * 2) {
            if (deltaX > settings.swipeThreshold && velocity > settings.velocityThreshold) {
                previousTrack();
                showMediaTouchHint('Previous', '⏮️', 1200);
            } else if (deltaX < -settings.swipeThreshold && velocity > settings.velocityThreshold) {
                nextTrack();
                showMediaTouchHint('Next', '⏭️', 1200);
            }
        } else if (Math.abs(deltaY) > Math.abs(deltaX) * 2) {
            if (deltaY > settings.swipeThreshold && velocity > settings.velocityThreshold) {
                increaseVolume();
                showMediaTouchHint('Volume Up', '🔊', 1200);
            } else if (deltaY < -settings.swipeThreshold && velocity > settings.velocityThreshold) {
                decreaseVolume();
                showMediaTouchHint('Volume Down', '🔇', 1200);
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

function showBeatDetectionSettings() {
    showMediaSyncSettings('beat');
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
                            <h5 style="color: #00d4ff; margin-bottom: 10px; font-size: 14px;"><i class="fas fa-info-circle"></i> How It Works</h5>
                            <ul style="color: #adb5bd; font-size: 12px; margin: 0; padding-left: 20px;">
                                <li>Beat mode pulses lights to music rhythm</li>
                                <li>Ambient mode creates smooth color transitions</li>
                                <li>Visualizer mode shows frequency spectrum</li>
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
