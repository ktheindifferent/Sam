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
    scenes: ['relax', 'focus', 'energize', 'night', 'sunset', 'ocean', 'reading', 'romance', 'party', 'golden', 'arctic', 'tropical', 'spring', 'autumn', 'meditation', 'gaming', 'cooking', 'creative', 'yoga', 'movie', 'study', 'dinner', 'morning', 'goodnight', 'rainbow', 'fireplace', 'ice', 'aurora', 'nebula', 'thunder', 'crystal', 'lagoon', 'cotton_candy', 'spring_blossom', 'punchbowl', 'smashing', 'glitter', 'golden_hour', 'late_night', 'midday', 'polar', 'cosmic', 'dream', 'chill', 'adventure', 'festival'],
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
        pinchDistance: 30,
        longPressDelay: 500,
        doubleTapDelay: 300
    },
    hapticEnabled: true,
    ambientLightSync: false,
    mediaPlaybackActive: false,
    colorCycleActive: false,
    colorHue: 0,
    touchHoldProgressEl: null,
    lastGestureHint: null,
    touchRippleEnabled: true,
    showGestureHints: true,
    gestureHintDuration: 1000,
    enhancedRippleMode: false,
    rippleColor: 'rgba(0, 212, 255, 0.6)',
    rippleSize: 50,
    rippleDuration: 600,
    glowEffectEnabled: true,
    voiceControlEnabled: false,
    zonePresets: {},
    effectQueue: [],
    effectActive: false,
    circadianRhythmEnabled: false,
    lastCircadianAdjustment: 0,
    touchGestureTrail: [],
    maxTrailLength: 5,
    accessibilityMode: false,
    highContrastHints: false,
    reducedMotionMode: false,
    mediaSyncMode: 'beat',
    beatDetectionSensitivity: 0.7,
    bpmValue: 0,
    lastBeatTime: 0,
    beatDebounce: 100,
    audioAnalyzer: null,
    audioContext: null,
    mediaSyncTargets: [],
    touchSensitivity: 'medium',
    swipeEdgeZone: 20,
    isEdgeSwipe: false,
    edgeSwipeDirection: null,
    
    enable: function(showTutorial = false) {
        if (this.enabled) return;
        
        this.enabled = true;
        this.isTouchDevice = typeof is_touch_enabled === 'function' && is_touch_enabled();
        this.loadGestureSensitivity();
        this.loadSavedPreferences();
        this.initGestureEnhancements();
        this.initMediaSync();
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
        
        // Add touch ripple effect handler
        if (this.touchRippleEnabled) {
            this.initTouchRipple();
        }
        
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
        
        // Touch and hold for brightness adjustment with visual progress
        document.addEventListener('touchstart', (e) => {
            const bulbEl = e.target.closest('.lifx-bulb-control, .lifx-bulb-card');
            if (bulbEl) {
                const bulbId = bulbEl.getAttribute('data-bulb-id');
                this.showTouchHoldProgress();
                this.touchHoldTimer = setTimeout(() => {
                    this.startBrightnessAdjustment(bulbId, e.touches[0].clientY);
                    this.hideTouchHoldProgress();
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
            this.hideTouchHoldProgress();
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
        this.updateSelectionToolbar();
        console.log('Multi-bulb selection:', this.multiBulbSelection);
    },
    
    setupMultiSelectMode: function() {
        this.isMultiSelectMode = false;
        this.isDragSelecting = false;
        this.dragSelectionState = false;
        const multiSelectBtn = document.getElementById('lifx-multi-select-btn');
        if (multiSelectBtn) {
            multiSelectBtn.addEventListener('click', () => {
                this.isMultiSelectMode = !this.isMultiSelectMode;
                multiSelectBtn.classList.toggle('active', this.isMultiSelectMode);
                document.querySelectorAll('.lifx-bulb-control').forEach(el => {
                    el.classList.toggle('multi-selectable', this.isMultiSelectMode);
                });
                this.showGestureFeedback(
                    this.isMultiSelectMode ? 'Multi-Select ON' : 'Multi-Select OFF',
                    this.isMultiSelectMode ? '✓' : '✗'
                );
            });
        }
        
        document.addEventListener('touchstart', (e) => {
            const bulbEl = e.target.closest('.lifx-bulb-control');
            if (bulbEl && this.isMultiSelectMode) {
                this.touchHoldTimer = setTimeout(() => {
                    this.isDragSelecting = true;
                    this.dragSelectionState = !bulbEl.classList.contains('selected');
                    this.hapticFeedback('light');
                }, this.touchHoldDelay);
            }
        }, { passive: true });
        
        document.addEventListener('touchmove', (e) => {
            if (!this.isDragSelecting) return;
            e.preventDefault();
            const touch = e.touches[0];
            const elements = document.elementsFromPoint(touch.clientX, touch.clientY);
            elements.forEach(el => {
                const bulbEl = el.closest('.lifx-bulb-control');
                if (bulbEl && !this.multiBulbSelection.includes(bulbEl.dataset.bulbId)) {
                    this.toggleBulbSelection(bulbEl, this.dragSelectionState);
                }
            });
        }, { passive: false });
        
        document.addEventListener('touchend', () => {
            if (this.touchHoldTimer) clearTimeout(this.touchHoldTimer);
            this.isDragSelecting = false;
        });
    },
    
    toggleBulbSelection: function(bulbEl, forceSelect = null) {
        const bulbId = bulbEl.dataset.bulbId;
        if (!bulbId) return;
        
        const index = this.multiBulbSelection.indexOf(bulbId);
        const shouldSelect = forceSelect !== null ? forceSelect : (index === -1);
        
        if (shouldSelect && index === -1) {
            this.multiBulbSelection.push(bulbId);
            bulbEl.classList.add('selected');
            this.hapticFeedback('light');
        } else if (!shouldSelect && index > -1) {
            this.multiBulbSelection.splice(index, 1);
            bulbEl.classList.remove('selected');
        }
        
        this.updateSelectionToolbar();
    },
    
    updateSelectionToolbar: function() {
        const toolbar = document.getElementById('lifx-selection-toolbar');
        const countEl = document.getElementById('lifx-selection-count');
        if (toolbar && countEl) {
            countEl.textContent = `${this.multiBulbSelection.length} selected`;
            toolbar.style.display = this.multiBulbSelection.length > 0 ? 'flex' : 'none';
        }
    },
    
    powerAllSelected: function(state) {
        if (this.multiBulbSelection.length === 0) return;
        
        $.ajax({
            url: '/api/services/lifx/set_state',
            method: 'POST',
            contentType: 'application/json',
            data: JSON.stringify({
                selector: `id:${this.multiBulbSelection.join(',')}`,
                power: state,
                duration: 0.5
            }),
            success: () => {
                this.showGestureFeedback(
                    `Power ${state} ${this.multiBulbSelection.length} bulbs`,
                    state === 'on' ? '💡' : '🌑'
                );
                this.multiBulbSelection = [];
                this.updateSelectionToolbar();
            }
        });
    },
    
    setupTouchHoldProgress: function() {
        this.touchHoldProgressEl = document.getElementById('touch-hold-progress');
        this.touchHoldStartTime = 0;
        this.touchHoldAnimation = null;
        
        document.addEventListener('touchstart', (e) => {
            const bulbEl = e.target.closest('.lifx-bulb-control');
            if (bulbEl && this.touchHoldProgressEl) {
                this.touchHoldStartTime = Date.now();
                this.touchHoldProgressEl.classList.add('visible');
            }
        }, { passive: true });
        
        document.addEventListener('touchend', () => {
            if (this.touchHoldProgressEl) {
                this.touchHoldProgressEl.classList.remove('visible');
            }
        });
    },
    
    setupQuickActions: function() {
        const quickActions = document.querySelectorAll('.lifx-quick-action-btn');
        quickActions.forEach(btn => {
            btn.addEventListener('click', (e) => {
                const action = btn.dataset.action;
                if (action) {
                    this.executeQuickAction(action);
                    btn.classList.toggle('active');
                }
            });
        });
    },
    
    executeQuickAction: function(action) {
        const selector = this.multiBulbSelection.length > 0 
            ? `id:${this.multiBulbSelection.join(',')}` 
            : 'all';
        
        const actions = {
            'party': () => this.applyScene('party', selector),
            'relax': () => this.applyScene('relax', selector),
            'focus': () => this.applyScene('focus', selector),
            'night': () => this.applyScene('night', selector),
            'rainbow': () => this.startRainbowCycle(selector),
            'pulse': () => this.startPulseEffect(selector),
            'breath': () => this.startBreathEffect(selector),
            'fireplace': () => this.startFireplaceEffect(selector),
            'aurora': () => this.startAuroraEffect(selector),
            'ocean': () => this.applyScene('ocean', selector),
            'golden_hour': () => this.applyScene('golden_hour', selector),
            'meditation': () => this.applyScene('meditation', selector),
            'movie': () => this.applyScene('movie', selector)
        };
        
        if (actions[action]) {
            actions[action]();
            this.recordGesture('quick_action', { action });
        }
    },
    
    applyScene: function(sceneName, selector = 'all') {
        $.ajax({
            url: '/api/services/lifx/apply_scene',
            method: 'POST',
            contentType: 'application/json',
            data: JSON.stringify({ scene: sceneName, selector, duration: 0.5 }),
            success: () => {
                this.currentScene = sceneName;
                this.showSceneIndicator(sceneName);
                this.showGestureFeedback(`Scene: ${sceneName}`, '🎨');
            }
        });
    },
    
    showSceneIndicator: function(sceneName) {
        let existing = document.querySelector('.scene-indicator');
        if (existing) existing.remove();
        
        const indicator = document.createElement('div');
        indicator.className = `scene-indicator ${sceneName}`;
        indicator.innerHTML = `<i class="fas fa-palette"></i> ${sceneName}`;
        indicator.style.cssText = 'position: fixed; top: 20px; right: 20px; padding: 8px 16px; border-radius: 20px; background: rgba(0, 212, 255, 0.2); color: #00d4ff; font-size: 14px; z-index: 9999; animation: scene-indicator-pop 0.3s ease;';
        document.body.appendChild(indicator);
        
        setTimeout(() => indicator.remove(), 3000);
    },
    
    startRainbowCycle: function(selector = 'all') {
        this.colorCycleActive = true;
        this.colorHue = 0;
        
        const cycleRainbow = () => {
            if (!this.colorCycleActive) return;
            
            this.colorHue = (this.colorHue + 5) % 360;
            $.ajax({
                url: '/api/services/lifx/set_color',
                method: 'POST',
                contentType: 'application/json',
                data: JSON.stringify({
                    selector,
                    color: `hue:${this.colorHue * 182} saturation:100%`,
                    duration: 0.3
                })
            });
            
            setTimeout(cycleRainbow, 50);
        };
        
        cycleRainbow();
        this.showGestureFeedback('Rainbow Cycle Started', '🌈');
    },
    
    stopRainbowCycle: function() {
        this.colorCycleActive = false;
        this.showGestureFeedback('Rainbow Cycle Stopped', '⏹');
    },
    
    startPulseEffect: function(selector = 'all') {
        let brightness = 100;
        let increasing = false;
        
        const pulse = () => {
            if (!this.colorCycleActive) return;
            
            brightness = increasing ? brightness + 10 : brightness - 10;
            if (brightness <= 20 || brightness >= 100) increasing = !increasing;
            
            $.ajax({
                url: '/api/services/lifx/set_state',
                method: 'POST',
                contentType: 'application/json',
                data: JSON.stringify({
                    selector,
                    brightness: brightness / 100,
                    duration: 0.1
                })
            });
            
            setTimeout(pulse, 50);
        };
        
        this.colorCycleActive = true;
        pulse();
        this.showGestureFeedback('Pulse Effect Started', '💓');
    },
    
    startBreathEffect: function(selector = 'all') {
        let brightness = 50;
        let increasing = true;
        
        const breath = () => {
            if (!this.colorCycleActive) return;
            
            brightness = increasing ? brightness + 2 : brightness - 2;
            if (brightness <= 10 || brightness >= 90) increasing = !increasing;
            
            $.ajax({
                url: '/api/services/lifx/set_state',
                method: 'POST',
                contentType: 'application/json',
                data: JSON.stringify({
                    selector,
                    brightness: brightness / 100,
                    duration: 0.2
                })
            });
            
            setTimeout(breath, 30);
        };
        
        this.colorCycleActive = true;
        breath();
        this.showGestureFeedback('Breath Effect Started', '🌬');
    },
    
    startFireplaceEffect: function(selector = 'all') {
        const flicker = () => {
            if (!this.colorCycleActive) return;
            
            const brightness = 40 + Math.random() * 30;
            const kelvin = 1800 + Math.random() * 400;
            
            $.ajax({
                url: '/api/services/lifx/set_state',
                method: 'POST',
                contentType: 'application/json',
                data: JSON.stringify({
                    selector,
                    brightness: brightness / 100,
                    duration: 0.3
                }),
                success: () => {
                    $.ajax({
                        url: '/api/services/lifx/set_color',
                        method: 'POST',
                        contentType: 'application/json',
                        data: JSON.stringify({
                            selector,
                            color: `kelvin:${Math.round(kelvin)}`
                        })
                    });
                }
            });
            
            setTimeout(flicker, 200 + Math.random() * 300);
        };
        
        this.colorCycleActive = true;
        flicker();
        this.showGestureFeedback('Fireplace Effect Started', '🔥');
    },
    
    startAuroraEffect: function(selector = 'all') {
        let hue = 180;
        const aurora = () => {
            if (!this.colorCycleActive) return;
            
            hue = (hue + 10) % 360;
            const saturation = 50 + Math.sin(hue * Math.PI / 180) * 30;
            
            $.ajax({
                url: '/api/services/lifx/set_color',
                method: 'POST',
                contentType: 'application/json',
                data: JSON.stringify({
                    selector,
                    color: `hue:${hue},saturation:${saturation}%,brightness:70%`,
                    duration: 0.5
                })
            });
            
            setTimeout(aurora, 500);
        };
        
        this.colorCycleActive = true;
        aurora();
        this.showGestureFeedback('Aurora Effect Started', '🌌');
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
    
    showGestureFeedback: function(text, icon, duration = 1000) {
        if (this.lastGestureHint && this.lastGestureHint.parentNode) {
            this.lastGestureHint.parentNode.removeChild(this.lastGestureHint);
        }
        
        const hint = document.createElement('div');
        hint.className = 'lifx-gesture-hint visible';
        hint.innerHTML = `<span style="font-size: 24px; display: block; margin-bottom: 5px;">${icon}</span>${text}`;
        document.body.appendChild(hint);
        this.lastGestureHint = hint;
        
        setTimeout(() => {
            hint.classList.remove('visible');
            setTimeout(() => {
                if (hint.parentNode) hint.parentNode.removeChild(hint);
                if (this.lastGestureHint === hint) this.lastGestureHint = null;
            }, 300);
        }, duration);
    },
    
    showGestureTutorial: function(force = false) {
        if (!force && localStorage.getItem('lifxGestureTutorialShown')) {
            return;
        }
        
        const tutorial = document.createElement('div');
        tutorial.className = 'lifx-gesture-tutorial active enhanced';
        tutorial.innerHTML = `
            <div class="lifx-gesture-tutorial-content">
                <div class="tutorial-header">
                    <h3>👆 LIFX Touch Gestures</h3>
                    <p class="tutorial-subtitle">Interactive guide for controlling your lights</p>
                </div>
                
                <div class="tutorial-section">
                    <h4><i class="fas fa-hand-pointer"></i> Basic Gestures</h4>
                    <div class="lifx-gesture-tutorial-grid">
                        <div class="lifx-gesture-tutorial-item enhanced">
                            <div class="lifx-gesture-tutorial-icon">⬆️⬇️</div>
                            <div class="lifx-gesture-tutorial-text">
                                <strong>Swipe Up/Down</strong>
                                <span>Adjust brightness</span>
                            </div>
                        </div>
                        <div class="lifx-gesture-tutorial-item enhanced">
                            <div class="lifx-gesture-tutorial-icon">➡️⬅️</div>
                            <div class="lifx-gesture-tutorial-text">
                                <strong>Swipe Left/Right</strong>
                                <span>Adjust color temperature</span>
                            </div>
                        </div>
                        <div class="lifx-gesture-tutorial-item enhanced">
                            <div class="lifx-gesture-tutorial-icon">🤏</div>
                            <div class="lifx-gesture-tutorial-text">
                                <strong>Pinch In/Out</strong>
                                <span>Cycle through scenes</span>
                            </div>
                        </div>
                        <div class="lifx-gesture-tutorial-item enhanced">
                            <div class="lifx-gesture-tutorial-icon">🖱️</div>
                            <div class="lifx-gesture-tutorial-text">
                                <strong>Tap</strong>
                                <span>Select bulb</span>
                            </div>
                        </div>
                    </div>
                </div>
                
                <div class="tutorial-section">
                    <h4><i class="fas fa-star"></i> Advanced Features</h4>
                    <div class="lifx-gesture-tutorial-grid">
                        <div class="lifx-gesture-tutorial-item enhanced">
                            <div class="lifx-gesture-tutorial-icon">✋</div>
                            <div class="lifx-gesture-tutorial-text">
                                <strong>Long Press</strong>
                                <span>Quick settings menu</span>
                            </div>
                        </div>
                        <div class="lifx-gesture-tutorial-item enhanced">
                            <div class="lifx-gesture-tutorial-icon">👆👆</div>
                            <div class="lifx-gesture-tutorial-text">
                                <strong>Double Tap</strong>
                                <span>Toggle power</span>
                            </div>
                        </div>
                        <div class="lifx-gesture-tutorial-item enhanced">
                            <div class="lifx-gesture-tutorial-icon">⊕</div>
                            <div class="lifx-gesture-tutorial-text">
                                <strong>Ctrl+Tap</strong>
                                <span>Multi-select bulbs</span>
                            </div>
                        </div>
                        <div class="lifx-gesture-tutorial-item enhanced">
                            <div class="lifx-gesture-tutorial-icon">💧</div>
                            <div class="lifx-gesture-tutorial-text">
                                <strong>Touch Ripples</strong>
                                <span>Visual feedback on touch</span>
                            </div>
                        </div>
                    </div>
                </div>
                
                <div class="tutorial-footer">
                    <div class="tutorial-tips">
                        <i class="fas fa-lightbulb"></i>
                        <p>Tip: Customize sensitivity and visual feedback in Touch Settings!</p>
                    </div>
                    <button class="lifx-gesture-tutorial-close" onclick="LifXTouchControls.closeGestureTutorial()">
                        <i class="fas fa-check"></i> Got It!
                    </button>
                </div>
            </div>
        `;
        document.body.appendChild(tutorial);
        localStorage.setItem('lifxGestureTutorialShown', 'true');
    },
    
    closeGestureTutorial: function() {
        const tutorial = document.querySelector('.lifx-gesture-tutorial');
        if (tutorial) {
            tutorial.classList.remove('active');
            setTimeout(() => {
                if (tutorial.parentNode) {
                    tutorial.parentNode.removeChild(tutorial);
                }
            }, 300);
        }
        localStorage.setItem('lifxGestureTutorialShown', 'true');
    },
    
    resetGestureTutorial: function() {
        localStorage.removeItem('lifxGestureTutorialShown');
        this.showGestureTutorial(true);
        showNotification('Gesture tutorial reset', 'info');
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
                this.hapticFeedback('power', 0.8);
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
            this.hapticFeedback('selection', 0.6);
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
                const intensity = Math.min(1.0, 0.5 + (Math.abs(delta) / 40));
                this.hapticFeedback('brightness', intensity);
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
                const intensity = Math.min(1.0, 0.5 + (Math.abs(delta) / 1000));
                this.hapticFeedback('colortemp', intensity);
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
        
        this.hapticFeedback('scene', 1.0);
        
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
            movie: { brightness: 35, kelvin: 2200, label: 'Movie' },
            study: { brightness: 75, kelvin: 4500, label: 'Study' },
            dinner: { brightness: 55, kelvin: 2700, label: 'Dinner' },
            morning: { brightness: 85, kelvin: 5500, label: 'Morning' },
            goodnight: { brightness: 10, kelvin: 2000, label: 'Goodnight' },
            rainbow: { brightness: 80, kelvin: 4000, label: 'Rainbow' },
            fireplace: { brightness: 60, kelvin: 2000, label: 'Fireplace' },
            ice: { brightness: 70, kelvin: 8000, label: 'Ice' },
            aurora: { brightness: 75, kelvin: 6000, label: 'Aurora' },
            nebula: { brightness: 65, kelvin: 5000, label: 'Nebula' },
            thunder: { brightness: 100, kelvin: 7000, label: 'Thunder' },
            crystal: { brightness: 75, kelvin: 6500, label: 'Crystal' },
            lagoon: { brightness: 65, kelvin: 5500, label: 'Lagoon' },
            cotton_candy: { brightness: 70, kelvin: 4500, label: 'Cotton Candy' },
            spring_blossom: { brightness: 80, kelvin: 4000, label: 'Spring Blossom' },
            punchbowl: { brightness: 90, kelvin: 5000, label: 'Punchbowl' },
            smashing: { brightness: 95, kelvin: 6000, label: 'Smashing' },
            glitter: { brightness: 85, kelvin: 5500, label: 'Glitter' },
            golden_hour: { brightness: 60, kelvin: 3000, label: 'Golden Hour' },
            late_night: { brightness: 25, kelvin: 2200, label: 'Late Night' },
            midday: { brightness: 90, kelvin: 5500, label: 'Midday' },
            polar: { brightness: 70, kelvin: 7500, label: 'Polar' }
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
                            this.showGestureFeedback(`Scene: ${settings.label}`, this.getSceneEmoji(scene));
                            this.recordGesture('scene', { scene, label: settings.label });
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
            const stepSize = Math.abs(newBrightness - this.brightnessLevel);
            this.brightnessLevel = newBrightness;
            this.showBrightnessFeedback(this.brightnessLevel);
            if (stepSize >= 5) {
                this.hapticFeedback('brightness', Math.min(0.5, stepSize / 20));
            }
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
    
    hapticFeedback: function(pattern = 'default', intensity = 1.0) {
        if (!this.hapticEnabled || !navigator.vibrate) return;
        
        const basePatterns = {
            'default': [50],
            'light': [30],
            'success': [50, 50, 50],
            'error': [100, 50, 100],
            'warning': [75, 50, 75],
            'selection': [40, 30, 40],
            'brightness': [25, 25, 25],
            'colortemp': [35, 35],
            'scene': [60, 40, 60],
            'power': [100, 50, 100, 50, 100],
            'gesture': [45, 45],
            'beat': [20],
            'doubleTap': [30, 30, 30],
            'longPress': [80, 40, 80],
            'swipe': [35, 35],
            'pinch': [40, 30, 40],
            'zone': [50, 40, 50],
            'media': [30, 30, 30, 30]
        };
        
        const basePattern = basePatterns[pattern] || basePatterns['default'];
        const scaledPattern = basePattern.map(duration => Math.round(duration * intensity));
        
        try {
            navigator.vibrate(scaledPattern);
        } catch (e) {
            console.warn('Haptic feedback failed:', e);
        }
    },
    
    setGestureSensitivity: function(level) {
        const settings = {
            'low': { swipeDistance: 80, swipeTime: 400, pinchDistance: 50, longPressDelay: 700, doubleTapDelay: 400 },
            'medium': { swipeDistance: 50, swipeTime: 300, pinchDistance: 30, longPressDelay: 500, doubleTapDelay: 300 },
            'high': { swipeDistance: 30, swipeTime: 200, pinchDistance: 20, longPressDelay: 300, doubleTapDelay: 200 }
        };
        
        this.gestureSensitivity = settings[level] || settings['medium'];
        this.touchHoldDelay = this.gestureSensitivity.longPressDelay;
        this.doubleTapDelay = this.gestureSensitivity.doubleTapDelay;
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
    
    getSceneEmoji: function(sceneName) {
        const emojis = {
            relax: '🧘', focus: '🎯', energize: '⚡', night: '🌙',
            sunset: '🌅', ocean: '🌊', reading: '📖', romance: '💕',
            party: '🎉', golden: '✨', arctic: '❄️', tropical: '🌴',
            spring: '🌸', autumn: '🍂', meditation: '🧘‍♀️', gaming: '🎮',
            cooking: '🍳', creative: '🎨', yoga: '🧘', movie: '🎬',
            study: '📚', dinner: '🍽️', morning: '🌅', goodnight: '😴',
            rainbow: '🌈', fireplace: '🔥', ice: '🧊', aurora: '🌌',
            nebula: '🌀', thunder: '⛈️', crystal: '💎', lagoon: '🏝️',
            cotton_candy: '🍭', spring_blossom: '🌺', punchbowl: '🥤',
            smashing: '💥', glitter: '✨', golden_hour: '🌇',
            late_night: '🌃', midday: '☀️', polar: '🐧',
            cosmic: '🌌', dream: '💭', chill: '🧊', adventure: '🗺️', festival: '🎪'
        };
        return emojis[sceneName] || '💡';
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
            'movie': '#d35400',
            'study': '#6ab0de',
            'dinner': '#ffcc5c',
            'morning': '#ffd93d',
            'goodnight': '#2c3e50',
            'rainbow': '#ff0080',
            'fireplace': '#ff4500',
            'ice': '#7fffd4',
            'aurora': '#00ff88',
            'nebula': '#9b59b6',
            'thunder': '#f1c40f'
        };
        return sceneColors[sceneName] || '#ffffff';
    },
    
    applyScene: function(sceneName) {
        if (sceneName === 'rainbow') {
            this.startRainbowCycle();
            return;
        }
        
        const dynamicScenes = {
            'crystal': { hue: 200, saturation: 50, brightness: 80, kelvin: 7000, effect: 'pulse' },
            'lagoon': { hue: 160, saturation: 70, brightness: 70, kelvin: 5000, effect: 'fade' },
            'cotton_candy': { hue: 320, saturation: 60, brightness: 75, kelvin: 4500, effect: 'gentle_pulse' },
            'spring_blossom': { hue: 140, saturation: 65, brightness: 80, kelvin: 4200, effect: 'slow_cycle' },
            'punchbowl': { hue: 300, saturation: 80, brightness: 85, kelvin: 4000, effect: 'vibrant' },
            'smashing': { hue: 280, saturation: 90, brightness: 90, kelvin: 5000, effect: 'energy' },
            'glitter': { hue: 50, saturation: 85, brightness: 90, kelvin: 4000, effect: 'sparkle' },
            'golden_hour': { hue: 35, saturation: 70, brightness: 75, kelvin: 3200, effect: 'pulse' },
            'late_night': { hue: 240, saturation: 30, brightness: 40, kelvin: 2700, effect: 'fade' },
            'midday': { hue: 50, saturation: 40, brightness: 95, kelvin: 5500, effect: 'gentle_pulse' },
            'polar': { hue: 200, saturation: 40, brightness: 85, kelvin: 8000, effect: 'slow_cycle' },
            'fireplace': { hue: 30, saturation: 80, brightness: 60, kelvin: 2000, effect: 'pulse' },
            'aurora': { hue: 140, saturation: 100, brightness: 75, kelvin: 6000, effect: 'fade' },
            'nebula': { hue: 280, saturation: 80, brightness: 65, kelvin: 5000, effect: 'gentle_pulse' },
            'thunder': { hue: 50, saturation: 90, brightness: 100, kelvin: 7000, effect: 'energy' },
            'cosmic': { hue: 280, saturation: 90, brightness: 70, kelvin: 6500, effect: 'cosmic_pulse' },
            'dream': { hue: 180, saturation: 60, brightness: 50, kelvin: 4000, effect: 'dream_flow' },
            'chill': { hue: 200, saturation: 40, brightness: 45, kelvin: 3500, effect: 'gentle_pulse' },
            'adventure': { hue: 30, saturation: 85, brightness: 80, kelvin: 4500, effect: 'energy' },
            'festival': { hue: 320, saturation: 95, brightness: 90, kelvin: 5000, effect: 'festival_lights' }
        };
        
        if (dynamicScenes[sceneName]) {
            this.applyDynamicScene(sceneName, dynamicScenes[sceneName]);
            return;
        }
        
        this.stopRainbowCycle();
        this.stopDynamicScene();
        
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
            'movie': { hue: 20, saturation: 30, brightness: 35, kelvin: 2200 },
            'study': { hue: 200, saturation: 30, brightness: 75, kelvin: 4500 },
            'dinner': { hue: 30, saturation: 40, brightness: 55, kelvin: 2700 },
            'morning': { hue: 50, saturation: 50, brightness: 85, kelvin: 5500 },
            'goodnight': { hue: 240, saturation: 10, brightness: 10, kelvin: 2000 },
            'rainbow': { hue: 0, saturation: 100, brightness: 90, kelvin: 4000 },
            'fireplace': { hue: 30, saturation: 80, brightness: 60, kelvin: 2000 },
            'ice': { hue: 200, saturation: 50, brightness: 70, kelvin: 8000 },
            'aurora': { hue: 140, saturation: 100, brightness: 75, kelvin: 6000 },
            'nebula': { hue: 280, saturation: 80, brightness: 65, kelvin: 5000 },
            'thunder': { hue: 50, saturation: 90, brightness: 100, kelvin: 7000 }
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
            undoBtn.setAttribute('title', `Undo last action (${this.gestureHistory.length} available)`);
        } else {
            undoBtn.classList.remove('visible');
            undoBtn.setAttribute('title', 'Undo Last Gesture');
        }
    },
    
    showTouchHoldProgress: function() {
        if (!this.touchHoldProgressEl) {
            this.touchHoldProgressEl = document.getElementById('touch-hold-progress');
        }
        if (this.touchHoldProgressEl) {
            this.touchHoldProgressEl.classList.add('visible');
        }
    },
    
    hideTouchHoldProgress: function() {
        if (this.touchHoldProgressEl) {
            this.touchHoldProgressEl.classList.remove('visible');
        }
    },
    
    setGestureSensitivityLevel: function(level) {
        this.setGestureSensitivity(level);
        this.showGestureFeedback(`Sensitivity: ${level}`, '✓');
    },
    
    showSensitivitySelector: function() {
        if (typeof Swal === 'undefined') {
            alert('Gesture Sensitivity: low, medium, high');
            return;
        }
        
        const current = localStorage.getItem('lifx_gesture_sensitivity') || 'medium';
        Swal.fire({
            title: 'Gesture Sensitivity',
            html: `
                <div style="display: flex; gap: 10px; justify-content: center;">
                    <button class="btn btn-sm ${current === 'low' ? 'btn-primary' : 'btn-outline-primary'}" 
                            onclick="LifXTouchControls.setGestureSensitivityLevel('low')">Low</button>
                    <button class="btn btn-sm ${current === 'medium' ? 'btn-primary' : 'btn-outline-primary'}" 
                            onclick="LifXTouchControls.setGestureSensitivityLevel('medium')">Medium</button>
                    <button class="btn btn-sm ${current === 'high' ? 'btn-primary' : 'btn-outline-primary'}" 
                            onclick="LifXTouchControls.setGestureSensitivityLevel('high')">High</button>
                </div>
            `,
            showConfirmButton: false,
            showCloseButton: true
        });
    },
    
    showTouchSensitivityPanel: function() {
        if (typeof Swal === 'undefined') {
            alert('Touch Settings: Gesture sensitivity, haptic feedback, visual feedback');
            return;
        }
        
        const current = localStorage.getItem('lifx_gesture_sensitivity') || 'medium';
        const currentSensitivityDesc = {
            'low': 'Requires larger movements',
            'medium': 'Balanced responsiveness',
            'high': 'Most responsive'
        };
        
        const swipeDistances = {
            'low': '80px',
            'medium': '50px', 
            'high': '30px'
        };
        
        const longPressDelays = {
            'low': '700ms',
            'medium': '500ms',
            'high': '300ms'
        };
        
        Swal.fire({
            title: '<i class="fas fa-fingerprint"></i> Touch Sensitivity Settings',
            html: `
                <div class="touch-sensitivity-panel">
                    <h4><i class="fas fa-sliders-h"></i> Gesture Sensitivity</h4>
                    <div class="sensitivity-option ${current === 'low' ? 'active' : ''}" onclick="LifXTouchControls.setGestureSensitivityLevel('low')">
                        <div class="sensitivity-option-label">
                            <span class="sensitivity-option-icon">🐢</span>
                            <div>
                                <div>Low</div>
                                <div class="sensitivity-option-description">Requires larger movements - fewer accidental triggers</div>
                                <div class="sensitivity-option-description">Swipe: ${swipeDistances['low']} | Hold: ${longPressDelays['low']}</div>
                            </div>
                        </div>
                        ${current === 'low' ? '<i class="fas fa-check-circle" style="color: #00d4ff;"></i>' : ''}
                    </div>
                    <div class="sensitivity-option ${current === 'medium' ? 'active' : ''}" onclick="LifXTouchControls.setGestureSensitivityLevel('medium')">
                        <div class="sensitivity-option-label">
                            <span class="sensitivity-option-icon">🚶</span>
                            <div>
                                <div>Medium</div>
                                <div class="sensitivity-option-description">Balanced responsiveness - recommended for most users</div>
                                <div class="sensitivity-option-description">Swipe: ${swipeDistances['medium']} | Hold: ${longPressDelays['medium']}</div>
                            </div>
                        </div>
                        ${current === 'medium' ? '<i class="fas fa-check-circle" style="color: #00d4ff;"></i>' : ''}
                    </div>
                    <div class="sensitivity-option ${current === 'high' ? 'active' : ''}" onclick="LifXTouchControls.setGestureSensitivityLevel('high')">
                        <div class="sensitivity-option-label">
                            <span class="sensitivity-option-icon">🐇</span>
                            <div>
                                <div>High</div>
                                <div class="sensitivity-option-description">Most responsive - detects subtle movements</div>
                                <div class="sensitivity-option-description">Swipe: ${swipeDistances['high']} | Hold: ${longPressDelays['high']}</div>
                            </div>
                        </div>
                        ${current === 'high' ? '<i class="fas fa-check-circle" style="color: #00d4ff;"></i>' : ''}
                    </div>
                </div>
                
                <div class="touch-sensitivity-panel">
                    <h4><i class="fas fa-eye"></i> Visual Feedback</h4>
                    <div style="display: flex; gap: 10px; flex-wrap: wrap;">
                        <button class="btn btn-sm ${this.touchRippleEnabled ? 'btn-success' : 'btn-outline-secondary'}" 
                                onclick="LifXTouchControls.toggleTouchRipple()">
                            <i class="fas fa-${this.touchRippleEnabled ? 'check' : 'times'}"></i> Touch Ripples
                        </button>
                        <button class="btn btn-sm ${this.enhancedRippleMode ? 'btn-success' : 'btn-outline-secondary'}" 
                                onclick="LifXTouchControls.toggleEnhancedRipple()">
                            <i class="fas fa-${this.enhancedRippleMode ? 'check' : 'times'}"></i> Enhanced Ripples
                        </button>
                        <button class="btn btn-sm ${this.glowEffectEnabled ? 'btn-success' : 'btn-outline-secondary'}" 
                                onclick="LifXTouchControls.toggleGlowEffect()">
                            <i class="fas fa-${this.glowEffectEnabled ? 'check' : 'times'}"></i> Glow Effect
                        </button>
                        <button class="btn btn-sm ${this.showGestureHints ? 'btn-success' : 'btn-outline-secondary'}" 
                                onclick="LifXTouchControls.toggleGestureHints()">
                            <i class="fas fa-${this.showGestureHints ? 'check' : 'times'}"></i> Gesture Hints
                        </button>
                        <button class="btn btn-sm ${this.highContrastHints ? 'btn-success' : 'btn-outline-secondary'}" 
                                onclick="LifXTouchControls.toggleHighContrast()">
                            <i class="fas fa-${this.highContrastHints ? 'check' : 'times'}"></i> High Contrast
                        </button>
                    </div>
                </div>
                
                <div class="touch-sensitivity-panel">
                    <h4><i class="fas fa-palette"></i> Ripple Customization</h4>
                    <div style="margin-bottom: 15px;">
                        <label style="display: block; margin-bottom: 5px; color: #adb5bd;">Ripple Size: ${this.rippleSize}px</label>
                        <input type="range" min="30" max="100" value="${this.rippleSize}" 
                               oninput="LifXTouchControls.setRippleSize(this.value)"
                               style="width: 100%;">
                    </div>
                    <div style="display: flex; gap: 10px; flex-wrap: wrap;">
                        ${['#00d4ff', '#ff6b6b', '#4ecdc4', '#ffe66d', '#ff9f43', '#a55eea'].map(color => `
                            <button class="btn btn-sm" style="width: 35px; height: 35px; border-radius: 50%; background: ${color}; border: 2px solid ${this.rippleColor === color ? '#fff' : 'transparent'};" 
                                    onclick="LifXTouchControls.setRippleColor('${color}')"></button>
                        `).join('')}
                    </div>
                </div>
                
                <div class="touch-sensitivity-panel">
                    <h4><i class="fas fa-mobile-alt"></i> Haptic Feedback</h4>
                    <div style="display: flex; gap: 10px; flex-wrap: wrap;">
                        <button class="btn btn-sm ${this.hapticEnabled ? 'btn-success' : 'btn-outline-secondary'}" 
                                onclick="LifXTouchControls.toggleHapticFeedback()">
                            <i class="fas fa-${this.hapticEnabled ? 'check' : 'times'}"></i> Vibration
                        </button>
                        <button class="btn btn-sm btn-outline-primary" onclick="LifXTouchControls.testHapticFeedback()">
                            <i class="fas fa-play"></i> Test
                        </button>
                    </div>
                </div>
                
                <div class="touch-sensitivity-panel">
                    <h4><i class="fas fa-music"></i> Media Sync & Beat Detection</h4>
                    <div style="margin-bottom: 15px;">
                        <label style="display: block; margin-bottom: 5px; color: #adb5bd;">Beat Detection Sensitivity: ${Math.round(this.beatDetectionSensitivity * 100)}%</label>
                        <input type="range" min="30" max="100" value="${Math.round(this.beatDetectionSensitivity * 100)}" 
                               oninput="LifXTouchControls.setBeatDetectionSensitivity(this.value / 100)"
                               style="width: 100%;">
                    </div>
                    <div style="display: flex; gap: 10px; flex-wrap: wrap; margin-bottom: 15px;">
                        <button class="btn btn-sm ${this.mediaSyncMode === 'beat' ? 'btn-success' : 'btn-outline-secondary'}" 
                                onclick="LifXTouchControls.setMediaSyncMode('beat')">
                            <i class="fas fa-heartbeat"></i> Beat Sync
                        </button>
                        <button class="btn btn-sm ${this.mediaSyncMode === 'color' ? 'btn-success' : 'btn-outline-secondary'}" 
                                onclick="LifXTouchControls.setMediaSyncMode('color')">
                            <i class="fas fa-palette"></i> Color Sync
                        </button>
                        <button class="btn btn-sm ${this.mediaSyncMode === 'ambient' ? 'btn-success' : 'btn-outline-secondary'}" 
                                onclick="LifXTouchControls.setMediaSyncMode('ambient')">
                            <i class="fas fa-film"></i> Ambient
                        </button>
                    </div>
                    <div style="display: flex; gap: 10px; flex-wrap: wrap;">
                        <button class="btn btn-sm ${this.ambientLightSync ? 'btn-success' : 'btn-outline-secondary'}" 
                                onclick="LifXTouchControls.toggleAmbientLightSync()">
                            <i class="fas fa-${this.ambientLightSync ? 'check' : 'times'}"></i> Ambient Light Sync
                        </button>
                        <button class="btn btn-sm ${this.lifxMediaSyncEnabled ? 'btn-success' : 'btn-outline-secondary'}" 
                                onclick="LifXTouchControls.toggleMediaSync()">
                            <i class="fas fa-${this.lifxMediaSyncEnabled ? 'check' : 'times'}"></i> Media Sync
                        </button>
                    </div>
                </div>
                
                <div class="touch-sensitivity-panel">
                    <h4><i class="fas fa-hand-pointer"></i> Edge Swipe Gestures</h4>
                    <div style="color: #adb5bd; font-size: 12px; margin-bottom: 10px;">
                        Swipe from screen edges for quick actions (requires edge swipe enabled)
                    </div>
                    <div style="display: grid; grid-template-columns: repeat(2, 1fr); gap: 10px;">
                        <div style="padding: 10px; background: rgba(0, 212, 255, 0.1); border-radius: 8px; border: 1px solid rgba(0, 212, 255, 0.3);">
                            <div style="color: #00d4ff; font-weight: bold; font-size: 13px;"><i class="fas fa-arrow-left"></i> Left Edge Swipe</div>
                            <div style="color: #adb5bd; font-size: 11px; margin-top: 4px;">Quick scenes panel</div>
                        </div>
                        <div style="padding: 10px; background: rgba(0, 255, 136, 0.1); border-radius: 8px; border: 1px solid rgba(0, 255, 136, 0.3);">
                            <div style="color: #00ff88; font-weight: bold; font-size: 13px;"><i class="fas fa-arrow-right"></i> Right Edge Swipe</div>
                            <div style="color: #adb5bd; font-size: 11px; margin-top: 4px;">Media controls</div>
                        </div>
                        <div style="padding: 10px; background: rgba(255, 107, 107, 0.1); border-radius: 8px; border: 1px solid rgba(255, 107, 107, 0.3);">
                            <div style="color: #ff6b6b; font-weight: bold; font-size: 13px;"><i class="fas fa-arrow-up"></i> Top Edge Swipe</div>
                            <div style="color: #adb5bd; font-size: 11px; margin-top: 4px;">Brightness boost</div>
                        </div>
                        <div style="padding: 10px; background: rgba(255, 193, 7, 0.1); border-radius: 8px; border: 1px solid rgba(255, 193, 7, 0.3);">
                            <div style="color: #ffc107; font-weight: bold; font-size: 13px;"><i class="fas fa-arrow-down"></i> Bottom Edge Swipe</div>
                            <div style="color: #adb5bd; font-size: 11px; margin-top: 4px;">Dim lights</div>
                        </div>
                    </div>
                </div>
            `,
            showConfirmButton: false,
            showCloseButton: true,
            width: '700px'
        });
    },
    
    toggleTouchRipple: function() {
        this.touchRippleEnabled = !this.touchRippleEnabled;
        localStorage.setItem('lifx_touch_ripple', this.touchRippleEnabled);
        this.showTouchSensitivityPanel();
        this.showEnhancedGestureFeedback(
            this.touchRippleEnabled ? 'Touch Ripples ON' : 'Touch Ripples OFF',
            this.touchRippleEnabled ? '💧' : '🚫'
        );
    },
    
    toggleEnhancedRipple: function() {
        this.enhancedRippleMode = !this.enhancedRippleMode;
        localStorage.setItem('lifx_enhanced_ripple', this.enhancedRippleMode);
        this.showTouchSensitivityPanel();
        this.showEnhancedGestureFeedback(
            this.enhancedRippleMode ? 'Enhanced Ripples ON' : 'Enhanced Ripples OFF',
            this.enhancedRippleMode ? '✨' : '💧'
        );
    },
    
    setRippleColor: function(color) {
        this.rippleColor = color;
        localStorage.setItem('lifx_ripple_color', color);
        this.showEnhancedGestureFeedback('Ripple color updated', '🎨');
    },
    
    setRippleSize: function(size) {
        this.rippleSize = Math.max(30, Math.min(100, size));
        localStorage.setItem('lifx_ripple_size', this.rippleSize);
        this.showEnhancedGestureFeedback(`Ripple size: ${this.rippleSize}px`, '📏');
    },
    
    toggleGestureHints: function() {
        this.showGestureHints = !this.showGestureHints;
        localStorage.setItem('lifx_gesture_hints', this.showGestureHints);
        this.showTouchSensitivityPanel();
        this.showEnhancedGestureFeedback(
            this.showGestureHints ? 'Gesture Hints ON' : 'Gesture Hints OFF',
            this.showGestureHints ? '👆' : '🔇'
        );
    },
    
    toggleGlowEffect: function() {
        this.glowEffectEnabled = !this.glowEffectEnabled;
        localStorage.setItem('lifx_glow_effect', this.glowEffectEnabled);
        this.showTouchSensitivityPanel();
        this.showEnhancedGestureFeedback(
            this.glowEffectEnabled ? 'Glow Effect ON' : 'Glow Effect OFF',
            this.glowEffectEnabled ? '✨' : '💡'
        );
    },
    
    toggleHighContrast: function() {
        this.highContrastHints = !this.highContrastHints;
        localStorage.setItem('lifx_high_contrast', this.highContrastHints);
        this.showTouchSensitivityPanel();
        this.showEnhancedGestureFeedback(
            this.highContrastHints ? 'High Contrast ON' : 'High Contrast OFF',
            this.highContrastHints ? '🔲' : '⬜'
        );
    },
    
    toggleHapticFeedback: function() {
        this.hapticEnabled = !this.hapticEnabled;
        localStorage.setItem('lifx_haptic', this.hapticEnabled);
        this.showTouchSensitivityPanel();
        this.showEnhancedGestureFeedback(
            this.hapticEnabled ? 'Haptic Feedback ON' : 'Haptic Feedback OFF',
            this.hapticEnabled ? '📳' : '🔕'
        );
    },
    
    testHapticFeedback: function() {
        this.hapticFeedback('success');
        this.showEnhancedGestureFeedback('Test Vibration', '📳', 500);
    },
    
    savePreferences: function() {
        const prefs = {
            brightnessLevel: this.brightnessLevel,
            colorTempLevel: this.colorTempLevel,
            currentScene: this.currentScene,
            ambientLightSync: this.ambientLightSync,
            hapticEnabled: this.hapticEnabled,
            touchRippleEnabled: this.touchRippleEnabled,
            showGestureHints: this.showGestureHints,
            enhancedRippleMode: this.enhancedRippleMode,
            glowEffectEnabled: this.glowEffectEnabled,
            highContrastHints: this.highContrastHints,
            rippleColor: this.rippleColor,
            rippleSize: this.rippleSize
        };
        localStorage.setItem('lifx_preferences', JSON.stringify(prefs));
    },
    
    initTouchRipple: function() {
        document.addEventListener('touchstart', (e) => {
            const bulbEl = e.target.closest('.lifx-bulb-control, .lifx-bulb-card');
            if (!bulbEl) return;
            
            const touch = e.touches[0];
            const rect = bulbEl.getBoundingClientRect();
            const x = touch.clientX - rect.left;
            const y = touch.clientY - rect.top;
            
            const ripple = document.createElement('span');
            ripple.className = 'lifx-touch-ripple' + (this.enhancedRippleMode ? ' enhanced' : '');
            ripple.style.left = (x - this.rippleSize / 2) + 'px';
            ripple.style.top = (y - this.rippleSize / 2) + 'px';
            ripple.style.width = this.rippleSize + 'px';
            ripple.style.height = this.rippleSize + 'px';
            
            if (this.enhancedRippleMode) {
                ripple.style.background = `radial-gradient(circle, ${this.rippleColor} 0%, transparent 70%)`;
                ripple.style.animationDuration = (this.rippleDuration / 1000) + 's';
                
                if (this.glowEffectEnabled) {
                    bulbEl.classList.add('touch-glow');
                    setTimeout(() => bulbEl.classList.remove('touch-glow'), 300);
                }
            }
            
            bulbEl.appendChild(ripple);
            
            setTimeout(() => {
                if (ripple.parentNode) {
                    ripple.parentNode.removeChild(ripple);
                }
            }, this.rippleDuration);
        }, { passive: true });
    },
    
    initGestureEnhancements: function() {
        this.touchSwipeThreshold = this.gestureSensitivity.swipeDistance;
        this.touchSwipeVelocityThreshold = 0.3;
        this.lastTouchPositions = new Map();
        this.touchVelocity = new Map();
        this.setupMultiSelectMode();
        this.setupTouchHoldProgress();
        this.setupQuickActions();
        this.setupZoneControl();
        this.loadRipplePreferences();
        this.initEdgeSwipeDetection();
        console.log('Gesture enhancements initialized with edge swipe detection');
    },
    
    showEnhancedGestureFeedback: function(text, icon, duration = null) {
        if (!this.showGestureHints) return;
        
        const hintDuration = duration || this.gestureHintDuration;
        
        if (this.lastGestureHint && this.lastGestureHint.parentNode) {
            this.lastGestureHint.parentNode.removeChild(this.lastGestureHint);
        }
        
        const hint = document.createElement('div');
        hint.className = 'lifx-gesture-hint enhanced visible' + (this.highContrastHints ? ' high-contrast' : '');
        hint.innerHTML = `
            <span class="gesture-icon">${icon}</span>
            <span class="gesture-text">${text}</span>
        `;
        document.body.appendChild(hint);
        this.lastGestureHint = hint;
        
        setTimeout(() => {
            hint.classList.remove('visible');
            setTimeout(() => {
                if (hint.parentNode) hint.parentNode.removeChild(hint);
                if (this.lastGestureHint === hint) this.lastGestureHint = null;
            }, 300);
        }, hintDuration);
    },
    
    showGestureTrail: function(x, y, gesture) {
        if (!this.touchRippleEnabled) return;
        
        const trail = document.createElement('div');
        trail.className = 'lifx-gesture-trail';
        trail.style.left = x + 'px';
        trail.style.top = y + 'px';
        trail.innerHTML = `<span class="trail-icon">${gesture}</span>`;
        document.body.appendChild(trail);
        
        this.touchGestureTrail.push({ element: trail, createdAt: Date.now() });
        
        if (this.touchGestureTrail.length > this.maxTrailLength) {
            const oldTrail = this.touchGestureTrail.shift();
            if (oldTrail.element.parentNode) {
                oldTrail.element.parentNode.removeChild(oldTrail.element);
            }
        }
        
        setTimeout(() => {
            if (trail.parentNode) {
                trail.parentNode.removeChild(trail);
            }
        }, 1000);
    },
    
    showBrightnessFeedback: function(value) {
        const feedback = document.createElement('div');
        feedback.className = 'touch-feedback-brightness visible';
        feedback.textContent = value + '%';
        document.body.appendChild(feedback);
        
        setTimeout(() => {
            if (feedback.parentNode) {
                feedback.parentNode.removeChild(feedback);
            }
        }, 1000);
    },
    
    setupZoneControl: function() {
        const zoneControlBtn = document.getElementById('lifx-zone-control-btn');
        if (zoneControlBtn) {
            zoneControlBtn.addEventListener('click', () => this.openZoneControl());
        }
    },
    
    openZoneControl: function() {
        if (typeof Swal === 'undefined') {
            alert('Zone control requires SweetAlert2');
            return;
        }
        
        const targets = this.multiBulbSelection.length > 0 
            ? this.multiBulbSelection 
            : (this.selectedBulb ? [this.selectedBulb] : ['all']);
        
        Swal.fire({
            title: 'Zone Control',
            html: `
                <div style="padding: 20px;">
                    <div class="zone-grid" style="display: grid; grid-template-columns: repeat(5, 1fr); gap: 8px; margin-bottom: 20px;">
                        ${Array.from({length: 10}, (_, i) => `
                            <div class="zone-item" data-zone="${i}" style="padding: 15px 10px; background: rgba(39, 160, 185, 0.2); border: 2px solid transparent; border-radius: 8px; text-align: center; cursor: pointer; transition: all 0.2s;"
                                 onclick="LifXTouchControls.selectZone(${i}, this)">
                                <i class="fas fa-lightbulb" style="font-size: 20px; margin-bottom: 5px;"></i>
                                <div style="font-size: 11px;">Zone ${i + 1}</div>
                            </div>
                        `).join('')}
                    </div>
                    <div class="zone-color-picker" style="display: flex; gap: 8px; justify-content: center; flex-wrap: wrap;">
                        ${['#ff0000', '#ff8000', '#ffff00', '#00ff00', '#00ffff', '#0000ff', '#8000ff', '#ff00ff', '#ffffff', '#ffcc00'].map(color => `
                            <button class="zone-color-btn" style="width: 40px; height: 40px; border-radius: 50%; border: 3px solid transparent; background: ${color}; cursor: pointer; transition: all 0.2s;"
                                    onclick="LifXTouchControls.applyZoneColor('${color}', '${targets.join(',')}')"></button>
                        `).join('')}
                    </div>
                </div>
            `,
            showConfirmButton: false,
            showCloseButton: true,
            width: '600px'
        });
    },
    
    selectZone: function(zoneIndex, element) {
        document.querySelectorAll('.zone-item').forEach(el => {
            el.style.borderColor = 'transparent';
            el.style.background = 'rgba(39, 160, 185, 0.2)';
        });
        element.style.borderColor = '#00d4ff';
        element.style.background = 'rgba(0, 212, 255, 0.3)';
        this.selectedZone = zoneIndex;
        this.hapticFeedback('selection', 0.6);
    },
    
    applyZoneColor: function(hexColor, targetString) {
        const targets = targetString.split(',');
        const rgb = this.hexToRgb(hexColor);
        const hsv = this.rgbToHsv(rgb.r, rgb.g, rgb.b);
        
        $.ajax({
            url: '/api/services/lifx/zones',
            method: 'POST',
            contentType: 'application/json',
            data: JSON.stringify({
                selector: `id:${targets.join(',')}`,
                start_index: this.selectedZone || 0,
                end_index: this.selectedZone || 0,
                color: `hue:${Math.round(hsv.h * 182)} saturation:${Math.round(hsv.s * 100)}%`,
                duration: 0.5
            }),
            success: () => {
                this.showGestureFeedback(`Zone ${this.selectedZone + 1} updated`, '🎨');
                this.hapticFeedback('success');
                if (typeof Swal !== 'undefined') Swal.close();
            }
        });
    },
    
    initMediaSync: function() {
        if (typeof MediaPlayer !== 'undefined') {
            MediaPlayer.onLifxUpdate = (data) => {
                if (this.lifxMediaSyncEnabled) {
                    this.applyMediaLighting(data);
                }
            };
        }
        this.initBeatDetection();
    },
    
    initBeatDetection: function() {
        const audioElem = document.querySelector('audio, video');
        if (!audioElem) {
            console.log('No audio/video element found for beat detection');
            return;
        }
        
        try {
            if (this.audioContext && this.audioContext.state === 'running') {
                console.log('Beat detection already initialized');
                return;
            }
            
            this.audioContext = new (window.AudioContext || window.webkitAudioContext)();
            this.audioAnalyzer = this.audioContext.createAnalyser();
            const source = this.audioContext.createMediaElementSource(audioElem);
            source.connect(this.audioAnalyzer);
            this.audioAnalyzer.connect(this.audioContext.destination);
            this.audioAnalyzer.fftSize = 256;
            this.audioAnalyzer.smoothingTimeConstant = 0.8;
            
            this.mediaPlaybackActive = true;
            this.monitorBeat();
            console.log('Beat detection initialized successfully');
        } catch (e) {
            console.warn('Beat detection not available:', e);
            this.showBeatDetectionFallback();
        }
    },
    
    showBeatDetectionFallback: function() {
        console.log('Using fallback beat detection via media player events');
        if (typeof MediaPlayer !== 'undefined') {
            MediaPlayer.onBeat = (beatData) => {
                if (this.lifxMediaSyncEnabled && beatData) {
                    this.triggerBeatEffect();
                }
            };
        }
    },
    
    monitorBeat: function() {
        if (!this.audioAnalyzer) return;
        
        const bufferLength = this.audioAnalyzer.frequencyBinCount;
        const dataArray = new Uint8Array(bufferLength);
        let lastBeatTime = 0;
        
        const detectBeat = () => {
            if (!this.mediaPlaybackActive) {
                requestAnimationFrame(detectBeat);
                return;
            }
            
            this.audioAnalyzer.getByteFrequencyData(dataArray);
            
            const bass = dataArray.slice(0, 10).reduce((a, b) => a + b, 0) / 10;
            const mids = dataArray.slice(10, 50).reduce((a, b) => a + b, 0) / 40;
            const threshold = this.beatDetectionSensitivity * 255;
            
            const now = Date.now();
            if (bass > threshold && now - lastBeatTime > this.beatDebounce) {
                lastBeatTime = now;
                this.bpmValue = Math.round(60000 / (now - this.lastBeatTime)) || 0;
                this.lastBeatTime = now;
                
                if (this.mediaSyncMode === 'beat') {
                    this.triggerBeatEffect();
                }
                
                this.updateBpmDisplay();
            }
            
            requestAnimationFrame(detectBeat);
        };
        
        detectBeat();
    },
    
    triggerBeatEffect: function() {
        const targets = this.multiBulbSelection.length > 0 
            ? this.multiBulbSelection 
            : (this.selectedBulb ? [this.selectedBulb] : ['all']);
        
        $.ajax({
            url: '/api/services/lifx/set_state',
            method: 'POST',
            contentType: 'application/json',
            data: JSON.stringify({
                selector: `id:${targets.join(',')}`,
                brightness: 100,
                duration: 0.05
            })
        });
        
        this.showBeatVisualization();
    },
    
    showBeatVisualization: function() {
        let existing = document.querySelector('.bpm-realtime-indicator');
        if (!existing) {
            const indicator = document.createElement('div');
            indicator.className = 'bpm-realtime-indicator visible';
            indicator.innerHTML = `
                <i class="fas fa-heartbeat bpm-icon"></i>
                <span class="bpm-value" id="bpm-value-display">${this.bpmValue || '--'}</span>
                <span class="bpm-label">BPM</span>
            `;
            document.body.appendChild(indicator);
            setTimeout(() => indicator.classList.remove('visible'), 2000);
        } else {
            const valueEl = existing.querySelector('#bpm-value-display');
            if (valueEl) valueEl.textContent = this.bpmValue || '--';
        }
    },
    
    updateBpmDisplay: function() {
        const bpmDisplay = document.querySelector('.bpm-display .bpm-value');
        if (bpmDisplay) {
            bpmDisplay.textContent = this.bpmValue || '--';
        }
    },
    
    applyMediaLighting: function(mediaData) {
        const targets = this.multiBulbSelection.length > 0 
            ? this.multiBulbSelection 
            : (this.selectedBulb ? [this.selectedBulb] : ['all']);
        
        if (mediaData.type === 'beat') {
            $.ajax({
                url: '/api/services/lifx/set_state',
                method: 'POST',
                contentType: 'application/json',
                data: JSON.stringify({
                    selector: `id:${targets.join(',')}`,
                    brightness: 100,
                    duration: 0.1
                })
            });
        } else if (mediaData.type === 'color') {
            $.ajax({
                url: '/api/services/lifx/set_color',
                method: 'POST',
                contentType: 'application/json',
                data: JSON.stringify({
                    selector: `id:${targets.join(',')}`,
                    color: `hue:${mediaData.hue} saturation:${mediaData.saturation}%`,
                    duration: 0.3
                })
            });
        }
    },
    
    setMediaSyncMode: function(mode) {
        this.mediaSyncMode = mode;
        localStorage.setItem('lifx_media_sync_mode', mode);
        this.showEnhancedGestureFeedback(`Sync Mode: ${mode}`, '🎵');
    },
    
    setBeatDetectionSensitivity: function(level) {
        this.beatDetectionSensitivity = Math.max(0.3, Math.min(1.0, level));
        localStorage.setItem('lifx_beat_sensitivity', this.beatDetectionSensitivity);
        this.showEnhancedGestureFeedback(`Sensitivity: ${Math.round(this.beatDetectionSensitivity * 100)}%`, '📊');
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
                if (prefs.touchRippleEnabled !== undefined) this.touchRippleEnabled = prefs.touchRippleEnabled;
                if (prefs.showGestureHints !== undefined) this.showGestureHints = prefs.showGestureHints;
                if (prefs.enhancedRippleMode !== undefined) this.enhancedRippleMode = prefs.enhancedRippleMode;
                if (prefs.glowEffectEnabled !== undefined) this.glowEffectEnabled = prefs.glowEffectEnabled;
                if (prefs.highContrastHints !== undefined) this.highContrastHints = prefs.highContrastHints;
                if (prefs.rippleColor) this.rippleColor = prefs.rippleColor;
                if (prefs.rippleSize) this.rippleSize = prefs.rippleSize;
            } catch (e) {
                console.error('Failed to load LIFX preferences:', e);
            }
        }
        
        const touchRipple = localStorage.getItem('lifx_touch_ripple');
        if (touchRipple !== null) this.touchRippleEnabled = touchRipple === 'true';
        
        const gestureHints = localStorage.getItem('lifx_gesture_hints');
        if (gestureHints !== null) this.showGestureHints = gestureHints === 'true';
    },
    
    loadRipplePreferences: function() {
        const enhancedRipple = localStorage.getItem('lifx_enhanced_ripple');
        if (enhancedRipple !== null) this.enhancedRippleMode = enhancedRipple === 'true';
        
        const rippleColor = localStorage.getItem('lifx_ripple_color');
        if (rippleColor !== null) this.rippleColor = rippleColor;
        
        const rippleSize = localStorage.getItem('lifx_ripple_size');
        if (rippleSize !== null) this.rippleSize = parseInt(rippleSize);
        
        const glowEffect = localStorage.getItem('lifx_glow_effect');
        if (glowEffect !== null) this.glowEffectEnabled = glowEffect === 'true';
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
    
    toggleMediaSync: function() {
        this.lifxMediaSyncEnabled = !this.lifxMediaSyncEnabled;
        localStorage.setItem('lifx_media_sync_enabled', this.lifxMediaSyncEnabled);
        
        if (this.lifxMediaSyncEnabled) {
            this.showGestureFeedback('Media Sync ON', '🎵');
        } else {
            this.showGestureFeedback('Media Sync OFF', '🔇');
        }
    },
    
    initEdgeSwipeDetection: function() {
        const edgeZone = this.swipeEdgeZone;
        let touchStartX = 0;
        let touchStartY = 0;
        let edgeSwipeDetected = false;
        let edgeDirection = null;
        
        document.addEventListener('touchstart', (e) => {
            const touch = e.touches[0];
            const screenWidth = window.innerWidth;
            const screenHeight = window.innerHeight;
            
            if (touch.clientX <= edgeZone) {
                edgeSwipeDetected = true;
                edgeDirection = 'left';
                touchStartX = touch.clientX;
                touchStartY = touch.clientY;
                this.isEdgeSwipe = true;
            } else if (touch.clientX >= screenWidth - edgeZone) {
                edgeSwipeDetected = true;
                edgeDirection = 'right';
                touchStartX = touch.clientX;
                touchStartY = touch.clientY;
                this.isEdgeSwipe = true;
            } else if (touch.clientY <= edgeZone) {
                edgeSwipeDetected = true;
                edgeDirection = 'top';
                touchStartX = touch.clientX;
                touchStartY = touch.clientY;
                this.isEdgeSwipe = true;
            } else if (touch.clientY >= screenHeight - edgeZone) {
                edgeSwipeDetected = true;
                edgeDirection = 'bottom';
                touchStartX = touch.clientX;
                touchStartY = touch.clientY;
                this.isEdgeSwipe = true;
            }
        }, { passive: true });
        
        document.addEventListener('touchmove', (e) => {
            if (!this.isEdgeSwipe || !edgeSwipeDetected) return;
            
            const touch = e.touches[0];
            const deltaX = touch.clientX - touchStartX;
            const deltaY = touch.clientY - touchStartY;
            
            this.edgeSwipeDirection = edgeDirection;
            
            if (edgeDirection === 'left' && deltaX > 100) {
                this.handleEdgeSwipe('left');
                this.isEdgeSwipe = false;
                edgeSwipeDetected = false;
            } else if (edgeDirection === 'right' && deltaX < -100) {
                this.handleEdgeSwipe('right');
                this.isEdgeSwipe = false;
                edgeSwipeDetected = false;
            } else if (edgeDirection === 'top' && deltaY > 100) {
                this.handleEdgeSwipe('top');
                this.isEdgeSwipe = false;
                edgeSwipeDetected = false;
            } else if (edgeDirection === 'bottom' && deltaY < -100) {
                this.handleEdgeSwipe('bottom');
                this.isEdgeSwipe = false;
                edgeSwipeDetected = false;
            }
        }, { passive: false });
        
        document.addEventListener('touchend', () => {
            this.isEdgeSwipe = false;
            edgeSwipeDetected = false;
            this.edgeSwipeDirection = null;
        });
    },
    
    handleEdgeSwipe: function(direction) {
        switch(direction) {
            case 'left':
                this.showQuickScenesPanel();
                this.showGestureFeedback('Quick Scenes', '🎨');
                break;
            case 'right':
                this.showMediaControls();
                this.showGestureFeedback('Media Controls', '🎵');
                break;
            case 'top':
                this.adjustBrightness(30);
                this.showGestureFeedback('Brightness Boost', '☀️');
                break;
            case 'bottom':
                this.adjustBrightness(-30);
                this.showGestureFeedback('Dim Lights', '🌙');
                break;
        }
        this.hapticFeedback('success');
    },
    
    showQuickScenesPanel: function() {
        const scenesPanel = document.createElement('div');
        scenesPanel.className = 'quick-scenes-panel';
        scenesPanel.innerHTML = `
            <div class="quick-scenes-content">
                <button class="quick-scenes-close" onclick="LifXTouchControls.closeQuickScenesPanel()">
                    <i class="fas fa-times"></i>
                </button>
                <h4><i class="fas fa-palette"></i> Quick Scenes</h4>
                <div class="quick-scenes-grid">
                    ${['relax', 'focus', 'energize', 'night', 'party', 'movie', 'romance', 'reading'].map(scene => `
                        <button class="quick-scene-btn ${scene}" onclick="LifXTouchControls.applyScene('${scene}'); LifXTouchControls.closeQuickScenesPanel()">
                            <i class="fas fa-lightbulb"></i>
                            <span>${scene.charAt(0).toUpperCase() + scene.slice(1)}</span>
                        </button>
                    `).join('')}
                </div>
            </div>
        `;
        document.body.appendChild(scenesPanel);
        setTimeout(() => scenesPanel.classList.add('visible'), 10);
    },
    
    closeQuickScenesPanel: function() {
        const panel = document.querySelector('.quick-scenes-panel');
        if (panel) {
            panel.classList.remove('visible');
            setTimeout(() => panel.remove(), 300);
        }
    },
    
    showMediaControls: function() {
        const mediaPanel = document.createElement('div');
        mediaPanel.className = 'media-controls-panel';
        mediaPanel.innerHTML = `
            <div class="media-controls-content">
                <button class="media-controls-close" onclick="LifXTouchControls.closeMediaControls()">
                    <i class="fas fa-times"></i>
                </button>
                <h4><i class="fas fa-music"></i> Media Sync Controls</h4>
                <div class="media-sync-options">
                    <button class="media-sync-btn ${this.mediaSyncMode === 'beat' ? 'active' : ''}" onclick="LifXTouchControls.setMediaSyncMode('beat')">
                        <i class="fas fa-heartbeat"></i> Beat Sync
                    </button>
                    <button class="media-sync-btn ${this.mediaSyncMode === 'color' ? 'active' : ''}" onclick="LifXTouchControls.setMediaSyncMode('color')">
                        <i class="fas fa-palette"></i> Color Sync
                    </button>
                    <button class="media-sync-btn ${this.mediaSyncMode === 'ambient' ? 'active' : ''}" onclick="LifXTouchControls.setMediaSyncMode('ambient')">
                        <i class="fas fa-film"></i> Ambient
                    </button>
                </div>
                <div class="bpm-display">
                    <i class="fas fa-heartbeat bpm-icon"></i>
                    <span class="bpm-value">${this.bpmValue || '--'}</span>
                    <span class="bpm-label">BPM</span>
                </div>
            </div>
        `;
        document.body.appendChild(mediaPanel);
        setTimeout(() => mediaPanel.classList.add('visible'), 10);
    },
    
    closeMediaControls: function() {
        const panel = document.querySelector('.media-controls-panel');
        if (panel) {
            panel.classList.remove('visible');
            setTimeout(() => panel.remove(), 300);
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
    },
    
    rainbowCycleInterval: null,
    rainbowHue: 0,
    
    startRainbowCycle: function() {
        if (this.rainbowCycleInterval) return;
        
        const targets = this.multiBulbSelection.length > 0 
            ? this.multiBulbSelection 
            : (this.selectedBulb ? [this.selectedBulb] : ['all']);
        
        this.rainbowCycleInterval = setInterval(() => {
            this.rainbowHue = (this.rainbowHue + 2) % 360;
            
            $.ajax({
                url: '/api/services/lifx/set_color',
                method: 'POST',
                contentType: 'application/json',
                data: JSON.stringify({
                    selector: targets === 'all' ? 'all' : `id:${targets.join(',')}`,
                    color: `hue:${Math.round(this.rainbowHue * 182)} saturation:100%`,
                    brightness: 80,
                    duration: 0.3
                })
            });
        }, 50);
        
        this.showGestureFeedback('Rainbow cycle started', '🌈');
    },
    
    stopRainbowCycle: function() {
        if (this.rainbowCycleInterval) {
            clearInterval(this.rainbowCycleInterval);
            this.rainbowCycleInterval = null;
            this.showGestureFeedback('Rainbow cycle stopped', '🌈');
        }
    },
    
    dynamicSceneInterval: null,
    dynamicSceneParams: null,
    
    applyDynamicScene: function(sceneName, params) {
        this.stopDynamicScene();
        this.dynamicSceneParams = { ...params, sceneName };
        const targets = this.multiBulbSelection.length > 0 
            ? this.multiBulbSelection 
            : (this.selectedBulb ? [this.selectedBulb] : ['all']);
        
        const applyEffect = () => {
            if (!this.dynamicSceneParams) return;
            
            const p = this.dynamicSceneParams;
            let hue = p.hue;
            let brightness = p.brightness;
            let saturation = p.saturation;
            
            switch(p.effect) {
                case 'pulse':
                    brightness = 40 + Math.sin(Date.now() / 500) * (p.brightness * 0.4);
                    break;
                case 'fade':
                    hue = (p.hue + Math.sin(Date.now() / 2000) * 20) % 360;
                    break;
                case 'slow_cycle':
                    hue = (p.hue + (Date.now() / 50) % 60) % 360;
                    break;
                case 'gentle_pulse':
                    brightness = p.brightness * (0.7 + Math.sin(Date.now() / 800) * 0.15);
                    saturation = p.saturation * (0.8 + Math.sin(Date.now() / 600) * 0.1);
                    break;
                case 'vibrant':
                    saturation = Math.min(100, p.saturation + Math.sin(Date.now() / 300) * 15);
                    break;
                case 'energy':
                    if (Date.now() % 1000 < 500) {
                        brightness = p.brightness;
                        saturation = p.saturation;
                    } else {
                        brightness = p.brightness * 0.7;
                        saturation = p.saturation * 0.8;
                    }
                    break;
                case 'sparkle':
                    if (Math.random() < 0.1) {
                        brightness = 100;
                        saturation = 100;
                    } else {
                        brightness = p.brightness;
                        saturation = p.saturation;
                    }
                    break;
                case 'cosmic_pulse':
                    const cosmicTime = Date.now() / 1000;
                    hue = (p.hue + Math.sin(cosmicTime * 0.5) * 40 + Math.cos(cosmicTime * 0.3) * 20) % 360;
                    brightness = p.brightness * (0.6 + Math.sin(cosmicTime * 2) * 0.2 + Math.random() * 0.1);
                    saturation = p.saturation * (0.8 + Math.sin(cosmicTime * 1.5) * 0.2);
                    break;
                case 'dream_flow':
                    const dreamTime = Date.now() / 2000;
                    hue = (p.hue + Math.sin(dreamTime) * 30) % 360;
                    brightness = p.brightness * (0.7 + Math.sin(dreamTime * 0.8) * 0.15);
                    saturation = p.saturation * (0.9 + Math.sin(dreamTime * 0.6) * 0.1);
                    break;
                case 'festival_lights':
                    const festivalTime = Date.now();
                    if (festivalTime % 800 < 400) {
                        brightness = p.brightness;
                        saturation = p.saturation;
                        hue = p.hue;
                    } else {
                        brightness = p.brightness * 0.8;
                        saturation = p.saturation * 0.9;
                        hue = (p.hue + 30) % 360;
                    }
                    if (festivalTime % 3000 < 500) {
                        brightness = 100;
                        saturation = 100;
                    }
                    break;
            }
            
            $.ajax({
                url: '/api/services/lifx/set_color',
                method: 'POST',
                contentType: 'application/json',
                data: JSON.stringify({
                    selector: targets === 'all' ? 'all' : `id:${targets.join(',')}`,
                    color: `hue:${Math.round(hue * 182)} saturation:${Math.round(saturation)}%`,
                    brightness: Math.round(brightness),
                    kelvin: p.kelvin,
                    duration: p.effect === 'sparkle' || p.effect === 'energy' ? 0.1 : 0.5
                })
            });
        };
        
        applyEffect();
        this.dynamicSceneInterval = setInterval(applyEffect, 200);
        this.showGestureFeedback(`Scene: ${sceneName}`, '✨');
        this.hapticFeedback('scene', 0.8);
    },
    
    stopDynamicScene: function() {
        if (this.dynamicSceneInterval) {
            clearInterval(this.dynamicSceneInterval);
            this.dynamicSceneInterval = null;
            this.dynamicSceneParams = null;
        }
    },
    
    initCircadianRhythm: function() {
        if (!this.circadianRhythmEnabled) return;
        
        const adjustCircadian = () => {
            const hour = new Date().getHours();
            const now = Date.now();
            
            if (now - this.lastCircadianAdjustment < 3600000) return;
            
            let kelvin, brightness;
            
            if (hour >= 6 && hour < 9) {
                kelvin = 4000;
                brightness = 0.7;
            } else if (hour >= 9 && hour < 12) {
                kelvin = 5500;
                brightness = 0.9;
            } else if (hour >= 12 && hour < 17) {
                kelvin = 6500;
                brightness = 1.0;
            } else if (hour >= 17 && hour < 21) {
                kelvin = 4000;
                brightness = 0.6;
            } else if (hour >= 21 && hour < 23) {
                kelvin = 2700;
                brightness = 0.4;
            } else {
                kelvin = 2200;
                brightness = 0.2;
            }
            
            $.ajax({
                url: '/api/services/lifx/set_color',
                method: 'POST',
                contentType: 'application/json',
                data: JSON.stringify({
                    selector: 'all',
                    kelvin: kelvin,
                    brightness: brightness,
                    duration: 300
                })
            });
            
            this.lastCircadianAdjustment = now;
            console.log('Circadian adjustment:', { kelvin, brightness });
        };
        
        adjustCircadian();
        setInterval(adjustCircadian, 300000);
    },
    
    initVoiceControl: function() {
        if (!('webkitSpeechRecognition' in window) && !('SpeechRecognition' in window)) {
            console.warn('Voice control not supported');
            return;
        }
        
        const SpeechRecognition = window.SpeechRecognition || window.webkitSpeechRecognition;
        const recognition = new SpeechRecognition();
        
        recognition.continuous = false;
        recognition.interimResults = false;
        recognition.lang = 'en-US';
        
        recognition.onresult = (event) => {
            const command = event.results[0][0].transcript.toLowerCase();
            console.log('LIFX voice command:', command);
            
            if (command.includes('bright') || command.includes('light')) {
                this.adjustBrightness(20);
                this.showGestureFeedback('Brighter', '☀️');
            } else if (command.includes('dim') || command.includes('darker')) {
                this.adjustBrightness(-20);
                this.showGestureFeedback('Dimmer', '🌙');
            } else if (command.includes('warmer')) {
                this.adjustColorTemp(300);
                this.showGestureFeedback('Warmer', '🔶');
            } else if (command.includes('cooler') || command.includes('colder')) {
                this.adjustColorTemp(-300);
                this.showGestureFeedback('Cooler', '❄️');
            } else if (command.includes('relax') || command.includes('relaxing')) {
                this.applyScene('relax');
            } else if (command.includes('focus') || command.includes('concentrate')) {
                this.applyScene('focus');
            } else if (command.includes('movie') || command.includes('film')) {
                this.applyScene('movie');
            } else if (command.includes('party')) {
                this.applyScene('party');
            } else if (command.includes('off') || command.includes('dark')) {
                this.togglePower(false);
            } else if (command.includes('on') || command.includes('light up')) {
                this.togglePower(true);
            } else if (command.includes('rainbow')) {
                this.applyScene('rainbow');
            } else if (command.includes('sunrise')) {
                this.applyScene('sunrise');
            } else if (command.includes('sunset')) {
                this.applyScene('sunset');
            }
        };
        
        recognition.onerror = (event) => {
            console.warn('LIFX voice recognition error:', event.error);
        };
        
        window.startLifxVoiceCommand = () => {
            recognition.start();
            showNotification('Listening for light command...', 'info');
        };
        
        this.voiceControlEnabled = true;
        console.log('LIFX voice control initialized');
    },
    
    saveZonePreset: function(name) {
        const config = {
            bulbs: [...this.multiBulbSelection],
            brightness: this.brightnessLevel,
            colorTemp: this.colorTempLevel,
            scene: this.currentScene
        };
        
        this.zonePresets[name] = config;
        localStorage.setItem('lifx_zone_presets', JSON.stringify(this.zonePresets));
        showNotification(`Zone preset "${name}" saved`, 'success');
    },
    
    loadZonePreset: function(name) {
        const preset = this.zonePresets[name];
        if (!preset) {
            showNotification(`Preset "${name}" not found`, 'warning');
            return;
        }
        
        this.multiBulbSelection = preset.bulbs || [];
        this.brightnessLevel = preset.brightness || 50;
        this.colorTempLevel = preset.colorTemp || 3500;
        
        if (preset.scene) {
            this.applyScene(preset.scene);
        }
        
        this.updateSelectionToolbar();
        showNotification(`Loaded preset "${name}"`, 'success');
    },
    
    loadZonePresets: function() {
        try {
            const stored = localStorage.getItem('lifx_zone_presets');
            if (stored) {
                this.zonePresets = JSON.parse(stored);
            }
        } catch (e) {
            console.warn('Failed to load zone presets:', e);
        }
    },
    
    queueEffect: function(effectName, duration = 5000) {
        this.effectQueue.push({ effect: effectName, duration, addedAt: Date.now() });
        console.log('Effect queued:', effectName);
        
        if (!this.effectActive) {
            this.processEffectQueue();
        }
    },
    
    processEffectQueue: function() {
        if (this.effectQueue.length === 0 || this.effectActive) return;
        
        const nextEffect = this.effectQueue.shift();
        this.effectActive = true;
        
        const originalScene = this.currentScene;
        this.applyScene(nextEffect.effect);
        
        setTimeout(() => {
            this.effectActive = false;
            this.applyScene(originalScene);
            this.processEffectQueue();
        }, nextEffect.duration);
    },
    
    clearEffectQueue: function() {
        this.effectQueue = [];
        this.stopDynamicScene();
        this.effectActive = false;
        showNotification('Light effects cleared', 'info');
    },
    
    initAccessibilityFeatures: function() {
        if (!this.accessibilityMode) return;
        
        document.querySelectorAll('.lifx-bulb-control, .lifx-bulb-card').forEach(el => {
            el.setAttribute('role', 'button');
            el.setAttribute('aria-label', el.getAttribute('data-bulb-name') || 'Light control');
            
            if (this.highContrastHints) {
                el.classList.add('high-contrast');
            }
            
            if (this.reducedMotionMode) {
                el.style.transition = 'none';
            }
        });
        
        console.log('LIFX accessibility features enabled');
    },
    
    setAccessibilityMode: function(enabled) {
        this.accessibilityMode = enabled;
        localStorage.setItem('lifx_accessibility_mode', enabled);
        
        if (enabled) {
            this.initAccessibilityFeatures();
            showNotification('Accessibility mode enabled', 'info');
        } else {
            document.querySelectorAll('.lifx-bulb-control').forEach(el => {
                el.classList.remove('high-contrast');
                el.style.transition = '';
            });
            showNotification('Accessibility mode disabled', 'info');
        }
    },
    
    initFromStorage: function() {
        this.loadZonePresets();
        
        try {
            const accessibilityStored = localStorage.getItem('lifx_accessibility_mode');
            if (accessibilityStored === 'true') {
                this.accessibilityMode = true;
                this.initAccessibilityFeatures();
            }
            
            const circadianStored = localStorage.getItem('lifx_circadian_enabled');
            if (circadianStored === 'true') {
                this.circadianRhythmEnabled = true;
                this.initCircadianRhythm();
            }
        } catch (e) {
            console.warn('Failed to load LIFX preferences:', e);
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
    
    // Initialize additional features
    LifXTouchControls.initFromStorage();
    
    // Expose global functions for voice and accessibility
    window.startLifxVoiceCommand = () => LifXTouchControls.initVoiceControl();
    window.setLifxAccessibilityMode = (enabled) => LifXTouchControls.setAccessibilityMode(enabled);
    window.saveLifxZonePreset = (name) => LifXTouchControls.saveZonePreset(name);
    window.loadLifxZonePreset = (name) => LifXTouchControls.loadZonePreset(name);
    window.queueLifxEffect = (effect, duration) => LifXTouchControls.queueEffect(effect, duration);
    
    console.log('LIFX Touch Controls initialized with enhanced features');
});

// Export for external use
if (typeof module !== 'undefined' && module.exports) {
    module.exports = LifXTouchControls;
}
