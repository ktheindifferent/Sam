// ███████     █████     ███    ███    
// ██         ██   ██    ████  ████    
// ███████    ███████    ██ ████ ██    
//      ██    ██   ██    ██  ██  ██    
// ███████ ██ ██   ██ ██ ██      ██ ██ 
// Copyright 2021-2026 The Open Sam Foundation (OSF)
// Developed by Caleb Mitchell Smith (ktheindifferent, PixelCoda, p0indexter)
// Licensed under GPLv3....see LICENSE file.

/**
 * Enhanced LIFX Touch Gesture Controls
 * Adds swipe and pinch gestures for intuitive lighting control
 */

const LifXTouchControls = {
    enabled: false,
    selectedBulb: null,
    brightnessLevel: 50,
    colorTempLevel: 3500,
    lastGestureTime: 0,
    gestureDebounce: 150,
    multiBulbSelection: [],
    touchHoldTimer: null,
    touchHoldDelay: 500,
    doubleTapDelay: 300,
    lastTapTime: 0,
    currentScene: 'relax',
    scenes: ['relax', 'focus', 'energize', 'night', 'sunset', 'ocean', 'reading', 'romance', 'party', 'golden', 'arctic', 'tropical', 'spring', 'autumn', 'meditation', 'gaming', 'cooking', 'creative', 'yoga', 'movie'],
    startY: null,
    startBrightness: null,
    startColorTemp: null,
    gestureStartTime: 0,
    lastSwipeDistance: 0,
    isTouchDevice: false,
    gestureHistory: [],
    maxGestureHistory: 10,
    gestureSensitivity: {
        swipeDistance: 50,
        swipeTime: 300,
        pinchDistance: 30
    },
    hapticEnabled: true,
    ambientLightSync: false,
    mediaPlaybackActive: false,
    
    enable: function(showTutorial = false) {
        if (this.enabled) return;
        
        this.enabled = true;
        this.isTouchDevice = typeof is_touch_enabled === 'function' && is_touch_enabled();
        this.loadGestureSensitivity();
        this.loadSavedPreferences();
        console.log('LIFX Touch Controls enabled', this.isTouchDevice ? '(Touch Device)' : '(Mouse/Keyboard)');
        
        // Add visual indicators for touch-controlled elements
        document.querySelectorAll('.lifx-bulb-control, .lifx-bulb-card').forEach(el => {
            el.classList.add('touch-feedback');
            el.setAttribute('data-lifx-touch', 'true');
            el.setAttribute('tabindex', '0');
        });
        
        // Show gesture tutorial on first enable
        if (showTutorial && !localStorage.getItem('lifxGestureTutorialShown')) {
            this.showGestureTutorial();
            localStorage.setItem('lifxGestureTutorialShown', 'true');
        }
        
        // Add touch mode indicator
        this.addTouchModeIndicator();
        
        // Initialize gesture history
        this.gestureHistory = [];
        
        // Register gesture callbacks with debouncing
        if (typeof onGesture === 'function') {
            // Swipe up/down on bulb card to adjust brightness
            onGesture('swipeUp', (data) => {
                if (!this.checkGestureDebounce()) return;
                const bulb = this.selectedBulb || this.getFirstSelectedBulb();
                if (bulb) {
                    this.adjustBrightness(10);
                    this.showGestureFeedback('Brightness +', '↑');
                    this.hapticFeedback('light');
                }
            });
            
            onGesture('swipeDown', (data) => {
                if (!this.checkGestureDebounce()) return;
                const bulb = this.selectedBulb || this.getFirstSelectedBulb();
                if (bulb) {
                    this.adjustBrightness(-10);
                    this.showGestureFeedback('Brightness -', '↓');
                    this.hapticFeedback('light');
                }
            });
            
            // Swipe left/right to adjust color temperature
            onGesture('swipeRight', (data) => {
                if (!this.checkGestureDebounce()) return;
                const bulb = this.selectedBulb || this.getFirstSelectedBulb();
                if (bulb) {
                    this.adjustColorTemp(200);
                    this.showGestureFeedback('Warmer', '☀️');
                    this.hapticFeedback('light');
                }
            });
            
            onGesture('swipeLeft', (data) => {
                if (!this.checkGestureDebounce()) return;
                const bulb = this.selectedBulb || this.getFirstSelectedBulb();
                if (bulb) {
                    this.adjustColorTemp(-200);
                    this.showGestureFeedback('Cooler', '❄️');
                    this.hapticFeedback('light');
                }
            });
            
            // Pinch to cycle through preset scenes
            onGesture('pinchOut', (data) => {
                if (!this.checkGestureDebounce()) return;
                const bulb = this.selectedBulb || this.getFirstSelectedBulb();
                if (bulb) {
                    this.nextScene();
                    this.showGestureFeedback('Next Scene', '🎨');
                    this.hapticFeedback('success');
                }
            });
            
            onGesture('pinchIn', (data) => {
                if (!this.checkGestureDebounce()) return;
                const bulb = this.selectedBulb || this.getFirstSelectedBulb();
                if (bulb) {
                    this.previousScene();
                    this.showGestureFeedback('Previous Scene', '🎨');
                    this.hapticFeedback('success');
                }
            });
            
            // Long press for quick settings
            onGesture('longPress', (data) => {
                const bulbEl = document.elementFromPoint(data.x, data.y)?.closest('.lifx-bulb-control');
                if (bulbEl) {
                    this.selectBulb(bulbEl.getAttribute('data-bulb-id'));
                    setTimeout(() => this.openQuickSettings(), 100);
                }
            });
        }
        
        // Add tap handlers for bulb selection with double-tap support
        document.addEventListener('click', (e) => {
            const bulbEl = e.target.closest('.lifx-bulb-control, .lifx-bulb-card');
            if (bulbEl) {
                const bulbId = bulbEl.getAttribute('data-bulb-id');
                const currentTime = Date.now();
                
                if (currentTime - this.lastTapTime < this.doubleTapDelay) {
                    this.lastTapTime = 0;
                    this.togglePower(bulbId);
                    this.showGestureFeedback('Power Toggle', '💡');
                    this.hapticFeedback('success');
                } else if (e.ctrlKey || e.metaKey) {
                    this.toggleBulbSelection(bulbId);
                    this.hapticFeedback('selection');
                    this.lastTapTime = currentTime;
                } else {
                    this.selectBulb(bulbId);
                    this.hapticFeedback('light');
                    this.lastTapTime = currentTime;
                }
            }
        });
        
        // Touch and hold for brightness adjustment
        document.addEventListener('touchstart', (e) => {
            const bulbEl = e.target.closest('.lifx-bulb-control, .lifx-bulb-card');
            if (bulbEl) {
                const bulbId = bulbEl.getAttribute('data-bulb-id');
                this.touchHoldTimer = setTimeout(() => {
                    this.startBrightnessAdjustment(bulbId, e.touches[0].clientY);
                }, this.touchHoldDelay);
            }
        }, { passive: true });
        
        document.addEventListener('touchmove', (e) => {
            if (this.touchHoldTimer && this.selectedBulb) {
                e.preventDefault();
                const touch = e.touches[0];
                this.adjustBrightnessByTouch(touch.clientY);
            }
        }, { passive: false });
        
        document.addEventListener('touchend', () => {
            if (this.touchHoldTimer) {
                clearTimeout(this.touchHoldTimer);
                this.touchHoldTimer = null;
            }
            if (this.selectedBulb) {
                this.endBrightnessAdjustment();
            }
        });
        
        // Keyboard accessibility
        document.addEventListener('keydown', (e) => {
            if (e.target.classList.contains('lifx-bulb-control')) {
                switch(e.key) {
                    case 'ArrowUp':
                        e.preventDefault();
                        this.adjustBrightness(10);
                        break;
                    case 'ArrowDown':
                        e.preventDefault();
                        this.adjustBrightness(-10);
                        break;
                    case 'ArrowLeft':
                        e.preventDefault();
                        this.adjustColorTemp(-200);
                        break;
                    case 'ArrowRight':
                        e.preventDefault();
                        this.adjustColorTemp(200);
                        break;
                    case ' ':
                    case 'Enter':
                        e.preventDefault();
                        this.togglePower();
                        break;
                }
            }
        });
    },
    
    checkGestureDebounce: function() {
        const now = Date.now();
        if (now - this.lastGestureTime < this.gestureDebounce) {
            return false;
        }
        this.lastGestureTime = now;
        return true;
    },
    
    getFirstSelectedBulb: function() {
        return this.multiBulbSelection.length > 0 
            ? this.multiBulbSelection[0] 
            : this.selectedBulb;
    },
    
    toggleBulbSelection: function(bulbId) {
        const index = this.multiBulbSelection.indexOf(bulbId);
        if (index > -1) {
            this.multiBulbSelection.splice(index, 1);
            const bulbEl = document.querySelector(`.lifx-bulb-control[data-bulb-id="${bulbId}"]`);
            if (bulbEl) bulbEl.classList.remove('multi-selected');
        } else {
            this.multiBulbSelection.push(bulbId);
            const bulbEl = document.querySelector(`.lifx-bulb-control[data-bulb-id="${bulbId}"]`);
            if (bulbEl) bulbEl.classList.add('multi-selected');
        }
        console.log('Multi-bulb selection:', this.multiBulbSelection);
    },
    
    addTouchModeIndicator: function() {
        if (document.querySelector('.lifx-touch-mode-indicator')) return;
        
        const indicator = document.createElement('div');
        indicator.className = 'lifx-touch-mode-indicator active';
        indicator.innerHTML = '<span>Touch Mode Active</span>';
        indicator.onclick = () => this.showGestureTutorial();
        document.body.appendChild(indicator);
        
        setTimeout(() => {
            if (indicator.parentNode) indicator.parentNode.removeChild(indicator);
        }, 5000);
    },
    
    showGestureFeedback: function(text, icon) {
        const hint = document.createElement('div');
        hint.className = 'lifx-gesture-hint visible';
        hint.innerHTML = `<span style="font-size: 24px; display: block; margin-bottom: 5px;">${icon}</span>${text}`;
        document.body.appendChild(hint);
        setTimeout(() => {
            hint.classList.remove('visible');
            setTimeout(() => {
                if (hint.parentNode) hint.parentNode.removeChild(hint);
            }, 300);
        }, 1000);
    },
    
    showGestureTutorial: function() {
        const tutorial = document.createElement('div');
        tutorial.className = 'lifx-gesture-tutorial active';
        tutorial.innerHTML = `
            <div class="lifx-gesture-tutorial-content">
                <h3>👆 LIFX Touch Gestures</h3>
                <div class="lifx-gesture-tutorial-item">
                    <div class="lifx-gesture-tutorial-icon">⬆️⬇️</div>
                    <div class="lifx-gesture-tutorial-text"><strong>Swipe Up/Down</strong><br>Adjust brightness</div>
                </div>
                <div class="lifx-gesture-tutorial-item">
                    <div class="lifx-gesture-tutorial-icon">➡️⬅️</div>
                    <div class="lifx-gesture-tutorial-text"><strong>Swipe Left/Right</strong><br>Adjust color temperature</div>
                </div>
                <div class="lifx-gesture-tutorial-item">
                    <div class="lifx-gesture-tutorial-icon">🤏</div>
                    <div class="lifx-gesture-tutorial-text"><strong>Pinch</strong><br>Cycle through scenes</div>
                </div>
                <div class="lifx-gesture-tutorial-item">
                    <div class="lifx-gesture-tutorial-icon">🖱️</div>
                    <div class="lifx-gesture-tutorial-text"><strong>Tap</strong><br>Select bulb<br><strong>Ctrl+Tap</strong> for multi-select</div>
                </div>
                <button class="lifx-gesture-tutorial-close" onclick="this.closest('.lifx-gesture-tutorial').remove()">Got It!</button>
            </div>
        `;
        document.body.appendChild(tutorial);
    },
    
    openQuickSettings: function() {
        if (typeof Swal !== 'undefined') {
            const bulbId = this.selectedBulb || this.getFirstSelectedBulb();
            Swal.fire({
                title: 'Quick Settings',
                html: `
                    <div style="padding: 20px;">
                        <label style="display: block; margin-bottom: 10px;">Brightness: <span id="brightnessValue">${this.brightnessLevel}%</span></label>
                        <input type="range" min="0" max="100" value="${this.brightnessLevel}" class="form-range" id="quickBrightness">
                        <label style="display: block; margin: 20px 0 10px;">Color Temp: <span id="kelvinValue">${this.colorTempLevel}K</span></label>
                        <input type="range" min="1500" max="9000" value="${this.colorTempLevel}" class="form-range" id="quickKelvin">
                    </div>
                `,
                showConfirmButton: true,
                confirmButtonText: 'Apply',
                didOpen: () => {
                    document.getElementById('quickBrightness').addEventListener('input', (e) => {
                        document.getElementById('brightnessValue').textContent = e.target.value + '%';
                    });
                    document.getElementById('quickKelvin').addEventListener('input', (e) => {
                        document.getElementById('kelvinValue').textContent = e.target.value + 'K';
                    });
                },
                confirmButtonColor: '#00d4ff',
            }).then((result) => {
                if (result.isConfirmed) {
                    const brightness = document.getElementById('quickBrightness').value;
                    const kelvin = document.getElementById('quickKelvin').value;
                    this.applyQuickSettings(bulbId, brightness, kelvin);
                }
            });
        }
    },
    
    applyQuickSettings: function(bulbId, brightness, kelvin) {
        const targets = this.multiBulbSelection.length > 0 
            ? this.multiBulbSelection 
            : (bulbId ? [bulbId] : (this.selectedBulb ? [this.selectedBulb] : []));
        
        if (targets.length === 0) return;
        
        this.brightnessLevel = parseInt(brightness);
        this.colorTempLevel = parseInt(kelvin);
        
        $.ajax({
            url: '/api/services/lifx/set_state',
            method: 'POST',
            contentType: 'application/json',
            data: JSON.stringify({
                selector: `id:${targets.join(',')}`,
                brightness: this.brightnessLevel,
                duration: 0.5
            }),
            success: () => {
                $.ajax({
                    url: '/api/services/lifx/set_color',
                    method: 'POST',
                    contentType: 'application/json',
                    data: JSON.stringify({
                        selector: `id:${targets.join(',')}`,
                        color: `kelvin:${this.colorTempLevel}`
                    }),
                    success: () => {
                        targets.forEach(id => this.updateBulbVisual(id));
                        showNotification(`Settings applied to ${targets.length} bulb(s)`, 'success');
                    }
                });
            }
        });
    },
    
    togglePower: function(bulbId) {
        const targets = this.multiBulbSelection.length > 0 
            ? this.multiBulbSelection 
            : (bulbId || this.selectedBulb || this.getFirstSelectedBulb());
        
        if (!targets || (Array.isArray(targets) && targets.length === 0)) return;
        
        const targetArray = Array.isArray(targets) ? targets : [targets];
        
        $.ajax({
            url: '/api/services/lifx/set_state',
            method: 'POST',
            contentType: 'application/json',
            data: JSON.stringify({
                selector: `id:${targetArray.join(',')}`,
                power: 'toggle',
                duration: 0.3
            }),
            success: () => {
                targetArray.forEach(bulbId => this.updateBulbVisual(bulbId));
                showNotification(`Power toggled for ${targetArray.length} bulb(s)`, 'info');
            }
        });
    },
    
    selectBulb: function(bulbId) {
        // Remove selection from previous bulb
        document.querySelectorAll('.lifx-bulb-control.selected').forEach(el => {
            el.classList.remove('selected');
        });
        
        // Select new bulb
        const bulbEl = document.querySelector(`.lifx-bulb-control[data-bulb-id="${bulbId}"]`);
        if (bulbEl) {
            bulbEl.classList.add('selected');
            this.selectedBulb = bulbId;
            console.log('Selected bulb:', bulbId);
        }
    },
    
    adjustBrightness: function(delta, smooth = true) {
        const targets = this.multiBulbSelection.length > 0 
            ? this.multiBulbSelection 
            : (this.selectedBulb ? [this.selectedBulb] : []);
        
        if (targets.length === 0) return;
        
        const newBrightness = Math.max(0, Math.min(100, this.brightnessLevel + delta));
        const duration = smooth && Math.abs(delta) < 20 ? 0.3 : 0.1;
        
        $.ajax({
            url: '/api/services/lifx/set_state',
            method: 'POST',
            contentType: 'application/json',
            data: JSON.stringify({
                selector: `id:${targets.join(',')}`,
                brightness: newBrightness,
                duration: duration
            }),
            success: (response) => {
                this.brightnessLevel = newBrightness;
                this.recordGesture('brightness', newBrightness);
                targets.forEach(bulbId => this.updateBulbVisual(bulbId));
            },
            error: (err) => {
                console.error('Failed to adjust brightness:', err);
            }
        });
    },
    
    adjustColorTemp: function(delta, smooth = true) {
        const targets = this.multiBulbSelection.length > 0 
            ? this.multiBulbSelection 
            : (this.selectedBulb ? [this.selectedBulb] : []);
        
        if (targets.length === 0) return;
        
        const newColorTemp = Math.max(1500, Math.min(9000, this.colorTempLevel + delta));
        const duration = smooth && Math.abs(delta) < 500 ? 0.3 : 0.1;
        
        $.ajax({
            url: '/api/services/lifx/set_color',
            method: 'POST',
            contentType: 'application/json',
            data: JSON.stringify({
                selector: `id:${targets.join(',')}`,
                color: `kelvin:${newColorTemp}`,
                duration: duration
            }),
            success: (response) => {
                this.colorTempLevel = newColorTemp;
                this.recordGesture('colorTemp', newColorTemp);
                targets.forEach(bulbId => this.updateBulbVisual(bulbId));
            },
            error: (err) => {
                console.error('Failed to adjust color temp:', err);
            }
        });
    },
    
    nextScene: function() {
        const currentIndex = this.scenes.indexOf(this.currentScene || 'relax');
        this.currentScene = this.scenes[(currentIndex + 1) % this.scenes.length];
        this.applyScene(this.currentScene);
    },
    
    previousScene: function() {
        const currentIndex = this.scenes.indexOf(this.currentScene || 'relax');
        this.currentScene = this.scenes[(currentIndex - 1 + this.scenes.length) % this.scenes.length];
        this.applyScene(this.currentScene);
    },
    
    applyScene: function(scene) {
        const targets = this.multiBulbSelection.length > 0 
            ? this.multiBulbSelection 
            : (this.selectedBulb ? [this.selectedBulb] : []);
        
        if (targets.length === 0) return;
        
        const sceneSettings = {
            relax: { brightness: 40, kelvin: 2700, label: 'Relax' },
            focus: { brightness: 80, kelvin: 5000, label: 'Focus' },
            energize: { brightness: 100, kelvin: 6500, label: 'Energize' },
            night: { brightness: 20, kelvin: 2000, label: 'Night' },
            sunset: { brightness: 30, kelvin: 2200, label: 'Sunset' },
            ocean: { brightness: 60, kelvin: 4500, label: 'Ocean' },
            reading: { brightness: 75, kelvin: 4500, label: 'Reading' },
            romance: { brightness: 50, kelvin: 3000, label: 'Romance' },
            party: { brightness: 100, kelvin: 5500, label: 'Party' },
            golden: { brightness: 70, kelvin: 3200, label: 'Golden' },
            arctic: { brightness: 80, kelvin: 7000, label: 'Arctic' },
            tropical: { brightness: 85, kelvin: 3800, label: 'Tropical' },
            spring: { brightness: 75, kelvin: 4200, label: 'Spring' },
            autumn: { brightness: 65, kelvin: 2800, label: 'Autumn' },
            meditation: { brightness: 35, kelvin: 2400, label: 'Meditation' },
            gaming: { brightness: 90, kelvin: 5500, label: 'Gaming' },
            cooking: { brightness: 95, kelvin: 4000, label: 'Cooking' },
            creative: { brightness: 85, kelvin: 4800, label: 'Creative' },
            yoga: { brightness: 50, kelvin: 3500, label: 'Yoga' },
            movie: { brightness: 35, kelvin: 2200, label: 'Movie' }
        };
        
        const settings = sceneSettings[scene];
        if (settings) {
            this.brightnessLevel = settings.brightness;
            this.colorTempLevel = settings.kelvin;
            
            const selector = targets.join(',');
            
            $.ajax({
                url: '/api/services/lifx/set_state',
                method: 'POST',
                contentType: 'application/json',
                data: JSON.stringify({
                    selector: `id:${selector}`,
                    brightness: settings.brightness,
                    duration: 0.5
                }),
                success: () => {
                    $.ajax({
                        url: '/api/services/lifx/set_color',
                        method: 'POST',
                        contentType: 'application/json',
                        data: JSON.stringify({
                            selector: `id:${selector}`,
                            color: `kelvin:${settings.kelvin}`
                        }),
                        success: () => {
                            console.log('Scene applied:', scene);
                            targets.forEach(bulbId => this.updateBulbVisual(bulbId));
                            this.showGestureFeedback(`Scene: ${settings.label}`, '🎨');
                        }
                    });
                }
            });
        }
    },
    
    updateBulbVisual: function(bulbId) {
        const bulbEl = document.querySelector(`.lifx-bulb-control[data-bulb-id="${bulbId}"]`);
        if (bulbEl) {
            // Update brightness indicator
            const brightnessIndicator = bulbEl.querySelector('.brightness-level');
            if (brightnessIndicator) {
                brightnessIndicator.textContent = `${this.brightnessLevel}%`;
            }
            
            // Update scene indicator
            let sceneIndicator = bulbEl.querySelector('.scene-indicator');
            if (!sceneIndicator && this.currentScene) {
                sceneIndicator = document.createElement('div');
                sceneIndicator.className = 'scene-indicator';
                bulbEl.appendChild(sceneIndicator);
            }
            if (sceneIndicator && this.currentScene) {
                const sceneLabel = this.scenes.find(s => s === this.currentScene);
                sceneIndicator.textContent = sceneLabel || '';
            }
            
            // Visual feedback
            bulbEl.classList.add('touch-updated');
            setTimeout(() => bulbEl.classList.remove('touch-updated'), 300);
        }
    },
    
    startBrightnessAdjustment: function(bulbId, startY) {
        this.selectBulb(bulbId);
        this.startY = startY;
        this.startBrightness = this.brightnessLevel;
        console.log('Starting brightness adjustment for', bulbId);
    },
    
    adjustBrightnessByTouch: function(currentY) {
        if (!this.selectedBulb || this.startY === undefined) return;
        
        const delta = this.startY - currentY;
        const brightnessDelta = Math.round((delta / 200) * 100);
        const newBrightness = Math.max(0, Math.min(100, this.startBrightness + brightnessDelta));
        
        if (newBrightness !== this.brightnessLevel) {
            this.brightnessLevel = newBrightness;
            this.showGestureFeedback(`${this.brightnessLevel}%`, '🔆');
            this.hapticFeedback('light');
        }
    },
    
    endBrightnessAdjustment: function() {
        const targets = this.multiBulbSelection.length > 0 
            ? this.multiBulbSelection 
            : (this.selectedBulb ? [this.selectedBulb] : []);
        
        if (targets.length === 0) return;
        
        $.ajax({
            url: '/api/services/lifx/set_state',
            method: 'POST',
            contentType: 'application/json',
            data: JSON.stringify({
                selector: `id:${targets.join(',')}`,
                brightness: this.brightnessLevel / 100,
                duration: 0.3
            }),
            success: () => {
                console.log('Brightness set to:', this.brightnessLevel);
                targets.forEach(bulbId => this.updateBulbVisual(bulbId));
            }
        });
        
        this.startY = undefined;
        this.startBrightness = undefined;
    },
    
    disable: function() {
        this.enabled = false;
        this.selectedBulb = null;
        document.querySelectorAll('.lifx-bulb-control').forEach(el => {
            el.classList.remove('touch-feedback');
            el.removeAttribute('data-lifx-touch');
        });
        console.log('LIFX Touch Controls disabled');
    },
    
    hapticFeedback: function(pattern = 'default') {
        if (!this.hapticEnabled || !navigator.vibrate) return;
        
        const patterns = {
            'default': [50],
            'light': [30],
            'success': [50, 50, 50],
            'error': [100, 50, 100],
            'selection': [40, 30, 40]
        };
        
        navigator.vibrate(patterns[pattern] || patterns['default']);
    },
    
    setGestureSensitivity: function(level) {
        const settings = {
            'low': { swipeDistance: 80, swipeTime: 400, pinchDistance: 50 },
            'medium': { swipeDistance: 50, swipeTime: 300, pinchDistance: 30 },
            'high': { swipeDistance: 30, swipeTime: 200, pinchDistance: 20 }
        };
        
        this.gestureSensitivity = settings[level] || settings['medium'];
        localStorage.setItem('lifx_gesture_sensitivity', level);
        console.log('Gesture sensitivity set to:', level);
    },
    
    loadGestureSensitivity: function() {
        const saved = localStorage.getItem('lifx_gesture_sensitivity') || 'medium';
        this.setGestureSensitivity(saved);
    },
    
    clearMultiSelection: function() {
        document.querySelectorAll('.lifx-bulb-control.multi-selected').forEach(el => {
            el.classList.remove('multi-selected');
        });
        this.multiBulbSelection = [];
        this.showGestureFeedback('Selection cleared', '✓');
    },
    
    selectAll: function() {
        const allBulbs = [];
        document.querySelectorAll('.lifx-bulb-control').forEach(el => {
            el.classList.add('multi-selected');
            const bulbId = el.getAttribute('data-bulb-id');
            if (bulbId) allBulbs.push(bulbId);
        });
        this.multiBulbSelection = allBulbs;
        this.showGestureFeedback(`Selected ${allBulbs.length} bulbs`, '💡');
    },
    
    powerAll: function(powerState) {
        const targets = this.multiBulbSelection.length > 0 
            ? this.multiBulbSelection 
            : 'all';
        
        $.ajax({
            url: '/api/services/lifx/set_state',
            method: 'POST',
            contentType: 'application/json',
            data: JSON.stringify({
                selector: targets === 'all' ? 'all' : `id:${targets.join(',')}`,
                power: powerState || 'toggle',
                duration: 0.3
            }),
            success: () => {
                showNotification(`Power ${powerState || 'toggled'} for all bulbs`, 'info');
            }
        });
    },
    
    cycleScene: function() {
        this.nextScene();
    },
    
    initMobileNav: function() {
        const nav = document.querySelector('.lifx-mobile-nav');
        if (!nav) return;
        
        nav.innerHTML = `
            <button class="lifx-mobile-nav-btn" onclick="LifXTouchControls.powerAll('on')" title="All On">
                <i class="fas fa-power-off"></i>
                <span>All On</span>
            </button>
            <button class="lifx-mobile-nav-btn" onclick="LifXTouchControls.powerAll('off')" title="All Off">
                <i class="fas fa-power-off"></i>
                <span>All Off</span>
            </button>
            <button class="lifx-mobile-nav-btn" onclick="LifXTouchControls.selectAll()" title="Select All">
                <i class="fas fa-layer-group"></i>
                <span>Select</span>
            </button>
            <button class="lifx-mobile-nav-btn" onclick="LifXTouchControls.cycleScene()" title="Next Scene">
                <i class="fas fa-palette"></i>
                <span>Scene</span>
            </button>
            <button class="lifx-mobile-nav-btn" onclick="LifXTouchControls.openQuickSettings()" title="Settings">
                <i class="fas fa-cog"></i>
                <span>Settings</span>
            </button>
        `;
    },
    
    recordGesture: function(type, value) {
        const record = {
            type: type,
            value: value,
            timestamp: Date.now()
        };
        
        this.gestureHistory.push(record);
        if (this.gestureHistory.length > this.maxGestureHistory) {
            this.gestureHistory.shift();
        }
    },
    
    undoLastGesture: function() {
        if (this.gestureHistory.length === 0) {
            this.showGestureFeedback('Nothing to undo', '↩️');
            return;
        }
        
        const lastGesture = this.gestureHistory.pop();
        this.savePreferences();
        const targets = this.multiBulbSelection.length > 0 
            ? this.multiBulbSelection 
            : (this.selectedBulb ? [this.selectedBulb] : []);
        
        if (targets.length === 0) return;
        
        if (lastGesture.type === 'brightness') {
            const previousValue = Math.max(0, Math.min(100, lastGesture.value - 10));
            this.brightnessLevel = previousValue;
            $.ajax({
                url: '/api/services/lifx/set_state',
                method: 'POST',
                contentType: 'application/json',
                data: JSON.stringify({
                    selector: `id:${targets.join(',')}`,
                    brightness: previousValue,
                    duration: 0.3
                }),
                success: () => {
                    targets.forEach(bulbId => this.updateBulbVisual(bulbId));
                    this.showGestureFeedback('Undo brightness', '↩️');
                }
            });
        } else if (lastGesture.type === 'colorTemp') {
            const previousValue = Math.max(1500, Math.min(9000, lastGesture.value - 200));
            this.colorTempLevel = previousValue;
            $.ajax({
                url: '/api/services/lifx/set_color',
                method: 'POST',
                contentType: 'application/json',
                data: JSON.stringify({
                    selector: `id:${targets.join(',')}`,
                    color: `kelvin:${previousValue}`,
                    duration: 0.3
                }),
                success: () => {
                    targets.forEach(bulbId => this.updateBulbVisual(bulbId));
                    this.showGestureFeedback('Undo color', '↩️');
                }
            });
        }
    },
    
    presetBrightness: function(level) {
        const targets = this.multiBulbSelection.length > 0 
            ? this.multiBulbSelection 
            : (this.selectedBulb ? [this.selectedBulb] : []);
        
        if (targets.length === 0) return;
        
        this.brightnessLevel = level;
        
        $.ajax({
            url: '/api/services/lifx/set_state',
            method: 'POST',
            contentType: 'application/json',
            data: JSON.stringify({
                selector: `id:${targets.join(',')}`,
                brightness: level,
                duration: 0.5
            }),
            success: () => {
                targets.forEach(bulbId => this.updateBulbVisual(bulbId));
                this.showGestureFeedback(`${level}% brightness`, '💡');
            }
        });
    },
    
    rampBrightness: function(direction) {
        const rampInterval = setInterval(() => {
            if (!this.selectedBulb && this.multiBulbSelection.length === 0) {
                clearInterval(rampInterval);
                return;
            }
            
            const delta = direction === 'up' ? 5 : -5;
            const newBrightness = this.brightnessLevel + delta;
            
            if (newBrightness <= 0 || newBrightness >= 100) {
                clearInterval(rampInterval);
                return;
            }
            
            this.adjustBrightness(delta, false);
        }, 100);
        
        setTimeout(() => clearInterval(rampInterval), 2000);
    },
    
    openQuickSettings: function() {
        if (typeof Swal === 'undefined') {
            alert('Quick Settings: Brightness ' + this.brightnessLevel + '%, Color Temp ' + this.colorTempLevel + 'K');
            return;
        }
        
        Swal.fire({
            title: 'Quick Settings',
            html: `
                <div class="quick-settings-container">
                    <div class="setting-group">
                        <label>Brightness</label>
                        <input type="range" id="quick-brightness" min="0" max="100" value="${this.brightnessLevel}" 
                               oninput="LifXTouchControls.presetBrightness(this.value)" 
                               style="width: 100%;">
                        <span id="brightness-value">${this.brightnessLevel}%</span>
                    </div>
                    <div class="setting-group">
                        <label>Color Temperature</label>
                        <input type="range" id="quick-colortemp" min="1500" max="9000" value="${this.colorTempLevel}" 
                               oninput="LifXTouchControls.adjustColorTemp(this.value - ${this.colorTempLevel}, false); this.nextElementSibling.textContent = this.value + 'K';" 
                               style="width: 100%;">
                        <span>${this.colorTempLevel}K</span>
                    </div>
                    <div class="setting-group">
                        <label>Quick Scenes</label>
                        <div class="scene-preview-grid">
                            ${this.scenes.slice(0, 6).map(scene => `
                                <div class="scene-preview-item" onclick="LifXTouchControls.applyScene('${scene}')" 
                                     style="background: ${this.getSceneColor(scene)};">
                                    <span class="scene-preview-label">${scene}</span>
                                </div>
                            `).join('')}
                        </div>
                    </div>
                    <div class="setting-group">
                        <label>Presets</label>
                        <div style="display: flex; gap: 8px; flex-wrap: wrap;">
                            <button class="btn btn-sm btn-outline-primary" onclick="LifXTouchControls.presetBrightness(25)">25%</button>
                            <button class="btn btn-sm btn-outline-primary" onclick="LifXTouchControls.presetBrightness(50)">50%</button>
                            <button class="btn btn-sm btn-outline-primary" onclick="LifXTouchControls.presetBrightness(75)">75%</button>
                            <button class="btn btn-sm btn-outline-primary" onclick="LifXTouchControls.presetBrightness(100)">100%</button>
                        </div>
                    </div>
                </div>
            `,
            showConfirmButton: false,
            showCloseButton: true,
            width: '600px'
        });
    },
    
    getSceneColor: function(sceneName) {
        const sceneColors = {
            'relax': '#ff6b6b',
            'focus': '#4ecdc4',
            'energize': '#ffe66d',
            'night': '#1a1a2e',
            'sunset': '#ff9f43',
            'ocean': '#45b7d1',
            'reading': '#feca57',
            'romance': '#ff9ff3',
            'party': '#00d4ff',
            'golden': '#f9ca24',
            'arctic': '#70a1ff',
            'tropical': '#00b894',
            'spring': '#55efc4',
            'autumn': '#e17055',
            'meditation': '#9b59b6',
            'gaming': '#e91e63',
            'cooking': '#f39c12',
            'creative': '#8e44ad',
            'yoga': '#27ae60',
            'movie': '#d35400'
        };
        return sceneColors[sceneName] || '#ffffff';
    },
    
    applyScene: function(sceneName) {
        const sceneColors = {
            'relax': { hue: 0, saturation: 50, brightness: 60, kelvin: 2700 },
            'focus': { hue: 160, saturation: 60, brightness: 80, kelvin: 5000 },
            'energize': { hue: 50, saturation: 80, brightness: 100, kelvin: 6500 },
            'night': { hue: 240, saturation: 20, brightness: 20, kelvin: 2000 },
            'sunset': { hue: 30, saturation: 70, brightness: 70, kelvin: 2500 },
            'ocean': { hue: 190, saturation: 65, brightness: 75, kelvin: 4000 },
            'reading': { hue: 45, saturation: 40, brightness: 80, kelvin: 3500 },
            'romance': { hue: 320, saturation: 50, brightness: 60, kelvin: 2700 },
            'party': { hue: 180, saturation: 100, brightness: 100, kelvin: 6000 },
            'golden': { hue: 45, saturation: 85, brightness: 90, kelvin: 3000 },
            'arctic': { hue: 210, saturation: 55, brightness: 85, kelvin: 7000 },
            'tropical': { hue: 150, saturation: 100, brightness: 72, kelvin: 3800 },
            'spring': { hue: 140, saturation: 76, brightness: 93, kelvin: 4200 },
            'autumn': { hue: 30, saturation: 66, brightness: 88, kelvin: 2800 },
            'meditation': { hue: 280, saturation: 30, brightness: 40, kelvin: 2400 },
            'gaming': { hue: 280, saturation: 80, brightness: 90, kelvin: 5500 },
            'cooking': { hue: 35, saturation: 60, brightness: 95, kelvin: 4000 },
            'creative': { hue: 290, saturation: 70, brightness: 85, kelvin: 4800 },
            'yoga': { hue: 120, saturation: 40, brightness: 50, kelvin: 3500 },
            'movie': { hue: 20, saturation: 30, brightness: 35, kelvin: 2200 }
        };
        
        const scene = sceneColors[sceneName];
        if (!scene) return;
        
        const targets = this.multiBulbSelection.length > 0 
            ? this.multiBulbSelection 
            : (this.selectedBulb ? [this.selectedBulb] : ['all']);
        
        $.ajax({
            url: '/api/services/lifx/set_color',
            method: 'POST',
            contentType: 'application/json',
            data: JSON.stringify({
                selector: `id:${targets.join(',')}`,
                color: `hue:${scene.hue * 182} saturation:${scene.saturation}%`,
                brightness: scene.brightness,
                kelvin: scene.kelvin,
                duration: 0.5
            }),
            success: () => {
                this.showGestureFeedback(`Applied ${sceneName} scene`, '🎨');
                if (typeof Swal !== 'undefined') Swal.close();
            }
        });
    },
    
    updateSelectionToolbar: function() {
        const toolbar = document.getElementById('lifx-selection-toolbar');
        const countEl = document.getElementById('lifx-selection-count');
        
        if (!toolbar || !countEl) return;
        
        if (this.multiBulbSelection.length > 0) {
            countEl.textContent = `${this.multiBulbSelection.length} selected`;
            toolbar.classList.add('visible');
        } else {
            toolbar.classList.remove('visible');
        }
    },
    
    updateUndoButton: function() {
        const undoBtn = document.getElementById('lifx-undo-btn');
        if (!undoBtn) return;
        
        if (this.gestureHistory.length > 0) {
            undoBtn.classList.add('visible');
        } else {
            undoBtn.classList.remove('visible');
        }
    },
    
    savePreferences: function() {
        const prefs = {
            brightnessLevel: this.brightnessLevel,
            colorTempLevel: this.colorTempLevel,
            currentScene: this.currentScene,
            ambientLightSync: this.ambientLightSync,
            hapticEnabled: this.hapticEnabled
        };
        localStorage.setItem('lifx_preferences', JSON.stringify(prefs));
    },
    
    loadSavedPreferences: function() {
        const saved = localStorage.getItem('lifx_preferences');
        if (saved) {
            try {
                const prefs = JSON.parse(saved);
                if (prefs.brightnessLevel) this.brightnessLevel = prefs.brightnessLevel;
                if (prefs.colorTempLevel) this.colorTempLevel = prefs.colorTempLevel;
                if (prefs.currentScene) this.currentScene = prefs.currentScene;
                if (prefs.ambientLightSync !== undefined) this.ambientLightSync = prefs.ambientLightSync;
                if (prefs.hapticEnabled !== undefined) this.hapticEnabled = prefs.hapticEnabled;
            } catch (e) {
                console.error('Failed to load LIFX preferences:', e);
            }
        }
    },
    
    toggleAmbientLightSync: function() {
        this.ambientLightSync = !this.ambientLightSync;
        this.savePreferences();
        
        if (this.ambientLightSync) {
            this.startAmbientSync();
            this.showGestureFeedback('Ambient Sync ON', '🌈');
        } else {
            this.stopAmbientSync();
            this.showGestureFeedback('Ambient Sync OFF', '⬜');
        }
    },
    
    startAmbientSync: function() {
        if (!this.mediaPlaybackActive) return;
        
        const syncColors = () => {
            if (!this.ambientLightSync) return;
            
            const video = document.querySelector('video');
            if (video && !video.paused) {
                const canvas = document.createElement('canvas');
                canvas.width = 1;
                canvas.height = 1;
                const ctx = canvas.getContext('2d');
                ctx.drawImage(video, 0, 0, 1, 1);
                const pixel = ctx.getImageData(0, 0, 1, 1).data;
                
                const rgb = { r: pixel[0], g: pixel[1], b: pixel[2] };
                const hsv = this.rgbToHsv(rgb.r, rgb.g, rgb.b);
                
                const targets = this.multiBulbSelection.length > 0 
                    ? this.multiBulbSelection 
                    : (this.selectedBulb ? [this.selectedBulb] : ['all']);
                
                $.ajax({
                    url: '/api/services/lifx/set_color',
                    method: 'POST',
                    contentType: 'application/json',
                    data: JSON.stringify({
                        selector: `id:${targets.join(',')}`,
                        color: `hue:${Math.round(hsv.h * 182)} saturation:${Math.round(hsv.s * 100)}%`,
                        brightness: Math.round(hsv.v * 100),
                        duration: 0.2
                    })
                });
            }
            
            setTimeout(syncColors, 500);
        };
        
        syncColors();
    },
    
    stopAmbientSync: function() {
        this.ambientLightSync = false;
    },
    
    rgbToHsv: function(r, g, b) {
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
    },
    
    setMediaPlaybackActive: function(active) {
        this.mediaPlaybackActive = active;
        if (active && this.ambientLightSync) {
            this.startAmbientSync();
        }
    },
    
    showColorPicker: function() {
        if (typeof Swal === 'undefined') {
            alert('Color picker requires SweetAlert2');
            return;
        }
        
        const targets = this.multiBulbSelection.length > 0 
            ? this.multiBulbSelection 
            : (this.selectedBulb ? [this.selectedBulb] : ['all']);
        
        Swal.fire({
            title: 'Color Picker',
            html: `
                <div style="padding: 20px;">
                    <input type="color" id="color-picker-input" value="${this.hsvToRgb(this.colorHue || 0, 1, 1)}" 
                           style="width: 200px; height: 200px; cursor: pointer; border: none;">
                    <div style="margin-top: 20px; display: flex; gap: 10px; justify-content: center;">
                        <button class="btn btn-sm btn-outline-primary" onclick="LifXTouchControls.applyPickedColor('${targets.join(',')}')">Apply</button>
                        <button class="btn btn-sm btn-outline-secondary" onclick="LifXTouchControls.cycleThroughColors('${targets.join(',')}')">Cycle Colors</button>
                    </div>
                    <div style="margin-top: 15px; display: flex; gap: 5px; justify-content: center; flex-wrap: wrap;">
                        ${['#ff0000', '#00ff00', '#0000ff', '#ffff00', '#00ffff', '#ff00ff', '#ff8000', '#8000ff'].map(color => 
                            `<button class="btn btn-sm" style="background: ${color}; width: 30px; height: 30px; border-radius: 50%; padding: 0;" 
                                     onclick="LifXTouchControls.applyQuickColor('${color}', '${targets.join(',')}')"></button>`
                        ).join('')}
                    </div>
                </div>
            `,
            showConfirmButton: false,
            showCloseButton: true,
            width: '400px'
        });
    },
    
    hsvToRgb: function(h, s, v) {
        let r, g, b;
        const i = Math.floor(h * 6);
        const f = h * 6 - i;
        const p = v * (1 - s);
        const q = v * (1 - f * s);
        const t = v * (1 - (1 - f) * s);
        
        switch (i % 6) {
            case 0: r = v; g = t; b = p; break;
            case 1: r = q; g = v; b = p; break;
            case 2: r = p; g = v; b = t; break;
            case 3: r = p; g = q; b = v; break;
            case 4: r = t; g = p; b = v; break;
            case 5: r = v; g = p; b = q; break;
        }
        
        const toHex = c => {
            const hex = Math.round(c * 255).toString(16);
            return hex.length === 1 ? '0' + hex : hex;
        };
        
        return `#${toHex(r)}${toHex(g)}${toHex(b)}`;
    },
    
    applyPickedColor: function(targetString) {
        const colorInput = document.getElementById('color-picker-input');
        if (!colorInput) return;
        
        const hex = colorInput.value;
        this.applyColorFromHex(hex, targetString);
        if (typeof Swal !== 'undefined') Swal.close();
    },
    
    applyQuickColor: function(hex, targetString) {
        this.applyColorFromHex(hex, targetString);
        if (typeof Swal !== 'undefined') Swal.close();
    },
    
    applyColorFromHex: function(hex, targetString) {
        const targets = targetString.split(',');
        const rgb = this.hexToRgb(hex);
        const hsv = this.rgbToHsv(rgb.r, rgb.g, rgb.b);
        
        $.ajax({
            url: '/api/services/lifx/set_color',
            method: 'POST',
            contentType: 'application/json',
            data: JSON.stringify({
                selector: `id:${targets.join(',')}`,
                color: `hue:${Math.round(hsv.h * 182)} saturation:${Math.round(hsv.s * 100)}%`,
                duration: 0.5
            }),
            success: () => {
                this.showGestureFeedback('Color applied', '🎨');
                targets.forEach(bulbId => this.updateBulbVisual(bulbId));
            }
        });
    },
    
    hexToRgb: function(hex) {
        const result = /^#?([a-f\d]{2})([a-f\d]{2})([a-f\d]{2})$/i.exec(hex);
        return result ? {
            r: parseInt(result[1], 16),
            g: parseInt(result[2], 16),
            b: parseInt(result[3], 16)
        } : { r: 255, g: 255, b: 255 };
    },
    
    cycleThroughColors: function(targetString) {
        const targets = targetString.split(',');
        let hue = 0;
        const cycleInterval = setInterval(() => {
            if (!this.colorCycleActive) {
                clearInterval(cycleInterval);
                return;
            }
            
            $.ajax({
                url: '/api/services/lifx/set_color',
                method: 'POST',
                contentType: 'application/json',
                data: JSON.stringify({
                    selector: `id:${targets.join(',')}`,
                    color: `hue:${hue * 182} saturation:100%`,
                    duration: 0.3
                })
            });
            
            hue = (hue + 0.02) % 1;
        }, 100);
        
        this.colorCycleActive = true;
        this.showGestureFeedback('Color cycle started', '🌈');
        
        setTimeout(() => {
            this.colorCycleActive = false;
        }, 10000);
    },
    
    startColorCycle: function() {
        this.colorCycleActive = true;
        this.showGestureFeedback('Color cycle ON', '🌈');
    },
    
    stopColorCycle: function() {
        this.colorCycleActive = false;
        this.showGestureFeedback('Color cycle OFF', '⬜');
    }
};

// Initialize when DOM is ready
document.addEventListener('DOMContentLoaded', function() {
    // Auto-enable if touch device detected
    if (typeof is_touch_enabled === 'function' && is_touch_enabled()) {
        LifXTouchControls.enable();
        console.log('Touch device detected - LIFX touch controls auto-enabled');
    }
    
    // Initialize mobile navigation
    LifXTouchControls.initMobileNav();
});

// Export for external use
if (typeof module !== 'undefined' && module.exports) {
    module.exports = LifXTouchControls;
}
