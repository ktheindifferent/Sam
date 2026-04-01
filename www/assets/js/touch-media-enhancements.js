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
    
    // Configuration
    const CONFIG = {
        gestureTrailCount: 5,
        bpmUpdateInterval: 100,
        sceneIndicatorDuration: 3000,
        touchSensitivity: 'medium',
        enableRipple: true,
        enableGestureTrails: true,
        enableBpmDisplay: true
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
        CONFIG
    };
})();

// Initialize on DOM ready
document.addEventListener('DOMContentLoaded', function() {
    TouchMediaEnhancements.init();
    TouchMediaEnhancements.setupColorTemperaturePicker();
});
