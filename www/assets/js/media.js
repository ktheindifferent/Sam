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
    snapcastClients: []
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
    document.querySelectorAll('.media-control-btn').forEach(btn => {
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
    const mediaPlayer = document.querySelector('#media-player');
    if (mediaPlayer) {
        onGesture('swipeLeft', function() {
            nextTrack();
            showSwipeHint('Next Track');
        });

        onGesture('swipeRight', function() {
            previousTrack();
            showSwipeHint('Previous Track');
        });

        onGesture('swipeUp', function() {
            increaseVolume();
            showSwipeHint('Volume Up');
        });

        onGesture('swipeDown', function() {
            decreaseVolume();
            showSwipeHint('Volume Down');
        });
    }
}

// Keyboard shortcuts
function initKeyboardShortcuts() {
    document.addEventListener('keydown', function(e) {
        // Only handle shortcuts when not typing in an input
        if (e.target.tagName === 'INPUT' || e.target.tagName === 'TEXTAREA') {
            return;
        }

        switch(e.code) {
            case 'Space':
                e.preventDefault();
                togglePlayPause();
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
        }
    });
}

// Media control functions
function togglePlayPause() {
    MediaPlayer.isPlaying = !MediaPlayer.isPlaying;
    const playBtn = document.querySelector('#play-pause-btn');
    if (playBtn) {
        playBtn.innerHTML = MediaPlayer.isPlaying
            ? '<i class="fas fa-pause"></i>'
            : '<i class="fas fa-play"></i>';
    }
    console.log('Play/Pause toggled:', MediaPlayer.isPlaying);
}

function nextTrack() {
    console.log('Next track');
    // TODO: Integrate with actual media backend
    showNotification('Next track', 'info');
}

function previousTrack() {
    console.log('Previous track');
    // TODO: Integrate with actual media backend
    showNotification('Previous track', 'info');
}

function setVolume(level) {
    MediaPlayer.volume = parseInt(level);
    console.log('Volume set to:', MediaPlayer.volume);
    // TODO: Send to Snapcast backend
}

function increaseVolume() {
    setVolume(Math.min(100, MediaPlayer.volume + 10));
}

function decreaseVolume() {
    setVolume(Math.max(0, MediaPlayer.volume - 10));
}

function toggleMute() {
    MediaPlayer.isMuted = !MediaPlayer.isMuted;
    const muteBtn = document.querySelector('#mute-btn');
    if (muteBtn) {
        muteBtn.innerHTML = MediaPlayer.isMuted
            ? '<i class="fas fa-volume-mute"></i>'
            : '<i class="fas fa-volume-up"></i>';
    }
    console.log('Mute toggled:', MediaPlayer.isMuted);
}

// Snapcast integration
function initSnapcastStatus() {
    // Check Snapcast server status
    fetch('/api/services/snapcast/status')
        .then(response => response.json())
        .then(data => {
            updateSnapcastStatus(data);
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
            <div class="snapcast-client">
                <i class="fas fa-${client.connected ? 'wifi' : 'wifi-off'}"></i>
                <span>${client.name || 'Unknown'}</span>
                <span class="volume">${client.volume?.muted ? '🔇' : '🔊'} ${client.volume?.percent ?? 0}%</span>
            </div>
        `).join('');
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
