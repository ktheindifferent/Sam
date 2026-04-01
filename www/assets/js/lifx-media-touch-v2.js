/**
 * SAM LIFX Media Touch Controls V2
 * Enhanced touch interface with media center integration and advanced LIFX controls
 */

(function() {
    'use strict';

    const LIFXMediaTouchV2 = {
        config: {
            touchSensitivity: 'medium',
            enableHapticFeedback: true,
            enableGestureTrails: true,
            enableMediaSync: true,
            beatDetectionThreshold: 0.75,
            colorTransitionDuration: 1000,
            gestureHoldDuration: 500
        },

        state: {
            selectedBulbs: new Set(),
            activeScene: null,
            activeEffect: null,
            mediaSyncActive: false,
            mediaSyncMode: 'beat',
            bpmDetected: 0,
            brightnessLevel: 50,
            colorTemperature: 4000,
            partyModeActive: false,
            bedtimeModeActive: false,
            circadianActive: false,
            lastTouchTime: 0,
            gestureHistory: []
        },

        scenePresets: [
            { id: 'relax', name: 'Relax', icon: '🧘', hue: 5800, saturation: 15000, brightness: 26214, kelvin: 2700 },
            { id: 'focus', name: 'Focus', icon: '🎯', hue: 19000, saturation: 8000, brightness: 52428, kelvin: 5000 },
            { id: 'energize', name: 'Energize', icon: '⚡', hue: 41000, saturation: 20000, brightness: 65535, kelvin: 6500 },
            { id: 'night', name: 'Night', icon: '🌙', hue: 5800, saturation: 10000, brightness: 13107, kelvin: 2000 },
            { id: 'reading', name: 'Reading', icon: '📚', hue: 19000, saturation: 5000, brightness: 45875, kelvin: 4500 },
            { id: 'romance', name: 'Romance', icon: '💕', hue: 60000, saturation: 25000, brightness: 32767, kelvin: 3000 },
            { id: 'party', name: 'Party', icon: '🎉', hue: 43680, saturation: 65535, brightness: 65535, kelvin: 5500 },
            { id: 'sunset', name: 'Sunset', icon: '🌅', hue: 7098, saturation: 40000, brightness: 39321, kelvin: 2500 },
            { id: 'arctic', name: 'Arctic', icon: '❄️', hue: 32760, saturation: 15000, brightness: 52428, kelvin: 7000 },
            { id: 'golden', name: 'Golden', icon: '☀️', hue: 8000, saturation: 30000, brightness: 45875, kelvin: 3200 },
            { id: 'ocean', name: 'Ocean', icon: '🌊', hue: 34580, saturation: 42598, brightness: 49151, kelvin: 4000 },
            { id: 'tropical', name: 'Tropical', icon: '🏖️', hue: 27300, saturation: 65535, brightness: 47185, kelvin: 3800 },
            { id: 'meditation', name: 'Meditation', icon: '🧘', hue: 50960, saturation: 19660, brightness: 22937, kelvin: 2400 },
            { id: 'gaming', name: 'Gaming', icon: '🎮', hue: 50960, saturation: 52428, brightness: 58982, kelvin: 5500 },
            { id: 'movie', name: 'Movie', icon: '🎬', hue: 3640, saturation: 19660, brightness: 22937, kelvin: 2200 },
            { id: 'morning', name: 'Morning', icon: '🌄', hue: 9100, saturation: 32767, brightness: 55705, kelvin: 5500 },
            { id: 'goodnight', name: 'Goodnight', icon: '😴', hue: 43680, saturation: 6553, brightness: 6553, kelvin: 2000 },
            { id: 'rainbow', name: 'Rainbow', icon: '🌈', hue: 0, saturation: 65535, brightness: 52428, kelvin: 4000 },
            { id: 'fireplace', name: 'Fireplace', icon: '🔥', hue: 5460, saturation: 52428, brightness: 39321, kelvin: 2000 },
            { id: 'ice', name: 'Ice', icon: '🧊', hue: 36400, saturation: 32767, brightness: 45875, kelvin: 8000 },
            { id: 'aurora', name: 'Aurora', icon: '🌌', hue: 32760, saturation: 45875, brightness: 49151, kelvin: 6000 },
            { id: 'nebula', name: 'Nebula', icon: '🌠', hue: 50960, saturation: 52428, brightness: 45875, kelvin: 4500 },
            { id: 'thunder', name: 'Thunder', icon: '⛈️', hue: 5460, saturation: 39321, brightness: 58982, kelvin: 5000 },
            { id: 'crystal', name: 'Crystal', icon: '💎', hue: 34580, saturation: 26214, brightness: 52428, kelvin: 7500 }
        ],

        effectPresets: [
            { id: 'pulse', name: 'Pulse', icon: '💓', duration: 5, cycles: 3 },
            { id: 'rainbow', name: 'Rainbow Cycle', icon: '🌈', duration: 10, cycles: 2 },
            { id: 'strobe', name: 'Strobe', icon: '⚡', duration: 3, cycles: 10 },
            { id: 'fireplace', name: 'Fireplace', icon: '🔥', duration: 30, cycles: 1 },
            { id: 'aurora', name: 'Aurora', icon: '🌌', duration: 15, cycles: 1 },
            { id: 'breath', name: 'Breath', icon: '🌬️', duration: 8, cycles: 4 },
            { id: 'color_cycle', name: 'Color Cycle', icon: '🎨', duration: 20, cycles: 1 }
        ],

        mediaPresets: [
            { id: 'spotify', name: 'Spotify', icon: '🎵', service: 'spotify' },
            { id: 'youtube', name: 'YouTube', icon: '📺', service: 'youtube' },
            { id: 'plex', name: 'Plex', icon: '🎬', service: 'plex' },
            { id: 'radio', name: 'Radio', icon: '📻', service: 'radio' },
            { id: 'tidal', name: 'Tidal', icon: '🌊', service: 'tidal' },
            { id: 'apple_music', name: 'Apple Music', icon: '🎶', service: 'apple_music' }
        ],

        init() {
            this.setupTouchGestures();
            this.setupMediaPlayers();
            this.setupLightGroups();
            this.setupVolumeSliders();
            this.setupBrightnessSliders();
            this.setupSceneSelector();
            this.setupQuickActions();
            this.setupMediaPresets();
            this.setupColorPicker();
            this.setupEffectSelector();
            this.setupZoneControl();
            this.setupGestureHints();
            this.syncStatus();
            this.startPeriodicSync();
            console.log('[LIFXMediaTouchV2] Initialized');
        },

        setupTouchGestures() {
            const touchableElements = document.querySelectorAll('.lifx-bulb-control, .media-control-btn, .scene-btn');
            
            touchableElements.forEach(el => {
                el.addEventListener('touchstart', this.handleTouchStart.bind(this), { passive: true });
                el.addEventListener('touchmove', this.handleTouchMove.bind(this), { passive: true });
                el.addEventListener('touchend', this.handleTouchEnd.bind(this), { passive: true });
                el.addEventListener('click', this.handleTouchClick.bind(this));
            });

            document.addEventListener('gesturestart', this.handleGestureStart.bind(this));
            document.addEventListener('gesturechange', this.handleGestureChange.bind(this));
            document.addEventListener('gestureend', this.handleGestureEnd.bind(this));
        },

        handleTouchStart(e) {
            const target = e.currentTarget;
            const touch = e.touches[0];
            
            target.dataset.touchStartX = touch.clientX;
            target.dataset.touchStartY = touch.clientY;
            target.dataset.touchStartTime = Date.now();
            
            target.classList.add('touch-active');
            
            if (this.config.enableHapticFeedback && navigator.vibrate) {
                navigator.vibrate(10);
            }
            
            this.showTouchRipple(e, target);
        },

        handleTouchMove(e) {
            const target = e.currentTarget;
            const touch = e.touches[0];
            const startX = parseFloat(target.dataset.touchStartX || 0);
            const startY = parseFloat(target.dataset.touchStartY || 0);
            
            const deltaX = touch.clientX - startX;
            const deltaY = touch.clientY - startY;
            
            if (Math.abs(deltaX) > 10 || Math.abs(deltaY) > 10) {
                target.classList.remove('touch-active');
            }
            
            if (this.config.enableGestureTrails) {
                this.showGestureTrail(touch.clientX, touch.clientY);
            }
        },

        handleTouchEnd(e) {
            const target = e.currentTarget;
            const touch = e.changedTouches[0];
            const startX = parseFloat(target.dataset.touchStartX || 0);
            const startY = parseFloat(target.dataset.touchStartY || 0);
            const startTime = parseFloat(target.dataset.touchStartTime || 0);
            
            const deltaX = touch.clientX - startX;
            const deltaY = touch.clientY - startY;
            const duration = Date.now() - startTime;
            
            target.classList.remove('touch-active');
            
            if (duration > this.config.gestureHoldDuration && Math.abs(deltaX) < 5 && Math.abs(deltaY) < 5) {
                this.handleLongPress(target, e);
            } else if (Math.abs(deltaX) > 50 || Math.abs(deltaY) > 50) {
                this.handleSwipe(target, deltaX > 0 ? 'right' : 'left', deltaY > 0 ? 'down' : 'up');
            }
        },

        handleTouchClick(e) {
            const target = e.currentTarget;
            const bulbId = target.dataset.bulbId;
            
            if (bulbId) {
                this.toggleBulbSelection(bulbId);
            }
        },

        handleLongPress(target, e) {
            e.preventDefault();
            const bulbId = target.dataset.bulbId;
            
            if (bulbId) {
                this.showBulbContextMenu(bulbId, e);
            }
            
            if (this.config.enableHapticFeedback && navigator.vibrate) {
                navigator.vibrate([50, 50, 50]);
            }
        },

        handleSwipe(target, horizontal, vertical) {
            const bulbId = target.dataset.bulbId;
            
            if (bulbId && horizontal === 'right') {
                this.toggleBulbPower(bulbId, 'toggle');
            } else if (bulbId && horizontal === 'left') {
                this.showBrightnessSlider(bulbId);
            }
            
            if (this.config.enableHapticFeedback && navigator.vibrate) {
                navigator.vibrate(20);
            }
        },

        handleGestureStart(e) {
            e.preventDefault();
            this.state.gestureScale = e.scale;
        },

        handleGestureChange(e) {
            e.preventDefault();
            const delta = e.scale - this.state.gestureScale;
            
            if (Math.abs(delta) > 0.1) {
                this.adjustGlobalBrightness(delta > 0 ? 10 : -10);
                this.state.gestureScale = e.scale;
            }
        },

        handleGestureEnd(e) {
            e.preventDefault();
        },

        showTouchRipple(e, target) {
            const ripple = document.createElement('span');
            ripple.classList.add('lifx-touch-ripple');
            
            const rect = target.getBoundingClientRect();
            const size = Math.max(rect.width, rect.height);
            ripple.style.width = ripple.style.height = size + 'px';
            ripple.style.left = (e.clientX - rect.left - size / 2) + 'px';
            ripple.style.top = (e.clientY - rect.top - size / 2) + 'px';
            
            target.appendChild(ripple);
            
            setTimeout(() => ripple.remove(), 600);
        },

        showGestureTrail(x, y) {
            const trail = document.createElement('div');
            trail.classList.add('lifx-gesture-trail');
            trail.style.left = (x - 10) + 'px';
            trail.style.top = (y - 10) + 'px';
            
            document.body.appendChild(trail);
            
            setTimeout(() => {
                trail.remove();
            }, 400);
        },

        setupSceneSelector() {
            const sceneSelector = document.getElementById('lifx-scene-selector');
            if (!sceneSelector) return;

            sceneSelector.innerHTML = this.scenePresets.map(scene => 
                `<option value="${scene.id}">${scene.icon} ${scene.name}</option>`
            ).join('');

            sceneSelector.addEventListener('change', (e) => {
                this.applyScene(e.target.value);
            });
        },

        setupQuickActions() {
            const quickActionsContainer = document.getElementById('lifx-quick-actions');
            if (!quickActionsContainer) return;

            quickActionsContainer.innerHTML = `
                <div class="quick-actions-grid">
                    <button class="quick-action-btn" data-action="all-off" data-label="All Off" data-icon="💡">
                        <span class="icon">💡</span>
                        <span class="label">All Off</span>
                    </button>
                    <button class="quick-action-btn" data-action="all-on" data-label="All On" data-icon="☀️">
                        <span class="icon">☀️</span>
                        <span class="label">All On</span>
                    </button>
                    <button class="quick-action-btn" data-action="circadian" data-label="Circadian" data-icon="🕐">
                        <span class="icon">🕐</span>
                        <span class="label">Circadian</span>
                    </button>
                    <button class="quick-action-btn" data-action="party" data-action-type="effect" data-label="Party" data-icon="🎉">
                        <span class="icon">🎉</span>
                        <span class="label">Party</span>
                    </button>
                    <button class="quick-action-btn" data-action="fireplace" data-action-type="effect" data-label="Fireplace" data-icon="🔥">
                        <span class="icon">🔥</span>
                        <span class="label">Fireplace</span>
                    </button>
                    <button class="quick-action-btn" data-action="aurora" data-action-type="effect" data-label="Aurora" data-icon="🌌">
                        <span class="icon">🌌</span>
                        <span class="label">Aurora</span>
                    </button>
                </div>
            `;

            quickActionsContainer.querySelectorAll('.quick-action-btn').forEach(btn => {
                btn.addEventListener('click', (e) => {
                    const action = e.currentTarget.dataset.action;
                    const actionType = e.currentTarget.dataset.actionType || 'scene';
                    this.handleQuickAction(action, actionType);
                });
                btn.addEventListener('touchstart', (e) => {
                    e.preventDefault();
                    btn.classList.add('active');
                });
                btn.addEventListener('touchend', (e) => {
                    e.preventDefault();
                    btn.classList.remove('active');
                });
            });
        },

        handleQuickAction(action, actionType) {
            switch(action) {
                case 'all-off':
                    this.setLifxState('all', 'off');
                    break;
                case 'all-on':
                    this.setLifxState('all', 'on');
                    break;
                case 'circadian':
                    this.applyCircadian();
                    break;
                case 'party':
                    this.applyEffect('rainbow', 3, 10);
                    break;
                case 'fireplace':
                    this.applyEffect('fireplace', 1, 5);
                    break;
                case 'aurora':
                    this.applyEffect('aurora', 1, 5);
                    break;
            }
        },

        setupMediaPresets() {
            const mediaPresetsContainer = document.getElementById('media-presets');
            if (!mediaPresetsContainer) return;

            mediaPresetsContainer.innerHTML = `
                <div class="media-presets-grid">
                    ${this.mediaPresets.map(preset => `
                        <button class="media-preset-btn" data-service="${preset.service}">
                            <span class="icon">${preset.icon}</span>
                            <span class="label">${preset.name}</span>
                        </button>
                    `).join('')}
                </div>
            `;

            mediaPresetsContainer.querySelectorAll('.media-preset-btn').forEach(btn => {
                btn.addEventListener('click', (e) => {
                    const service = e.currentTarget.dataset.service;
                    this.launchMediaService(service);
                });
            });
        },

        async launchMediaService(service) {
            try {
                const response = await fetch(`/api/services/${service}/launch`, {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' }
                });
                const data = await response.json();
                this.showToast(`${service} launched!`, 'success');
            } catch (error) {
                this.showToast(`Failed to launch ${service}`, 'error');
            }
        },

        async applyScene(sceneName) {
            const preset = this.scenePresets.find(p => p.id === sceneName);
            if (!preset) return;
            
            try {
                const response = await fetch('/api/services/lifx/scenes', {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify({
                        selector: 'all',
                        scene: sceneName,
                        duration: 1.0
                    })
                });
                const data = await response.json();
                if (data.success) {
                    this.state.activeScene = sceneName;
                    this.showToast(`${preset.icon} Scene '${preset.name}' applied!`, 'success');
                    this.showSceneIndicator(sceneName);
                }
            } catch (error) {
                console.error('Error applying scene:', error);
                this.showToast('Failed to apply scene', 'error');
            }
        },

        async applyCircadian() {
            try {
                const response = await fetch('/api/services/lifx/circadian', {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify({
                        selector: 'all',
                        enable: true
                    })
                });
                const data = await response.json();
                if (data.success) {
                    this.state.circadianActive = true;
                    this.showToast(`🕐 Circadian rhythm applied (${data.time_of_day})`, 'success');
                }
            } catch (error) {
                console.error('Error applying circadian:', error);
                this.showToast('Failed to apply circadian', 'error');
            }
        },

        async applyEffect(effectName, cycles = 1, duration = 5) {
            const preset = this.effectPresets.find(p => p.id === effectName);
            
            try {
                const response = await fetch('/api/services/lifx/effect', {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify({
                        selector: 'all',
                        effect: effectName,
                        cycles: cycles,
                        duration: duration
                    })
                });
                const data = await response.json();
                if (data.success) {
                    this.state.activeEffect = effectName;
                    this.showToast(`✨ ${preset ? preset.name : effectName} effect started!`, 'success');
                    this.showEffectIndicator(effectName);
                }
            } catch (error) {
                console.error('Error applying effect:', error);
                this.showToast('Failed to apply effect', 'error');
            }
        },

        async setLifxState(selector, power) {
            try {
                const response = await fetch('/api/services/lifx/set_state', {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify({
                        selector: selector,
                        power: power
                    })
                });
                const data = await response.json();
                if (data.success) {
                    this.showToast(`${power === 'on' ? '💡' : '🌑'} Lights ${power}!`, 'success');
                }
            } catch (error) {
                console.error('Error setting state:', error);
                this.showToast('Failed to set state', 'error');
            }
        },

        toggleBulbSelection(bulbId) {
            if (this.state.selectedBulbs.has(bulbId)) {
                this.state.selectedBulbs.delete(bulbId);
                document.querySelector(`[data-bulb-id="${bulbId}"]`)?.classList.remove('multi-selected');
            } else {
                this.state.selectedBulbs.add(bulbId);
                document.querySelector(`[data-bulb-id="${bulbId}"]`)?.classList.add('multi-selected');
            }
            this.updateSelectionToolbar();
        },

        updateSelectionToolbar() {
            const toolbar = document.getElementById('lifx-selection-toolbar');
            if (!toolbar) return;
            
            const count = this.state.selectedBulbs.size;
            toolbar.querySelector('.selected-count').textContent = count;
            
            if (count > 0) {
                toolbar.classList.add('visible');
            } else {
                toolbar.classList.remove('visible');
            }
        },

        showSceneIndicator(sceneName) {
            const indicator = document.createElement('div');
            indicator.className = 'scene-indicator visible';
            const scene = this.scenePresets.find(s => s.id === sceneName);
            indicator.innerHTML = `${scene ? scene.icon : '🎨'} ${scene ? scene.name : sceneName}`;
            document.body.appendChild(indicator);
            
            setTimeout(() => {
                indicator.classList.remove('visible');
                setTimeout(() => indicator.remove(), 300);
            }, 3000);
        },

        showEffectIndicator(effectName) {
            const indicator = document.createElement('div');
            indicator.className = 'effect-active-indicator visible';
            const effect = this.effectPresets.find(e => e.id === effectName);
            indicator.innerHTML = `
                <span class="effect-icon">${effect ? effect.icon : '✨'}</span>
                <span class="effect-name">${effect ? effect.name : effectName}</span>
            `;
            document.body.appendChild(indicator);
        },

        showToast(message, type = 'info') {
            const toast = document.createElement('div');
            toast.className = `lifx-toast lifx-toast-${type}`;
            toast.textContent = message;
            document.body.appendChild(toast);
            
            setTimeout(() => {
                toast.classList.add('visible');
            }, 10);
            
            setTimeout(() => {
                toast.classList.remove('visible');
                setTimeout(() => toast.remove(), 300);
            }, 3000);
        },

        syncStatus() {
            fetch('/api/services/lifx/status')
                .then(res => res.json())
                .then(data => {
                    this.updateLifxStatus(data);
                })
                .catch(err => console.error('LIFX status sync error:', err));
        },

        updateLifxStatus(data) {
            const statusElement = document.getElementById('lifx-status');
            if (statusElement) {
                statusElement.innerHTML = `
                    <span class="status-dot ${data.connected ? 'connected' : 'disconnected'}"></span>
                    <span>${data.bulbs_found || 0} bulbs found</span>
                `;
            }
        },

        startPeriodicSync() {
            setInterval(() => {
                this.syncStatus();
            }, 5000);
        },

        setupColorPicker() {
            const colorPicker = document.getElementById('lifx-color-picker');
            if (!colorPicker) return;
            
            colorPicker.addEventListener('input', (e) => {
                const color = e.target.value;
                this.applyColorToSelected(color);
            });
        },

        setupEffectSelector() {
            const effectSelector = document.getElementById('lifx-effect-selector');
            if (!effectSelector) return;
            
            effectSelector.innerHTML = this.effectPresets.map(effect => 
                `<option value="${effect.id}">${effect.icon} ${effect.name}</option>`
            ).join('');
            
            effectSelector.addEventListener('change', (e) => {
                const effect = this.effectPresets.find(p => p.id === e.target.value);
                if (effect) {
                    this.applyEffect(effect.id, effect.cycles, effect.duration);
                }
            });
        },

        setupZoneControl() {
            const zoneControl = document.getElementById('lifx-zone-control');
            if (!zoneControl) return;
            
            zoneControl.innerHTML = `
                <div class="zone-control-header">
                    <span class="zone-icon">📍</span>
                    <span class="zone-title">Zone Control</span>
                </div>
                <div class="zone-selection">
                    <button class="zone-btn" data-zone="all">All Zones</button>
                    <button class="zone-btn" data-zone="start">Start</button>
                    <button class="zone-btn" data-zone="middle">Middle</button>
                    <button class="zone-btn" data-zone="end">End</button>
                </div>
            `;
            
            zoneControl.querySelectorAll('.zone-btn').forEach(btn => {
                btn.addEventListener('click', (e) => {
                    const zone = e.currentTarget.dataset.zone;
                    this.applyZoneColor(zone);
                });
            });
        },

        setupGestureHints() {
            const hintsContainer = document.getElementById('lifx-gesture-hints');
            if (!hintsContainer) return;
            
            hintsContainer.innerHTML = `
                <div class="gesture-hint-item">
                    <span class="gesture-icon">👆</span>
                    <span class="gesture-text">Tap to select</span>
                </div>
                <div class="gesture-hint-item">
                    <span class="gesture-icon">👆👆</span>
                    <span class="gesture-text">Long press for menu</span>
                </div>
                <div class="gesture-hint-item">
                    <span class="gesture-icon">👉</span>
                    <span class="gesture-text">Swipe right to toggle power</span>
                </div>
                <div class="gesture-hint-item">
                    <span class="gesture-icon">👈</span>
                    <span class="gesture-text">Swipe left for brightness</span>
                </div>
                <div class="gesture-hint-item">
                    <span class="gesture-icon">🤏</span>
                    <span class="gesture-text">Pinch to adjust global brightness</span>
                </div>
            `;
        },

        applyColorToSelected(hexColor) {
            if (this.state.selectedBulbs.size === 0) return;
            
            const bulbIds = Array.from(this.state.selectedBulbs);
            
            fetch('/api/services/lifx/set_color', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({
                    selector: bulbIds.map(id => `id:${id}`).join(','),
                    color: hexColor
                })
            }).then(res => res.json())
              .then(data => {
                  if (data.success) {
                      this.showToast(`Color applied to ${bulbIds.length} bulbs`, 'success');
                  }
              });
        },

        applyZoneColor(zone) {
            const zoneRanges = {
                'all': { start: 0, end: 255 },
                'start': { start: 0, end: 85 },
                'middle': { start: 86, end: 170 },
                'end': { start: 171, end: 255 }
            };
            
            const range = zoneRanges[zone];
            if (!range) return;
            
            fetch('/api/services/lifx/zones', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({
                    selector: 'all',
                    start_index: range.start,
                    end_index: range.end,
                    color: '#00d4ff',
                    duration: 0.5
                })
            }).then(res => res.json())
              .then(data => {
                  if (data.success) {
                      this.showToast(`Zone ${zone} updated`, 'success');
                  }
              });
        },

        adjustGlobalBrightness(delta) {
            this.state.brightnessLevel = Math.max(0, Math.min(100, this.state.brightnessLevel + delta));
            
            fetch('/api/services/lifx/set_state', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({
                    selector: 'all',
                    brightness: this.state.brightnessLevel / 100
                })
            });
            
            this.showBrightnessFeedback(this.state.brightnessLevel);
        },

        showBrightnessFeedback(level) {
            const feedback = document.createElement('div');
            feedback.className = 'touch-feedback-brightness visible';
            feedback.textContent = `${level}%`;
            document.body.appendChild(feedback);
            
            setTimeout(() => {
                feedback.classList.remove('visible');
                setTimeout(() => feedback.remove(), 300);
            }, 1000);
        },

        showBulbContextMenu(bulbId, e) {
            const menu = document.createElement('div');
            menu.className = 'lifx-context-menu';
            menu.innerHTML = `
                <button class="context-menu-item" data-action="power">Toggle Power</button>
                <button class="context-menu-item" data-action="brightness">Brightness</button>
                <button class="context-menu-item" data-action="color">Color</button>
                <button class="context-menu-item" data-action="scene">Apply Scene</button>
            `;
            
            menu.style.left = e.clientX + 'px';
            menu.style.top = e.clientY + 'px';
            
            document.body.appendChild(menu);
            
            menu.querySelectorAll('.context-menu-item').forEach(item => {
                item.addEventListener('click', (ev) => {
                    const action = ev.currentTarget.dataset.action;
                    this.handleBulbAction(bulbId, action);
                    menu.remove();
                });
            });
            
            setTimeout(() => menu.remove(), 5000);
        },

        handleBulbAction(bulbId, action) {
            switch(action) {
                case 'power':
                    this.toggleBulbPower(bulbId);
                    break;
                case 'brightness':
                    this.showBrightnessSlider(bulbId);
                    break;
                case 'color':
                    this.showColorPicker(bulbId);
                    break;
                case 'scene':
                    this.showSceneSelector(bulbId);
                    break;
            }
        },

        toggleBulbPower(bulbId, state = 'toggle') {
            fetch('/api/services/lifx/set_state', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({
                    selector: `id:${bulbId}`,
                    power: state === 'toggle' ? 'toggle' : state
                })
            });
        },

        showBrightnessSlider(bulbId) {
            const slider = document.createElement('div');
            slider.className = 'lifx-brightness-slider';
            slider.innerHTML = `
                <input type="range" min="0" max="100" value="50" />
            `;
            
            slider.style.position = 'fixed';
            slider.style.left = '50%';
            slider.style.top = '50%';
            slider.style.transform = 'translate(-50%, -50%)';
            
            document.body.appendChild(slider);
            
            slider.querySelector('input').addEventListener('input', (e) => {
                const brightness = e.target.value / 100;
                fetch('/api/services/lifx/set_state', {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify({
                        selector: `id:${bulbId}`,
                        brightness: brightness
                    })
                });
            });
            
            setTimeout(() => slider.remove(), 5000);
        },

        showColorPicker(bulbId) {
            const picker = document.createElement('input');
            picker.type = 'color';
            picker.style.position = 'fixed';
            picker.style.left = '50%';
            picker.style.top = '50%';
            picker.style.transform = 'translate(-50%, -50%)';
            picker.style.zIndex = '10000';
            
            document.body.appendChild(picker);
            
            picker.addEventListener('input', (e) => {
                const color = e.target.value;
                fetch('/api/services/lifx/set_color', {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify({
                        selector: `id:${bulbId}`,
                        color: color
                    })
                });
                picker.remove();
            });
            
            picker.click();
        },

        showSceneSelector(bulbId) {
            const selector = document.createElement('select');
            selector.innerHTML = this.scenePresets.map(scene => 
                `<option value="${scene.id}">${scene.icon} ${scene.name}</option>`
            ).join('');
            
            selector.style.position = 'fixed';
            selector.style.left = '50%';
            selector.style.top = '50%';
            selector.style.transform = 'translate(-50%, -50%)';
            selector.style.zIndex = '10000';
            selector.style.padding = '10px';
            selector.style.fontSize = '16px';
            
            document.body.appendChild(selector);
            
            selector.addEventListener('change', (e) => {
                const sceneId = e.target.value;
                const scene = this.scenePresets.find(s => s.id === sceneId);
                if (scene) {
                    fetch('/api/services/lifx/scenes', {
                        method: 'POST',
                        headers: { 'Content-Type': 'application/json' },
                        body: JSON.stringify({
                            selector: `id:${bulbId}`,
                            scene: sceneId,
                            duration: 0.5
                        })
                    });
                }
                selector.remove();
            });
        },

        setupMediaPlayers() {
            // Initialize media player connections
            this.mediaPlayers = {
                spotify: null,
                youtube: null,
                plex: null
            };
        },

        setupLightGroups() {
            // Initialize light group management
            this.lightGroups = new Map();
        },

        setupVolumeSliders() {
            // Setup volume control sliders for media
            const volumeSliders = document.querySelectorAll('.media-volume-slider');
            volumeSliders.forEach(slider => {
                slider.addEventListener('input', (e) => {
                    this.setVolume(e.target.value);
                });
            });
        },

        setupBrightnessSliders() {
            // Setup brightness sliders
            const brightnessSliders = document.querySelectorAll('.lifx-brightness-slider-input');
            brightnessSliders.forEach(slider => {
                slider.addEventListener('input', (e) => {
                    this.setGlobalBrightness(e.target.value);
                });
            });
        },

        setVolume(level) {
            // Implement volume control via WebSocket
            if (window.websocket && websocket.readyState === WebSocket.OPEN) {
                websocket.send(JSON.stringify({
                    type: 'command',
                    id: `volume_${Date.now()}`,
                    command: 'set_volume',
                    args: { level: parseInt(level) }
                }));
            }
        },

        setGlobalBrightness(level) {
            this.state.brightnessLevel = parseInt(level);
            fetch('/api/services/lifx/set_state', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({
                    selector: 'all',
                    brightness: level / 100
                })
            });
        },

        enableMediaSync() {
            this.state.mediaSyncActive = true;
            this.startBeatDetection();
        },

        disableMediaSync() {
            this.state.mediaSyncActive = false;
            if (this.audioContext) {
                this.audioContext.suspend();
            }
        },

        startBeatDetection() {
            if (!navigator.mediaDevices || !navigator.mediaDevices.getUserMedia) {
                console.warn('Media devices not available');
                return;
            }
            
            navigator.mediaDevices.getUserMedia({ audio: true })
                .then(stream => {
                    this.audioContext = new (window.AudioContext || window.webkitAudioContext)();
                    this.analyser = this.audioContext.createAnalyser();
                    const source = this.audioContext.createMediaStreamSource(stream);
                    source.connect(this.analyser);
                    this.analyser.fftSize = 256;
                    this.detectBeats();
                })
                .catch(err => console.error('Beat detection error:', err));
        },

        detectBeats() {
            if (!this.analyser || !this.state.mediaSyncActive) return;
            
            const dataArray = new Uint8Array(this.analyser.frequencyBinCount);
            this.analyser.getByteFrequencyData(dataArray);
            
            const bassRange = dataArray.slice(0, 8);
            const bassAvg = bassRange.reduce((a, b) => a + b, 0) / bassRange.length;
            
            if (bassAvg / 255 > this.config.beatDetectionThreshold) {
                this.triggerBeatEffect();
            }
            
            requestAnimationFrame(this.detectBeats.bind(this));
        },

        triggerBeatEffect() {
            if (this.state.mediaSyncActive) {
                fetch('/api/services/lifx/set_state', {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify({
                        selector: 'all',
                        brightness: 1.0
                    })
                });
                
                setTimeout(() => {
                    fetch('/api/services/lifx/set_state', {
                        method: 'POST',
                        headers: { 'Content-Type': 'application/json' },
                        body: JSON.stringify({
                            selector: 'all',
                            brightness: this.state.brightnessLevel / 100
                        })
                    });
                }, 100);
            }
        }
    };

    window.LIFXMediaTouchV2 = LIFXMediaTouchV2;

    document.addEventListener('DOMContentLoaded', () => {
        LIFXMediaTouchV2.init();
    });
})();
