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
    scenes: ['relax', 'focus', 'energize', 'night', 'sunset', 'ocean', 'reading', 'romance', 'party', 'golden', 'arctic'],
    startY: null,
    startBrightness: null,
    startColorTemp: null,
    gestureStartTime: 0,
    lastSwipeDistance: 0,
    isTouchDevice: false,
    gestureHistory: [],
    maxGestureHistory: 10,
    
    enable: function(showTutorial = false) {
        if (this.enabled) return;
        
        this.enabled = true;
        this.isTouchDevice = typeof is_touch_enabled === 'function' && is_touch_enabled();
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
                }
            });
            
            onGesture('swipeDown', (data) => {
                if (!this.checkGestureDebounce()) return;
                const bulb = this.selectedBulb || this.getFirstSelectedBulb();
                if (bulb) {
                    this.adjustBrightness(-10);
                    this.showGestureFeedback('Brightness -', '↓');
                }
            });
            
            // Swipe left/right to adjust color temperature
            onGesture('swipeRight', (data) => {
                if (!this.checkGestureDebounce()) return;
                const bulb = this.selectedBulb || this.getFirstSelectedBulb();
                if (bulb) {
                    this.adjustColorTemp(200);
                    this.showGestureFeedback('Warmer', '☀️');
                }
            });
            
            onGesture('swipeLeft', (data) => {
                if (!this.checkGestureDebounce()) return;
                const bulb = this.selectedBulb || this.getFirstSelectedBulb();
                if (bulb) {
                    this.adjustColorTemp(-200);
                    this.showGestureFeedback('Cooler', '❄️');
                }
            });
            
            // Pinch to cycle through preset scenes
            onGesture('pinchOut', (data) => {
                if (!this.checkGestureDebounce()) return;
                const bulb = this.selectedBulb || this.getFirstSelectedBulb();
                if (bulb) {
                    this.nextScene();
                    this.showGestureFeedback('Next Scene', '🎨');
                }
            });
            
            onGesture('pinchIn', (data) => {
                if (!this.checkGestureDebounce()) return;
                const bulb = this.selectedBulb || this.getFirstSelectedBulb();
                if (bulb) {
                    this.previousScene();
                    this.showGestureFeedback('Previous Scene', '🎨');
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
                } else if (e.ctrlKey || e.metaKey) {
                    this.toggleBulbSelection(bulbId);
                    this.lastTapTime = currentTime;
                } else {
                    this.selectBulb(bulbId);
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
            arctic: { brightness: 80, kelvin: 7000, label: 'Arctic' }
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
        this.brightnessLevel = Math.max(0, Math.min(100, this.startBrightness + brightnessDelta));
        
        // Show live brightness feedback
        this.showGestureFeedback(`${this.brightnessLevel}%`, '🔆');
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
            'arctic': '#70a1ff'
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
            'arctic': { hue: 210, saturation: 55, brightness: 85, kelvin: 7000 }
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
