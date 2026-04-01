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
            gestureHoldDuration: 500,
            swipeThreshold: 50,
            doubleTapDelay: 300,
            rippleDuration: 400,
            gestureTrailSize: 20,
            enableVelocityRipples: true,
            enableSwipeTrails: true,
            enableMultiSelectDrag: true,
            enableAdaptiveSensitivity: true,
            minSwipeVelocity: 0.3,
            maxTouchHistory: 10,
            hapticPatterns: {
                tap: [10],
                doubleTap: [15, 50, 15],
                longPress: [50, 50, 50],
                swipe: [20],
                beat: [25],
                success: [10, 50, 10, 50, 10]
            }
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
            lastTapTime: 0,
            gestureHistory: [],
            touchHoldProgress: 0,
            isTouchHoldActive: false,
            frequencyData: new Uint8Array(6),
            beatHistory: [],
            adaptiveThreshold: 0.75,
            bpmHistory: [],
            bpmSmoothed: 0,
            lastBeatTime: 0,
            touchVelocity: null,
            gestureScale: 1,
            sensitivityCalibrated: false,
            baselineEnergy: 128,
            visualizationMode: 'bars'
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
            { id: 'crystal', name: 'Crystal', icon: '💎', hue: 34580, saturation: 26214, brightness: 52428, kelvin: 7500 },
            { id: 'cyberpunk', name: 'Cyberpunk', icon: '🤖', hue: 30940, saturation: 52428, brightness: 58982, kelvin: 4500 },
            { id: 'vaporwave', name: 'Vaporwave', icon: '🌴', hue: 58240, saturation: 39321, brightness: 52428, kelvin: 4000 },
            { id: 'halloween', name: 'Halloween', icon: '🎃', hue: 5460, saturation: 52428, brightness: 49151, kelvin: 2800 },
            { id: 'christmas', name: 'Christmas', icon: '🎄', hue: 5800, saturation: 45875, brightness: 55705, kelvin: 3500 },
            { id: 'beach', name: 'Beach', icon: '🏖️', hue: 18200, saturation: 32767, brightness: 52428, kelvin: 5000 },
            { id: 'forest', name: 'Forest', icon: '🌲', hue: 25480, saturation: 39321, brightness: 42598, kelvin: 4200 },
            { id: 'yoga', name: 'Yoga', icon: '🧘', hue: 25480, saturation: 26214, brightness: 39321, kelvin: 3800 },
            { id: 'cooking', name: 'Cooking', icon: '🍳', hue: 5460, saturation: 32767, brightness: 58982, kelvin: 4500 },
            { id: 'creative', name: 'Creative', icon: '🎨', hue: 58240, saturation: 45875, brightness: 52428, kelvin: 5000 },
            { id: 'dinner', name: 'Dinner', icon: '🍽️', hue: 6000, saturation: 26214, brightness: 32767, kelvin: 3000 },
            { id: 'spa', name: 'Spa', icon: '💆', hue: 32760, saturation: 19660, brightness: 26214, kelvin: 3500 },
            { id: 'festival', name: 'Festival', icon: '🎪', hue: 27300, saturation: 58982, brightness: 65535, kelvin: 4200 }
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
            this.setupCleanupHandlers();
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
            if (!target) return;
            
            const touch = e.touches[0];
            if (!touch) return;
            
            target.dataset.touchStartX = touch.clientX;
            target.dataset.touchStartY = touch.clientY;
            target.dataset.touchStartTime = Date.now();
            target.dataset.lastTouchX = touch.clientX;
            target.dataset.lastTouchY = touch.clientY;
            
            target.classList.add('touch-active');
            
            if (this.config.enableHapticFeedback && navigator.vibrate) {
                try {
                    navigator.vibrate(this.config.hapticPatterns.tap);
                } catch (e) {
                    console.warn('[LIFXMediaTouchV2] Haptic feedback failed:', e);
                }
            }
            
            this.showTouchRipple(e, target);
            this.startTouchHoldTimer(target);
            
            if (this.config.enableVelocityRipples) {
                this.touchVelocity = [];
            }
        },

        handleTouchMove(e) {
            const target = e.currentTarget;
            if (!target) return;
            
            const touch = e.touches[0];
            if (!touch) return;
            
            const startX = parseFloat(target.dataset.touchStartX || 0);
            const startY = parseFloat(target.dataset.touchStartY || 0);
            const lastX = parseFloat(target.dataset.lastTouchX || touch.clientX);
            const lastY = parseFloat(target.dataset.lastTouchY || touch.clientY);
            
            const deltaX = touch.clientX - startX;
            const deltaY = touch.clientY - startY;
            const instantDeltaX = touch.clientX - lastX;
            const instantDeltaY = touch.clientY - lastY;
            
            target.dataset.lastTouchX = touch.clientX;
            target.dataset.lastTouchY = touch.clientY;
            
            if (Math.abs(deltaX) > 10 || Math.abs(deltaY) > 10) {
                target.classList.remove('touch-active');
                this.cancelTouchHoldTimer();
            }
            
            if (this.config.enableGestureTrails) {
                this.showGestureTrail(touch.clientX, touch.clientY);
            }
            
            if (this.config.enableVelocityRipples && this.touchVelocity) {
                this.touchVelocity.push({ x: instantDeltaX, y: instantDeltaY, time: Date.now() });
                if (this.touchVelocity.length > 5) {
                    this.touchVelocity.shift();
                }
            }
            
            if (this.config.enableSwipeTrails && (Math.abs(instantDeltaX) > 3 || Math.abs(instantDeltaY) > 3)) {
                this.showSwipeTrail(touch.clientX, touch.clientY, instantDeltaX, instantDeltaY);
            }
            
            this.updateTouchHoldProgress(target, deltaX, deltaY);
        },

        handleTouchEnd(e) {
            const target = e.currentTarget;
            if (!target) return;
            
            const touch = e.changedTouches[0];
            if (!touch) return;
            
            const startX = parseFloat(target.dataset.touchStartX || 0);
            const startY = parseFloat(target.dataset.touchStartY || 0);
            const startTime = parseFloat(target.dataset.touchStartTime || 0);
            
            const deltaX = touch.clientX - startX;
            const deltaY = touch.clientY - startY;
            const duration = Date.now() - startTime;
            const currentTime = Date.now();
            
            target.classList.remove('touch-active');
            this.cancelTouchHoldTimer();
            
            if (this.state.isTouchHoldActive) {
                this.handleLongPress(target, e);
                this.state.isTouchHoldActive = false;
            } else if (Math.abs(deltaX) > this.config.swipeThreshold || Math.abs(deltaY) > this.config.swipeThreshold) {
                const horizontal = deltaX > 0 ? 'right' : 'left';
                const vertical = deltaY > 0 ? 'down' : 'up';
                this.handleSwipe(target, horizontal, vertical);
                
                if (this.config.enableSwipeTrails) {
                    this.showSwipeTrailEnd(touch.clientX, touch.clientY, horizontal === 'right' || horizontal === 'left' ? deltaX : deltaY);
                }
            } else if (duration < this.config.doubleTapDelay && currentTime - this.state.lastTapTime < this.config.doubleTapDelay) {
                this.handleDoubleTap(target, e);
                this.state.lastTapTime = 0;
            } else {
                this.state.lastTapTime = currentTime;
                
                if (this.config.enableVelocityRipples && this.touchVelocity && this.touchVelocity.length > 2) {
                    const avgVelocity = this.touchVelocity.reduce((sum, v) => sum + Math.sqrt(v.x * v.x + v.y * v.y), 0) / this.touchVelocity.length;
                    if (avgVelocity > 2) {
                        this.showVelocityRipple(target, avgVelocity);
                    }
                }
                this.touchVelocity = null;
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
                navigator.vibrate(this.config.hapticPatterns.longPress);
            }
        },

        handleSwipe(target, horizontal, vertical) {
            const bulbId = target.dataset.bulbId;
            
            if (bulbId && horizontal === 'right') {
                this.toggleBulbPower(bulbId, 'toggle');
                this.showSwipeHint(horizontal);
            } else if (bulbId && horizontal === 'left') {
                this.showBrightnessSlider(bulbId);
                this.showSwipeHint(horizontal);
            } else if (bulbId && vertical === 'up') {
                this.showColorPicker(bulbId);
                this.showSwipeHint(vertical);
            } else if (bulbId && vertical === 'down') {
                this.showSceneSelector(bulbId);
                this.showSwipeHint(vertical);
            }
            
            if (this.config.enableHapticFeedback && navigator.vibrate) {
                navigator.vibrate(this.config.hapticPatterns.swipe);
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

        touchHoldTimer: null,

        startTouchHoldTimer(target) {
            this.cancelTouchHoldTimer();
            this.state.touchHoldProgress = 0;
            this.state.isTouchHoldActive = true;
            
            const startTime = Date.now();
            const duration = this.config.gestureHoldDuration;
            
            this.touchHoldTimer = setInterval(() => {
                const elapsed = Date.now() - startTime;
                const progress = Math.min(100, (elapsed / duration) * 100);
                this.state.touchHoldProgress = progress;
                
                if (progress >= 100) {
                    this.cancelTouchHoldTimer();
                }
            }, 50);
        },

        cancelTouchHoldTimer() {
            if (this.touchHoldTimer) {
                clearInterval(this.touchHoldTimer);
                this.touchHoldTimer = null;
            }
            this.state.touchHoldProgress = 0;
        },

        updateTouchHoldProgress(target, deltaX, deltaY) {
            if (!this.state.isTouchHoldActive) return;
            
            const movement = Math.sqrt(deltaX * deltaX + deltaY * deltaY);
            if (movement > 20) {
                this.cancelTouchHoldTimer();
                this.state.isTouchHoldActive = false;
            }
        },

        handleDoubleTap(target, e) {
            const bulbId = target.dataset.bulbId;
            
            if (bulbId) {
                this.toggleBulbPower(bulbId, 'toggle');
                this.showTouchFeedback(target, 'Double Tap');
            }
            
            if (this.config.enableHapticFeedback && navigator.vibrate) {
                navigator.vibrate(this.config.hapticPatterns.doubleTap);
            }
        },

        showTouchFeedback(target, message) {
            const feedback = document.createElement('div');
            feedback.className = 'touch-feedback-message';
            feedback.textContent = message;
            feedback.style.position = 'absolute';
            feedback.style.top = '50%';
            feedback.style.left = '50%';
            feedback.style.transform = 'translate(-50%, -50%)';
            feedback.style.color = '#00d4ff';
            feedback.style.fontWeight = 'bold';
            feedback.style.fontSize = '14px';
            feedback.style.textShadow = '0 0 10px rgba(0, 212, 255, 0.8)';
            feedback.style.pointerEvents = 'none';
            feedback.style.opacity = '0';
            feedback.style.transition = 'opacity 0.2s ease';
            
            target.style.position = 'relative';
            target.appendChild(feedback);
            
            setTimeout(() => feedback.style.opacity = '1', 10);
            setTimeout(() => {
                feedback.style.opacity = '0';
                setTimeout(() => feedback.remove(), 200);
            }, 800);
        },

        showTouchRipple(e, target) {
            const ripple = document.createElement('span');
            ripple.classList.add('lifx-touch-ripple');
            
            const rect = target.getBoundingClientRect();
            const size = Math.max(rect.width, rect.height);
            ripple.style.width = ripple.style.height = size + 'px';
            ripple.style.left = (e.clientX - rect.left - size / 2) + 'px';
            ripple.style.top = (e.clientY - rect.top - size / 2) + 'px';
            
            const bulbId = target.dataset.bulbId;
            if (bulbId && this.state.selectedBulbs.has(bulbId)) {
                ripple.style.background = 'radial-gradient(circle, rgba(255, 107, 107, 0.8) 0%, rgba(255, 107, 107, 0.4) 40%, transparent 70%)';
            }
            
            target.appendChild(ripple);
            
            setTimeout(() => ripple.remove(), 600);
        },

        showVelocityRipple(target, velocity) {
            const ripple = document.createElement('span');
            ripple.classList.add('velocity-ripple');
            
            const scale = Math.min(3.0, 1 + velocity / 8);
            const duration = Math.max(250, 500 - velocity * 40);
            const hue = Math.min(200, 160 + velocity * 2);
            
            ripple.style.setProperty('--ripple-scale', scale);
            ripple.style.setProperty('--ripple-duration', duration + 'ms');
            ripple.style.setProperty('--ripple-hue', hue);
            
            const rect = target.getBoundingClientRect();
            const size = Math.max(rect.width, rect.height) * 0.8;
            ripple.style.width = ripple.style.height = size + 'px';
            ripple.style.left = '50%';
            ripple.style.top = '50%';
            ripple.style.transform = 'translate(-50%, -50%)';
            ripple.style.background = `radial-gradient(circle, hsla(${hue}, 80%, 60%, 0.7) 0%, hsla(${hue}, 80%, 60%, 0.3) 40%, transparent 70%)`;
            
            target.appendChild(ripple);
            
            setTimeout(() => ripple.remove(), duration);
        },

        showSwipeTrail(x, y, dx, dy) {
            const trail = document.createElement('div');
            trail.classList.add('swipe-trail-particle');
            
            const size = 8 + Math.random() * 6;
            trail.style.width = trail.style.height = size + 'px';
            trail.style.left = (x - size / 2) + 'px';
            trail.style.top = (y - size / 2) + 'px';
            trail.style.background = `radial-gradient(circle, hsla(${180 + Math.random() * 40}, 80%, 60%, 0.6) 0%, transparent 70%)`;
            
            const travelX = dx > 0 ? 20 : -20;
            const travelY = dy > 0 ? 20 : -20;
            trail.style.setProperty('--travel-x', travelX + 'px');
            trail.style.setProperty('--travel-y', travelY + 'px');
            
            document.body.appendChild(trail);
            
            setTimeout(() => trail.remove(), 400);
        },

        showSwipeTrailEnd(x, y, distance) {
            const trail = document.createElement('div');
            trail.classList.add('swipe-trail');
            
            const scale = Math.min(1.5, 0.5 + Math.abs(distance) / 200);
            trail.style.setProperty('--trail-scale', scale);
            trail.style.left = (x - 50) + 'px';
            trail.style.top = (y - 50) + 'px';
            
            document.body.appendChild(trail);
            
            setTimeout(() => trail.remove(), 500);
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

        showSwipeHint(direction) {
            const hint = document.createElement('div');
            hint.className = 'gesture-hint-overlay visible';
            
            const icons = {
                'right': '➡️',
                'left': '⬅️',
                'up': '⬆️',
                'down': '⬇️'
            };
            
            const texts = {
                'right': 'Power Toggle',
                'left': 'Brightness',
                'up': 'Color Picker',
                'down': 'Scenes'
            };
            
            hint.innerHTML = `
                <i class="gesture-icon">${icons[direction] || '👆'}</i>
                <span class="hint-text">${texts[direction] || ''}</span>
            `;
            
            document.body.appendChild(hint);
            
            setTimeout(() => {
                hint.classList.remove('visible');
                setTimeout(() => hint.remove(), 300);
            }, 1000);
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
            if (!preset) {
                this.showToast('Unknown scene', 'error');
                return;
            }
            
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
                
                if (!response.ok) {
                    throw new Error(`HTTP ${response.status}`);
                }
                
                const data = await response.json();
                if (data.success) {
                    this.state.activeScene = sceneName;
                    this.showToast(`${preset.icon} Scene '${preset.name}' applied!`, 'success');
                    this.showSceneIndicator(sceneName);
                } else {
                    throw new Error(data.error || 'Unknown error');
                }
            } catch (error) {
                console.error('[LIFXMediaTouchV2] Error applying scene:', error);
                this.showToast(`Failed to apply scene: ${error.message}`, 'error');
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
                
                if (!response.ok) {
                    throw new Error(`HTTP ${response.status}`);
                }
                
                const data = await response.json();
                if (data.success) {
                    this.state.circadianActive = true;
                    this.showToast(`🕐 Circadian rhythm applied (${data.time_of_day})`, 'success');
                } else {
                    throw new Error(data.error || 'Unknown error');
                }
            } catch (error) {
                console.error('[LIFXMediaTouchV2] Error applying circadian:', error);
                this.showToast(`Failed to apply circadian: ${error.message}`, 'error');
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
                
                if (!response.ok) {
                    throw new Error(`HTTP ${response.status}`);
                }
                
                const data = await response.json();
                if (data.success) {
                    this.state.activeEffect = effectName;
                    this.showToast(`✨ ${preset ? preset.name : effectName} effect started!`, 'success');
                    this.showEffectIndicator(effectName);
                } else {
                    throw new Error(data.error || 'Unknown error');
                }
            } catch (error) {
                console.error('[LIFXMediaTouchV2] Error applying effect:', error);
                this.showToast(`Failed to apply effect: ${error.message}`, 'error');
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
                
                if (!response.ok) {
                    throw new Error(`HTTP ${response.status}`);
                }
                
                const data = await response.json();
                if (data.success) {
                    this.showToast(`${power === 'on' ? '💡' : '🌑'} Lights ${power}!`, 'success');
                } else {
                    throw new Error(data.error || 'Unknown error');
                }
            } catch (error) {
                console.error('[LIFXMediaTouchV2] Error setting state:', error);
                this.showToast(`Failed to set state: ${error.message}`, 'error');
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
                .then(res => {
                    if (!res.ok) {
                        throw new Error(`HTTP ${res.status}`);
                    }
                    return res.json();
                })
                .then(data => {
                    this.updateLifxStatus(data);
                })
                .catch(err => {
                    console.error('[LIFXMediaTouchV2] LIFX status sync error:', err);
                    this.updateLifxStatus({ connected: false, bulbs_found: 0 });
                });
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
            
            const isTouchDevice = typeof is_touch_enabled === 'function' && is_touch_enabled();
            if (!isTouchDevice) {
                hintsContainer.innerHTML = `
                    <div class="gesture-hint-item">
                        <span class="gesture-icon">🖱️</span>
                        <span class="gesture-text">Click to select</span>
                    </div>
                    <div class="gesture-hint-item">
                        <span class="gesture-icon">🖱️🖱️</span>
                        <span class="gesture-text">Double-click to toggle power</span>
                    </div>
                    <div class="gesture-hint-item">
                        <span class="gesture-icon">⌨️</span>
                        <span class="gesture-text">Use scene selector for presets</span>
                    </div>
                `;
                return;
            }
            
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
        
        setupCleanupHandlers() {
            window.addEventListener('beforeunload', () => {
                this.cancelTouchHoldTimer();
                if (this.audioContext) {
                    this.audioContext.close();
                }
                this.state.mediaSyncActive = false;
            });
            
            document.addEventListener('visibilitychange', () => {
                if (document.hidden) {
                    this.cancelTouchHoldTimer();
                }
            });
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

        async undoLastGesture() {
            if (this.state.gestureHistory.length === 0) {
                this.showToast('No actions to undo', 'info');
                return;
            }
            
            const lastAction = this.state.gestureHistory.pop();
            this.showToast(`Undoing: ${lastAction.type}`, 'info');
            
            try {
                if (lastAction.type === 'color') {
                    await fetch('/api/services/lifx/set_color', {
                        method: 'POST',
                        headers: { 'Content-Type': 'application/json' },
                        body: JSON.stringify({
                            selector: lastAction.selector,
                            color: lastAction.previousColor
                        })
                    });
                } else if (lastAction.type === 'power') {
                    await fetch('/api/services/lifx/set_state', {
                        method: 'POST',
                        headers: { 'Content-Type': 'application/json' },
                        body: JSON.stringify({
                            selector: lastAction.selector,
                            power: lastAction.previousPower ? 'on' : 'off'
                        })
                    });
                } else if (lastAction.type === 'brightness') {
                    await fetch('/api/services/lifx/set_state', {
                        method: 'POST',
                        headers: { 'Content-Type': 'application/json' },
                        body: JSON.stringify({
                            selector: lastAction.selector,
                            brightness: lastAction.previousBrightness
                        })
                    });
                } else if (lastAction.type === 'scene') {
                    await fetch('/api/services/lifx/scenes', {
                        method: 'POST',
                        headers: { 'Content-Type': 'application/json' },
                        body: JSON.stringify({
                            selector: lastAction.selector,
                            scene: lastAction.previousScene,
                            duration: 0.5
                        })
                    });
                }
                this.showToast('Action undone', 'success');
            } catch (error) {
                this.showToast(`Undo failed: ${error.message}`, 'error');
            }
        },

        recordGesture(action) {
            this.state.gestureHistory.push(action);
            if (this.state.gestureHistory.length > this.config.maxTouchHistory) {
                this.state.gestureHistory.shift();
            }
            this.updateUndoButtonState();
        },

        updateUndoButtonState() {
            const undoBtn = document.getElementById('lifx-undo-btn');
            if (!undoBtn) return;
            
            if (this.state.gestureHistory.length > 0) {
                undoBtn.classList.add('visible', 'has-history');
                undoBtn.disabled = false;
            } else {
                undoBtn.classList.remove('visible', 'has-history');
                undoBtn.disabled = true;
            }
        },

        async powerAll(state) {
            const selector = this.state.selectedBulbs.size > 0 
                ? Array.from(this.state.selectedBulbs).map(id => `id:${id}`).join(',')
                : 'all';
            
            try {
                const response = await fetch('/api/services/lifx/set_state', {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify({
                        selector: selector,
                        power: state
                    })
                });
                
                const data = await response.json();
                if (data.success) {
                    this.showToast(`${state === 'on' ? '💡' : '🌑'} ${this.state.selectedBulbs.size > 0 ? this.state.selectedBulbs.size + ' bulbs' : 'All lights'} turned ${state}!`, 'success');
                    this.clearMultiSelection();
                }
            } catch (error) {
                this.showToast(`Failed: ${error.message}`, 'error');
            }
        },

        clearMultiSelection() {
            this.state.selectedBulbs.clear();
            document.querySelectorAll('.lifx-bulb-control.multi-selected').forEach(el => {
                el.classList.remove('multi-selected');
            });
            this.updateSelectionToolbar();
        },

        showGroupManagementPanel() {
            Swal.fire({
                title: 'Manage Bulb Groups',
                html: `
                    <div style="text-align: left;">
                        <p>Create and manage groups of LIFX bulbs for batch control.</p>
                        <div class="form-group">
                            <label>Group Name</label>
                            <input type="text" id="group-name" class="form-control" placeholder="Living Room">
                        </div>
                        <div class="form-group">
                            <label>Select Bulbs</label>
                            <div id="group-bulb-selector" style="max-height: 200px; overflow-y: auto;">
                                ${this.getBulbSelectorHTML()}
                            </div>
                        </div>
                    </div>
                `,
                confirmButtonText: 'Create Group',
                showCancelButton: true,
                cancelButtonText: 'Cancel'
            }).then((result) => {
                if (result.isConfirmed) {
                    this.createLightGroup();
                }
            });
        },

        getBulbSelectorHTML() {
            return `<p style="color: #adb5bd; text-align: center;">Bulb selection coming soon...</p>`;
        },

        createLightGroup() {
            const groupName = document.getElementById('group-name')?.value;
            if (!groupName) {
                this.showToast('Please enter a group name', 'error');
                return;
            }
            this.showToast(`Group "${groupName}" created!`, 'success');
        },

        adjustBrightnessBatch(delta) {
            if (this.state.selectedBulbs.size === 0) {
                this.showToast('Select bulbs first', 'warning');
                return;
            }
            
            const newBrightness = Math.max(0, Math.min(100, this.state.brightnessLevel + delta));
            this.state.brightnessLevel = newBrightness;
            
            fetch('/api/services/lifx/set_state', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({
                    selector: Array.from(this.state.selectedBulbs).map(id => `id:${id}`).join(','),
                    brightness: newBrightness / 100
                })
            }).then(res => res.json())
              .then(data => {
                  if (data.success) {
                      this.showToast(`Brightness: ${newBrightness}%`, 'success');
                  }
              });
        },

        setVisualizationMode(mode) {
            this.state.visualizationMode = mode;
            const vizContainer = document.querySelector('.frequency-viz');
            if (!vizContainer) return;
            
            vizContainer.className = `frequency-viz visualization-${mode}`;
            
            switch(mode) {
                case 'bars':
                    vizContainer.style.flexDirection = 'row';
                    vizContainer.style.alignItems = 'flex-end';
                    break;
                case 'wave':
                    vizContainer.style.flexDirection = 'row';
                    vizContainer.style.alignItems = 'center';
                    break;
                case 'circular':
                    vizContainer.style.display = 'flex';
                    vizContainer.style.flexWrap = 'wrap';
                    vizContainer.style.justifyContent = 'center';
                    break;
            }
            
            this.showToast(`Visualization: ${mode}`, 'info');
        },

        setupMediaPlayers() {
            this.mediaPlayers = {
                spotify: null,
                youtube: null,
                plex: null,
                tidal: null,
                apple_music: null,
                radio: null
            };
            
            this.connectSpotify();
            this.setupMediaSyncButton();
            this.setupNowPlayingDisplay();
        },
        
        async connectSpotify() {
            try {
                const response = await fetch('/api/services/spotify/status');
                const data = await response.json();
                if (data.connected) {
                    this.mediaPlayers.spotify = data;
                    this.startMediaPlaybackMonitor();
                }
            } catch (error) {
                console.warn('Spotify not available');
            }
        },
        
        startMediaPlaybackMonitor() {
            setInterval(async () => {
                try {
                    const response = await fetch('/api/services/spotify/now-playing');
                    const data = await response.json();
                    if (data.track) {
                        this.updateMediaDisplay(data.track);
                    }
                } catch (error) {
                    // Silent fail - media may not be playing
                }
            }, 5000);
        },
        
        updateMediaDisplay(track) {
            const trackName = document.getElementById('media-track-name');
            const artistName = document.getElementById('media-artist-name');
            if (trackName) trackName.textContent = track.name || 'No Track';
            if (artistName) artistName.textContent = track.artist || 'Unknown Artist';
        },
        
        setupMediaSyncButton() {
            const syncBtn = document.getElementById('media-sync-toggle');
            if (!syncBtn) return;
            
            syncBtn.addEventListener('click', () => {
                this.toggleMediaSync();
                syncBtn.classList.toggle('active', this.state.mediaSyncActive);
            });
            
            this.setupMediaSyncModes();
        },
        
        setupMediaSyncModes() {
            const modeButtons = document.querySelectorAll('.media-sync-mode-btn');
            modeButtons.forEach(btn => {
                btn.addEventListener('click', (e) => {
                    const mode = e.currentTarget.dataset.mode;
                    this.setMediaSyncMode(mode);
                    modeButtons.forEach(b => b.classList.remove('active'));
                    e.currentTarget.classList.add('active');
                });
            });
        },
        
        setMediaSyncMode(mode) {
            this.state.mediaSyncMode = mode;
            this.showToast(`Media sync mode: ${mode}`, 'info');
            
            switch(mode) {
                case 'beat':
                    this.startBeatDetection();
                    break;
                case 'ambient':
                    this.startAmbientAnalysis();
                    break;
                case 'spectrum':
                    this.startSpectrumAnalysis();
                    break;
                case 'off':
                    this.disableMediaSync();
                    break;
            }
        },
        
        toggleMediaSync() {
            if (this.state.mediaSyncActive) {
                this.disableMediaSync();
                this.showToast('Media sync disabled', 'info');
            } else {
                this.enableMediaSync();
                this.showToast('Media sync enabled - lights will pulse to the beat!', 'success');
            }
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
                try {
                    this.audioContext.suspend();
                } catch (error) {
                    console.warn('[LIFXMediaTouchV2] Audio context suspend failed:', error);
                }
            }
            if (this.analyser) {
                this.analyser.disconnect();
                this.analyser = null;
            }
            this.showToast('Media sync disabled', 'info');
        },

        startBeatDetection() {
            if (!navigator.mediaDevices || !navigator.mediaDevices.getUserMedia) {
                console.warn('[LIFXMediaTouchV2] Media devices not available');
                this.showToast('Beat detection not available on this device', 'warning');
                return;
            }
            
            navigator.mediaDevices.getUserMedia({ audio: true })
                .then(stream => {
                    try {
                        this.audioContext = new (window.AudioContext || window.webkitAudioContext)();
                        this.analyser = this.audioContext.createAnalyser();
                        const source = this.audioContext.createMediaStreamSource(stream);
                        source.connect(this.analyser);
                        this.analyser.fftSize = 256;
                        this.detectBeats();
                        this.showToast('Beat detection active', 'success');
                    } catch (error) {
                        console.error('[LIFXMediaTouchV2] Audio context error:', error);
                        this.showToast('Audio initialization failed', 'error');
                    }
                })
                .catch(err => {
                    console.error('[LIFXMediaTouchV2] Beat detection error:', err);
                    this.showToast(`Beat detection unavailable: ${err.message}`, 'warning');
                });
        },

        detectBeats() {
            if (!this.analyser || !this.state.mediaSyncActive) return;
            
            try {
                const dataArray = new Uint8Array(this.analyser.frequencyBinCount);
                this.analyser.getByteFrequencyData(dataArray);
                
                this.state.frequencyData = new Uint8Array(6);
                
                const bands = {
                    subBass: dataArray.slice(0, 4),
                    bass: dataArray.slice(4, 10),
                    lowMid: dataArray.slice(10, 20),
                    mid: dataArray.slice(20, 40),
                    highMid: dataArray.slice(40, 80),
                    treble: dataArray.slice(80, 128)
                };
                
                const bandAverages = {
                    subBass: bands.subBass.reduce((a, b) => a + b, 0) / bands.subBass.length,
                    bass: bands.bass.reduce((a, b) => a + b, 0) / bands.bass.length,
                    lowMid: bands.lowMid.reduce((a, b) => a + b, 0) / bands.lowMid.length,
                    mid: bands.mid.reduce((a, b) => a + b, 0) / bands.mid.length,
                    highMid: bands.highMid.reduce((a, b) => a + b, 0) / bands.highMid.length,
                    treble: bands.treble.reduce((a, b) => a + b, 0) / bands.treble.length
                };
                
                this.state.frequencyData[0] = bandAverages.subBass;
                this.state.frequencyData[1] = bandAverages.bass;
                this.state.frequencyData[2] = bandAverages.lowMid;
                this.state.frequencyData[3] = bandAverages.mid;
                this.state.frequencyData[4] = bandAverages.highMid;
                this.state.frequencyData[5] = bandAverages.treble;
                
                const bassEnergy = (bandAverages.subBass + bandAverages.bass) / 2;
                const totalEnergy = Object.values(bandAverages).reduce((a, b) => a + b, 0) / 6;
                
                this.state.beatHistory.push(bassEnergy);
                if (this.state.beatHistory.length > 30) {
                    this.state.beatHistory.shift();
                }
                
                const avgEnergy = this.state.beatHistory.reduce((a, b) => a + b, 0) / this.state.beatHistory.length;
                const variance = this.state.beatHistory.reduce((sum, val) => sum + Math.pow(val - avgEnergy, 2), 0) / this.state.beatHistory.length;
                const stdDev = Math.sqrt(variance);
                
                if (this.config.enableAdaptiveSensitivity && !this.state.sensitivityCalibrated) {
                    this.state.baselineEnergy = avgEnergy;
                    this.state.sensitivityCalibrated = true;
                }
                
                this.state.adaptiveThreshold = Math.max(0.5, Math.min(0.9, 
                    (avgEnergy / 255) + (stdDev / 50) + 0.15
                ));
                
                const beatThreshold = Math.max(
                    this.state.adaptiveThreshold,
                    this.config.beatDetectionThreshold
                );
                
                const isBeat = (bassEnergy / 255) > beatThreshold && bassEnergy > 160;
                
                if (isBeat && Date.now() - this.state.lastBeatTime > 180) {
                    const prevBeatTime = this.state.lastBeatTime;
                    this.state.lastBeatTime = Date.now();
                    
                    const interval = prevBeatTime ? (this.state.lastBeatTime - prevBeatTime) : 500;
                    const instantBPM = Math.round(60000 / interval);
                    
                    if (instantBPM > 60 && instantBPM < 200) {
                        this.state.bpmHistory.push(instantBPM);
                        if (this.state.bpmHistory.length > 8) {
                            this.state.bpmHistory.shift();
                        }
                        this.state.bpmSmoothed = Math.round(
                            this.state.bpmHistory.reduce((a, b) => a + b, 0) / this.state.bpmHistory.length
                        );
                    }
                    
                    this.state.bpmDetected = this.state.bpmSmoothed || instantBPM;
                    this.triggerBeatEffect(bandAverages);
                    this.updateFrequencyVisualization(bandAverages);
                }
                
                this.updateRealtimeBPM();
                requestAnimationFrame(this.detectBeats.bind(this));
            } catch (error) {
                console.error('[LIFXMediaTouchV2] Beat detection error:', error);
            }
        },

        triggerBeatEffect(bandAverages = {}) {
            if (!this.state.mediaSyncActive) return;
            
            try {
                const intensity = Math.min(1, (bandAverages.bass || 200) / 255);
                const duration = 60 + (1 - intensity) * 100;
                
                const effectConfig = {
                    selector: 'all',
                    brightness: Math.min(1, 0.6 + intensity * 0.4),
                    duration: duration / 1000
                };
                
                if (this.state.mediaSyncMode === 'color' || this.state.mediaSyncMode === 'spectrum') {
                    const hue = this.bpmToHue(this.state.bpmDetected);
                    effectConfig.color = `hsb(${hue},100,100)`;
                }
                
                fetch('/api/services/lifx/set_state', {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify(effectConfig)
                }).catch(err => {
                    console.warn('[LIFXMediaTouchV2] Beat effect failed:', err);
                });
                
                setTimeout(() => {
                    fetch('/api/services/lifx/set_state', {
                        method: 'POST',
                        headers: { 'Content-Type': 'application/json' },
                        body: JSON.stringify({
                            selector: 'all',
                            brightness: this.state.brightnessLevel / 100,
                            duration: 0.15
                        })
                    }).catch(err => {
                        console.warn('[LIFXMediaTouchV2] Beat recovery failed:', err);
                    });
                }, duration);
                
                this.showBeatFlashOverlay(intensity);
                
                if (this.config.enableHapticFeedback && navigator.vibrate && intensity > 0.6) {
                    const hapticPattern = intensity > 0.85 
                        ? [20, 15, 20, 15, 20]
                        : intensity > 0.7
                        ? [30, 20, 30]
                        : [25];
                    try {
                        navigator.vibrate(hapticPattern);
                    } catch (e) {
                        console.warn('[LIFXMediaTouchV2] Beat haptic failed:', e);
                    }
                }
            } catch (error) {
                console.error('[LIFXMediaTouchV2] Trigger beat effect error:', error);
            }
        },

        bpmToHue(bpm) {
            if (!bpm || bpm < 60) bpm = 60;
            if (bpm > 200) bpm = 200;
            return Math.round(((bpm - 60) / 140) * 360) % 360;
        },

        showBeatFlashOverlay(intensity) {
            let overlay = document.querySelector('.lifx-beat-flash-overlay');
            if (!overlay) {
                overlay = document.createElement('div');
                overlay.className = 'lifx-beat-flash-overlay';
                overlay.style.cssText = `
                    position: fixed;
                    top: 0;
                    left: 0;
                    right: 0;
                    bottom: 0;
                    background: radial-gradient(circle, rgba(0, 212, 255, ${0.15 * intensity}) 0%, transparent 70%);
                    pointer-events: none;
                    z-index: 99998;
                    opacity: 0;
                    transition: opacity 0.08s ease;
                `;
                document.body.appendChild(overlay);
            }
            
            overlay.style.opacity = intensity * 0.3;
            setTimeout(() => {
                overlay.style.opacity = 0;
            }, 80);
        },

        updateFrequencyVisualization(bandAverages) {
            const vizContainer = document.querySelector('.frequency-viz');
            if (!vizContainer) return;
            
            const bands = ['subBass', 'bass', 'lowMid', 'mid', 'highMid', 'treble'];
            const bandColors = {
                subBass: '#ff4545',
                bass: '#ff6b6b',
                lowMid: '#ffa500',
                mid: '#ffc93c',
                highMid: '#7fdbca',
                treble: '#00d4ff'
            };
            
            let peakDetected = false;
            let maxEnergy = 0;
            
            bands.forEach((band, index) => {
                const bar = document.getElementById(`band-${band.toLowerCase()}`);
                if (bar) {
                    const height = Math.max(5, (bandAverages[band] / 255) * 100);
                    const energy = bandAverages[band] / 255;
                    
                    if (energy > maxEnergy) {
                        maxEnergy = energy;
                    }
                    
                    bar.style.height = `${height}%`;
                    bar.style.background = bandColors[band];
                    
                    if (bandAverages[band] > 220) {
                        bar.classList.add('peak');
                        peakDetected = true;
                        setTimeout(() => bar.classList.remove('peak'), 100);
                    }
                    
                    const glowIntensity = Math.min(1, energy * 1.5);
                    bar.style.boxShadow = `0 0 ${10 + glowIntensity * 20}px ${bandColors[band]}`;
                }
            });
            
            if (peakDetected && maxEnergy > 0.85) {
                this.triggerBeatFlash(maxEnergy);
            }
        },
        
        triggerBeatFlash(energy) {
            const flash = document.createElement('div');
            flash.className = 'beat-flash';
            flash.style.cssText = `
                position: fixed;
                top: 0;
                left: 0;
                right: 0;
                bottom: 0;
                background: radial-gradient(circle, rgba(255, 107, 107, ${energy * 0.3}) 0%, transparent 70%);
                pointer-events: none;
                z-index: 9999;
                animation: beat-flash-anim 0.3s ease-out forwards;
            `;
            
            if (!document.getElementById('beat-flash-style')) {
                const style = document.createElement('style');
                style.id = 'beat-flash-style';
                style.textContent = `
                    @keyframes beat-flash-anim {
                        0% { opacity: 1; transform: scale(1); }
                        100% { opacity: 0; transform: scale(1.5); }
                    }
                `;
                document.head.appendChild(style);
            }
            
            document.body.appendChild(flash);
            setTimeout(() => {
                if (flash.parentNode) flash.parentNode.removeChild(flash);
            }, 300);
        },

        updateRealtimeBPM() {
            const bpmDisplay = document.querySelector('.bpm-value');
            const bpmIndicator = document.querySelector('.bpm-realtime-indicator');
            
            if (!bpmDisplay) return;
            
            const now = Date.now();
            const lastBeat = this.state.lastBeatTime || 0;
            
            if (now - lastBeat < 2000 && this.state.mediaSyncActive) {
                if (bpmIndicator) bpmIndicator.classList.add('visible');
                bpmDisplay.textContent = this.state.bpmDetected || '--';
                bpmDisplay.style.color = '#ff6b6b';
            } else {
                if (bpmIndicator) bpmIndicator.classList.remove('visible');
                bpmDisplay.textContent = '--';
                bpmDisplay.style.color = '#adb5bd';
            }
        },
    };

    window.LIFXMediaTouchV2 = LIFXMediaTouchV2;

    document.addEventListener('DOMContentLoaded', () => {
        LIFXMediaTouchV2.init();
    });
})();
