/**
 * SAM Touch Interface & Media Center Enhancements
 * Enhanced touch feedback, gesture controls, and media visualizations
 */

const TouchMediaEnhancements = (function() {
    'use strict';

    let bpmIndicator = null;
    let sceneIndicator = null;
    let bedtimeIndicator = null;
    let selectionToolbar = null;
    let gestureTrails = [];
    let mediaVisualizerBars = [];
    let mediaSyncActive = false;
    let mediaSyncMode = 'beat';
    let currentBpm = 0;
    let audioContext = null;
    let analyser = null;
    let mediaPlaybackState = {
        isPlaying: false,
        trackName: '',
        artistName: '',
        progress: 0,
        duration: 0
    };
    
    // Configuration
    const CONFIG = {
        gestureTrailCount: 5,
        bpmUpdateInterval: 100,
        sceneIndicatorDuration: 3000,
        touchSensitivity: 'medium',
        enableRipple: true,
        enableGestureTrails: true,
        enableBpmDisplay: true,
        enableKeyboardShortcuts: true,
        enableEdgeSwipe: true,
        enableThreeFingerSwipe: true,
        swipeThreshold: 50,
        pinchThreshold: 0.5,
        doubleTapDelay: 300,
        longPressDelay: 500,
        hapticFeedback: true,
        enableDoubleTap: true,
        enableLongPress: true,
        enableVoiceControl: false,
        animationsEnabled: true,
        lowPowerMode: false
    };

    // Initialize all enhancements
    function init() {
        setupTouchFeedback();
        setupGestureTrails();
        createBpmIndicator();
        createSceneIndicator();
        createBedtimeIndicator();
        createSelectionToolbar();
        setupMediaVisualizer();
        setupSwipeGestures();
        setupMultiSelectGestures();
        setupPinchZoom();
        setupEdgeSwipe();
        setupKeyboardShortcuts();
        setupHapticFeedback();
        setupDoubleTap();
        setupLongPress();
        setupMiniPlayer();
        setupNowPlayingToast();
        console.log('[TouchMediaEnhancements] Initialized');
    }

    // Enhanced touch feedback with ripple effect
    function setupTouchFeedback() {
        document.addEventListener('click', function(e) {
            if (!CONFIG.enableRipple) return;
            
            const target = e.target.closest('.touch-feedback, button, .btn, [role="button"]');
            if (!target) return;
            
            const ripple = document.createElement('span');
            ripple.classList.add('ripple');
            
            const rect = target.getBoundingClientRect();
            const size = Math.max(rect.width, rect.height);
            ripple.style.width = ripple.style.height = size + 'px';
            ripple.style.left = (e.clientX - rect.left - size / 2) + 'px';
            ripple.style.top = (e.clientY - rect.top - size / 2) + 'px';
            
            target.appendChild(ripple);
            
            setTimeout(() => ripple.remove(), 600);
        });
    }

    // Gesture trail effect
    function setupGestureTrails() {
        if (!CONFIG.enableGestureTrails) return;
        
        let trailTimeout;
        document.addEventListener('mousemove', function(e) {
            clearTimeout(trailTimeout);
            
            const trail = document.createElement('div');
            trail.classList.add('gesture-trail');
            trail.style.left = (e.clientX - 10) + 'px';
            trail.style.top = (e.clientY - 10) + 'px';
            
            document.body.appendChild(trail);
            gestureTrails.push(trail);
            
            setTimeout(() => {
                trail.remove();
                gestureTrails = gestureTrails.filter(t => t !== trail);
            }, 400);
            
            trailTimeout = setTimeout(() => {
                gestureTrails.forEach(t => t.remove());
                gestureTrails = [];
            }, 500);
        });
    }

    // BPM Real-time Indicator
    function createBpmIndicator() {
        if (!CONFIG.enableBpmDisplay) return;
        
        bpmIndicator = document.createElement('div');
        bpmIndicator.className = 'bpm-realtime-indicator';
        bpmIndicator.innerHTML = `
            <i class="bpm-icon fas fa-heartbeat"></i>
            <div>
                <div class="bpm-value">--</div>
                <div class="bpm-label">BPM</div>
            </div>
        `;
        document.body.appendChild(bpmIndicator);
    }

    function updateBpm(bpm) {
        if (!bpmIndicator) return;
        
        bpmIndicator.classList.add('visible');
        bpmIndicator.querySelector('.bpm-value').textContent = bpm || '--';
        
        if (!bpm) {
            setTimeout(() => bpmIndicator.classList.remove('visible'), 2000);
        }
    }

    // Scene Change Indicator
    function createSceneIndicator() {
        sceneIndicator = document.createElement('div');
        sceneIndicator.className = 'scene-indicator';
        document.body.appendChild(sceneIndicator);
    }

    function showSceneChange(sceneName) {
        if (!sceneIndicator) return;
        
        sceneIndicator.textContent = `🎨 ${sceneName}`;
        sceneIndicator.style.display = 'block';
        sceneIndicator.style.animation = 'scene-slide-in 0.3s ease, scene-fade-out 0.5s ease 2.5s forwards';
        
        setTimeout(() => {
            sceneIndicator.style.display = 'none';
        }, 3000);
    }

    // Bedtime Mode Indicator
    function createBedtimeIndicator() {
        bedtimeIndicator = document.createElement('div');
        bedtimeIndicator.className = 'bedtime-mode-active';
        bedtimeIndicator.innerHTML = `
            <i class="fas fa-moon"></i>
            <span>Bedtime Mode Active</span>
        `;
        document.body.appendChild(bedtimeIndicator);
    }

    function setBedtimeMode(active) {
        if (!bedtimeIndicator) return;
        
        if (active) {
            bedtimeIndicator.classList.add('visible');
        } else {
            bedtimeIndicator.classList.remove('visible');
        }
    }

    // Multi-bulb Selection Toolbar
    function createSelectionToolbar() {
        selectionToolbar = document.createElement('div');
        selectionToolbar.id = 'lifx-selection-toolbar';
        selectionToolbar.innerHTML = `
            <span style="color: #fff; font-size: 14px;">
                <i class="fas fa-lightbulb"></i>
                <span class="selected-count">0</span> bulbs selected
            </span>
            <button class="btn-quick-action" onclick="TouchMediaEnhancements.applyToSelected('on')">
                <i class="fas fa-power-off"></i>
                On
            </button>
            <button class="btn-quick-action" onclick="TouchMediaEnhancements.applyToSelected('off')">
                <i class="fas fa-power-off" style="opacity: 0.5;"></i>
                Off
            </button>
            <button class="btn-quick-action" onclick="TouchMediaEnhancements.clearSelection()">
                <i class="fas fa-times"></i>
                Clear
            </button>
        `;
        document.body.appendChild(selectionToolbar);
    }

    let selectedBulbs = new Set();
    
    function addToSelection(bulbId) {
        selectedBulbs.add(bulbId);
        updateSelectionToolbar();
    }

    function removeFromSelection(bulbId) {
        selectedBulbs.delete(bulbId);
        updateSelectionToolbar();
    }

    function updateSelectionToolbar() {
        if (!selectionToolbar) return;
        
        const count = selectedBulbs.size;
        selectionToolbar.querySelector('.selected-count').textContent = count;
        
        if (count > 0) {
            selectionToolbar.classList.add('visible');
        } else {
            selectionToolbar.classList.remove('visible');
        }
    }

    function clearSelection() {
        selectedBulbs.clear();
        updateSelectionToolbar();
        
        document.querySelectorAll('.lifx-bulb-control.multi-selected').forEach(el => {
            el.classList.remove('multi-selected');
        });
    }

    function applyToSelected(action) {
        if (selectedBulbs.size === 0) return;
        
        const bulbIds = Array.from(selectedBulbs);
        console.log(`[TouchMediaEnhancements] Applying ${action} to ${bulbIds.length} bulbs`);
        
        // Dispatch custom event for LIFX controls to handle
        const event = new CustomEvent('lifx-bulk-action', {
            detail: { action, bulbIds }
        });
        document.dispatchEvent(event);
        
        clearSelection();
    }

    // Setup multi-select gestures for LIFX bulbs
    function setupMultiSelectGestures() {
        let isDragging = false;
        let dragStart = null;
        let dragBox = null;
        
        document.addEventListener('mousedown', function(e) {
            if (e.target.closest('.lifx-bulb-control')) {
                isDragging = true;
                dragStart = { x: e.clientX, y: e.clientY };
            }
        });
        
        document.addEventListener('mousemove', function(e) {
            if (!isDragging || !dragStart) return;
            
            const deltaX = e.clientX - dragStart.x;
            const deltaY = e.clientY - dragStart.y;
            
            if (Math.abs(deltaX) > 10 || Math.abs(deltaY) > 10) {
                if (!dragBox) {
                    dragBox = document.createElement('div');
                    dragBox.className = 'multi-select-drag-line';
                    document.body.appendChild(dragBox);
                }
                
                const left = Math.min(dragStart.x, e.clientX);
                const top = Math.min(dragStart.y, e.clientY);
                const width = Math.abs(deltaX);
                const height = Math.abs(deltaY);
                
                dragBox.style.left = left + 'px';
                dragBox.style.top = top + 'px';
                dragBox.style.width = width + 'px';
                dragBox.style.height = height + 'px';
            }
        });
        
        document.addEventListener('mouseup', function(e) {
            if (!isDragging) return;
            isDragging = false;
            
            if (dragBox) {
                const rect = dragBox.getBoundingClientRect();
                
                document.querySelectorAll('.lifx-bulb-control').forEach(bulb => {
                    const bulbRect = bulb.getBoundingClientRect();
                    
                    if (rect.left <= bulbRect.right &&
                        rect.right >= bulbRect.left &&
                        rect.top <= bulbRect.bottom &&
                        rect.bottom >= bulbRect.top) {
                        
                        const bulbId = bulb.dataset.bulbId;
                        if (bulbId) {
                            if (selectedBulbs.has(bulbId)) {
                                removeFromSelection(bulbId);
                                bulb.classList.remove('multi-selected');
                            } else {
                                addToSelection(bulbId);
                                bulb.classList.add('multi-selected');
                            }
                        }
                    }
                });
                
                dragBox.remove();
                dragBox = null;
            }
            
            dragStart = null;
        });
    }

    // Pinch-to-zoom gesture for brightness control
    function setupPinchZoom() {
        let initialDistance = null;
        let initialBrightness = 50;
        
        document.addEventListener('touchstart', function(e) {
            if (e.touches.length === 2) {
                initialDistance = Math.hypot(
                    e.touches[0].clientX - e.touches[1].clientX,
                    e.touches[0].clientY - e.touches[1].clientY
                );
                initialBrightness = LifXTouchControls.state.brightness || 50;
                if (CONFIG.hapticFeedback) {
                    navigator.vibrate?.(10);
                }
            }
        });
        
        document.addEventListener('touchmove', function(e) {
            if (e.touches.length !== 2 || initialDistance === null) return;
            e.preventDefault();
            
            const currentDistance = Math.hypot(
                e.touches[0].clientX - e.touches[1].clientX,
                e.touches[0].clientY - e.touches[1].clientY
            );
            
            const delta = currentDistance - initialDistance;
            const brightnessChange = (delta / 10);
            const newBrightness = Math.max(0, Math.min(100, initialBrightness + brightnessChange));
            
            if (typeof LifXTouchControls !== 'undefined') {
                LifXTouchControls.setBrightness(Math.round(newBrightness));
            }
            
            const pinchIndicator = document.querySelector('.pinch-brightness-indicator');
            if (!pinchIndicator) {
                const indicator = document.createElement('div');
                indicator.className = 'pinch-brightness-indicator';
                indicator.innerHTML = `<i class="fas fa-sun"></i> <span>${Math.round(newBrightness)}%</span>`;
                document.body.appendChild(indicator);
                setTimeout(() => indicator.classList.add('visible'), 10);
            } else {
                pinchIndicator.querySelector('span').textContent = `${Math.round(newBrightness)}%`;
            }
        });
        
        document.addEventListener('touchend', function(e) {
            if (e.touches.length < 2) {
                initialDistance = null;
                const indicator = document.querySelector('.pinch-brightness-indicator');
                if (indicator) {
                    indicator.classList.remove('visible');
                    setTimeout(() => indicator.remove(), 300);
                }
            }
        });
    }

    // Edge swipe for quick panel access
    function setupEdgeSwipe() {
        if (!CONFIG.enableEdgeSwipe) return;
        
        let touchStartX = null;
        let touchStartY = null;
        const edgeThreshold = 30;
        
        document.addEventListener('touchstart', function(e) {
            touchStartX = e.changedTouches[0].screenX;
            touchStartY = e.changedTouches[0].screenY;
        });
        
        document.addEventListener('touchend', function(e) {
            const touchEndX = e.changedTouches[0].screenX;
            const touchEndY = e.changedTouches[0].screenY;
            const deltaX = touchEndX - touchStartX;
            const deltaY = touchEndY - touchStartY;
            
            if (touchStartX < edgeThreshold && deltaX > 100) {
                handleEdgeSwipe('left-edge');
            } else if (touchStartX > window.innerWidth - edgeThreshold && deltaX < -100) {
                handleEdgeSwipe('right-edge');
            } else if (touchStartY < edgeThreshold && deltaY > 100) {
                handleEdgeSwipe('top-edge');
            } else if (touchStartY > window.innerHeight - edgeThreshold && deltaY < -100) {
                handleEdgeSwipe('bottom-edge');
            }
        });
    }

    function handleEdgeSwipe(edge) {
        console.log(`[TouchMediaEnhancements] Edge swipe: ${edge}`);
        
        switch(edge) {
            case 'left-edge':
                toggleQuickScenesPanel();
                break;
            case 'right-edge':
                toggleMediaSyncPanel();
                break;
            case 'top-edge':
                document.querySelector('.header')?.classList.toggle('expanded');
                break;
            case 'bottom-edge':
                document.getElementById('media-controls-panel')?.classList.toggle('expanded');
                break;
        }
        
        if (CONFIG.hapticFeedback) {
            navigator.vibrate?.(15);
        }
    }

    // Keyboard shortcuts for media control
    function setupKeyboardShortcuts() {
        if (!CONFIG.enableKeyboardShortcuts) return;
        
        document.addEventListener('keydown', function(e) {
            if (e.target.tagName === 'INPUT' || e.target.tagName === 'TEXTAREA') return;
            
            switch(e.code) {
                case 'Space':
                    e.preventDefault();
                    mediaPlayPause();
                    break;
                case 'ArrowRight':
                    mediaNext();
                    break;
                case 'ArrowLeft':
                    mediaPrevious();
                    break;
                case 'ArrowUp':
                    e.preventDefault();
                    if (typeof LifXTouchControls !== 'undefined') {
                        const current = LifXTouchControls.state.brightness || 50;
                        LifXTouchControls.setBrightness(Math.min(100, current + 10));
                    }
                    break;
                case 'ArrowDown':
                    e.preventDefault();
                    if (typeof LifXTouchControls !== 'undefined') {
                        const current = LifXTouchControls.state.brightness || 50;
                        LifXTouchControls.setBrightness(Math.max(0, current - 10));
                    }
                    break;
                case 'KeyM':
                    if (e.ctrlKey) {
                        e.preventDefault();
                        toggleMediaSyncPanel();
                    }
                    break;
                case 'KeyL':
                    if (e.ctrlKey) {
                        e.preventDefault();
                        document.getElementById('lifx-color-picker-container')?.classList.toggle('visible');
                    }
                    break;
                case 'KeyB':
                    if (e.ctrlKey) {
                        e.preventDefault();
                        setMediaSyncMode('beat');
                    }
                    break;
                case 'KeyS':
                    if (e.ctrlKey) {
                        e.preventDefault();
                        toggleQuickScenesPanel();
                    }
                    break;
            }
        });
        
        console.log('[TouchMediaEnhancements] Keyboard shortcuts enabled');
    }

    // Haptic feedback for touch interactions
    function setupHapticFeedback() {
        if (!CONFIG.hapticFeedback || !navigator.vibrate) {
            console.log('[TouchMediaEnhancements] Haptic feedback not available');
            return;
        }
        
        document.addEventListener('click', function(e) {
            const target = e.target.closest('button, .btn, [role="button"]');
            if (target) {
                navigator.vibrate(10);
            }
        });
        
        document.addEventListener('touchstart', function(e) {
            if (e.target.closest('.lifx-bulb-control')) {
                navigator.vibrate(5);
            }
        });
    }

    // Double-tap gesture detection
    function setupDoubleTap() {
        if (!CONFIG.enableDoubleTap) return;
        
        let lastTapTime = 0;
        let lastTapTarget = null;
        
        document.addEventListener('touchend', function(e) {
            const now = Date.now();
            const target = e.target.closest('.lifx-bulb-control, .media-item, .scene-item');
            
            if (target && now - lastTapTime < CONFIG.doubleTapDelay && target === lastTapTarget) {
                e.preventDefault();
                handleDoubleTap(target);
                lastTapTime = 0;
                lastTapTarget = null;
            } else {
                lastTapTime = now;
                lastTapTarget = target;
            }
        });
    }

    function handleDoubleTap(target) {
        if (target.classList.contains('lifx-bulb-control')) {
            const bulbId = target.dataset.bulbId;
            if (bulbId) {
                if (typeof LifXTouchControls !== 'undefined') {
                    LifXTouchControls.selectBulb(bulbId);
                    LifXTouchControls.togglePower();
                }
                showGestureHint('Power Toggle', 'fa-power-off');
            }
        } else if (target.classList.contains('media-item')) {
            const mediaUrl = target.dataset.url;
            if (mediaUrl) {
                openMediaPlayer(mediaUrl, target.dataset.title);
            }
        } else if (target.classList.contains('scene-item')) {
            const sceneName = target.dataset.scene;
            if (sceneName) {
                applyQuickScene(sceneName);
            }
        }
        
        if (CONFIG.hapticFeedback) {
            navigator.vibrate([10, 30, 10]);
        }
    }

    // Long-press gesture detection
    function setupLongPress() {
        if (!CONFIG.enableLongPress) return;
        
        let pressTimer;
        let pressTarget = null;
        let pressStartX = 0;
        let pressStartY = 0;
        const moveThreshold = 10;
        
        document.addEventListener('touchstart', function(e) {
            const target = e.target.closest('.lifx-bulb-control, .media-item, .scene-item');
            if (!target) return;
            
            pressTarget = target;
            pressStartX = e.touches[0].clientX;
            pressStartY = e.touches[0].clientY;
            
            const progressBar = document.getElementById('touch-hold-progress');
            if (progressBar) {
                progressBar.innerHTML = '<div class="touch-hold-progress-bar"></div>';
                progressBar.classList.add('visible');
            }
            
            pressTimer = setTimeout(function() {
                if (pressTarget) {
                    handleLongPress(pressTarget);
                    const bar = progressBar?.querySelector('.touch-hold-progress-bar');
                    if (bar) bar.style.width = '100%';
                    setTimeout(() => {
                        progressBar?.classList.remove('visible');
                        pressTarget = null;
                    }, 200);
                }
            }, CONFIG.longPressDelay);
        });
        
        document.addEventListener('touchmove', function(e) {
            if (!pressTarget) return;
            
            const deltaX = Math.abs(e.touches[0].clientX - pressStartX);
            const deltaY = Math.abs(e.touches[0].clientY - pressStartY);
            
            if (deltaX > moveThreshold || deltaY > moveThreshold) {
                clearTimeout(pressTimer);
                pressTarget = null;
                document.getElementById('touch-hold-progress')?.classList.remove('visible');
            }
        });
        
        document.addEventListener('touchend', function(e) {
            if (pressTimer) {
                clearTimeout(pressTimer);
                pressTimer = null;
            }
            if (pressTarget) {
                document.getElementById('touch-hold-progress')?.classList.remove('visible');
                pressTarget = null;
            }
        });
    }

    function handleLongPress(target) {
        if (target.classList.contains('lifx-bulb-control')) {
            const bulbId = target.dataset.bulbId;
            if (bulbId) {
                if (typeof LifXTouchControls !== 'undefined') {
                    LifXTouchControls.addToMultiSelect(bulbId);
                }
                showGestureHint('Multi-Select', 'fa-users');
            }
        } else if (target.classList.contains('media-item')) {
            showMediaContextMenu(target);
        } else if (target.classList.contains('scene-item')) {
            showSceneContextMenu(target);
        }
        
        if (CONFIG.hapticFeedback) {
            navigator.vibrate([50, 50, 50]);
        }
    }

    function showGestureHint(text, iconClass) {
        let hint = document.querySelector('.gesture-hint-overlay');
        if (!hint) {
            hint = document.createElement('div');
            hint.className = 'gesture-hint-overlay';
            document.body.appendChild(hint);
        }
        
        hint.innerHTML = `
            <i class="fas ${iconClass}"></i>
            <div class="hint-text">${text}</div>
        `;
        
        hint.classList.add('visible');
        setTimeout(() => hint.classList.remove('visible'), 1500);
    }

    function showMediaContextMenu(target) {
        const menu = document.createElement('div');
        menu.className = 'context-menu';
        menu.innerHTML = `
            <button onclick="playMedia('${target.dataset.url}')"><i class="fas fa-play"></i> Play</button>
            <button onclick="addToPlaylist('${target.dataset.url}')"><i class="fas fa-plus"></i> Add to Playlist</button>
            <button onclick="showMediaInfo('${target.dataset.url}')"><i class="fas fa-info"></i> Info</button>
        `;
        menu.style.cssText = `
            position: fixed;
            z-index: 10000;
            background: rgba(30, 30, 45, 0.98);
            border: 1px solid rgba(39, 160, 185, 0.3);
            border-radius: 12px;
            padding: 10px;
            min-width: 200px;
        `;
        
        const rect = target.getBoundingClientRect();
        menu.style.left = Math.min(rect.left, window.innerWidth - 220) + 'px';
        menu.style.top = Math.min(rect.bottom + 10, window.innerHeight - 150) + 'px';
        
        document.body.appendChild(menu);
        setTimeout(() => menu.remove(), 5000);
    }

    function showSceneContextMenu(target) {
        const sceneName = target.dataset.scene;
        const menu = document.createElement('div');
        menu.className = 'context-menu';
        menu.innerHTML = `
            <button onclick="setAsFavorite('${sceneName}')"><i class="fas fa-star"></i> Favorite</button>
            <button onclick="editScene('${sceneName}')"><i class="fas fa-edit"></i> Edit</button>
            <button onclick="scheduleScene('${sceneName}')"><i class="fas fa-clock"></i> Schedule</button>
        `;
        menu.style.cssText = `
            position: fixed;
            z-index: 10000;
            background: rgba(30, 30, 45, 0.98);
            border: 1px solid rgba(39, 160, 185, 0.3);
            border-radius: 12px;
            padding: 10px;
            min-width: 200px;
        `;
        
        const rect = target.getBoundingClientRect();
        menu.style.left = Math.min(rect.left, window.innerWidth - 220) + 'px';
        menu.style.top = Math.min(rect.bottom + 10, window.innerHeight - 150) + 'px';
        
        document.body.appendChild(menu);
        setTimeout(() => menu.remove(), 5000);
    }

    // Mini Player for floating playback
    let miniPlayerVisible = false;
    
    function setupMiniPlayer() {
        const miniPlayer = document.getElementById('mini-player');
        if (!miniPlayer) return;
        
        makeDraggable(miniPlayer);
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
            element.style.transform = 'none';
        }
        
        function closeDragElement() {
            document.onmouseup = null;
            document.onmousemove = null;
        }
        
        function dragTouchStart(e) {
            const touch = e.touches[0];
            pos3 = touch.clientX;
            pos4 = touch.clientY;
            document.ontouchend = closeDragElement;
            document.ontouchmove = dragTouchMove;
        }
        
        function dragTouchMove(e) {
            const touch = e.touches[0];
            pos1 = pos3 - touch.clientX;
            pos2 = pos4 - touch.clientY;
            pos3 = touch.clientX;
            pos4 = touch.clientY;
            element.style.top = (element.offsetTop - pos2) + 'px';
            element.style.left = (element.offsetLeft - pos1) + 'px';
            element.style.transform = 'none';
        }
    }

    function toggleMiniPlayer() {
        const miniPlayer = document.getElementById('mini-player');
        if (miniPlayer) {
            miniPlayerVisible = !miniPlayerVisible;
            miniPlayer.classList.toggle('visible', miniPlayerVisible);
            miniPlayer.style.display = miniPlayerVisible ? 'block' : 'none';
        }
    }

    function updateMiniPlayer(info) {
        const miniPlayer = document.getElementById('mini-player');
        if (!miniPlayer) return;
        
        const title = miniPlayer.querySelector('.mini-player-title');
        const artist = miniPlayer.querySelector('.mini-player-artist');
        const playIcon = miniPlayer.querySelector('#mini-play-icon');
        
        if (title) title.textContent = info.trackName || 'No Track';
        if (artist) artist.textContent = info.artistName || 'Unknown Artist';
        if (playIcon) playIcon.className = info.isPlaying ? 'fas fa-pause' : 'fas fa-play';
        
        if (!miniPlayerVisible && info.isPlaying) {
            toggleMiniPlayer();
        }
    }

    // Now Playing Toast notification
    let nowPlayingTimeout;
    
    function setupNowPlayingToast() {
        const toast = document.getElementById('now-playing-toast');
        if (!toast) return;
        
        const closeBtn = toast.querySelector('.now-playing-close');
        if (closeBtn) {
            closeBtn.addEventListener('click', hideNowPlayingToast);
        }
    }

    function showNowPlayingToast(info) {
        const toast = document.getElementById('now-playing-toast');
        if (!toast) return;
        
        const art = toast.querySelector('.now-playing-art');
        const title = toast.querySelector('.now-playing-title');
        const artist = toast.querySelector('.now-playing-artist');
        
        if (art) {
            art.style.backgroundImage = info.artwork ? `url(${info.artwork})` : '';
            art.style.background = info.artwork ? '' : 'linear-gradient(135deg, #27a0b9, #1f8999)';
        }
        if (title) title.textContent = info.trackName || 'No Track';
        if (artist) artist.textContent = info.artistName || 'Unknown Artist';
        
        toast.classList.add('visible');
        
        clearTimeout(nowPlayingTimeout);
        nowPlayingTimeout = setTimeout(hideNowPlayingToast, 5000);
    }

    function hideNowPlayingToast() {
        const toast = document.getElementById('now-playing-toast');
        if (toast) {
            toast.classList.remove('visible');
        }
    }

    function hideNowPlaying() {
        hideNowPlayingToast();
    }

    // Swipe gesture detection
    function setupSwipeGestures() {
        let touchStartX = 0;
        let touchStartY = 0;
        let swipeThreshold = 50;
        
        document.addEventListener('touchstart', function(e) {
            touchStartX = e.changedTouches[0].screenX;
            touchStartY = e.changedTouches[0].screenY;
        });
        
        document.addEventListener('touchend', function(e) {
            const touchEndX = e.changedTouches[0].screenX;
            const touchEndY = e.changedTouches[0].screenY;
            
            const deltaX = touchEndX - touchStartX;
            const deltaY = touchEndY - touchStartY;
            
            if (Math.abs(deltaX) > swipeThreshold || Math.abs(deltaY) > swipeThreshold) {
                let direction;
                if (Math.abs(deltaX) > Math.abs(deltaY)) {
                    direction = deltaX > 0 ? 'right' : 'left';
                } else {
                    direction = deltaY > 0 ? 'down' : 'up';
                }
                
                handleSwipe(direction);
            }
        });
    }

    function handleSwipe(direction) {
        console.log(`[TouchMediaEnhancements] Swipe detected: ${direction}`);
        
        const event = new CustomEvent('swipe-gesture', { detail: { direction } });
        document.dispatchEvent(event);
        
        // Show swipe indicator
        const indicator = document.createElement('div');
        indicator.className = 'swipe-indicator visible';
        
        const arrows = {
            'up': '↑',
            'down': '↓',
            'left': '←',
            'right': '→'
        };
        
        indicator.innerHTML = `<div class="swipe-direction-arrow">${arrows[direction]}</div>`;
        document.body.appendChild(indicator);
        
        setTimeout(() => {
            indicator.classList.remove('visible');
            setTimeout(() => indicator.remove(), 200);
        }, 500);
    }

    // Media Visualizer
    function setupMediaVisualizer() {
        const container = document.getElementById('media-visualization-container');
        if (!container) return;
        
        const numBars = 32;
        mediaVisualizerBars = [];
        
        for (let i = 0; i < numBars; i++) {
            const bar = document.createElement('div');
            bar.className = 'media-viz-bar';
            bar.style.background = `hsl(${(i / numBars) * 360}, 80%, 50%)`;
            container.appendChild(bar);
            mediaVisualizerBars.push(bar);
        }
    }

    function updateMediaVisualization(data) {
        if (!mediaVisualizerBars.length) return;
        
        const values = data || Array(mediaVisualizerBars.length).fill(0).map(() => Math.random());
        
        mediaVisualizerBars.forEach((bar, i) => {
            const value = values[i] || values[values.length - 1] || 0;
            const height = Math.max(5, value * 140);
            bar.style.height = height + 'px';
            
            if (value > 0.9) {
                bar.classList.add('peak');
            } else {
                bar.classList.remove('peak');
            }
        });
    }

    // Party Mode Visualizer
    function activatePartyMode(active) {
        let visualizer = document.querySelector('.party-mode-visualizer');
        
        if (active) {
            if (!visualizer) {
                visualizer = document.createElement('div');
                visualizer.className = 'party-mode-visualizer';
                
                for (let i = 0; i < 50; i++) {
                    const dot = document.createElement('div');
                    dot.className = 'party-dot';
                    dot.style.left = Math.random() * 100 + '%';
                    dot.style.top = Math.random() * 100 + '%';
                    dot.style.animationDelay = Math.random() * 2 + 's';
                    visualizer.appendChild(dot);
                }
                
                document.body.appendChild(visualizer);
            }
            
            visualizer.classList.add('active');
        } else {
            if (visualizer) {
                visualizer.classList.remove('active');
                setTimeout(() => visualizer.remove(), 300);
            }
        }
    }

    // Quick scene presets
    const scenePresets = {
        'focus': { color: '#00d4ff', brightness: 80, temperature: 4000 },
        'relax': { color: '#ff9966', brightness: 60, temperature: 2700 },
        'party': { color: '#ff0080', brightness: 100, effect: 'party' },
        'bedtime': { color: '#4a0080', brightness: 30, temperature: 2000 },
        'reading': { color: '#ffffff', brightness: 90, temperature: 5000 },
        'movie': { color: '#8800ff', brightness: 40, temperature: 3000 }
    };

    function applyScenePreset(presetName) {
        const preset = scenePresets[presetName];
        if (!preset) return;
        
        console.log(`[TouchMediaEnhancements] Applying scene: ${presetName}`);
        
        const event = new CustomEvent('apply-lifx-scene', { detail: { name: presetName, ...preset } });
        document.dispatchEvent(event);
        
        showSceneChange(presetName);
        
        if (presetName === 'bedtime') {
            setBedtimeMode(true);
        } else if (presetName === 'party') {
            activatePartyMode(true);
        } else {
            setBedtimeMode(false);
            activatePartyMode(false);
        }
    }

    // Color temperature picker
    function setupColorTemperaturePicker() {
        const picker = document.querySelector('.color-temp-gradient');
        if (!picker) return;
        
        let isDragging = false;
        
        picker.addEventListener('mousedown', handleDrag);
        picker.addEventListener('mousemove', handleDrag);
        picker.addEventListener('mouseup', () => isDragging = false);
        picker.addEventListener('touchstart', handleTouch);
        picker.addEventListener('touchmove', handleTouch);
        picker.addEventListener('touchend', () => isDragging = false);
        
        function handleDrag(e) {
            if (e.type === 'mousedown') isDragging = true;
            if (!isDragging) return;
            
            const rect = picker.getBoundingClientRect();
            const x = (e.clientX - rect.left) / rect.width;
            updateTemperature(x);
        }
        
        function handleTouch(e) {
            if (e.type === 'touchstart') isDragging = true;
            if (!isDragging) return;
            
            const rect = picker.getBoundingClientRect();
            const x = (e.touches[0].clientX - rect.left) / rect.width;
            updateTemperature(x);
        }
        
        function updateTemperature(x) {
            x = Math.max(0, Math.min(1, x));
            const kelvin = 2000 + (x * 7000);
            
            let indicator = picker.querySelector('.color-temp-indicator');
            if (!indicator) {
                indicator = document.createElement('div');
                indicator.className = 'color-temp-indicator';
                picker.appendChild(indicator);
            }
            
            indicator.style.left = (x * 100) + '%';
            
            const event = new CustomEvent('color-temperature-change', { detail: { kelvin, x } });
            document.dispatchEvent(event);
        }
    }

    // Beat detection calibration UI
    function updateBeatDetectionStats(stats) {
        const container = document.querySelector('.beat-detection-calibration');
        if (!container || !stats) return;
        
        let statsEl = container.querySelector('.calibration-stats');
        if (!statsEl) {
            statsEl = document.createElement('div');
            statsEl.className = 'calibration-stats';
            container.appendChild(statsEl);
        }
        
        statsEl.innerHTML = `
            <div class="stat-item">
                <span class="stat-label">Detected BPM</span>
                <span class="stat-value">${stats.bpm || '--'}</span>
            </div>
            <div class="stat-item">
                <span class="stat-label">Confidence</span>
                <span class="stat-value">${stats.confidence ? (stats.confidence * 100).toFixed(0) + '%' : '--'}</span>
            </div>
            <div class="stat-item">
                <span class="stat-label">Last Beat</span>
                <span class="stat-value">${stats.lastBeat ? stats.lastBeat.toFixed(3) + 's' : '--'}</span>
            </div>
            <div class="stat-item">
                <span class="stat-label">Energy</span>
                <span class="stat-value">${stats.energy ? (stats.energy * 100).toFixed(0) + '%' : '--'}</span>
            </div>
        `;
    }

    // Media sync dashboard
    function updateMediaSyncDashboard(status) {
        const container = document.querySelector('.media-sync-dashboard');
        if (!container) return;
        
        let grid = container.querySelector('.sync-status-grid');
        if (!grid) {
            grid = document.createElement('div');
            grid.className = 'sync-status-grid';
            container.appendChild(grid);
        }
        
        grid.innerHTML = `
            <div class="sync-status-card ${status.audioActive ? 'active' : ''}">
                <div class="status-icon"><i class="fas fa-music"></i></div>
                <div class="status-label">Audio Analysis</div>
                <div class="status-value">${status.audioActive ? 'Active' : 'Inactive'}</div>
                <button class="btn-toggle" onclick="TouchMediaEnhancements.toggleAudioAnalysis()">
                    <i class="fas ${status.audioActive ? 'fa-toggle-on' : 'fa-toggle-off'}"></i>
                </button>
            </div>
            <div class="sync-status-card ${status.lightsActive ? 'active' : ''}">
                <div class="status-icon"><i class="fas fa-lightbulb"></i></div>
                <div class="status-label">Light Sync</div>
                <div class="status-value">${status.lightsActive ? 'Active' : 'Inactive'}</div>
                <button class="btn-toggle" onclick="TouchMediaEnhancements.toggleLightSync()">
                    <i class="fas ${status.lightsActive ? 'fa-toggle-on' : 'fa-toggle-off'}"></i>
                </button>
            </div>
        `;
    }

    function toggleAudioAnalysis() {
        document.dispatchEvent(new CustomEvent('toggle-audio-analysis'));
    }

    function toggleLightSync() {
        document.dispatchEvent(new CustomEvent('toggle-light-sync'));
    }

    // Touch sensitivity settings
    function setTouchSensitivity(level) {
        CONFIG.touchSensitivity = level;
        
        const sensitivityMap = {
            'low': 100,
            'medium': 50,
            'high': 20
        };
        
        CONFIG.gestureTrailCount = sensitivityMap[level] || 50;
        
        const event = new CustomEvent('touch-sensitivity-change', { detail: { level } });
        document.dispatchEvent(event);
    }

    function toggleMediaSyncPanel() {
        const panel = document.getElementById('media-sync-panel');
        if (panel) {
            panel.classList.toggle('visible');
            mediaSyncActive = !mediaSyncActive;
            document.getElementById('sync-stat').textContent = mediaSyncActive ? 'On' : 'Off';
        }
    }
    
    function setMediaSyncMode(mode) {
        mediaSyncMode = mode;
        document.querySelectorAll('.media-sync-btn').forEach(btn => {
            btn.classList.remove('active');
            btn.classList.add('inactive');
        });
        const activeBtn = document.getElementById(`${mode}-sync-btn`);
        if (activeBtn) {
            activeBtn.classList.remove('inactive');
            activeBtn.classList.add('active');
        }
        document.getElementById('sync-stat').textContent = mode.charAt(0).toUpperCase() + mode.slice(1);
        
        if (mode === 'beat') {
            startBeatDetection();
        } else if (mode === 'ambient') {
            startAmbientAnalysis();
        } else if (mode === 'spectrum') {
            startSpectrumAnalysis();
        }
    }
    
    function toggleQuickScenesPanel() {
        const panel = document.getElementById('quick-scenes-panel');
        if (panel) {
            panel.classList.toggle('visible');
            if (panel.classList.contains('visible')) {
                renderQuickScenes();
            }
        }
    }
    
    function renderQuickScenes() {
        const grid = document.getElementById('quick-scenes-grid');
        if (!grid) return;
        
        const scenes = [
            { name: 'relax', icon: '🧘', color: '#ff6b6b' },
            { name: 'focus', icon: '🎯', color: '#4ecdc4' },
            { name: 'energize', icon: '⚡', color: '#ffe66d' },
            { name: 'night', icon: '🌙', color: '#1a1a2e' },
            { name: 'sunset', icon: '🌅', color: '#ff9f43' },
            { name: 'ocean', icon: '🌊', color: '#45b7d1' },
            { name: 'reading', icon: '📖', color: '#feca57' },
            { name: 'romance', icon: '💕', color: '#ff9ff3' },
            { name: 'party', icon: '🎉', color: '#00d4ff' },
            { name: 'golden', icon: '☀️', color: '#f9ca24' },
            { name: 'arctic', icon: '❄️', color: '#70a1ff' },
            { name: 'tropical', icon: '🏖️', color: '#00b894' }
        ];
        
        grid.innerHTML = scenes.map(scene => `
            <div class="scene-item" onclick="applyQuickScene('${scene.name}')">
                <div class="scene-icon">${scene.icon}</div>
                <div class="scene-name">${scene.name}</div>
            </div>
        `).join('');
    }
    
    function applyQuickScene(sceneName) {
        if (typeof LifXTouchControls !== 'undefined') {
            LifXTouchControls.applyScene(sceneName);
        }
        toggleQuickScenesPanel();
    }
    
    function mediaPlayPause() {
        const icon = document.getElementById('media-play-icon');
        if (mediaPlaybackState.isPlaying) {
            mediaPlaybackState.isPlaying = false;
            if (icon) icon.className = 'fas fa-play';
            sendMediaCommand('pause');
        } else {
            mediaPlaybackState.isPlaying = true;
            if (icon) icon.className = 'fas fa-pause';
            sendMediaCommand('play');
        }
    }
    
    function mediaPrevious() {
        sendMediaCommand('previous');
    }
    
    function mediaNext() {
        sendMediaCommand('next');
    }
    
    function sendMediaCommand(command) {
        if (typeof websocket !== 'undefined' && websocket && websocket.readyState === WebSocket.OPEN) {
            websocket.send(JSON.stringify({
                type: 'command',
                id: `media_${command}_${Date.now()}`,
                command: `media_${command}`,
                args: {}
            }));
        }
    }
    
    function updateMediaPlayback(info) {
        mediaPlaybackState = { ...mediaPlaybackState, ...info };
        
        const trackName = document.getElementById('media-track-name');
        const artistName = document.getElementById('media-artist-name');
        const progress = document.getElementById('media-progress');
        const playIcon = document.getElementById('media-play-icon');
        
        if (trackName) trackName.textContent = info.trackName || 'No Track';
        if (artistName) artistName.textContent = info.artistName || 'Unknown Artist';
        if (progress) progress.style.width = `${(info.progress / info.duration) * 100 || 0}%`;
        if (playIcon) playIcon.className = info.isPlaying ? 'fas fa-pause' : 'fas fa-play';
    }
    
    function startBeatDetection() {
        initAudioAnalyzer();
        const vizContainer = document.getElementById('beat-visualization');
        if (!vizContainer) return;
        
        if (vizContainer.children.length === 0) {
            for (let i = 0; i < 16; i++) {
                const bar = document.createElement('div');
                bar.className = 'beat-bar';
                bar.style.height = '5px';
                vizContainer.appendChild(bar);
                mediaVisualizerBars.push(bar);
            }
        }
        
        requestAnimationFrame(updateBeatVisualization);
    }
    
    function startAmbientAnalysis() {
        initAudioAnalyzer();
    }
    
    function startSpectrumAnalysis() {
        initAudioAnalyzer();
    }
    
    let beatHistory = [];
    let lastBeatTime = 0;
    let beatThreshold = 0.8;
    let bpmHistory = [];
    let beatEnergyHistory = [];
    let maxBeatEnergyHistory = 10;
    let consecutiveBeatCount = 0;
    let beatConfidence = 0;
    let lastBeatEnergy = 0;
    
    function initAudioAnalyzer() {
        if (!audioContext) {
            audioContext = new (window.AudioContext || window.webkitAudioContext)();
            analyser = audioContext.createAnalyser();
            analyser.fftSize = 512;
            analyser.smoothingTimeConstant = 0.8;
            beatHistory = [];
            bpmHistory = [];
        }
    }
    
    function detectBeat(dataArray) {
        const bassRange = dataArray.slice(0, 8);
        const bassAvg = bassRange.reduce((a, b) => a + b, 0) / bassRange.length;
        const bassPeak = Math.max(...bassRange);
        const currentBeatEnergy = bassAvg / 255;
        
        beatEnergyHistory.push(currentBeatEnergy);
        if (beatEnergyHistory.length > maxBeatEnergyHistory) {
            beatEnergyHistory.shift();
        }
        
        const avgEnergy = beatEnergyHistory.reduce((a, b) => a + b, 0) / beatEnergyHistory.length;
        const energyChange = currentBeatEnergy - lastBeatEnergy;
        
        const now = Date.now();
        const timeSinceLastBeat = now - lastBeatTime;
        
        const dynamicThreshold = Math.max(0.5, Math.min(0.95, 
            beatThreshold - (avgEnergy * 0.15) - (Math.max(0, energyChange) * 0.1)
        ));
        
        if (bassPeak > dynamicThreshold * 255 && timeSinceLastBeat > 150) {
            lastBeatTime = now;
            beatHistory.push(now);
            consecutiveBeatCount++;
            lastBeatEnergy = currentBeatEnergy;
            
            beatConfidence = Math.min(1.0, beatConfidence + 0.1);
            
            if (beatHistory.length > 2) {
                const intervals = [];
                for (let i = 1; i < beatHistory.length; i++) {
                    intervals.push(beatHistory[i] - beatHistory[i - 1]);
                }
                
                const sortedIntervals = [...intervals].sort((a, b) => a - b);
                const trimmedIntervals = sortedIntervals.slice(1, -1);
                const avgInterval = trimmedIntervals.length > 0 
                    ? trimmedIntervals.reduce((a, b) => a + b, 0) / trimmedIntervals.length
                    : intervals.reduce((a, b) => a + b, 0) / intervals.length;
                
                const detectedBpm = Math.round(60000 / avgInterval);
                
                if (detectedBpm > 60 && detectedBpm < 200) {
                    bpmHistory.push(detectedBpm);
                    if (bpmHistory.length > 12) bpmHistory.shift();
                    
                    const weights = bpmHistory.map((_, i) => i + 1);
                    const weightedSum = bpmHistory.reduce((sum, bpm, i) => sum + bpm * weights[i], 0);
                    const weightTotal = weights.reduce((a, b) => a + b, 0);
                    const smoothedBpm = Math.round(weightedSum / weightTotal);
                    updateBpm(smoothedBpm);
                    updateBeatDetectionStats({
                        bpm: smoothedBpm,
                        confidence: beatConfidence,
                        lastBeat: now / 1000,
                        energy: currentBeatEnergy,
                        consecutiveBeats: consecutiveBeatCount
                    });
                    
                    if (mediaSyncActive && mediaSyncMode === 'beat') {
                        triggerLifxBeat();
                        triggerBeatVisualization(currentBeatEnergy);
                    }
                    
                    return true;
                }
            }
        } else {
            beatConfidence = Math.max(0, beatConfidence - 0.02);
            if (timeSinceLastBeat > 3000) {
                consecutiveBeatCount = 0;
            }
        }
        
        if (beatHistory.length > 0 && now - beatHistory[beatHistory.length - 1] > 5000) {
            beatHistory.shift();
            beatConfidence = Math.max(0, beatConfidence - 0.1);
        }
        
        return false;
    }
    
    function triggerLifxBeat() {
        const event = new CustomEvent('lifx-beat-trigger', {
            detail: { timestamp: Date.now() }
        });
        document.dispatchEvent(event);
    }
    
    function triggerBeatVisualization(energy) {
        const visualizerContainer = document.getElementById('beat-visualization');
        if (!visualizerContainer) return;
        
        const beatPulse = document.createElement('div');
        beatPulse.className = 'beat-pulse-visual';
        beatPulse.style.cssText = `
            position: absolute;
            top: 50%;
            left: 50%;
            transform: translate(-50%, -50%);
            width: ${100 + (energy * 150)}px;
            height: ${100 + (energy * 150)}px;
            border-radius: 50%;
            background: radial-gradient(circle, rgba(0, 212, 255, ${0.3 + energy * 0.4}) 0%, transparent 70%);
            pointer-events: none;
            z-index: 100;
            animation: beat-pulse-expand 0.6s ease-out forwards;
        `;
        
        visualizerContainer.appendChild(beatPulse);
        setTimeout(() => beatPulse.remove(), 600);
        
        const bars = visualizerContainer.querySelectorAll('.beat-bar');
        bars.forEach((bar, i) => {
            const delay = i * 0.02;
            setTimeout(() => {
                bar.style.background = `hsla(${(i / bars.length) * 360}, 100%, ${50 + energy * 30}%, ${0.7 + energy * 0.3})`;
                bar.style.boxShadow = `0 0 ${Math.floor(energy * 20)}px hsla(${(i / bars.length) * 360}, 100%, 50%, ${0.5 + energy * 0.3})`;
                setTimeout(() => {
                    bar.style.background = '';
                    bar.style.boxShadow = '';
                }, 200);
            }, delay);
        });
    }
    
    function updateBeatVisualization() {
        if (!analyser) return;
        
        const dataArray = new Uint8Array(analyser.frequencyBinCount);
        analyser.getByteFrequencyData(dataArray);
        
        const bpmStat = document.getElementById('bpm-stat');
        const intensityStat = document.getElementById('intensity-stat');
        const syncStat = document.getElementById('sync-stat');
        
        let sum = 0;
        let maxFreq = 0;
        let subBass = 0, bass = 0, lowMid = 0, mid = 0, highMid = 0, treble = 0;
        
        const bands = {
            subBass: { start: 0, end: 4, value: 0, count: 0 },
            bass: { start: 4, end: 10, value: 0, count: 0 },
            lowMid: { start: 10, end: 18, value: 0, count: 0 },
            mid: { start: 18, end: 28, value: 0, count: 0 },
            highMid: { start: 28, end: 40, value: 0, count: 0 },
            treble: { start: 40, end: 64, value: 0, count: 0 }
        };
        
        for (let i = 0; i < dataArray.length; i++) {
            sum += dataArray[i];
            if (dataArray[i] > maxFreq) maxFreq = dataArray[i];
            
            for (const [bandName, band] of Object.entries(bands)) {
                if (i >= band.start && i < band.end) {
                    band.value += dataArray[i];
                    band.count++;
                }
            }
            
            if (mediaVisualizerBars[i]) {
                const normalizedValue = dataArray[i] / 255;
                const height = Math.max(5, normalizedValue * 120);
                const hue = (i / dataArray.length) * 360;
                
                mediaVisualizerBars[i].style.height = `${height}px`;
                mediaVisualizerBars[i].style.background = `hsla(${hue}, 80%, ${40 + normalizedValue * 30}%, ${0.7 + normalizedValue * 0.3})`;
                mediaVisualizerBars[i].style.boxShadow = `0 0 ${Math.floor(normalizedValue * 20)}px hsla(${hue}, 80%, 50%, ${0.3 + normalizedValue * 0.4})`;
                
                if (dataArray[i] > 220) {
                    mediaVisualizerBars[i].classList.add('peak');
                    mediaVisualizerBars[i].style.transform = 'scale(1.05)';
                } else {
                    mediaVisualizerBars[i].classList.remove('peak');
                    mediaVisualizerBars[i].style.transform = 'scale(1)';
                }
            }
        }
        
        for (const [bandName, band] of Object.entries(bands)) {
            const avg = band.count > 0 ? band.value / band.count : 0;
            const bandEl = document.getElementById(`band-${bandName}`);
            if (bandEl) {
                const targetHeight = Math.max(10, (avg / 255) * 100);
                const currentHeight = parseFloat(bandEl.style.height) || targetHeight;
                const smoothedHeight = currentHeight + (targetHeight - currentHeight) * 0.3;
                bandEl.style.height = `${smoothedHeight}%`;
                bandEl.style.background = getBandColor(bandName, avg / 255);
                bandEl.style.boxShadow = `0 0 ${Math.floor((avg / 255) * 15)}px ${getBandColor(bandName, avg / 255)}`;
                
                if (avg > 200) {
                    bandEl.style.filter = 'brightness(1.3)';
                    bandEl.style.transform = 'scaleX(1.1)';
                } else {
                    bandEl.style.filter = 'brightness(1)';
                    bandEl.style.transform = 'scaleX(1)';
                }
            }
        }
        
        const avgIntensity = sum / dataArray.length;
        const bassAvg = bands.bass.value / bands.bass.count;
        const subBassAvg = bands.subBass.value / bands.subBass.count;
        
        detectBeat(dataArray);
        
        if (intensityStat) intensityStat.textContent = `${Math.floor((avgIntensity / 255) * 100)}%`;
        if (syncStat) syncStat.textContent = mediaSyncActive ? mediaSyncMode.charAt(0).toUpperCase() + mediaSyncMode.slice(1) : 'Off';
        
        if (mediaSyncActive && mediaSyncMode === 'beat') {
            updateLifxFromAudio(dataArray, maxFreq, bassAvg, subBassAvg);
        }
        
        requestAnimationFrame(updateBeatVisualization);
    }
    
    function getBandColor(bandName, intensity) {
        const colors = {
            subBass: `hsla(0, 80%, ${50 + intensity * 20}%, ${0.6 + intensity * 0.4})`,
            bass: `hsla(30, 80%, ${50 + intensity * 20}%, ${0.6 + intensity * 0.4})`,
            lowMid: `hsla(60, 80%, ${50 + intensity * 20}%, ${0.6 + intensity * 0.4})`,
            mid: `hsla(120, 80%, ${50 + intensity * 20}%, ${0.6 + intensity * 0.4})`,
            highMid: `hsla(180, 80%, ${50 + intensity * 20}%, ${0.6 + intensity * 0.4})`,
            treble: `hsla(240, 80%, ${50 + intensity * 20}%, ${0.6 + intensity * 0.4})`
        };
        return colors[bandName] || colors.mid;
    }
    
    let lastLifxUpdate = 0;
    let lifxColorHistory = { hue: 0, saturation: 50, brightness: 30 };
    
    function updateLifxFromAudio(dataArray, peak, bassAvg, subBassAvg) {
        const now = Date.now();
        if (now - lastLifxUpdate < 50) return;
        lastLifxUpdate = now;
        
        const normalizedPeak = peak / 255;
        const normalizedBass = bassAvg / 255;
        const normalizedSubBass = subBassAvg / 255;
        
        const bassImpact = normalizedSubBass > 0.7 ? 1.3 : 1.0;
        const beatBoost = normalizedSubBass > 0.75 ? 1.2 : 1.0;
        
        const targetHue = ((Date.now() / 30) + (normalizedBass * 60)) % 360;
        const targetSaturation = Math.min(100, 50 + normalizedPeak * 50);
        const targetBrightness = Math.min(100, 30 + normalizedBass * 70);
        
        lifxColorHistory.hue = lifxColorHistory.hue + (targetHue - lifxColorHistory.hue) * 0.2;
        lifxColorHistory.saturation = lifxColorHistory.saturation + (targetSaturation - lifxColorHistory.saturation) * 0.3;
        lifxColorHistory.brightness = lifxColorHistory.brightness + (targetBrightness - lifxColorHistory.brightness) * 0.3;
        
        const smoothedHue = lifxColorHistory.hue;
        const smoothedSaturation = lifxColorHistory.saturation;
        const smoothedBrightness = Math.min(100, lifxColorHistory.brightness * bassImpact * beatBoost);
        const temperature = 2700 + (normalizedPeak * 2300);
        
        if (typeof LifXTouchControls !== 'undefined') {
            const event = new CustomEvent('lifx-media-sync', {
                detail: {
                    hue: smoothedHue,
                    saturation: smoothedSaturation,
                    brightness: smoothedBrightness,
                    temperature: temperature,
                    intensity: normalizedPeak,
                    bassEnergy: normalizedBass,
                    subBassEnergy: normalizedSubBass,
                    isBeat: normalizedSubBass > 0.75,
                    beatBoost: beatBoost > 1.0
                }
            });
            document.dispatchEvent(event);
        }
    }
    
    // Public API
    return {
        init,
        updateBpm,
        showSceneChange,
        setBedtimeMode,
        addToSelection,
        removeFromSelection,
        clearSelection,
        applyToSelected,
        activatePartyMode,
        applyScenePreset,
        setupColorTemperaturePicker,
        updateMediaVisualization,
        updateBeatDetectionStats,
        updateMediaSyncDashboard,
        toggleAudioAnalysis,
        toggleLightSync,
        setTouchSensitivity,
        toggleMediaSyncPanel,
        setMediaSyncMode,
        toggleQuickScenesPanel,
        applyQuickScene,
        mediaPlayPause,
        mediaPrevious,
        mediaNext,
        updateMediaPlayback,
        updateMiniPlayer,
        showNowPlayingToast,
        hideNowPlaying,
        toggleMiniPlayer,
        makeDraggable,
        CONFIG
    };
})();

// Initialize on DOM ready
document.addEventListener('DOMContentLoaded', function() {
    TouchMediaEnhancements.init();
    TouchMediaEnhancements.setupColorTemperaturePicker();
});

// Global functions for HTML onclick handlers
function toggleMediaSyncPanel() {
    TouchMediaEnhancements.toggleMediaSyncPanel();
}

function setMediaSyncMode(mode) {
    TouchMediaEnhancements.setMediaSyncMode(mode);
}

function toggleQuickScenesPanel() {
    TouchMediaEnhancements.toggleQuickScenesPanel();
}

function applyQuickScene(sceneName) {
    TouchMediaEnhancements.applyQuickScene(sceneName);
}

function mediaPlayPause() {
    TouchMediaEnhancements.mediaPlayPause();
}

function mediaPrevious() {
    TouchMediaEnhancements.mediaPrevious();
}

function mediaNext() {
    TouchMediaEnhancements.mediaNext();
}
