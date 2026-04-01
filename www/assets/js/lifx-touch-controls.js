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
    scenes: ['relax', 'focus', 'energize', 'night', 'sunset', 'ocean', 'reading', 'romance', 'party', 'golden', 'arctic', 'tropical', 'spring', 'autumn', 'meditation', 'gaming', 'cooking', 'creative', 'yoga', 'movie', 'study', 'dinner', 'morning', 'goodnight', 'rainbow', 'fireplace', 'ice', 'aurora', 'nebula', 'thunder', 'crystal', 'lagoon', 'cotton_candy', 'spring_blossom', 'punchbowl', 'smashing', 'glitter', 'golden_hour', 'late_night', 'midday', 'polar', 'cosmic', 'dream', 'chill', 'adventure', 'festival', 'bioluminescent', 'cyberpunk', 'vaporwave', 'northern_lights', 'desert_dawn', 'forest_mist', 'volcanic', 'underwater', 'space_station', 'wizard_tower', 'dragon_fire', 'fairy_grove', 'haunted', 'santas_workshop', 'new_year', 'valentines', 'halloween', 'thanksgiving', 'christmas', 'easter', 'st_patricks', 'independence_day'],
    favoriteScenes: [],
    recentScenes: [],
    maxRecentScenes: 5,
    startY: null,
    startBrightness: null,
    startColorTemp: null,
    gestureStartTime: 0,
    lastSwipeDistance: 0,
    isTouchDevice: false,
    gestureHistory: [],
    maxGestureHistory: 10,
    gestureSensitivity: {
        swipeDistance: 40,
        swipeTime: 250,
        pinchDistance: 25,
        longPressDelay: 400,
        doubleTapDelay: 250
    },
    gestureHints: {
        enabled: true,
        position: 'center',
        duration: 1200,
        showIcon: true,
        showValue: true
    },
    touchSensitivityLevels: {
        low: { swipeDistance: 80, swipeTime: 400, pinchDistance: 50 },
        medium: { swipeDistance: 50, swipeTime: 300, pinchDistance: 30 },
        high: { swipeDistance: 30, swipeTime: 200, pinchDistance: 20 },
        very_high: { swipeDistance: 15, swipeTime: 150, pinchDistance: 10 }
    },
    adaptiveSensitivity: {
        enabled: true,
        adjustmentFactor: 0.1,
        minAdjustments: 5,
        currentAdjustments: 0,
        successThreshold: 0.8,
        failThreshold: 0.3
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
    bpmSmoothed: 0,
    lastBeatTime: 0,
    beatDebounce: 100,
    audioAnalyzer: null,
    audioContext: null,
    mediaSyncTargets: [],
    beatFlashEnabled: true,
    touchSensitivity: 'medium',
    swipeEdgeZone: 20,
    lastTouchCoordinates: { x: 0, y: 0 },
    touchVelocity: 0,
    touchDirection: null,
    isEdgeSwipe: false,
    edgeSwipeDirection: null,
    touchpadModeEnabled: false,
    touchpadX: 0,
    touchpadY: 0,
    microGestureEnabled: true,
    microGestureThreshold: 10,
    gestureMacros: {},
    lastMultiTouchDistance: 0,
    multiTouchActive: false,
    radialMenuActive: false,
    radialMenuAngle: 0,
    quickBrightnessStep: 5,
    quickColorTempStep: 100,
    savedGroups: [],
    touchVelocityHistory: [],
    maxVelocityHistory: 5,
    gestureAccuracyScore: 100,
    touchTrailEnabled: true,
    adaptiveSensitivityEnabled: false,
    gestureSuccessCount: 0,
    gestureFailCount: 0,
    lastGestureVelocity: 0,
    touchPressure: 0,
    pressureSensitiveEnabled: false,
    colorWheelActive: false,
    colorWheelAngle: 0,
    colorWheelRadius: 0,
    gestureTrails: [],
    maxGestureTrails: 8,
    touchTrailColor: 'rgba(0, 212, 255, 0.7)',
    audioFrequencyData: null,
    beatHistory: [],
    maxBeatHistory: 8,
    lastBeatEnergy: 0,
    colorFlowPoints: [],
    maxColorFlowPoints: 12,
    threeFingerSwipeActive: false,
    fourFingerSwipeActive: false,
    circularGestureActive: false,
    circularGesturePoints: [],
    zSwipeGesture: false,
    lastZSwipePoints: [],
    wSwipeGesture: false,
    lastWSwipePoints: [],
    doubleSwipeEnabled: true,
    lastSwipeTime: 0,
    lastSwipeDirection: null,
    crossGestureActive: false,
    spiralGestureActive: false,
    spiralPoints: [],
    touchDrawingEnabled: false,
    touchDrawingPath: [],
    lightPaintingActive: false,
    lightPaintingBulbs: [],
    gestureMacroRecording: false,
    recordedGestureSequence: [],
    smartSceneSuggestions: [],
    lastUsedScenes: [],
    adaptiveBrightnessEnabled: false,
    ambientLightLevel: 0,
    autoColorTempAdjustment: false,
    colorPickerActive: false,
    colorPickerElement: null,
    lastColorHue: 180,
    lastColorSaturation: 100,
    quickColorPalette: ['#FF0000', '#FF8800', '#FFFF00', '#00FF00', '#00FFFF', '#0088FF', '#0000FF', '#FF00FF', '#FF88FF', '#FFFFFF', '#FFB6C1', '#87CEEB'],
    smoothTransitionsEnabled: true,
    transitionDuration: 0.5,
    adaptiveColorMode: false,
    circadianSyncEnabled: false,
    sunriseMode: false,
    sunsetMode: false,
    focusTimer: null,
    relaxationTimer: null,
    breathingLightActive: false,
    breathingLightPhase: 0,
    breathingLightSpeed: 0.5,
    zoneControlActive: false,
    activeZones: [],
    zonePresets: {
        'morning': { zones: [1, 2], brightness: 80, kelvin: 5000 },
        'evening': { zones: [3, 4], brightness: 40, kelvin: 2700 },
        'night': { zones: [1, 2, 3, 4], brightness: 20, kelvin: 2000 },
        'party': { zones: [1, 2, 3, 4], brightness: 100, effect: 'pulse' }
    },
    colorFlowActive: false,
    colorFlowDirection: 'clockwise',
    colorFlowSpeed: 1000,
    colorFlowInterval: null,
    touchAccuracyMode: 'precision',
    gestureLearningEnabled: true,
    personalizedGestures: {},
    quickActionPresets: {
        'doubleTap': 'toggle_power',
        'longPress': 'brightness_adjust',
        'swipeUp': 'scene_next',
        'swipeDown': 'scene_prev'
    },
    touchZones: {},
    activeZone: null,
    zoneBoundaries: { top: 0, bottom: 0, left: 0, right: 0 },
    lastTouchZone: null,
    touchTrailPoints: [],
    maxTrailPoints: 20,
    gestureSoundEnabled: false,
    gestureSounds: {},
    adaptiveBrightnessActive: false,
    ambientLightSensor: null,
    voiceCommandActive: false,
    voiceTimeout: null,
    quickSettingsPanel: null,
    colorWheelPanel: null,
    sceneFavoritesPanel: null,
    multiSelectMode: false,
    selectionBox: null,
    isSelecting: false,
    selectionStart: null,
    selectionEnd: null,
    isModalOpen: function() {
        return !!(document.querySelector('.swal2-show') || 
                  document.querySelector('.modal.show') ||
                  document.querySelector('.media-sync-panel.visible') ||
                  document.querySelector('.quick-scenes-panel.visible') ||
                  document.querySelector('.media-controls-panel.visible') ||
                  document.querySelector('.favorite-scenes-panel.visible') ||
                  document.querySelector('.touch-gesture-tutorial'));
    },
    
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
                    const currentBrightness = this.brightnessLevel;
                    const velocity = data.velocity || this.lastGestureVelocity;
                    const step = Math.max(10, Math.floor(velocity * 15));
                    this.adjustBrightness(step);
                    this.recordGesture('brightness', step, currentBrightness, [bulb]);
                    this.showGestureFeedback(`Brightness +${step}%`, '↑', 1000, velocity);
                    this.hapticFeedback('brightness', Math.min(1.0, 0.5 + velocity * 0.2));
                    this.recordGestureSuccess();
                    this.adjustSensitivity(true, 'swipeUp');
                }
            });
            
            onGesture('swipeDown', (data) => {
                if (!this.checkGestureDebounce()) return;
                const bulb = this.selectedBulb || this.getFirstSelectedBulb();
                if (bulb) {
                    const currentBrightness = this.brightnessLevel;
                    const velocity = data.velocity || this.lastGestureVelocity;
                    const step = Math.max(10, Math.floor(velocity * 15));
                    this.adjustBrightness(-step);
                    this.recordGesture('brightness', -step, currentBrightness, [bulb]);
                    this.showGestureFeedback(`Brightness -${step}%`, '↓', 1000, velocity);
                    this.hapticFeedback('brightness', Math.min(1.0, 0.5 + velocity * 0.2));
                    this.recordGestureSuccess();
                    this.adjustSensitivity(true, 'swipeDown');
                }
            });
            
            // Swipe left/right to adjust color temperature
            onGesture('swipeRight', (data) => {
                if (!this.checkGestureDebounce()) return;
                const bulb = this.selectedBulb || this.getFirstSelectedBulb();
                if (bulb) {
                    const currentTemp = this.colorTempLevel;
                    const velocity = data.velocity || this.lastGestureVelocity;
                    const step = Math.max(200, Math.floor(velocity * 300));
                    this.adjustColorTemp(step);
                    this.recordGesture('colorTemp', step, currentTemp, [bulb]);
                    this.showGestureFeedback('Warmer', '☀️', 1000, velocity);
                    this.hapticFeedback('colortemp', Math.min(1.0, 0.5 + velocity * 0.2));
                    this.recordGestureSuccess();
                    this.adjustSensitivity(true, 'swipeRight');
                }
            });
            
            onGesture('swipeLeft', (data) => {
                if (!this.checkGestureDebounce()) return;
                const bulb = this.selectedBulb || this.getFirstSelectedBulb();
                if (bulb) {
                    const currentTemp = this.colorTempLevel;
                    const velocity = data.velocity || this.lastGestureVelocity;
                    const step = Math.max(200, Math.floor(velocity * 300));
                    this.adjustColorTemp(-step);
                    this.recordGesture('colorTemp', -step, currentTemp, [bulb]);
                    this.showGestureFeedback('Cooler', '❄️', 1000, velocity);
                    this.hapticFeedback('colortemp', Math.min(1.0, 0.5 + velocity * 0.2));
                    this.recordGestureSuccess();
                    this.adjustSensitivity(true, 'swipeLeft');
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
                    this.recordGestureSuccess();
                    this.adjustSensitivity(true, 'pinchOut');
                }
            });
            
            onGesture('pinchIn', (data) => {
                if (!this.checkGestureDebounce()) return;
                const bulb = this.selectedBulb || this.getFirstSelectedBulb();
                if (bulb) {
                    this.previousScene();
                    this.showGestureFeedback('Previous Scene', '🎨');
                    this.hapticFeedback('success');
                    this.recordGestureSuccess();
                    this.adjustSensitivity(true, 'pinchIn');
                }
            });
            
            // Long press for quick settings
            onGesture('longPress', (data) => {
                const bulbEl = document.elementFromPoint(data.x, data.y)?.closest('.lifx-bulb-control');
                if (bulbEl) {
                    this.selectBulb(bulbEl.getAttribute('data-bulb-id'));
                    setTimeout(() => this.openQuickSettings(), 100);
                    this.recordGestureSuccess();
                }
            });
            
            // Three-finger swipe up for party mode
            onGesture('threeFingerSwipeUp', (data) => {
                if (!this.checkGestureDebounce()) return;
                this.activatePartyMode();
                this.showEnhancedGestureFeedback('Party Mode Activated', '🎉');
                this.hapticFeedback('success');
                this.recordGestureSuccess();
            });
            
            // Three-finger swipe down for calm mode
            onGesture('threeFingerSwipeDown', (data) => {
                if (!this.checkGestureDebounce()) return;
                this.activateCalmMode();
                this.showEnhancedGestureFeedback('Calm Mode Activated', '🧘');
                this.hapticFeedback('light');
                this.recordGestureSuccess();
            });
            
            // Three-finger swipe left for focus mode
            onGesture('threeFingerSwipeLeft', (data) => {
                if (!this.checkGestureDebounce()) return;
                this.applyScene('focus');
                this.showEnhancedGestureFeedback('Focus Mode', '🎯');
                this.hapticFeedback('success');
                this.recordGestureSuccess();
            });
            
            // Three-finger swipe right for relax mode
            onGesture('threeFingerSwipeRight', (data) => {
                if (!this.checkGestureDebounce()) return;
                this.applyScene('relax');
                this.showEnhancedGestureFeedback('Relax Mode', '😌');
                this.hapticFeedback('success');
                this.recordGestureSuccess();
            });
            
            // Circular gesture for color cycle
            onGesture('circular', (data) => {
                if (!this.checkGestureDebounce()) return;
                this.toggleColorCycle();
                this.showEnhancedGestureFeedback('Color Cycle ' + (this.colorCycleActive ? 'ON' : 'OFF'), '🔄');
                this.hapticFeedback('scene_change');
                this.recordGestureSuccess();
            });
            
            // Z-swipe for scene shuffle
            onGesture('zSwipe', (data) => {
                if (!this.checkGestureDebounce()) return;
                this.randomScene();
                this.showEnhancedGestureFeedback('Random Scene', '🎲');
                this.hapticFeedback('success');
                this.recordGestureSuccess();
            });
            
            // Cross gesture for all lights on
            onGesture('cross', (data) => {
                if (!this.checkGestureDebounce()) return;
                this.powerAllSelected('on');
                this.showEnhancedGestureFeedback('All Lights ON', '💡');
                this.hapticFeedback('success');
                this.recordGestureSuccess();
            });
            
            // Inverse cross for all lights off
            onGesture('inverseCross', (data) => {
                if (!this.checkGestureDebounce()) return;
                this.powerAllSelected('off');
                this.showEnhancedGestureFeedback('All Lights OFF', '🌙');
                this.hapticFeedback('light');
                this.recordGestureSuccess();
            });
            
            // Four-finger swipe up for maximum brightness all
            onGesture('fourFingerSwipeUp', (data) => {
                if (!this.checkGestureDebounce()) return;
                this.brightnessLevel = 100;
                $.ajax({
                    url: '/api/services/lifx/set_state',
                    method: 'POST',
                    contentType: 'application/json',
                    data: JSON.stringify({ selector: 'all', brightness: 1, duration: 0.3 }),
                    success: () => {
                        this.showEnhancedGestureFeedback('Maximum Brightness All', '☀️');
                        this.hapticFeedback('success');
                        this.recordGestureSuccess();
                    }
                });
            });
            
            // Four-finger swipe down for night mode all
            onGesture('fourFingerSwipeDown', (data) => {
                if (!this.checkGestureDebounce()) return;
                this.brightnessLevel = 10;
                this.colorTempLevel = 2000;
                $.ajax({
                    url: '/api/services/lifx/set_state',
                    method: 'POST',
                    contentType: 'application/json',
                    data: JSON.stringify({ selector: 'all', brightness: 0.1, duration: 0.3 }),
                    success: () => {
                        $.ajax({
                            url: '/api/services/lifx/set_color',
                            method: 'POST',
                            contentType: 'application/json',
                            data: JSON.stringify({ selector: 'all', color: 'kelvin:2000' }),
                            success: () => {
                                this.showEnhancedGestureFeedback('Night Mode All', '🌙');
                                this.hapticFeedback('light');
                                this.recordGestureSuccess();
                            }
                        });
                    }
                });
            });
            
            // Edge swipe from left for media sync toggle
            onGesture('edgeSwipeLeft', (data) => {
                if (!this.checkGestureDebounce()) return;
                this.toggleMediaSync();
                this.showEnhancedGestureFeedback('Media Sync ' + (this.mediaSyncActive ? 'ON' : 'OFF'), '🎵');
                this.hapticFeedback('media');
                this.recordGestureSuccess();
            });
            
            // Edge swipe from right for circadian mode toggle
            onGesture('edgeSwipeRight', (data) => {
                if (!this.checkGestureDebounce()) return;
                this.toggleCircadianMode();
                this.showEnhancedGestureFeedback('Circadian ' + (this.circadianModeActive ? 'ON' : 'OFF'), '🕐');
                this.hapticFeedback('success');
                this.recordGestureSuccess();
            });
            
            // Double swipe up for brightness boost
            onGesture('doubleSwipeUp', (data) => {
                if (!this.checkGestureDebounce()) return;
                this.brightnessLevel = 100;
                this.adjustBrightness(0);
                this.showEnhancedGestureFeedback('Maximum Brightness', '☀️');
                this.hapticFeedback('success');
                this.recordGestureSuccess();
            });
            
            // Double swipe down for minimum brightness
            onGesture('doubleSwipeDown', (data) => {
                if (!this.checkGestureDebounce()) return;
                this.brightnessLevel = 10;
                this.adjustBrightness(0);
                this.showEnhancedGestureFeedback('Night Mode', '🌙');
                this.hapticFeedback('light');
                this.recordGestureSuccess();
            });
            
            // Spiral gesture for light painting mode
            onGesture('spiral', (data) => {
                if (!this.checkGestureDebounce()) return;
                this.toggleLightPainting();
                this.showEnhancedGestureFeedback('Light Painting ' + (this.lightPaintingActive ? 'ON' : 'OFF'), '🎨');
                this.hapticFeedback('scene_change');
                this.recordGestureSuccess();
            });
            
            // W-swipe for wave effect
            onGesture('wSwipe', (data) => {
                if (!this.checkGestureDebounce()) return;
                this.activateWaveEffect();
                this.showEnhancedGestureFeedback('Wave Effect', '🌊');
                this.hapticFeedback('success');
                this.recordGestureSuccess();
            });
            
            // Diamond gesture for disco mode
            onGesture('diamond', (data) => {
                if (!this.checkGestureDebounce()) return;
                this.toggleDiscoMode();
                this.showEnhancedGestureFeedback('Disco Mode ' + (this.discoModeActive ? 'ON' : 'OFF'), '💃');
                this.hapticFeedback('scene_change');
                this.recordGestureSuccess();
            });
        }
        
        // Initialize touch zones for spatial awareness
        this.initTouchZones();
        
        // Setup multi-select drag gesture
        this.initMultiSelectGesture();
        
        // Initialize touch trail visualization
        this.initTouchTrail();
        
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
            if (this.isModalOpen()) return;
            
            const bulbEl = e.target.closest('.lifx-bulb-control, .lifx-bulb-card');
            if (bulbEl) {
                const bulbId = bulbEl.getAttribute('data-bulb-id');
                this.showTouchHoldProgress();
                this.touchHoldTimer = setTimeout(() => {
                    this.startBrightnessAdjustment(bulbId, e.touches[0].clientY);
                    this.hideTouchHoldProgress();
                }, this.touchHoldDelay);
            }
            
            // Enhanced touch tracking for velocity and pressure
            this.initEnhancedTouchTracking(e);
        }, { passive: true });
        
        document.addEventListener('touchmove', (e) => {
            if (this.isModalOpen()) return;
            
            if (this.touchHoldTimer && this.selectedBulb) {
                e.preventDefault();
                const touch = e.touches[0];
                this.adjustBrightnessByTouch(touch.clientY);
            }
            
            // Update velocity tracking
            this.updateTouchVelocity(e);
        }, { passive: false });
        
        document.addEventListener('touchend', (e) => {
            if (this.isModalOpen()) return;
            
            if (this.touchHoldTimer) {
                clearTimeout(this.touchHoldTimer);
                this.touchHoldTimer = null;
            }
            if (this.selectedBulb) {
                this.endBrightnessAdjustment();
            }
            this.hideTouchHoldProgress();
            
            // Process gesture velocity on touch end
            this.processGestureVelocity(e);
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
                    case ' ':
                        e.preventDefault();
                        this.togglePower();
                        break;
                    case 'c':
                    case 'C':
                        e.preventDefault();
                        this.showQuickColorPicker();
                        break;
                }
            }
        });
        
        this.initColorPicker();
    },
    
    initColorPicker: function() {
        const colorPickerContainer = document.getElementById('lifx-color-picker-container');
        if (!colorPickerContainer) return;
        
        colorPickerContainer.innerHTML = `
            <div id="lifx-quick-color-picker" class="lifx-quick-color-picker" style="display: none;">
                <div class="color-picker-header">
                    <span>Quick Color</span>
                    <button class="btn-close" onclick="LifXTouchControls.hideQuickColorPicker()">×</button>
                </div>
                <div class="color-palette-grid" id="color-palette-grid"></div>
                <div class="color-sliders">
                    <div class="slider-row">
                        <label>Hue</label>
                        <input type="range" id="hue-slider" min="0" max="360" value="${this.lastColorHue}" />
                    </div>
                    <div class="slider-row">
                        <label>Saturation</label>
                        <input type="range" id="saturation-slider" min="0" max="100" value="${this.lastColorSaturation}" />
                    </div>
                    <div class="slider-row">
                        <label>Brightness</label>
                        <input type="range" id="brightness-slider" min="0" max="100" value="${this.brightnessLevel}" />
                    </div>
                </div>
                <div class="color-preview">
                    <div id="color-preview-box" style="background: hsl(${this.lastColorHue}, ${this.lastColorSaturation}%, 50%)"></div>
                    <span id="color-hex-value">#FF00FF</span>
                </div>
            </div>
            <div id="lifx-zone-picker" class="lifx-zone-picker" style="display: none;">
                <div class="color-picker-header">
                    <span>Zone Control</span>
                    <button class="btn-close" onclick="LifXTouchControls.hideZonePicker()">×</button>
                </div>
                <div class="zone-grid" id="zone-grid"></div>
                <div class="zone-presets" id="zone-presets"></div>
            </div>
            <div id="lifx-breathing-light-panel" class="lifx-breathing-panel" style="display: none;">
                <div class="color-picker-header">
                    <span>Breathing Light</span>
                    <button class="btn-close" onclick="LifXTouchControls.hideBreathingPanel()">×</button>
                </div>
                <div class="breathing-controls">
                    <div class="slider-row">
                        <label>Speed</label>
                        <input type="range" id="breathing-speed" min="0.2" max="2" step="0.1" value="${this.breathingLightSpeed}" />
                    </div>
                    <button class="btn-breathing-toggle" onclick="LifXTouchControls.toggleBreathingLight()">
                        <i class="fas fa-lungs"></i> <span class="breathing-status">Start</span>
                    </button>
                </div>
            </div>
        `;
        
        this.setupColorPickerEvents();
        this.setupZonePickerEvents();
        this.setupBreathingControlEvents();
    },
    
    setupColorPickerEvents: function() {
        const paletteGrid = document.getElementById('color-palette-grid');
        if (paletteGrid) {
            paletteGrid.innerHTML = this.quickColorPalette.map(color => 
                `<div class="color-swatch" style="background: ${color}" data-color="${color}"></div>`
            ).join('');
            
            paletteGrid.addEventListener('click', (e) => {
                const swatch = e.target.closest('.color-swatch');
                if (swatch) {
                    this.applyColor(swatch.dataset.color);
                }
            });
        }
        
        const hueSlider = document.getElementById('hue-slider');
        const saturationSlider = document.getElementById('saturation-slider');
        const brightnessSlider = document.getElementById('brightness-slider');
        const previewBox = document.getElementById('color-preview-box');
        const hexValue = document.getElementById('color-hex-value');
        
        const updateColorPreview = () => {
            const h = hueSlider.value;
            const s = saturationSlider.value;
            const b = brightnessSlider.value;
            this.lastColorHue = h;
            this.lastColorSaturation = s;
            this.brightnessLevel = b;
            
            previewBox.style.background = `hsl(${h}, ${s}%, ${b/2}%)`;
            
            const rgb = this.hslToRgb(h/360, s/100, b/200);
            hexValue.textContent = this.rgbToHex(rgb[0], rgb[1], rgb[2]);
        };
        
        hueSlider.addEventListener('input', updateColorPreview);
        saturationSlider.addEventListener('input', updateColorPreview);
        brightnessSlider.addEventListener('input', updateColorPreview);
        
        hueSlider.addEventListener('change', () => this.applyHSLColor());
        saturationSlider.addEventListener('change', () => this.applyHSLColor());
        brightnessSlider.addEventListener('change', () => this.applyBrightnessFromSlider());
    },
    
    setupZonePickerEvents: function() {
        const zoneGrid = document.getElementById('zone-grid');
        if (!zoneGrid) return;
        
        const zones = [
            { id: 1, name: 'Zone 1', icon: '🏠' },
            { id: 2, name: 'Zone 2', icon: '🛋️' },
            { id: 3, name: 'Zone 3', icon: '🍽️' },
            { id: 4, name: 'Zone 4', icon: '🛏️' }
        ];
        
        zoneGrid.innerHTML = zones.map(zone => `
            <div class="zone-item" data-zone-id="${zone.id}">
                <span class="zone-icon">${zone.icon}</span>
                <span class="zone-name">${zone.name}</span>
                <button class="zone-toggle-btn" onclick="LifXTouchControls.toggleZone(${zone.id})">
                    <i class="fas fa-toggle-off"></i>
                </button>
            </div>
        `).join('');
        
        const presetsContainer = document.getElementById('zone-presets');
        if (presetsContainer) {
            presetsContainer.innerHTML = `
                <div class="zone-preset-title">Quick Presets</div>
                <div class="zone-preset-buttons">
                    <button class="btn-zone-preset" onclick="LifXTouchControls.applyZonePreset('morning')">
                        <i class="fas fa-sun"></i> Morning
                    </button>
                    <button class="btn-zone-preset" onclick="LifXTouchControls.applyZonePreset('evening')">
                        <i class="fas fa-cloud-sun"></i> Evening
                    </button>
                    <button class="btn-zone-preset" onclick="LifXTouchControls.applyZonePreset('night')">
                        <i class="fas fa-moon"></i> Night
                    </button>
                    <button class="btn-zone-preset" onclick="LifXTouchControls.applyZonePreset('party')">
                        <i class="fas fa-party-horn"></i> Party
                    </button>
                </div>
            `;
        }
    },
    
    setupBreathingControlEvents: function() {
        const breathingSpeed = document.getElementById('breathing-speed');
        if (breathingSpeed) {
            breathingSpeed.addEventListener('input', (e) => {
                this.breathingLightSpeed = parseFloat(e.target.value);
                if (this.breathingLightActive) {
                    this.stopBreathingLight();
                    this.startBreathingLight();
                }
            });
        }
    },
    
    showZonePicker: function() {
        const picker = document.getElementById('lifx-zone-picker');
        if (picker) {
            picker.style.display = 'block';
            setTimeout(() => picker.classList.add('visible'), 10);
            this.zoneControlActive = true;
            this.hapticFeedback('light');
            this.renderZoneStatus();
        }
    },
    
    hideZonePicker: function() {
        const picker = document.getElementById('lifx-zone-picker');
        if (picker) {
            picker.classList.remove('visible');
            setTimeout(() => picker.style.display = 'none', 300);
            this.zoneControlActive = false;
        }
    },
    
    renderZoneStatus: function() {
        const zoneGrid = document.getElementById('zone-grid');
        if (!zoneGrid) return;
        
        zoneGrid.querySelectorAll('.zone-item').forEach(item => {
            const zoneId = parseInt(item.dataset.zoneId);
            const isActive = this.activeZones.includes(zoneId);
            const btn = item.querySelector('.zone-toggle-btn i');
            if (btn) {
                btn.className = isActive ? 'fas fa-toggle-on' : 'fas fa-toggle-off';
                item.classList.toggle('zone-active', isActive);
            }
        });
    },
    
    toggleZone: function(zoneId) {
        const index = this.activeZones.indexOf(zoneId);
        if (index > -1) {
            this.activeZones.splice(index, 1);
            this.showGestureFeedback(`Zone ${zoneId} OFF`, '🔴');
        } else {
            this.activeZones.push(zoneId);
            this.showGestureFeedback(`Zone ${zoneId} ON`, '🟢');
        }
        this.hapticFeedback('light');
        this.renderZoneStatus();
    },
    
    applyZonePreset: function(presetName) {
        const preset = this.zonePresets[presetName];
        if (!preset) return;
        
        this.activeZones = preset.zones;
        this.renderZoneStatus();
        
        $.ajax({
            url: '/api/services/lifx/zone_control',
            method: 'POST',
            contentType: 'application/json',
            data: JSON.stringify({
                zones: preset.zones,
                brightness: preset.brightness,
                kelvin: preset.kelvin,
                effect: preset.effect || null,
                duration: this.transitionDuration
            }),
            success: () => {
                this.showGestureFeedback(`Applied ${presetName} preset`, '✅');
                this.hapticFeedback('success');
            }
        });
    },
    
    showBreathingPanel: function() {
        const panel = document.getElementById('lifx-breathing-light-panel');
        if (panel) {
            panel.style.display = 'block';
            setTimeout(() => panel.classList.add('visible'), 10);
            this.updateBreathingButton();
        }
    },
    
    hideBreathingPanel: function() {
        const panel = document.getElementById('lifx-breathing-light-panel');
        if (panel) {
            panel.classList.remove('visible');
            setTimeout(() => panel.style.display = 'none', 300);
        }
    },
    
    toggleBreathingLight: function() {
        if (this.breathingLightActive) {
            this.stopBreathingLight();
        } else {
            this.startBreathingLight();
        }
    },
    
    startBreathingLight: function() {
        this.breathingLightActive = true;
        this.updateBreathingButton();
        this.showGestureFeedback('Breathing Light ON', '🫁');
        this.hapticFeedback('success');
        
        const animate = () => {
            if (!this.breathingLightActive) return;
            
            this.breathingLightPhase += this.breathingLightSpeed * 0.05;
            const brightness = 30 + Math.sin(this.breathingLightPhase) * 20;
            const kelvin = 2000 + Math.sin(this.breathingLightPhase * 0.5) * 500;
            
            if (this.selectedBulb) {
                $.ajax({
                    url: '/api/services/lifx/set_state',
                    method: 'POST',
                    contentType: 'application/json',
                    data: JSON.stringify({
                        bulb_id: this.selectedBulb,
                        brightness: brightness / 100,
                        kelvin: Math.round(kelvin),
                        duration: 0.1
                    })
                });
            }
            
            requestAnimationFrame(animate);
        };
        
        this.breathingLightPhase = 0;
        animate();
    },
    
    stopBreathingLight: function() {
        this.breathingLightActive = false;
        this.updateBreathingButton();
        this.showGestureFeedback('Breathing Light OFF', '⬛');
        this.hapticFeedback('light');
    },
    
    updateBreathingButton: function() {
        const btn = document.querySelector('.btn-breathing-toggle');
        if (btn) {
            const status = btn.querySelector('.breathing-status');
            if (this.breathingLightActive) {
                btn.classList.add('active');
                status.textContent = 'Stop';
            } else {
                btn.classList.remove('active');
                status.textContent = 'Start';
            }
        }
    },
    
    showQuickColorPicker: function() {
        const picker = document.getElementById('lifx-quick-color-picker');
        if (picker) {
            picker.style.display = 'block';
            this.colorPickerActive = true;
            this.hapticFeedback('light');
        }
    },
    
    hideQuickColorPicker: function() {
        const picker = document.getElementById('lifx-quick-color-picker');
        if (picker) {
            picker.style.display = 'none';
            this.colorPickerActive = false;
        }
    },
    
    applyColor: function(hexColor) {
        const bulb = this.selectedBulb || this.getFirstSelectedBulb();
        if (!bulb) {
            this.showGestureFeedback('Select a bulb first', '💡');
            return;
        }
        
        const rgb = this.hexToRgb(hexColor);
        if (!rgb) return;
        
        const hsl = this.rgbToHsl(rgb[0], rgb[1], rgb[2]);
        this.lastColorHue = hsl[0] * 360;
        this.lastColorSaturation = hsl[1] * 100;
        
        if (typeof sendLifxCommand !== 'undefined') {
            sendLifxCommand('set_color', {
                bulb_id: bulb,
                hue: this.lastColorHue,
                saturation: this.lastColorSaturation,
                brightness: this.brightnessLevel / 100
            });
        }
        
        document.getElementById('color-preview-box').style.background = hexColor;
        document.getElementById('color-hex-value').textContent = hexColor;
        
        this.showGestureFeedback(`Color: ${hexColor}`, '🎨');
        this.hapticFeedback('success');
    },
    
    applyHSLColor: function() {
        const bulb = this.selectedBulb || this.getFirstSelectedBulb();
        if (!bulb) return;
        
        if (typeof sendLifxCommand !== 'undefined') {
            sendLifxCommand('set_color', {
                bulb_id: bulb,
                hue: this.lastColorHue,
                saturation: this.lastColorSaturation,
                brightness: this.brightnessLevel / 100
            });
        }
        
        this.showGestureFeedback('Color updated', '🎨');
    },
    
    applyBrightnessFromSlider: function() {
        const bulb = this.selectedBulb || this.getFirstSelectedBulb();
        if (!bulb) return;
        
        this.adjustBrightness(0);
        this.showGestureFeedback(`Brightness: ${this.brightnessLevel}%`, '💡');
    },
    
    hslToRgb: function(h, s, l) {
        let r, g, b;
        if (s === 0) {
            r = g = b = l;
        } else {
            const hue2rgb = (p, q, t) => {
                if (t < 0) t += 1;
                if (t > 1) t -= 1;
                if (t < 1/6) return p + (q - p) * 6 * t;
                if (t < 1/2) return q;
                if (t < 2/3) return p + (q - p) * (2/3 - t) * 6;
                return p;
            };
            const q = l < 0.5 ? l * (1 + s) : l + s - l * s;
            const p = 2 * l - q;
            r = hue2rgb(p, q, h + 1/3);
            g = hue2rgb(p, q, h);
            b = hue2rgb(p, q, h - 1/3);
        }
        return [Math.round(r * 255), Math.round(g * 255), Math.round(b * 255)];
    },
    
    rgbToHsl: function(r, g, b) {
        r /= 255; g /= 255; b /= 255;
        const max = Math.max(r, g, b), min = Math.min(r, g, b);
        let h, s, l = (max + min) / 2;
        if (max === min) {
            h = s = 0;
        } else {
            const d = max - min;
            s = l > 0.5 ? d / (2 - max - min) : d / (max + min);
            switch (max) {
                case r: h = (g - b) / d + (g < b ? 6 : 0); break;
                case g: h = (b - r) / d + 2; break;
                case b: h = (r - g) / d + 4; break;
            }
            h /= 6;
        }
        return [h, s, l];
    },
    
    rgbToHex: function(r, g, b) {
        return '#' + [r, g, b].map(x => {
            const hex = Math.round(x).toString(16);
            return hex.length === 1 ? '0' + hex : hex;
        }).join('').toUpperCase();
    },
    
    hexToRgb: function(hex) {
        const result = /^#?([a-f\d]{2})([a-f\d]{2})([a-f\d]{2})$/i.exec(hex);
        return result ? [
            parseInt(result[1], 16),
            parseInt(result[2], 16),
            parseInt(result[3], 16)
        ] : null;
    },
    
    checkGestureDebounce: function() {
        const now = Date.now();
        if (now - this.lastGestureTime < this.gestureDebounce) {
            this.recordGestureFail('debounced');
            this.adjustSensitivity(false, 'debounced');
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
        this.isLassoGestureActive = false;
        this.lassoPath = [];
        this.lassoSvg = null;
        this.lassoPathElement = null;
        
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
                    this.isLassoGestureActive = true;
                    this.lassoPath = [{ x: e.touches[0].clientX, y: e.touches[0].clientY }];
                    this.dragSelectionState = !bulbEl.classList.contains('selected');
                    this.hapticFeedback('light');
                    this.createLassoVisual();
                }, this.touchHoldDelay);
            }
        }, { passive: true });
        
        document.addEventListener('touchmove', (e) => {
            if (!this.isLassoGestureActive) return;
            e.preventDefault();
            const touch = e.touches[0];
            const currentPoint = { x: touch.clientX, y: touch.clientY };
            this.lassoPath.push(currentPoint);
            this.updateLassoVisual(currentPoint);
            
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
            if (this.isLassoGestureActive) {
                this.closeLassoVisual();
                this.isLassoGestureActive = false;
                this.lassoPath = [];
            }
        });
    },
    
    createLassoVisual: function() {
        if (this.lassoSvg) return;
        
        this.lassoSvg = document.createElementNS('http://www.w3.org/2000/svg', 'svg');
        this.lassoSvg.style.position = 'fixed';
        this.lassoSvg.style.top = '0';
        this.lassoSvg.style.left = '0';
        this.lassoSvg.style.width = '100%';
        this.lassoSvg.style.height = '100%';
        this.lassoSvg.style.pointerEvents = 'none';
        this.lassoSvg.style.zIndex = '9999';
        
        this.lassoPathElement = document.createElementNS('http://www.w3.org/2000/svg', 'path');
        this.lassoPathElement.setAttribute('fill', 'rgba(0, 212, 255, 0.2)');
        this.lassoPathElement.setAttribute('stroke', 'rgba(0, 212, 255, 0.8)');
        this.lassoPathElement.setAttribute('stroke-width', '3');
        this.lassoPathElement.setAttribute('stroke-dasharray', '10,5');
        
        this.lassoSvg.appendChild(this.lassoPathElement);
        document.body.appendChild(this.lassoSvg);
    },
    
    updateLassoVisual: function(currentPoint) {
        if (!this.lassoSvg || this.lassoPath.length < 2) return;
        
        let pathData = `M ${this.lassoPath[0].x} ${this.lassoPath[0].y}`;
        for (let i = 1; i < this.lassoPath.length; i++) {
            pathData += ` L ${this.lassoPath[i].x} ${this.lassoPath[i].y}`;
        }
        
        this.lassoPathElement.setAttribute('d', pathData);
    },
    
    closeLassoVisual: function() {
        if (!this.lassoSvg) return;
        
        let pathData = `M ${this.lassoPath[0].x} ${this.lassoPath[0].y}`;
        for (let i = 1; i < this.lassoPath.length; i++) {
            pathData += ` L ${this.lassoPath[i].x} ${this.lassoPath[i].y}`;
        }
        pathData += ' Z';
        
        this.lassoPathElement.setAttribute('d', pathData);
        this.lassoPathElement.style.transition = 'opacity 0.3s ease';
        this.lassoPathElement.style.opacity = '0';
        
        setTimeout(() => {
            if (this.lassoSvg.parentNode) {
                this.lassoSvg.parentNode.removeChild(this.lassoSvg);
            }
            this.lassoSvg = null;
            this.lassoPathElement = null;
        }, 300);
        
        this.showEnhancedGestureFeedback(`${this.multiBulbSelection.length} bulbs selected`, '💡');
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
    
    adjustBrightnessBatch: function(delta) {
        if (this.multiBulbSelection.length === 0) return;
        
        $.ajax({
            url: '/api/services/lifx/set_state',
            method: 'POST',
            contentType: 'application/json',
            data: JSON.stringify({
                selector: `id:${this.multiBulbSelection.join(',')}`,
                brightness: Math.max(0, Math.min(100, this.brightnessLevel + delta)) / 100,
                duration: 0.3
            }),
            success: () => {
                this.brightnessLevel = Math.max(0, Math.min(100, this.brightnessLevel + delta));
                this.showGestureFeedback(
                    `Brightness ${delta > 0 ? '+' : ''}${delta} (${this.multiBulbSelection.length} bulbs)`,
                    delta > 0 ? '☀️' : '🌙'
                );
                this.hapticFeedback('light');
            }
        });
    },
    
    adjustColorTempBatch: function(delta) {
        if (this.multiBulbSelection.length === 0) return;
        
        const newTemp = Math.max(1500, Math.min(9000, this.colorTempLevel + delta));
        
        $.ajax({
            url: '/api/services/lifx/set_state',
            method: 'POST',
            contentType: 'application/json',
            data: JSON.stringify({
                selector: `id:${this.multiBulbSelection.join(',')}`,
                color: `kelvin:${newTemp}`,
                duration: 0.3
            }),
            success: () => {
                this.colorTempLevel = newTemp;
                this.showGestureFeedback(
                    `Color Temp ${newTemp}K (${this.multiBulbSelection.length} bulbs)`,
                    delta > 0 ? '🔥' : '❄️'
                );
                this.hapticFeedback('light');
            }
        });
    },
    
    applySceneBatch: function(sceneName) {
        if (this.multiBulbSelection.length === 0) return;
        
        $.ajax({
            url: '/api/services/lifx/apply_scene',
            method: 'POST',
            contentType: 'application/json',
            data: JSON.stringify({
                scene: sceneName,
                selector: `id:${this.multiBulbSelection.join(',')}`,
                duration: 0.5
            }),
            success: () => {
                this.currentScene = sceneName;
                this.showGestureFeedback(
                    `Scene "${sceneName}" applied to ${this.multiBulbSelection.length} bulbs`,
                    '🎨'
                );
                this.hapticFeedback('success');
                this.multiBulbSelection = [];
                this.updateSelectionToolbar();
            }
        });
    },
    
    saveBulbGroup: function(groupName) {
        if (this.multiBulbSelection.length === 0) {
            this.showGestureFeedback('No bulbs selected to save', '⚠️');
            return;
        }
        
        const groupKey = `lifx_bulb_group_${groupName}`;
        localStorage.setItem(groupKey, JSON.stringify(this.multiBulbSelection));
        
        if (!this.savedGroups) {
            this.savedGroups = [];
        }
        if (!this.savedGroups.includes(groupName)) {
            this.savedGroups.push(groupName);
            localStorage.setItem('lifx_saved_groups', JSON.stringify(this.savedGroups));
        }
        
        this.showGestureFeedback(`Saved group "${groupName}" (${this.multiBulbSelection.length} bulbs)`, '💾');
        this.hapticFeedback('success');
    },
    
    loadBulbGroup: function(groupName) {
        const groupKey = `lifx_bulb_group_${groupName}`;
        const savedGroup = localStorage.getItem(groupKey);
        
        if (!savedGroup) {
            this.showGestureFeedback(`Group "${groupName}" not found`, '⚠️');
            return;
        }
        
        const bulbIds = JSON.parse(savedGroup);
        
        document.querySelectorAll('.lifx-bulb-control.multi-selected').forEach(el => {
            el.classList.remove('multi-selected');
        });
        
        this.multiBulbSelection = bulbIds;
        
        bulbIds.forEach(id => {
            const bulbEl = document.querySelector(`.lifx-bulb-control[data-bulb-id="${id}"]`);
            if (bulbEl) {
                bulbEl.classList.add('multi-selected');
            }
        });
        
        this.updateSelectionToolbar();
        this.showGestureFeedback(`Loaded group "${groupName}" (${bulbIds.length} bulbs)`, '📋');
        this.hapticFeedback('success');
    },
    
    deleteBulbGroup: function(groupName) {
        const groupKey = `lifx_bulb_group_${groupName}`;
        localStorage.removeItem(groupKey);
        
        if (this.savedGroups) {
            this.savedGroups = this.savedGroups.filter(g => g !== groupName);
            localStorage.setItem('lifx_saved_groups', JSON.stringify(this.savedGroups));
        }
        
        this.showGestureFeedback(`Deleted group "${groupName}"`, '🗑️');
        this.hapticFeedback('light');
    },
    
    showGroupManagementPanel: function() {
        if (typeof Swal === 'undefined') {
            alert('Group Management: Save and load bulb groups for quick selection');
            return;
        }
        
        this.savedGroups = this.savedGroups || JSON.parse(localStorage.getItem('lifx_saved_groups') || '[]');
        
        Swal.fire({
            title: '<i class="fas fa-users"></i> Bulb Group Management',
            html: `
                <div style="padding: 15px; max-width: 500px;">
                    <div class="group-save-section" style="margin-bottom: 25px;">
                        <h5 style="color: #00d4ff; margin-bottom: 10px;">
                            <i class="fas fa-save"></i> Save Current Selection
                        </h5>
                        <div style="display: flex; gap: 10px;">
                            <input type="text" id="new-group-name" class="form-control" 
                                   placeholder="Group name (e.g., Kitchen, Living Room)"
                                   style="background: rgba(42, 42, 58, 0.8); border: 1px solid #00d4ff; color: white; flex: 1;">
                            <button class="btn btn-primary" onclick="LifXTouchControls.saveGroupFromPanel()">
                                <i class="fas fa-save"></i> Save
                            </button>
                        </div>
                        <p style="color: #adb5bd; font-size: 12px; margin-top: 8px;">
                            Currently selected: ${this.multiBulbSelection.length} bulbs
                        </p>
                    </div>
                    
                    <div class="saved-groups-section">
                        <h5 style="color: #00d4ff; margin-bottom: 10px;">
                            <i class="fas fa-folder-open"></i> Saved Groups
                        </h5>
                        ${this.savedGroups.length === 0 
                            ? '<p style="color: #adb5bd; text-align: center; padding: 20px;">No saved groups yet</p>'
                            : `<div style="max-height: 200px; overflow-y: auto;">
                                ${this.savedGroups.map(name => {
                                    const group = JSON.parse(localStorage.getItem(`lifx_bulb_group_${name}`) || '[]');
                                    return `
                                        <div class="saved-group-item" style="
                                            display: flex;
                                            justify-content: space-between;
                                            align-items: center;
                                            padding: 10px 15px;
                                            background: rgba(42, 42, 58, 0.5);
                                            border-radius: 8px;
                                            margin-bottom: 8px;
                                            border: 1px solid rgba(0, 212, 255, 0.2);
                                        ">
                                            <div>
                                                <strong style="color: white;">${name}</strong>
                                                <span style="color: #adb5bd; font-size: 12px; margin-left: 10px;">
                                                    ${group.length} bulbs
                                                </span>
                                            </div>
                                            <div style="display: flex; gap: 8px;">
                                                <button class="btn btn-sm btn-success" onclick="LifXTouchControls.loadBulbGroup('${name}'); Swal.close();">
                                                    <i class="fas fa-folder-open"></i> Load
                                                </button>
                                                <button class="btn btn-sm btn-danger" onclick="LifXTouchControls.deleteBulbGroup('${name}'); LifXTouchControls.refreshGroupPanel();">
                                                    <i class="fas fa-trash"></i>
                                                </button>
                                            </div>
                                        </div>
                                    `;
                                }).join('')}
                            </div>`
                        }
                    </div>
                    
                    ${this.multiBulbSelection.length > 0 ? `
                        <div style="margin-top: 20px; padding-top: 15px; border-top: 1px solid rgba(0, 212, 255, 0.2);">
                            <h5 style="color: #00d4ff; margin-bottom: 10px;">
                                <i class="fas fa-bolt"></i> Quick Actions for Selection
                            </h5>
                            <div style="display: flex; flex-wrap: wrap; gap: 8px;">
                                ${['relax', 'focus', 'energize', 'night', 'party', 'movie'].map(scene => `
                                    <button class="btn btn-sm btn-outline-info" onclick="LifXTouchControls.applySceneBatch('${scene}'); Swal.close();">
                                        <i class="fas fa-palette"></i> ${scene}
                                    </button>
                                `).join('')}
                            </div>
                        </div>
                    ` : ''}
                </div>
            `,
            showConfirmButton: false,
            showCloseButton: true,
            width: '600px',
            backdrop: 'rgba(0,0,0,0.8)'
        });
    },
    
    saveGroupFromPanel: function() {
        const nameInput = document.getElementById('new-group-name');
        if (!nameInput) return;
        
        const groupName = nameInput.value.trim();
        if (!groupName) {
            Swal.showValidationMessage('Please enter a group name');
            return;
        }
        
        this.saveBulbGroup(groupName);
        nameInput.value = '';
        this.refreshGroupPanel();
    },
    
    refreshGroupPanel: function() {
        if (typeof Swal === 'undefined') return;
        setTimeout(() => this.showGroupManagementPanel(), 300);
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
    
    applyHue: function(hue, saturation, brightness) {
        const bulb = this.selectedBulb || this.getFirstSelectedBulb();
        if (!bulb) return;
        
        $.ajax({
            url: '/api/services/lifx/set_color',
            method: 'POST',
            contentType: 'application/json',
            data: JSON.stringify({
                selector: `id:${bulb}`,
                color: `hue:${hue} saturation:${saturation}%`,
                brightness: brightness / 100,
                duration: 0.1
            })
        });
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
    
    showGestureFeedback: function(text, icon, duration = 1000, velocity = null) {
        if (this.reducedMotionMode) {
            duration = 300;
        }
        
        if (this.lastGestureHint && this.lastGestureHint.parentNode) {
            this.lastGestureHint.parentNode.removeChild(this.lastGestureHint);
        }
        
        const hint = document.createElement('div');
        hint.className = 'lifx-gesture-hint visible enhanced';
        if (this.highContrastHints) {
            hint.classList.add('high-contrast');
        }
        
        let velocityIndicator = '';
        if (velocity !== null && velocity > 0) {
            const intensityClass = velocity > 2 ? 'high' : velocity > 1 ? 'medium' : 'low';
            const intensityDots = velocity > 2 ? '●●●' : velocity > 1 ? '●●○' : '●○○';
            velocityIndicator = `<div class="velocity-indicator ${intensityClass}" title="Gesture intensity: ${intensityClass}">${intensityDots}</div>`;
        }
        
        hint.innerHTML = `
            <div class="gesture-icon" style="animation: gesture-bounce 0.5s ease;">${icon}</div>
            <span class="gesture-text">${text}</span>
            ${velocityIndicator}
        `;
        document.body.appendChild(hint);
        this.lastGestureHint = hint;
        
        if (this.showGestureHints) {
            this.createGestureTrail(hint, velocity);
        }
        
        setTimeout(() => {
            if (hint.parentNode) {
                hint.classList.remove('visible');
                setTimeout(() => {
                    if (hint.parentNode) hint.parentNode.removeChild(hint);
                    if (this.lastGestureHint === hint) this.lastGestureHint = null;
                }, 300);
            }
        }, duration);
    },
    
    createGestureTrail: function(hintElement, velocity = null) {
        if (this.reducedMotionMode) return;
        
        const rect = hintElement.getBoundingClientRect();
        const centerX = rect.left + rect.width / 2;
        const centerY = rect.top + rect.height / 2;
        
        const trailCount = velocity ? Math.min(8, 3 + Math.floor(velocity)) : 3;
        const trailSize = velocity ? Math.min(40, 20 + velocity * 5) : 20;
        
        for (let i = 0; i < trailCount; i++) {
            setTimeout(() => {
                const trail = document.createElement('div');
                trail.className = 'lifx-gesture-trail';
                trail.style.left = (centerX + (Math.random() - 0.5) * 60) + 'px';
                trail.style.top = (centerY + (Math.random() - 0.5) * 60) + 'px';
                trail.style.width = (trailSize + Math.random() * 15) + 'px';
                trail.style.height = trail.style.width;
                
                if (velocity && velocity > 2) {
                    trail.style.background = 'radial-gradient(circle, rgba(0, 255, 136, 0.8) 0%, rgba(0, 212, 255, 0.4) 70%, transparent 100%)';
                    trail.style.boxShadow = '0 0 15px rgba(0, 255, 136, 0.6)';
                }
                
                document.body.appendChild(trail);
                
                setTimeout(() => {
                    if (trail.parentNode) trail.parentNode.removeChild(trail);
                }, 800);
            }, i * 80);
        }
    },
    
    showEnhancedGestureFeedback: function(text, icon, duration = 1200) {
        if (this.isModalOpen()) return;
        
        this.showGestureFeedback(text, icon, duration);
        
        const feedback = document.createElement('div');
        feedback.className = 'touch-gesture-feedback active';
        feedback.innerHTML = `<div class="swipe-indicator visible">${icon}</div>`;
        document.body.appendChild(feedback);
        
        setTimeout(() => {
            if (feedback.parentNode) feedback.parentNode.removeChild(feedback);
        }, duration + 200);
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
    
    adjustColorTemp: function(delta, smooth = true, applyImmediately = false) {
        const targets = this.multiBulbSelection.length > 0 
            ? this.multiBulbSelection 
            : (this.selectedBulb ? [this.selectedBulb] : []);
        
        if (targets.length === 0) return;
        
        const newColorTemp = Math.max(1500, Math.min(9000, this.colorTempLevel + delta));
        const duration = smooth && Math.abs(delta) < 500 ? 0.3 : 0.1;
        
        // Update preview immediately for touch adjustments
        if (applyImmediately) {
            this.colorTempLevel = newColorTemp;
            targets.forEach(bulbId => this.updateBulbColorTempPreview(bulbId, newColorTemp));
        }
        
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
                this.showColorTempFeedback(newColorTemp);
            },
            error: (err) => {
                console.error('Failed to adjust color temp:', err);
            }
        });
    },
    
    updateBulbColorTempPreview: function(bulbId, kelvin) {
        const bulbEl = document.querySelector(`.lifx-bulb-control[data-bulb-id="${bulbId}"]`);
        if (!bulbEl) return;
        
        // Calculate color based on kelvin value
        const color = this.kelvinToRgb(kelvin);
        const rgbaColor = `rgba(${color.r}, ${color.g}, ${color.b}, 0.4)`;
        
        // Update visual indicator
        bulbEl.style.borderColor = rgbaColor;
        bulbEl.style.boxShadow = `0 0 20px ${rgbaColor}`;
        
        // Add temperature label
        let tempLabel = kelvin < 3000 ? 'Warm' : kelvin < 5000 ? 'Neutral' : 'Cool';
        let existingLabel = bulbEl.querySelector('.temp-label');
        if (!existingLabel) {
            existingLabel = document.createElement('span');
            existingLabel.className = 'temp-label';
            existingLabel.style.cssText = 'position: absolute; bottom: 8px; right: 8px; background: rgba(0,0,0,0.7); color: white; padding: 2px 6px; border-radius: 4px; font-size: 10px;';
            bulbEl.appendChild(existingLabel);
        }
        existingLabel.textContent = `${tempLabel} ${kelvin}K`;
        setTimeout(() => existingLabel.remove(), 500);
    },
    
    kelvinToRgb: function(kelvin) {
        // Simplified kelvin to RGB conversion
        const temp = kelvin / 100;
        let r, g, b;
        
        if (temp <= 66) {
            r = 255;
            g = Math.max(0, Math.min(255, 99.4708025861 * Math.log(temp) - 161.1195681661));
        } else {
            r = Math.max(0, Math.min(255, 329.698727446 * Math.pow(temp - 60, -0.1332047592)));
            g = Math.max(0, Math.min(255, 288.1221695283 * Math.pow(temp - 60, -0.0755148492)));
        }
        
        if (temp >= 66) {
            b = 255;
        } else if (temp <= 19) {
            b = 0;
        } else {
            b = Math.max(0, Math.min(255, 138.5177312231 * Math.log(temp - 10) - 305.0447927307));
        }
        
        return { r: Math.round(r), g: Math.round(g), b: Math.round(b) };
    },
    
    showColorTempFeedback: function(kelvin) {
        let icon = '❄️';
        let label = 'Cooler';
        
        if (kelvin < 3000) {
            icon = '🔥';
            label = 'Warmer';
        } else if (kelvin < 5000) {
            icon = '☀️';
            label = 'Neutral';
        }
        
        this.showGestureFeedback(`${label} ${kelvin}K`, icon);
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
            polar: { brightness: 70, kelvin: 7500, label: 'Polar' },
            cosmic: { brightness: 65, kelvin: 6000, label: 'Cosmic' },
            dream: { brightness: 45, kelvin: 3500, label: 'Dream' },
            chill: { brightness: 50, kelvin: 3000, label: 'Chill' },
            adventure: { brightness: 85, kelvin: 5000, label: 'Adventure' },
            festival: { brightness: 90, kelvin: 4500, label: 'Festival' },
            bioluminescent: { brightness: 60, kelvin: 5500, label: 'Bioluminescent' },
            cyberpunk: { brightness: 75, kelvin: 5000, label: 'Cyberpunk' },
            vaporwave: { brightness: 70, kelvin: 4800, label: 'Vaporwave' },
            northern_lights: { brightness: 65, kelvin: 6000, label: 'Northern Lights' },
            desert_dawn: { brightness: 55, kelvin: 3800, label: 'Desert Dawn' },
            forest_mist: { brightness: 50, kelvin: 4500, label: 'Forest Mist' },
            volcanic: { brightness: 80, kelvin: 2500, label: 'Volcanic' },
            underwater: { brightness: 55, kelvin: 6500, label: 'Underwater' },
            space_station: { brightness: 75, kelvin: 5500, label: 'Space Station' },
            wizard_tower: { brightness: 60, kelvin: 3500, label: 'Wizard Tower' },
            dragon_fire: { brightness: 85, kelvin: 2800, label: 'Dragon Fire' },
            fairy_grove: { brightness: 55, kelvin: 4000, label: 'Fairy Grove' },
            haunted: { brightness: 40, kelvin: 3000, label: 'Haunted' },
            santas_workshop: { brightness: 90, kelvin: 4000, label: 'Santa Workshop' },
            new_year: { brightness: 95, kelvin: 5000, label: 'New Year' },
            valentines: { brightness: 65, kelvin: 3000, label: 'Valentines' },
            halloween: { brightness: 70, kelvin: 2500, label: 'Halloween' },
            thanksgiving: { brightness: 60, kelvin: 2700, label: 'Thanksgiving' },
            christmas: { brightness: 80, kelvin: 4000, label: 'Christmas' },
            easter: { brightness: 75, kelvin: 4500, label: 'Easter' },
            st_patricks: { brightness: 70, kelvin: 5000, label: 'St Patricks' },
            independence_day: { brightness: 85, kelvin: 5500, label: 'Independence Day' },
            aurora_borealis: { brightness: 55, kelvin: 5500, label: 'Aurora Borealis', effect: 'aurora_borealis' },
            candle_flicker: { brightness: 35, kelvin: 2200, label: 'Candle Flicker', effect: 'candle_flicker' },
            ocean_waves: { brightness: 50, kelvin: 5000, label: 'Ocean Waves', effect: 'ocean_waves' },
            sunset_glow: { brightness: 55, kelvin: 3200, label: 'Sunset Glow', effect: 'sunset_glow' },
            neon_sign: { brightness: 80, kelvin: 4500, label: 'Neon Sign', effect: 'neon_sign' },
            breathing: { brightness: 50, kelvin: 3500, label: 'Breathing', effect: 'breathing' }
        };
        
        const settings = sceneSettings[scene];
        if (settings) {
            this.brightnessLevel = settings.brightness;
            this.colorTempLevel = settings.kelvin;
            
            const selector = targets.join(',');
            
            if (settings.effect) {
                this.applyDynamicScene(scene, {
                    hue: this.getSceneHue(scene),
                    brightness: settings.brightness,
                    saturation: 80,
                    effect: settings.effect
                });
            } else {
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
        }
    },
    
    getSceneHue: function(scene) {
        const hueMap = {
            'relax': 0, 'focus': 160, 'energize': 60, 'night': 240,
            'sunset': 30, 'ocean': 200, 'reading': 50, 'romance': 320,
            'party': 190, 'golden': 55, 'arctic': 210, 'tropical': 160,
            'spring': 150, 'autumn': 25, 'meditation': 270, 'gaming': 330,
            'cooking': 40, 'creative': 280, 'aurora_borealis': 120,
            'candle_flicker': 30, 'ocean_waves': 180, 'sunset_glow': 20,
            'neon_sign': 0, 'breathing': 240
        };
        return hueMap[scene] || 0;
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
        const sensitivity = this.touchSensitivityLevels[this.touchSensitivity] || this.touchSensitivityLevels.medium;
        const brightnessDelta = Math.round((delta / sensitivity.swipeDistance) * 100);
        const newBrightness = Math.max(0, Math.min(100, this.startBrightness + brightnessDelta));
        
        if (newBrightness !== this.brightnessLevel) {
            const stepSize = Math.abs(newBrightness - this.brightnessLevel);
            const previousBrightness = this.brightnessLevel;
            this.brightnessLevel = newBrightness;
            
            // Show real-time visual feedback
            this.showBrightnessFeedback(this.brightnessLevel);
            this.updateBulbBrightnessPreview(this.selectedBulb, this.brightnessLevel);
            
            // Provide haptic feedback on any change
            if (stepSize >= 1) {
                this.hapticFeedback('brightness', Math.min(0.3, stepSize / 50));
            }
            
            // Record gesture for undo
            if (!this.gestureHistory.length || this.gestureHistory[this.gestureHistory.length - 1].type !== 'brightness') {
                this.recordGesture('brightness', newBrightness - previousBrightness, previousBrightness, [this.selectedBulb]);
            }
        }
    },
    
    updateBulbBrightnessPreview: function(bulbId, brightness) {
        const bulbEl = document.querySelector(`.lifx-bulb-control[data-bulb-id="${bulbId}"]`);
        if (!bulbEl) return;
        
        // Update visual brightness indicator
        const brightnessPercent = brightness + '%';
        bulbEl.style.setProperty('--brightness-level', brightnessPercent);
        
        // Add glow effect based on brightness
        const glowIntensity = brightness / 100;
        bulbEl.style.boxShadow = `0 0 ${10 + glowIntensity * 30}px rgba(0, 212, 255, ${0.3 + glowIntensity * 0.5})`;
        
        // Update brightness level indicator if present
        const indicator = bulbEl.querySelector('.brightness-level');
        if (indicator) {
            indicator.textContent = brightness + '%';
            indicator.style.opacity = brightness > 0 ? 1 : 0.5;
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
    
    initColorWheel: function() {
        const colorWheelContainer = document.getElementById('lifx-color-wheel-container');
        if (!colorWheelContainer) return;
        
        colorWheelContainer.innerHTML = `
            <div id="lifx-color-wheel" class="lifx-color-wheel" style="display: none;">
                <canvas id="color-wheel-canvas" width="300" height="300"></canvas>
                <div class="color-wheel-center">
                    <div class="color-wheel-preview" id="color-wheel-preview"></div>
                </div>
                <div class="color-wheel-controls">
                    <button class="btn-close" onclick="LifXTouchControls.hideColorWheel()">×</button>
                </div>
            </div>
        `;
        
        this.setupColorWheelEvents();
    },
    
    setupColorWheelEvents: function() {
        const canvas = document.getElementById('color-wheel-canvas');
        if (!canvas) return;
        
        const ctx = canvas.getContext('2d');
        this.colorWheelCtx = ctx;
        this.drawColorWheel();
        
        let isDragging = false;
        
        const getColorFromPosition = (x, y) => {
            const rect = canvas.getBoundingClientRect();
            const centerX = rect.width / 2;
            const centerY = rect.height / 2;
            const dx = x - centerX;
            const dy = y - centerY;
            
            const angle = Math.atan2(dy, dx) + Math.PI;
            const hue = (angle / (2 * Math.PI)) * 360;
            
            const distance = Math.sqrt(dx * dx + dy * dy);
            const maxDistance = rect.width / 2;
            const saturation = Math.min(100, (distance / maxDistance) * 100);
            
            return { hue, saturation };
        };
        
        const handleColorSelect = (clientX, clientY) => {
            const rect = canvas.getBoundingClientRect();
            const x = clientX - rect.left;
            const y = clientY - rect.top;
            const color = getColorFromPosition(x, y);
            
            this.lastColorHue = color.hue;
            this.lastColorSaturation = color.saturation;
            
            const previewEl = document.getElementById('color-wheel-preview');
            if (previewEl) {
                previewEl.style.background = `hsl(${color.hue}, ${color.saturation}%, 50%)`;
            }
            
            this.applyHSLColor();
        };
        
        canvas.addEventListener('touchstart', (e) => {
            e.preventDefault();
            isDragging = true;
            const touch = e.touches[0];
            handleColorSelect(touch.clientX, touch.clientY);
            this.hapticFeedback('light');
        }, { passive: false });
        
        canvas.addEventListener('touchmove', (e) => {
            e.preventDefault();
            if (!isDragging) return;
            const touch = e.touches[0];
            handleColorSelect(touch.clientX, touch.clientY);
            this.createColorWheelTrail(touch.clientX, touch.clientY);
        }, { passive: false });
        
        canvas.addEventListener('touchend', () => {
            isDragging = false;
            this.showGestureFeedback('Color updated', '🎨');
            this.hapticFeedback('success');
        });
        
        canvas.addEventListener('mousedown', (e) => {
            isDragging = true;
            handleColorSelect(e.clientX, e.clientY);
        });
        
        canvas.addEventListener('mousemove', (e) => {
            if (!isDragging) return;
            handleColorSelect(e.clientX, e.clientY);
        });
        
        canvas.addEventListener('mouseup', () => {
            isDragging = false;
        });
    },
    
    drawColorWheel: function() {
        const ctx = this.colorWheelCtx;
        if (!ctx) return;
        
        const canvas = document.getElementById('color-wheel-canvas');
        const width = canvas.width;
        const height = canvas.height;
        const centerX = width / 2;
        const centerY = height / 2;
        const radius = width / 2 - 10;
        
        for (let angle = 0; angle < 360; angle++) {
            const startAngle = (angle - 1) * Math.PI / 180;
            const endAngle = (angle + 1) * Math.PI / 180;
            
            ctx.beginPath();
            ctx.moveTo(centerX, centerY);
            ctx.arc(centerX, centerY, radius, startAngle, endAngle);
            ctx.closePath();
            
            const gradient = ctx.createRadialGradient(centerX, centerY, 0, centerX, centerY, radius);
            gradient.addColorStop(0, `hsl(${angle}, 0%, 50%)`);
            gradient.addColorStop(1, `hsl(${angle}, 100%, 50%)`);
            
            ctx.fillStyle = gradient;
            ctx.fill();
        }
    },
    
    createColorWheelTrail: function(x, y) {
        if (this.reducedMotionMode) return;
        
        const trail = document.createElement('div');
        trail.className = 'color-wheel-trail';
        trail.style.left = x + 'px';
        trail.style.top = y + 'px';
        trail.style.background = `hsl(${this.lastColorHue}, ${this.lastColorSaturation}%, 50%)`;
        document.body.appendChild(trail);
        
        setTimeout(() => {
            trail.classList.add('fade-out');
            setTimeout(() => {
                if (trail.parentNode) trail.parentNode.removeChild(trail);
            }, 300);
        }, 50);
    },
    
    showColorWheel: function() {
        const wheel = document.getElementById('lifx-color-wheel');
        if (wheel) {
            wheel.style.display = 'block';
            setTimeout(() => wheel.classList.add('visible'), 10);
            this.colorWheelActive = true;
            this.hapticFeedback('light');
        }
    },
    
    hideColorWheel: function() {
        const wheel = document.getElementById('lifx-color-wheel');
        if (wheel) {
            wheel.classList.remove('visible');
            setTimeout(() => wheel.style.display = 'none', 300);
            this.colorWheelActive = false;
        }
    },
    
    showBrightnessFeedback: function(brightness) {
        if (!this.gestureHints.enabled) return;
        
        const existingFeedback = document.querySelector('.brightness-feedback-overlay');
        if (existingFeedback) existingFeedback.remove();
        
        const feedback = document.createElement('div');
        feedback.className = 'brightness-feedback-overlay';
        feedback.innerHTML = `
            <div class="brightness-icon">${brightness > 50 ? '☀️' : brightness > 20 ? '💡' : '🌙'}</div>
            <div class="brightness-bar">
                <div class="brightness-fill" style="width: ${brightness}%"></div>
            </div>
            <span class="brightness-value">${brightness}%</span>
        `;
        document.body.appendChild(feedback);
        
        setTimeout(() => {
            feedback.classList.add('visible');
        }, 10);
        
        setTimeout(() => {
            feedback.classList.remove('visible');
            setTimeout(() => {
                if (feedback.parentNode) feedback.parentNode.removeChild(feedback);
            }, 300);
        }, this.gestureHints.duration);
    },
    
    recordGesture: function(type, data, previousValue = null, targets = []) {
        const gesture = {
            type,
            data,
            previousValue,
            targets: [...targets],
            timestamp: Date.now()
        };
        
        this.gestureHistory.push(gesture);
        if (this.gestureHistory.length > this.maxGestureHistory) {
            this.gestureHistory.shift();
        }
    },
    
    undoLastGesture: function() {
        if (this.gestureHistory.length === 0) {
            this.showGestureFeedback('Nothing to undo', '⊘');
            return;
        }
        
        const lastGesture = this.gestureHistory.pop();
        this.applyUndoGesture(lastGesture);
        this.showGestureFeedback('Undone', '↩️');
        this.hapticFeedback('light');
    },
    
    applyUndoGesture: function(gesture) {
        switch (gesture.type) {
            case 'brightness':
                this.brightnessLevel = gesture.previousValue;
                $.ajax({
                    url: '/api/services/lifx/set_state',
                    method: 'POST',
                    contentType: 'application/json',
                    data: JSON.stringify({
                        selector: `id:${gesture.targets.join(',')}`,
                        brightness: this.brightnessLevel / 100,
                        duration: 0.3
                    })
                });
                break;
            case 'colorTemp':
                this.colorTempLevel = gesture.previousValue;
                $.ajax({
                    url: '/api/services/lifx/set_color',
                    method: 'POST',
                    contentType: 'application/json',
                    data: JSON.stringify({
                        selector: `id:${gesture.targets.join(',')}`,
                        color: `kelvin:${this.colorTempLevel}`
                    })
                });
                break;
        }
    },
    
    recordGestureSuccess: function() {
        this.gestureSuccessCount++;
        this.adjustSensitivity(true);
    },
    
    adjustSensitivity: function(success, gestureType = null) {
        if (!this.adaptiveSensitivity.enabled) return;
        
        this.currentAdjustments++;
        
        if (success) {
            this.gestureSuccessCount++;
        } else {
            this.gestureFailCount++;
        }
        
        const successRate = this.gestureSuccessCount / (this.gestureSuccessCount + this.gestureFailCount);
        
        if (this.currentAdjustments >= this.adaptiveSensitivity.minAdjustments) {
            if (successRate < this.adaptiveSensitivity.failThreshold) {
                this.increaseSensitivity();
            } else if (successRate > this.adaptiveSensitivity.successThreshold) {
                this.decreaseSensitivity();
            }
        }
    },
    
    increaseSensitivity: function() {
        const levels = ['low', 'medium', 'high', 'very_high'];
        const currentIndex = levels.indexOf(this.touchSensitivity);
        if (currentIndex < levels.length - 1) {
            this.touchSensitivity = levels[currentIndex + 1];
            this.saveGestureSensitivity();
            this.showGestureFeedback('Sensitivity increased', '📈');
        }
    },
    
    decreaseSensitivity: function() {
        const levels = ['low', 'medium', 'high', 'very_high'];
        const currentIndex = levels.indexOf(this.touchSensitivity);
        if (currentIndex > 0) {
            this.touchSensitivity = levels[currentIndex - 1];
            this.saveGestureSensitivity();
            this.showGestureFeedback('Sensitivity decreased', '📉');
        }
    },
    
    saveGestureSensitivity: function() {
        localStorage.setItem('lifx-touch-sensitivity', this.touchSensitivity);
    },
    
    loadGestureSensitivity: function() {
        const saved = localStorage.getItem('lifx-touch-sensitivity');
        if (saved && this.touchSensitivityLevels[saved]) {
            this.touchSensitivity = saved;
        }
    },
    
    loadSavedPreferences: function() {
        const saved = localStorage.getItem('lifx-preferences');
        if (saved) {
            try {
                const prefs = JSON.parse(saved);
                if (prefs.currentScene) this.currentScene = prefs.currentScene;
                if (prefs.brightnessLevel) this.brightnessLevel = prefs.brightnessLevel;
                if (prefs.colorTempLevel) this.colorTempLevel = prefs.colorTempLevel;
            } catch (e) {
                console.error('Failed to load LIFX preferences:', e);
            }
        }
    },
    
    savePreferences: function() {
        const prefs = {
            currentScene: this.currentScene,
            brightnessLevel: this.brightnessLevel,
            colorTempLevel: this.colorTempLevel,
            touchSensitivity: this.touchSensitivity
        };
        localStorage.setItem('lifx-preferences', JSON.stringify(prefs));
    },
    
    getFirstSelectedBulb: function() {
        if (this.multiBulbSelection.length > 0) {
            return this.multiBulbSelection[0];
        }
        const selectedEl = document.querySelector('.lifx-bulb-control.selected');
        return selectedEl ? selectedEl.dataset.bulbId : null;
    },
    
    initEnhancedTouchTracking: function(e) {
        if (!e.touches || e.touches.length === 0) return;
        
        const touch = e.touches[0];
        this.lastTouchCoordinates = { x: touch.clientX, y: touch.clientY };
        this.touchVelocityHistory = [];
        this.touchStartTime = Date.now();
        
        if (touch.force !== undefined) {
            this.touchPressure = touch.force;
        }
        
        this.showEdgeSwipeZone(touch.clientX, touch.clientY);
    },
    
    showEdgeSwipeZone: function(x, y) {
        if (!this.swipeEdgeZone || this.swipeEdgeZone <= 0) return;
        
        const edgeThreshold = this.swipeEdgeZone;
        let edgePosition = null;
        
        if (x < edgeThreshold) edgePosition = 'left';
        else if (x > window.innerWidth - edgeThreshold) edgePosition = 'right';
        else if (y < edgeThreshold) edgePosition = 'top';
        else if (y > window.innerHeight - edgeThreshold) edgePosition = 'bottom';
        
        if (edgePosition) {
            this.highlightEdgeZone(edgePosition);
        }
    },
    
    highlightEdgeZone: function(position) {
        const existingZone = document.querySelector('.edge-swipe-zone');
        if (existingZone) existingZone.remove();
        
        const zone = document.createElement('div');
        zone.className = 'edge-swipe-zone';
        
        const styles = {
            left: { left: 0, top: 0, width: this.swipeEdgeZone + 'px', height: '100%' },
            right: { right: 0, top: 0, width: this.swipeEdgeZone + 'px', height: '100%' },
            top: { top: 0, left: 0, width: '100%', height: this.swipeEdgeZone + 'px' },
            bottom: { bottom: 0, left: 0, width: '100%', height: this.swipeEdgeZone + 'px' }
        };
        
        Object.assign(zone.style, {
            position: 'fixed',
            pointerEvents: 'none',
            zIndex: '9997',
            background: 'linear-gradient(to ' + position + ', rgba(0, 212, 255, 0.3), transparent)',
            transition: 'opacity 0.2s ease'
        }, styles[position]);
        
        document.body.appendChild(zone);
        setTimeout(() => {
            zone.style.opacity = '0';
            setTimeout(() => { if (zone.parentNode) zone.parentNode.removeChild(zone); }, 200);
        }, 300);
    },
    
    updateTouchVelocity: function(e) {
        if (!e.touches || e.touches.length === 0) return;
        
        const touch = e.touches[0];
        const dx = touch.clientX - this.lastTouchCoordinates.x;
        const dy = touch.clientY - this.lastTouchCoordinates.y;
        const velocity = Math.sqrt(dx * dx + dy * dy);
        
        this.touchVelocityHistory.push(velocity);
        if (this.touchVelocityHistory.length > this.maxVelocityHistory) {
            this.touchVelocityHistory.shift();
        }
        
        this.touchVelocity = velocity;
        
        const angle = Math.atan2(dy, dx) * 180 / Math.PI;
        this.touchDirection = angle;
        
        this.lastTouchCoordinates = { x: touch.clientX, y: touch.clientY };
    },
    
    processGestureVelocity: function(e) {
        if (this.touchVelocityHistory.length === 0) return;
        
        const avgVelocity = this.touchVelocityHistory.reduce((a, b) => a + b, 0) / this.touchVelocityHistory.length;
        this.lastGestureVelocity = avgVelocity;
        
        if (avgVelocity > 2.0 && this.enhancedRippleMode) {
            this.createVelocityRipple(e.changedTouches[0].clientX, e.changedTouches[0].clientY, avgVelocity);
        }
    },
    
    createVelocityRipple: function(x, y, velocity) {
        const ripple = document.createElement('div');
        ripple.className = 'velocity-ripple';
        ripple.style.left = x + 'px';
        ripple.style.top = y + 'px';
        ripple.style.setProperty('--ripple-scale', Math.min(3, 1 + velocity / 10));
        ripple.style.setProperty('--ripple-duration', Math.max(0.3, 1 - velocity / 50) + 's');
        document.body.appendChild(ripple);
        
        setTimeout(() => {
            if (ripple.parentNode) ripple.parentNode.removeChild(ripple);
        }, 1000);
    },
    
    hapticFeedback: function(pattern = 'default', intensity = 1.0) {
        if (!this.hapticEnabled || !navigator.vibrate) return;
        
        if (this.pressureSensitiveEnabled && this.touchPressure > 0) {
            intensity = Math.min(1.5, intensity * (1 + this.touchPressure / 100));
        }
        
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
            'media': [30, 30, 30, 30],
            'edge_swipe': [60, 40, 60, 40, 60],
            'zone_select': [70, 30, 70, 30, 70],
            'scene_change': [40, 30, 40, 30, 40],
            'media_sync_beat': [15, 10, 15],
            'media_sync_bass': [25, 15, 25],
            'media_sync_spectrum': [20, 20, 20, 20],
            'gesture_preview': [30, 20, 30, 20, 30, 20, 30],
            'tutorial': [50, 30, 50, 30, 50],
            'ripple': [15, 20, 15],
            'trail': [20, 15, 20, 15],
            'macro': [40, 30, 40, 30, 40],
            'preset': [55, 35, 55],
            'group': [45, 35, 45, 35],
            'effect_start': [60, 40, 60, 40, 60],
            'effect_stop': [30, 30],
            'color_pick': [25, 20, 25],
            'favorite': [35, 25, 35, 25, 35],
            'undo': [40, 30, 40],
            'redo': [40, 30, 40, 30],
            'multi_select': [30, 25, 30, 25, 30],
            'batch_operation': [50, 40, 50, 40, 50, 40, 50]
        };
        
        const basePattern = basePatterns[pattern] || basePatterns['default'];
        const scaledPattern = basePattern.map(duration => Math.round(duration * intensity));
        
        try {
            navigator.vibrate(scaledPattern);
        } catch (e) {
            console.warn('Haptic feedback failed:', e);
        }
    },
    
    hapticSequence: function(patterns, delays) {
        if (!this.hapticEnabled || !navigator.vibrate) return;
        
        let index = 0;
        const playNext = () => {
            if (index >= patterns.length) return;
            this.hapticFeedback(patterns[index]);
            setTimeout(() => {
                index++;
                if (index < patterns.length) playNext();
            }, delays[index] || 100);
        };
        playNext();
    },
    
    setGestureSensitivity: function(level) {
        const settings = {
            'low': { swipeDistance: 80, swipeTime: 400, pinchDistance: 50, longPressDelay: 700, doubleTapDelay: 400 },
            'medium': { swipeDistance: 50, swipeTime: 300, pinchDistance: 30, longPressDelay: 500, doubleTapDelay: 300 },
            'high': { swipeDistance: 30, swipeTime: 200, pinchDistance: 20, longPressDelay: 300, doubleTapDelay: 200 },
            'very_high': { swipeDistance: 15, swipeTime: 150, pinchDistance: 10, longPressDelay: 200, doubleTapDelay: 150 }
        };
        
        this.gestureSensitivity = settings[level] || settings['medium'];
        this.touchHoldDelay = this.gestureSensitivity.longPressDelay;
        this.doubleTapDelay = this.gestureSensitivity.doubleTapDelay;
        this.swipeEdgeZone = level === 'very_high' ? 30 : (level === 'high' ? 25 : 20);
        this.touchSensitivity = level;
        localStorage.setItem('lifx_gesture_sensitivity', level);
        console.log('Gesture sensitivity set to:', level, this.gestureSensitivity);
        
        if (typeof updateSensitivityUI === 'function') {
            updateSensitivityUI(level);
        }
    },
    
    loadGestureSensitivity: function() {
        const saved = localStorage.getItem('lifx_gesture_sensitivity') || 'medium';
        this.setGestureSensitivity(saved);
        this.initAdaptiveSensitivity();
    },
    
    adaptiveSensitivityEnabled: true,
    gestureSuccessCount: 0,
    gestureFailCount: 0,
    lastMissedGestures: [],
    
    initAdaptiveSensitivity: function() {
        if (!this.adaptiveSensitivityEnabled) return;
        
        const stats = JSON.parse(localStorage.getItem('lifx_adaptive_sensitivity') || '{"success": 0, "fail": 0, "level": "medium"}');
        this.gestureSuccessCount = stats.success || 0;
        this.gestureFailCount = stats.fail || 0;
        
        if (stats.level && stats.level !== localStorage.getItem('lifx_gesture_sensitivity')) {
            console.log('Adaptive sensitivity recommends:', stats.level);
        }
        
        setInterval(() => this.analyzeGesturePatterns(), 60000);
    },
    
    recordGestureSuccess: function() {
        if (!this.adaptiveSensitivityEnabled) return;
        this.gestureSuccessCount++;
        this.saveAdaptiveSensitivityStats();
    },
    
    recordGestureFail: function(gestureType, distance = 0) {
        if (!this.adaptiveSensitivityEnabled) return;
        this.gestureFailCount++;
        this.lastMissedGestures.push({ type: gestureType, distance, timestamp: Date.now() });
        if (this.lastMissedGestures.length > 5) this.lastMissedGestures.shift();
        this.saveAdaptiveSensitivityStats();
    },
    
    saveAdaptiveSensitivityStats: function() {
        const currentLevel = localStorage.getItem('lifx_gesture_sensitivity') || 'medium';
        localStorage.setItem('lifx_adaptive_sensitivity', JSON.stringify({
            success: this.gestureSuccessCount,
            fail: this.gestureFailCount,
            level: currentLevel
        }));
    },
    
    analyzeGesturePatterns: function() {
        if (this.gestureSuccessCount + this.gestureFailCount < 20) return;
        
        const failRate = this.gestureFailCount / (this.gestureSuccessCount + this.gestureFailCount);
        const currentLevel = localStorage.getItem('lifx_gesture_sensitivity') || 'medium';
        
        let recommendedLevel = currentLevel;
        
        if (failRate > 0.3) {
            if (currentLevel === 'low') recommendedLevel = 'medium';
            else if (currentLevel === 'medium') recommendedLevel = 'high';
            else if (currentLevel === 'high') recommendedLevel = 'very_high';
        } else if (failRate < 0.1 && currentLevel !== 'low') {
            if (currentLevel === 'very_high') recommendedLevel = 'high';
            else if (currentLevel === 'high') recommendedLevel = 'medium';
            else if (currentLevel === 'medium') recommendedLevel = 'low';
        }
        
        if (recommendedLevel !== currentLevel) {
            console.log(`Adaptive sensitivity suggests changing from ${currentLevel} to ${recommendedLevel} (fail rate: ${(failRate * 100).toFixed(1)}%)`);
            this.showSensitivitySuggestion(currentLevel, recommendedLevel, failRate);
        }
        
        this.gestureSuccessCount = 0;
        this.gestureFailCount = 0;
    },
    
    adjustSensitivity: function(success, gestureType = 'unknown') {
        if (!this.adaptiveSensitivity.enabled) return;
        
        const currentLevel = localStorage.getItem('lifx_gesture_sensitivity') || 'medium';
        const stats = this.getAdaptiveSensitivityStats();
        
        stats.totalGestures = (stats.totalGestures || 0) + 1;
        stats.currentAdjustments = (stats.currentAdjustments || 0) + 1;
        
        if (!stats.gestureStats) stats.gestureStats = {};
        if (!stats.gestureStats[gestureType]) {
            stats.gestureStats[gestureType] = { success: 0, fail: 0 };
        }
        
        if (success) {
            stats.gestureStats[gestureType].success++;
            stats.totalSuccess = (stats.totalSuccess || 0) + 1;
        } else {
            stats.gestureStats[gestureType].fail++;
            stats.totalFails = (stats.totalFails || 0) + 1;
        }
        
        const minAdjustments = this.adaptiveSensitivity.minAdjustments;
        if (stats.currentAdjustments < minAdjustments) {
            this.saveAdaptiveSensitivityStats(stats);
            return;
        }
        
        const gestureFailRate = stats.gestureStats[gestureType].fail / 
            (stats.gestureStats[gestureType].success + stats.gestureStats[gestureType].fail);
        
        let newLevel = currentLevel;
        const adjustmentFactor = this.adaptiveSensitivity.adjustmentFactor;
        
        if (gestureFailRate > this.adaptiveSensitivity.failThreshold) {
            if (currentLevel === 'low') newLevel = 'medium';
            else if (currentLevel === 'medium') newLevel = 'high';
            else if (currentLevel === 'high') newLevel = 'very_high';
            
            if (newLevel !== currentLevel) {
                this.gestureSensitivity.swipeDistance = Math.max(15, 
                    this.gestureSensitivity.swipeDistance * (1 - adjustmentFactor));
                this.gestureSensitivity.pinchDistance = Math.max(10,
                    this.gestureSensitivity.pinchDistance * (1 - adjustmentFactor));
                console.log(`[AdaptiveSensitivity] Increased sensitivity for ${gestureType} (fail rate: ${(gestureFailRate * 100).toFixed(1)}%)`);
                this.showMicroFeedback('sensitivity-up', gestureType);
            }
        } else if (gestureFailRate < (1 - this.adaptiveSensitivity.successThreshold)) {
            if (currentLevel === 'very_high') newLevel = 'high';
            else if (currentLevel === 'high') newLevel = 'medium';
            else if (currentLevel === 'medium') newLevel = 'low';
            
            if (newLevel !== currentLevel) {
                this.gestureSensitivity.swipeDistance = Math.min(80,
                    this.gestureSensitivity.swipeDistance * (1 + adjustmentFactor));
                this.gestureSensitivity.pinchDistance = Math.min(50,
                    this.gestureSensitivity.pinchDistance * (1 + adjustmentFactor));
                console.log(`[AdaptiveSensitivity] Decreased sensitivity for ${gestureType} (success rate: ${(gestureFailRate * 100).toFixed(1)}%)`);
                this.showMicroFeedback('sensitivity-down', gestureType);
            }
        }
        
        if (newLevel !== currentLevel) {
            localStorage.setItem('lifx_gesture_sensitivity', newLevel);
            this.touchSensitivity = newLevel;
        }
        
        this.saveAdaptiveSensitivityStats(stats);
    },
    
    getAdaptiveSensitivityStats: function() {
        try {
            const stored = localStorage.getItem('lifx_adaptive_sensitivity_stats');
            if (stored) {
                return JSON.parse(stored);
            }
        } catch (e) {
            console.warn('[AdaptiveSensitivity] Failed to load stats:', e);
        }
        return { totalGestures: 0, totalSuccess: 0, totalFails: 0, currentAdjustments: 0, gestureStats: {} };
    },
    
    saveAdaptiveSensitivityStats: function(stats) {
        try {
            localStorage.setItem('lifx_adaptive_sensitivity_stats', JSON.stringify(stats));
        } catch (e) {
            console.warn('[AdaptiveSensitivity] Failed to save stats:', e);
        }
    },
    
    showMicroFeedback: function(type, gestureType) {
        const indicator = document.createElement('div');
        indicator.className = 'adaptive-sensitivity-feedback';
        indicator.style.cssText = `
            position: fixed;
            top: 20px;
            right: 20px;
            padding: 8px 12px;
            background: ${type === 'sensitivity-up' ? 'rgba(255, 107, 107, 0.9)' : 'rgba(0, 212, 255, 0.9)'};
            color: #fff;
            border-radius: 20px;
            font-size: 11px;
            z-index: 10001;
            opacity: 0;
            transform: translateY(-10px);
            transition: all 0.3s ease;
        `;
        
        const messages = {
            'sensitivity-up': `↑ Sensitivity for ${gestureType}`,
            'sensitivity-down': `↓ Sensitivity for ${gestureType}`
        };
        
        indicator.textContent = messages[type] || type;
        document.body.appendChild(indicator);
        
        requestAnimationFrame(() => {
            indicator.style.opacity = '1';
            indicator.style.transform = 'translateY(0)';
        });
        
        setTimeout(() => {
            indicator.style.opacity = '0';
            indicator.style.transform = 'translateY(-10px)';
            setTimeout(() => indicator.remove(), 300);
        }, 1500);
    },
    
    resetAdaptiveSensitivity: function() {
        localStorage.removeItem('lifx_adaptive_sensitivity_stats');
        this.gestureSuccessCount = 0;
        this.gestureFailCount = 0;
        this.setGestureSensitivity('medium');
        console.log('[AdaptiveSensitivity] Reset to defaults');
        this.showGestureFeedback('Sensitivity Reset', '🔄');
    },
    
    showSensitivitySuggestion: function(currentLevel, recommendedLevel, failRate) {
        if (typeof Swal === 'undefined') return;
        
        Swal.fire({
            title: 'Sensitivity Optimization Suggestion',
            html: `
                <p style="color: #adb5bd; margin-bottom: 15px;">
                    Based on your gesture patterns, we recommend adjusting sensitivity.
                </p>
                <div style="text-align: left; margin: 15px 0;">
                    <p style="color: #ff6b6b;">Missed gestures: ${(failRate * 100).toFixed(1)}%</p>
                    <p style="color: #00d4ff;">Current: ${currentLevel}</p>
                    <p style="color: #00ff88;">Recommended: ${recommendedLevel}</p>
                </div>
            `,
            showCancelButton: true,
            confirmButtonText: 'Apply Recommended',
            cancelButtonText: 'Keep Current',
            confirmButtonColor: '#00d4ff',
            cancelButtonColor: '#6c757d',
            background: 'rgba(30, 30, 45, 0.98)',
            backdrop: 'rgba(0, 0, 0, 0.8)'
        }).then((result) => {
            if (result.isConfirmed) {
                this.setGestureSensitivity(recommendedLevel);
                this.showGestureFeedback(`Sensitivity: ${recommendedLevel}`, '✓');
            }
        });
    },
    
    showTouchSensitivityPanel: function() {
        if (typeof Swal === 'undefined') {
            alert('Touch Sensitivity Settings:\n- Low: Requires larger gestures\n- Medium: Balanced sensitivity\n- High: Very responsive to small gestures\n- Very High: Maximum sensitivity');
            return;
        }
        
        const currentLevel = localStorage.getItem('lifx_gesture_sensitivity') || 'medium';
        const sensitivityLevels = [
            { level: 'low', icon: '🐢', title: 'Low', description: 'Requires deliberate gestures, fewer false positives' },
            { level: 'medium', icon: '🚶', title: 'Medium', description: 'Balanced sensitivity for most users' },
            { level: 'high', icon: '🏃', title: 'High', description: 'Quick response to gestures' },
            { level: 'very_high', icon: '⚡', title: 'Very High', description: 'Maximum sensitivity, responds to micro-gestures' }
        ];
        
        Swal.fire({
            title: '<i class="fas fa-sliders-h"></i> Touch Sensitivity',
            html: `
                <div class="touch-sensitivity-panel" style="padding: 10px;">
                    <div style="display: grid; gap: 12px;">
                        ${sensitivityLevels.map(item => `
                            <div class="sensitivity-option ${item.level === currentLevel ? 'active' : ''}" 
                                 data-level="${item.level}"
                                 style="
                                    display: flex;
                                    align-items: center;
                                    justify-content: space-between;
                                    padding: 15px;
                                    background: rgba(42, 42, 58, 0.6);
                                    border: 2px solid ${item.level === currentLevel ? '#00d4ff' : 'transparent'};
                                    border-radius: 12px;
                                    cursor: pointer;
                                    transition: all 0.2s ease;
                                 "
                                 onclick="LifXTouchControls.setGestureSensitivity('${item.level}'); LifXTouchControls.showTouchSensitivityPanel();">
                                <div class="sensitivity-option-label" style="display: flex; align-items: center; gap: 12px;">
                                    <span class="sensitivity-option-icon" style="font-size: 28px;">${item.icon}</span>
                                    <div>
                                        <strong style="color: ${item.level === currentLevel ? '#00d4ff' : '#adb5bd'}; font-size: 14px;">${item.title}</strong>
                                        <p style="color: #6c757d; font-size: 11px; margin: 0;">${item.description}</p>
                                    </div>
                                </div>
                                ${item.level === currentLevel ? '<i class="fas fa-check-circle" style="color: #00d4ff; font-size: 20px;"></i>' : ''}
                            </div>
                        `).join('')}
                    </div>
                    
                    <div style="margin-top: 20px; padding: 15px; background: rgba(0, 212, 255, 0.1); border-radius: 10px; border: 1px solid rgba(0, 212, 255, 0.3);">
                        <p style="color: #00d4ff; font-size: 12px; margin: 0;">
                            <i class="fas fa-info-circle"></i> Changes apply immediately. Test different levels to find your perfect sensitivity.
                        </p>
                    </div>
                </div>
            `,
            showConfirmButton: false,
            showCloseButton: true,
            width: '500px',
            backdrop: 'rgba(0,0,0,0.8)'
        });
    },
    
    undoLastGesture: function() {
        if (this.gestureHistory.length === 0) {
            showNotification('No gestures to undo', 'info');
            return;
        }
        
        const lastGesture = this.gestureHistory.pop();
        console.log('Undoing gesture:', lastGesture);
        
        if (lastGesture.type === 'brightness') {
            this.adjustBrightness(-lastGesture.delta);
            showNotification(`Brightness reverted (${lastGesture.delta > 0 ? '-' : '+'}${Math.abs(lastGesture.delta)})`, 'info');
        } else if (lastGesture.type === 'colorTemp') {
            this.adjustColorTemp(-lastGesture.delta);
            showNotification(`Color temperature reverted (${lastGesture.delta > 0 ? 'cooler' : 'warmer'})`, 'info');
        } else if (lastGesture.type === 'scene') {
            const previousScene = lastGesture.previousScene || 'relax';
            this.applyScene(previousScene);
            showNotification(`Scene reverted to ${previousScene}`, 'info');
        } else if (lastGesture.type === 'power') {
            const target = lastGesture.targets || (this.selectedBulb ? [this.selectedBulb] : []);
            if (target.length > 0) {
                $.ajax({
                    url: '/api/services/lifx/set_state',
                    method: 'POST',
                    contentType: 'application/json',
                    data: JSON.stringify({
                        selector: `id:${target.join(',')}`,
                        power: lastGesture.previousState === 'on' ? 'on' : 'off',
                        duration: 0.3
                    })
                });
                showNotification(`Power state reverted`, 'info');
            }
        }
        
        this.hapticFeedback('success');
        this.showGestureFeedback('Undone', '↩️');
    },
    
    recordGesture: function(type, delta, previousValue, targets) {
        this.gestureHistory.push({
            type: type,
            delta: delta,
            previousValue: previousValue,
            previousScene: this.currentScene,
            targets: targets,
            timestamp: Date.now()
        });
        
        if (this.gestureHistory.length > this.maxGestureHistory) {
            this.gestureHistory.shift();
        }
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
            cosmic: '🌌', dream: '💭', chill: '🧊', adventure: '🗺️', festival: '🎪',
            bioluminescent: '🪼', cyberpunk: '🤖', vaporwave: '🌆', northern_lights: '🌠',
            desert_dawn: '🏜️', forest_mist: '🌲', volcanic: '🌋', underwater: '🐠',
            space_station: '🛰️', wizard_tower: '🧙', dragon_fire: '🐉', fairy_grove: '🧚',
            haunted: '👻', santas_workshop: '🎅', new_year: '🎆', valentines: '💝',
            halloween: '🎃', thanksgiving: '🦃', christmas: '🎄', easter: '🐰',
            st_patricks: '☘️', independence_day: '🎆'
        };
        return emojis[sceneName] || '💡';
    },
    
    getSceneDefinition: function(sceneName) {
        const scenes = {
            bioluminescent: { hue: 200, saturation: 80, brightness: 70, temperature: 4000, effect: 'pulse' },
            cyberpunk: { hue: 280, saturation: 90, brightness: 80, temperature: 6000, effect: 'flicker' },
            vaporwave: { hue: 300, saturation: 70, brightness: 60, temperature: 5000, effect: 'gradient' },
            northern_lights: { hue: 120, saturation: 60, brightness: 50, temperature: 5500, effect: 'aurora' },
            desert_dawn: { hue: 30, saturation: 50, brightness: 60, temperature: 3000, effect: 'fade' },
            forest_mist: { hue: 100, saturation: 40, brightness: 40, temperature: 4500, effect: 'breathe' },
            volcanic: { hue: 10, saturation: 90, brightness: 75, temperature: 2500, effect: 'flicker' },
            underwater: { hue: 190, saturation: 70, brightness: 55, temperature: 6500, effect: 'wave' },
            space_station: { hue: 220, saturation: 50, brightness: 65, temperature: 7000, effect: 'pulse' },
            wizard_tower: { hue: 270, saturation: 80, brightness: 50, temperature: 3500, effect: 'mystic' },
            dragon_fire: { hue: 15, saturation: 95, brightness: 85, temperature: 2200, effect: 'dragon' },
            fairy_grove: { hue: 140, saturation: 60, brightness: 70, temperature: 4000, effect: 'twinkle' },
            haunted: { hue: 300, saturation: 70, brightness: 30, temperature: 3000, effect: 'haunted' },
            santas_workshop: { hue: 0, saturation: 80, brightness: 90, temperature: 4500, effect: 'festive' },
            new_year: { hue: 50, saturation: 90, brightness: 100, temperature: 5000, effect: 'celebration' },
            valentines: { hue: 340, saturation: 80, brightness: 70, temperature: 3500, effect: 'romance' },
            halloween: { hue: 30, saturation: 90, brightness: 60, temperature: 2700, effect: 'spooky' },
            thanksgiving: { hue: 25, saturation: 70, brightness: 65, temperature: 3000, effect: 'warm' },
            christmas: { hue: 0, saturation: 85, brightness: 80, temperature: 4000, effect: 'festive' },
            easter: { hue: 100, saturation: 60, brightness: 75, temperature: 5000, effect: 'bounce' },
            st_patricks: { hue: 120, saturation: 85, brightness: 70, temperature: 4500, effect: 'irish' },
            independence_day: { hue: 220, saturation: 80, brightness: 85, temperature: 5500, effect: 'firework' }
        };
        return scenes[sceneName] || null;
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
    
    showTouchHoldProgress: function(touchElement) {
        if (!this.touchHoldProgressEl) {
            this.touchHoldProgressEl = document.getElementById('touch-hold-progress');
        }
        if (this.touchHoldProgressEl) {
            this.touchHoldProgressEl.classList.add('visible');
            this.touchHoldProgressEl.style.opacity = '1';
            this.touchHoldProgressEl.style.transform = 'scale(1)';
            
            // Add percentage display if element supports it
            const progressWithPercent = this.touchHoldProgressEl.querySelector('.touch-hold-progress-value');
            if (progressWithPercent) {
                progressWithPercent.textContent = '0%';
            }
            
            // Animate progress ring
            this.touchHoldProgressStartTime = Date.now();
            this.animateTouchHoldProgress();
        }
    },
    
    animateTouchHoldProgress: function() {
        if (!this.touchHoldProgressEl || !this.touchHoldTimer) return;
        
        const elapsed = Date.now() - this.touchHoldProgressStartTime;
        const progress = Math.min(100, Math.round((elapsed / this.touchHoldDelay) * 100));
        
        // Update percentage display
        const progressValue = this.touchHoldProgressEl.querySelector('.touch-hold-percentage');
        if (progressValue) {
            progressValue.textContent = progress + '%';
        }
        
        // Update progress bar using conic-gradient
        const progressBar = this.touchHoldProgressEl.querySelector('.touch-hold-progress-bar');
        if (progressBar) {
            progressBar.style.background = `conic-gradient(#00d4ff ${progress * 3.6}deg, transparent 0deg)`;
        }
        
        // Update ring rotation based on progress
        this.touchHoldProgressEl.style.transform = `translate(-50%, -50%) scale(${0.8 + (progress / 200)}) rotate(${progress * 3.6}deg)`;
        
        if (progress < 100) {
            requestAnimationFrame(() => this.animateTouchHoldProgress());
        }
    },
    
    hideTouchHoldProgress: function() {
        if (this.touchHoldProgressEl) {
            this.touchHoldProgressEl.classList.remove('visible');
            this.touchHoldProgressEl.style.opacity = '0';
            this.touchHoldProgressEl.style.transform = 'scale(0.8) rotate(0deg)';
            this.touchHoldProgressStartTime = null;
        }
    },
    
    setGestureSensitivityLevel: function(level) {
        this.setGestureSensitivity(level);
        this.showGestureFeedback(`Sensitivity: ${level}`, '✓');
    },
    
    showSensitivitySelector: function() {
        if (typeof Swal === 'undefined') {
            alert('Gesture Sensitivity: low, medium, high, very_high');
            return;
        }
        
        const current = localStorage.getItem('lifx_gesture_sensitivity') || 'medium';
        Swal.fire({
            title: 'Gesture Sensitivity',
            html: `
                <div style="display: flex; gap: 10px; justify-content: center; flex-wrap: wrap;">
                    <button class="btn btn-sm ${current === 'low' ? 'btn-primary' : 'btn-outline-primary'}" 
                            onclick="LifXTouchControls.setGestureSensitivityLevel('low')">Low</button>
                    <button class="btn btn-sm ${current === 'medium' ? 'btn-primary' : 'btn-outline-primary'}" 
                            onclick="LifXTouchControls.setGestureSensitivityLevel('medium')">Medium</button>
                    <button class="btn btn-sm ${current === 'high' ? 'btn-primary' : 'btn-outline-primary'}" 
                            onclick="LifXTouchControls.setGestureSensitivityLevel('high')">High</button>
                    <button class="btn btn-sm ${current === 'very_high' ? 'btn-primary' : 'btn-outline-primary'}" 
                            onclick="LifXTouchControls.setGestureSensitivityLevel('very_high')">Very High</button>
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
            'high': 'Most responsive',
            'very_high': 'Ultra-sensitive detection'
        };
        
        const swipeDistances = {
            'low': '80px',
            'medium': '50px', 
            'high': '30px',
            'very_high': '15px'
        };
        
        const longPressDelays = {
            'low': '700ms',
            'medium': '500ms',
            'high': '300ms',
            'very_high': '200ms'
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
                    <div class="sensitivity-option ${current === 'very_high' ? 'active' : ''}" onclick="LifXTouchControls.setGestureSensitivityLevel('very_high')">
                        <div class="sensitivity-option-label">
                            <span class="sensitivity-option-icon">⚡</span>
                            <div>
                                <div>Very High</div>
                                <div class="sensitivity-option-description">Maximum responsiveness - instant gesture detection</div>
                                <div class="sensitivity-option-description">Swipe: ${swipeDistances['very_high']} | Hold: ${longPressDelays['very_high']}</div>
                            </div>
                        </div>
                        ${current === 'very_high' ? '<i class="fas fa-check-circle" style="color: #00d4ff;"></i>' : ''}
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
                        <button class="btn btn-sm btn-outline-info" onclick="LifXTouchControls.testAllHapticPatterns()">
                            <i class="fas fa-layer-group"></i> Test All
                        </button>
                    </div>
                </div>
                
                <div class="touch-sensitivity-panel">
                    <h4><i class="fas fa-hand-paper"></i> Gesture Sensitivity Preview</h4>
                    <div style="display: flex; gap: 10px; flex-wrap: wrap;">
                        <button class="btn btn-sm btn-outline-primary" onclick="LifXTouchControls.previewGestureSensitivity()">
                            <i class="fas fa-eye"></i> Preview
                        </button>
                        <button class="btn btn-sm btn-outline-success" onclick="LifXTouchControls.openGestureTestPanel()">
                            <i class="fas fa-gamepad"></i> Test Panel
                        </button>
                        <button class="btn btn-sm btn-outline-warning" onclick="LifXTouchControls.setGestureSensitivity('very_high')">
                            <i class="fas fa-tachometer-alt"></i> Ultra Mode
                        </button>
                    </div>
                    <div style="margin-top: 10px; padding: 10px; background: rgba(0, 212, 255, 0.1); border-radius: 8px;">
                        <div style="display: flex; justify-content: space-between; align-items: center; font-size: 12px; color: #adb5bd;">
                            <span><i class="fas fa-sliders-h"></i> Current: <strong style="color: #00d4ff;">${current}</strong></span>
                            <span><i class="fas fa-ruler-horizontal"></i> Swipe: ${swipeDistances[current]}</span>
                            <span><i class="fas fa-clock"></i> Hold: ${longPressDelays[current]}</span>
                        </div>
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
    
    testAllHapticPatterns: function() {
        const patterns = ['light', 'success', 'swipe', 'pinch', 'scene_change', 'edge_swipe', 'zone_select', 'beat', 'media_sync_beat'];
        let index = 0;
        
        const testNext = () => {
            if (index >= patterns.length) {
                this.showEnhancedGestureFeedback('All patterns tested', '✅', 800);
                return;
            }
            const pattern = patterns[index];
            this.hapticFeedback(pattern);
            this.showEnhancedGestureFeedback(`Pattern: ${pattern}`, '📳', 400);
            index++;
            setTimeout(testNext, 600);
        };
        testNext();
    },
    
    previewGestureSensitivity: function() {
        if (typeof Swal === 'undefined') return;
        
        Swal.fire({
            title: '<i class="fas fa-hand-pointer"></i> Gesture Sensitivity Preview',
            html: `
                <div style="padding: 20px; background: rgba(0,0,0,0.3); border-radius: 10px; margin: 15px 0;">
                    <div style="display: flex; justify-content: space-around; align-items: center; margin-bottom: 20px;">
                        <div style="text-align: center;">
                            <div style="font-size: 24px; color: #00d4ff;">👆</div>
                            <div style="font-size: 11px; color: #adb5bd; margin-top: 5px;">Tap</div>
                        </div>
                        <div style="text-align: center;">
                            <div style="font-size: 24px; color: #ff6b6b;">👆👆</div>
                            <div style="font-size: 11px; color: #adb5bd; margin-top: 5px;">Double Tap</div>
                        </div>
                        <div style="text-align: center;">
                            <div style="font-size: 24px; color: #4ecdc4;">✋</div>
                            <div style="font-size: 11px; color: #adb5bd; margin-top: 5px;">Long Press</div>
                        </div>
                    </div>
                    <div style="display: flex; justify-content: space-around; align-items: center;">
                        <div style="text-align: center;">
                            <div style="font-size: 24px; color: #ffe66d;">🖐️</div>
                            <div style="font-size: 11px; color: #adb5bd; margin-top: 5px;">Swipe</div>
                        </div>
                        <div style="text-align: center;">
                            <div style="font-size: 24px; color: #a55eea;">🤏</div>
                            <div style="font-size: 11px; color: #adb5bd; margin-top: 5px;">Pinch</div>
                        </div>
                        <div style="text-align: center;">
                            <div style="font-size: 24px; color: #00ff88;">🎨</div>
                            <div style="font-size: 11px; color: #adb5bd; margin-top: 5px;">Scene</div>
                        </div>
                    </div>
                </div>
                <div style="text-align: center; color: #adb5bd; font-size: 12px;">
                    <p>Current: <strong style="color: #00d4ff;">${this.touchSensitivity}</strong></p>
                    <p>Swipe distance: <strong>${this.gestureSensitivity.swipeDistance}px</strong></p>
                    <p>Long press delay: <strong>${this.gestureSensitivity.longPressDelay}ms</strong></p>
                </div>
            `,
            confirmButtonText: 'Test Gestures',
            showCancelButton: true,
            cancelButtonText: 'Close',
            confirmButtonColor: '#00d4ff',
            cancelButtonColor: '#6c757d',
            background: 'rgba(30, 30, 45, 0.98)',
            backdrop: 'rgba(0, 0, 0, 0.8)'
        }).then((result) => {
            if (result.isConfirmed) {
                this.openGestureTestPanel();
            }
        });
    },
    
    openGestureTestPanel: function() {
        if (typeof Swal === 'undefined') return;
        
        let testResult = '';
        const updateResult = (gesture, detected) => {
            testResult = `<div style="margin-top: 10px; padding: 10px; background: ${detected ? 'rgba(0,255,136,0.2)' : 'rgba(255,107,107,0.2)'}; border-radius: 8px; color: ${detected ? '#00ff88' : '#ff6b6b'};">
                ${detected ? '✓' : '✗'} Detected: ${gesture}
            </div>`;
            Swal.getHtmlContainer().querySelector('.gesture-test-result').innerHTML = testResult;
        };
        
        Swal.fire({
            title: '<i class="fas fa-gamepad"></i> Gesture Test Panel',
            html: `
                <div style="padding: 20px; background: rgba(0,0,0,0.3); border-radius: 10px;">
                    <p style="color: #adb5bd; margin-bottom: 15px;">Perform gestures on this area to test sensitivity:</p>
                    <div id="gesture-test-area" style="height: 200px; background: rgba(0, 212, 255, 0.1); border: 2px dashed rgba(0, 212, 255, 0.3); border-radius: 8px; display: flex; align-items: center; justify-content: center; cursor: pointer;">
                        <span style="color: #00d4ff; font-size: 14px;">👆 Tap, swipe, or pinch here</span>
                    </div>
                    <div class="gesture-test-result" style="margin-top: 15px; min-height: 40px;"></div>
                </div>
            `,
            showConfirmButton: false,
            showCloseButton: true,
            didOpen: () => {
                const testArea = document.getElementById('gesture-test-area');
                let startX, startY, startTime;
                
                testArea.addEventListener('touchstart', (e) => {
                    startX = e.touches[0].clientX;
                    startY = e.touches[0].clientY;
                    startTime = Date.now();
                }, { passive: true });
                
                testArea.addEventListener('touchend', (e) => {
                    const endX = e.changedTouches[0].clientX;
                    const endY = e.changedTouches[0].clientY;
                    const deltaX = endX - startX;
                    const deltaY = endY - startY;
                    const deltaTime = Date.now() - startTime;
                    
                    if (Math.abs(deltaX) > this.gestureSensitivity.swipeDistance) {
                        updateResult(deltaX > 0 ? 'Swipe Right' : 'Swipe Left', true);
                        this.hapticFeedback('swipe');
                    } else if (Math.abs(deltaY) > this.gestureSensitivity.swipeDistance) {
                        updateResult(deltaY > 0 ? 'Swipe Down' : 'Swipe Up', true);
                        this.hapticFeedback('swipe');
                    } else if (deltaTime < this.gestureSensitivity.doubleTapDelay) {
                        updateResult('Tap', true);
                        this.hapticFeedback('light');
                    } else {
                        updateResult('Invalid gesture', false);
                    }
                });
            },
            background: 'rgba(30, 30, 45, 0.98)',
            backdrop: 'rgba(0, 0, 0, 0.8)'
        });
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
            
            const touches = Array.from(e.touches);
            const rect = bulbEl.getBoundingClientRect();
            
            touches.forEach((touch, index) => {
                const x = touch.clientX - rect.left;
                const y = touch.clientY - rect.top;
                
                const velocity = this.getTouchVelocity(touch);
                const sizeMultiplier = 1 + Math.min(velocity, 1.5);
                
                const rippleSize = this.rippleSize * sizeMultiplier;
                const ripple = document.createElement('span');
                ripple.className = 'lifx-touch-ripple' + (this.enhancedRippleMode ? ' enhanced' : '');
                ripple.style.left = (x - rippleSize / 2) + 'px';
                ripple.style.top = (y - rippleSize / 2) + 'px';
                ripple.style.width = rippleSize + 'px';
                ripple.style.height = rippleSize + 'px';
                
                if (this.enhancedRippleMode) {
                    const bulbState = this.getBulbState(bulbEl);
                    const rippleColor = this.getRippleColorForState(bulbState);
                    ripple.style.background = `radial-gradient(circle, ${rippleColor} 0%, transparent 70%)`;
                    ripple.style.animationDuration = (this.rippleDuration / 1000) + 's';
                    
                    if (this.glowEffectEnabled) {
                        bulbEl.classList.add('touch-glow');
                        setTimeout(() => bulbEl.classList.remove('touch-glow'), 300);
                    }
                }
                
                if (this.multiBulbSelection && this.multiBulbSelection.size > 1 && index === 0) {
                    this.createMultiSelectRipple(bulbEl, x, y);
                }
                
                bulbEl.appendChild(ripple);
                
                setTimeout(() => {
                    if (ripple.parentNode) {
                        ripple.parentNode.removeChild(ripple);
                    }
                }, this.rippleDuration);
            });
        }, { passive: true });
    },
    
    initGestureEnhancements: function() {
        this.touchSwipeThreshold = this.gestureSensitivity.swipeDistance;
        this.touchSwipeVelocityThreshold = 0.3;
        this.lastTouchPositions = new Map();
        this.touchVelocity = new Map();
        this.gestureRecognitionActive = false;
        this.gestureDebounceCount = 0;
        this.maxGestureDebounceCount = 3;
        this.setupMultiSelectMode();
        this.setupTouchHoldProgress();
        this.setupQuickActions();
        this.setupZoneControl();
        this.loadRipplePreferences();
        this.initEdgeSwipeDetection();
        this.initAdaptiveSensitivity();
        this.initGestureReliabilityImprovements();
        console.log('Gesture enhancements initialized with edge swipe detection');
    },
    
    initGestureReliabilityImprovements: function() {
        // Add gesture accuracy tracking
        this.gestureAccuracyWindow = [];
        this.maxGestureAccuracyWindow = 20;
        this.minGestureConfidence = 0.6;
        
        // Track gesture patterns for improved recognition
        this.gesturePatterns = {
            swipeUp: [],
            swipeDown: [],
            swipeLeft: [],
            swipeRight: []
        };
        
        // Add gesture validation
        this.validateGestureBeforeExecute = true;
        
        console.log('Gesture reliability improvements initialized');
    },
    
    initEnhancedTouchTracking: function(e) {
        const touch = e.touches[0];
        const timestamp = Date.now();
        
        this.touchStartTime = timestamp;
        this.touchStartX = touch.clientX;
        this.touchStartY = touch.clientY;
        this.touchVelocityHistory = [];
        
        if (this.pressureSensitiveEnabled && touch.force !== undefined) {
            this.touchPressure = touch.force;
        }
        
        if (this.touchTrailEnabled) {
            this.createTouchTrailDot(touch.clientX, touch.clientY);
        }
    },
    
    updateTouchVelocity: function(e) {
        const touch = e.touches[0];
        const timestamp = Date.now();
        const currentX = touch.clientX;
        const currentY = touch.clientY;
        
        if (!this.lastTouchTime) {
            this.lastTouchTime = timestamp;
            this.lastTouchX = currentX;
            this.lastTouchY = currentY;
            return;
        }
        
        const deltaTime = (timestamp - this.lastTouchTime) / 1000;
        if (deltaTime === 0) return;
        
        const deltaX = currentX - this.lastTouchX;
        const deltaY = currentY - this.lastTouchY;
        const distance = Math.sqrt(deltaX * deltaX + deltaY * deltaY);
        const velocity = distance / deltaTime;
        
        this.touchVelocityHistory.push({
            velocity: velocity,
            deltaX: deltaX,
            deltaY: deltaY,
            timestamp: timestamp
        });
        
        if (this.touchVelocityHistory.length > this.maxVelocityHistory) {
            this.touchVelocityHistory.shift();
        }
        
        const avgVelocity = this.touchVelocityHistory.reduce((sum, v) => sum + v.velocity, 0) / this.touchVelocityHistory.length;
        this.touchVelocity = avgVelocity;
        
        const angle = Math.atan2(deltaY, deltaX) * (180 / Math.PI);
        this.touchDirection = angle;
        
        if (this.touchTrailEnabled) {
            this.createTouchTrailDot(currentX, currentY, velocity);
        }
        
        this.lastTouchTime = timestamp;
        this.lastTouchX = currentX;
        this.lastTouchY = currentY;
    },
    
    createTouchTrailDot: function(x, y, velocity = 0) {
        if (!this.touchTrailEnabled) return;
        
        const trailDot = document.createElement('div');
        trailDot.className = 'lifx-gesture-trail';
        trailDot.style.left = (x - 10) + 'px';
        trailDot.style.top = (y - 10) + 'px';
        
        const intensity = Math.min(1, velocity / 500);
        trailDot.style.opacity = 0.3 + (intensity * 0.5);
        trailDot.style.transform = `scale(${0.8 + intensity * 0.4})`;
        
        document.body.appendChild(trailDot);
        
        setTimeout(() => {
            if (trailDot.parentNode) {
                trailDot.parentNode.removeChild(trailDot);
            }
        }, 500);
    },
    
    processGestureVelocity: function(e) {
        if (this.touchVelocityHistory.length === 0) return;
        
        const avgVelocity = this.touchVelocityHistory.reduce((sum, v) => sum + v.velocity, 0) / this.touchVelocityHistory.length;
        const maxVelocity = Math.max(...this.touchVelocityHistory.map(v => v.velocity));
        
        this.lastGestureVelocity = maxVelocity;
        
        if (this.adaptiveSensitivityEnabled) {
            this.updateAdaptiveSensitivity(avgVelocity);
        }
        
        this.touchVelocityHistory = [];
        this.lastTouchTime = null;
    },
    
    initAdaptiveSensitivity: function() {
        const saved = localStorage.getItem('lifx_adaptive_sensitivity');
        if (saved !== null) {
            this.adaptiveSensitivityEnabled = saved === 'true';
        }
        
        if (!this.adaptiveSensitivityEnabled) return;
        
        this.gestureSuccessCount = 0;
        this.gestureFailCount = 0;
        
        setInterval(() => {
            if (this.gestureSuccessCount + this.gestureFailCount > 10) {
                const successRate = this.gestureSuccessCount / (this.gestureSuccessCount + this.gestureFailCount);
                if (successRate < 0.7) {
                    this.increaseSensitivity();
                } else if (successRate > 0.95) {
                    this.decreaseSensitivity();
                }
            }
            this.gestureSuccessCount = 0;
            this.gestureFailCount = 0;
        }, 60000);
    },
    
    updateAdaptiveSensitivity: function(avgVelocity) {
        if (avgVelocity > 300 && this.touchSensitivity !== 'high') {
            this.setGestureSensitivityLevel('high');
        } else if (avgVelocity < 100 && this.touchSensitivity !== 'low') {
            this.setGestureSensitivityLevel('low');
        }
    },
    
    increaseSensitivity: function() {
        const levels = ['low', 'medium', 'high'];
        const currentIndex = levels.indexOf(this.touchSensitivity);
        if (currentIndex < levels.length - 1) {
            this.setGestureSensitivityLevel(levels[currentIndex + 1]);
        }
    },
    
    decreaseSensitivity: function() {
        const levels = ['low', 'medium', 'high'];
        const currentIndex = levels.indexOf(this.touchSensitivity);
        if (currentIndex > 0) {
            this.setGestureSensitivityLevel(levels[currentIndex - 1]);
        }
    },
    
    getTouchVelocity: function(touch) {
        if (!this.lastTouchPositions.has(touch.identifier)) {
            return 0;
        }
        
        const lastPos = this.lastTouchPositions.get(touch.identifier);
        const deltaX = touch.clientX - lastPos.x;
        const deltaY = touch.clientY - lastPos.y;
        const deltaTime = Date.now() - lastPos.time;
        
        if (deltaTime === 0) return 0;
        
        const distance = Math.sqrt(deltaX * deltaX + deltaY * deltaY);
        const velocity = distance / deltaTime;
        
        this.lastTouchPositions.set(touch.identifier, {
            x: touch.clientX,
            y: touch.clientY,
            time: Date.now()
        });
        
        return Math.min(velocity, 10);
    },
    
    getBulbState: function(bulbEl) {
        const bulbId = bulbEl.dataset.bulbId;
        if (!bulbId) return { on: false, brightness: 0, color: '#000000' };
        
        const bulbData = this.bulbs.find(b => b.id === bulbId);
        if (!bulbData) return { on: false, brightness: 0, color: '#000000' };
        
        return {
            on: bulbData.power || false,
            brightness: bulbData.brightness || 0,
            color: bulbData.color || '#000000',
            temperature: bulbData.temperature || 3500
        };
    },
    
    getRippleColorForState: function(bulbState) {
        if (!bulbState.on) {
            return 'rgba(255, 255, 255, 0.3)';
        }
        
        const brightnessFactor = bulbState.brightness / 100;
        
        if (bulbState.color && bulbState.color !== '#000000') {
            const hex = bulbState.color.replace('#', '');
            const r = parseInt(hex.substr(0, 2), 16);
            const g = parseInt(hex.substr(2, 2), 16);
            const b = parseInt(hex.substr(4, 2), 16);
            return `rgba(${r}, ${g}, ${b}, ${0.4 + brightnessFactor * 0.4})`;
        }
        
        const kelvin = bulbState.temperature || 3500;
        if (kelvin < 2500) {
            return `rgba(255, 180, 100, ${0.4 + brightnessFactor * 0.4})`;
        } else if (kelvin < 4000) {
            return `rgba(255, 220, 180, ${0.4 + brightnessFactor * 0.4})`;
        } else {
            return `rgba(180, 220, 255, ${0.4 + brightnessFactor * 0.4})`;
        }
    },
    
    createMultiSelectRipple: function(bulbEl, x, y) {
        if (!this.multiBulbSelection || this.multiBulbSelection.size <= 1) return;
        
        const selectionRipple = document.createElement('div');
        selectionRipple.className = 'lifx-multi-select-ripple';
        selectionRipple.style.left = x + 'px';
        selectionRipple.style.top = y + 'px';
        selectionRipple.innerHTML = `<span class="ripple-count">${this.multiBulbSelection.size}</span>`;
        
        bulbEl.appendChild(selectionRipple);
        
        setTimeout(() => {
            if (selectionRipple.parentNode) {
                selectionRipple.parentNode.removeChild(selectionRipple);
            }
        }, this.rippleDuration);
    },
    
    saveAdaptiveSensitivityStats: function() {
        localStorage.setItem('lifx_adaptive_sensitivity', this.adaptiveSensitivityEnabled);
        localStorage.setItem('lifx_gesture_success_count', this.gestureSuccessCount);
        localStorage.setItem('lifx_gesture_fail_count', this.gestureFailCount);
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
        this.initZonePresets();
    },
    
    zonePresets: {},
    selectedZone: null,
    
    initZonePresets: function() {
        const saved = localStorage.getItem('lifx_zone_presets');
        if (saved) {
            try {
                this.zonePresets = JSON.parse(saved);
            } catch (e) {
                console.error('Failed to load zone presets:', e);
            }
        }
    },
    
    saveZonePreset: function(name) {
        if (!this.selectedZone) return;
        this.zonePresets[name] = { zone: this.selectedZone };
        localStorage.setItem('lifx_zone_presets', JSON.stringify(this.zonePresets));
        this.showGestureFeedback(`Preset "${name}" saved`, '💾');
    },
    
    loadZonePreset: function(name) {
        const preset = this.zonePresets[name];
        if (preset) {
            this.selectedZone = preset.zone;
            this.showGestureFeedback(`Preset "${name}" loaded`, '📂');
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
        
        const presetNames = Object.keys(this.zonePresets);
        
        Swal.fire({
            title: '<i class="fas fa-sliders-h"></i> Zone Control',
            html: `
                <div style="padding: 15px;">
                    <div style="text-align: center; margin-bottom: 15px;">
                        <p style="color: #adb5bd; font-size: 12px;">Select a zone and apply colors or effects</p>
                        ${this.selectedZone !== null ? `<span style="color: #00d4ff; font-weight: bold;">Selected: Zone ${this.selectedZone + 1}</span>` : ''}
                    </div>
                    
                    <div class="zone-visualization" style="display: flex; gap: 4px; justify-content: center; margin-bottom: 20px; padding: 10px; background: rgba(0, 0, 0, 0.3); border-radius: 10px;">
                        ${Array.from({length: 10}, (_, i) => `
                            <div class="zone-visual-item" data-zone="${i}" 
                                 style="width: 30px; height: ${60 + Math.sin(i * 0.5) * 20}px; background: linear-gradient(to top, rgba(39, 160, 185, ${0.3 + (i/10) * 0.7}), rgba(0, 212, 255, ${0.5 + (i/10) * 0.5})); border-radius: 4px; cursor: pointer; transition: all 0.2s; ${this.selectedZone === i ? 'border: 2px solid #00d4ff; transform: scaleY(1.2);' : 'border: 2px solid transparent;'}"
                                 onclick="LifXTouchControls.selectZone(${i}, this)">
                            </div>
                        `).join('')}
                    </div>
                    
                    <div class="zone-grid" style="display: grid; grid-template-columns: repeat(5, 1fr); gap: 8px; margin-bottom: 20px;">
                        ${Array.from({length: 10}, (_, i) => `
                            <div class="zone-item" data-zone="${i}" style="padding: 12px 8px; background: rgba(39, 160, 185, 0.2); border: 2px solid ${this.selectedZone === i ? '#00d4ff' : 'transparent'}; border-radius: 8px; text-align: center; cursor: pointer; transition: all 0.2s;"
                                 onclick="LifXTouchControls.selectZone(${i}, this)">
                                <i class="fas fa-lightbulb" style="font-size: 18px; margin-bottom: 5px; color: ${this.selectedZone === i ? '#00d4ff' : '#adb5bd'};"></i>
                                <div style="font-size: 10px; color: #adb5bd;">Zone ${i + 1}</div>
                            </div>
                        `).join('')}
                    </div>
                    
                    <div class="zone-color-picker" style="display: flex; gap: 8px; justify-content: center; flex-wrap: wrap; margin-bottom: 20px;">
                        ${['#ff0000', '#ff8000', '#ffff00', '#00ff00', '#00ffff', '#0000ff', '#8000ff', '#ff00ff', '#ffffff', '#ffcc00', '#ff6b6b', '#00d4ff'].map(color => `
                            <button class="zone-color-btn" style="width: 36px; height: 36px; border-radius: 50%; border: 2px solid rgba(255,255,255,0.3); background: ${color}; cursor: pointer; transition: all 0.2s; transform: scale(1);"
                                    onmouseover="this.style.transform='scale(1.2)'" onmouseout="this.style.transform='scale(1)'"
                                    onclick="LifXTouchControls.applyZoneColor('${color}', '${targets.join(',')}')"></button>
                        `).join('')}
                    </div>
                    
                    ${presetNames.length > 0 ? `
                    <div class="zone-presets" style="margin-top: 15px;">
                        <label style="color: #adb5bd; font-size: 12px; display: block; margin-bottom: 8px;">Saved Presets</label>
                        <div style="display: flex; gap: 8px; flex-wrap: wrap;">
                            ${presetNames.map(name => `
                                <button class="zone-preset-btn" style="padding: 8px 16px; background: rgba(0, 212, 255, 0.2); border: 1px solid rgba(0, 212, 255, 0.4); border-radius: 20px; color: #00d4ff; cursor: pointer; transition: all 0.2s;"
                                        onclick="LifXTouchControls.loadZonePreset('${name}')">
                                    <i class="fas fa-folder"></i> ${name}
                                </button>
                            `).join('')}
                        </div>
                    </div>
                    ` : ''}
                    
                    <div class="zone-effects" style="margin-top: 15px;">
                        <label style="color: #adb5bd; font-size: 12px; display: block; margin-bottom: 8px;">Zone Effects</label>
                        <div style="display: flex; gap: 8px; flex-wrap: wrap;">
                            <button style="padding: 8px 16px; background: rgba(255, 107, 107, 0.2); border: 1px solid rgba(255, 107, 107, 0.4); border-radius: 8px; color: #ff6b6b; cursor: pointer;"
                                    onclick="LifXTouchControls.applyZoneEffect('pulse')">💓 Pulse</button>
                            <button style="padding: 8px 16px; background: rgba(0, 255, 136, 0.2); border: 1px solid rgba(0, 255, 136, 0.4); border-radius: 8px; color: #00ff88; cursor: pointer;"
                                    onclick="LifXTouchControls.applyZoneEffect('wave')">🌊 Wave</button>
                            <button style="padding: 8px 16px; background: rgba(255, 193, 7, 0.2); border: 1px solid rgba(255, 193, 7, 0.4); border-radius: 8px; color: #ffc107; cursor: pointer;"
                                    onclick="LifXTouchControls.applyZoneEffect('rainbow')">🌈 Rainbow</button>
                        </div>
                    </div>
                </div>
            `,
            showConfirmButton: false,
            showCloseButton: true,
            width: '650px',
            backdrop: 'rgba(0, 0, 0, 0.8)'
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
    
    applyZoneEffect: function(effectType) {
        if (this.selectedZone === null) {
            this.showGestureFeedback('Select a zone first', '⚠️');
            return;
        }
        
        const targets = this.multiBulbSelection.length > 0 
            ? this.multiBulbSelection 
            : (this.selectedBulb ? [this.selectedBulb] : ['all']);
        
        const effectConfigs = {
            'pulse': { brightness: 100, duration: 0.3, cycles: 3 },
            'wave': { brightness: 80, duration: 0.5, wave: true },
            'rainbow': { hue: 0, saturation: 100, brightness: 90, duration: 2, rainbow: true }
        };
        
        const config = effectConfigs[effectType];
        if (!config) return;
        
        $.ajax({
            url: '/api/services/lifx/zones',
            method: 'POST',
            contentType: 'application/json',
            data: JSON.stringify({
                selector: `id:${targets.join(',')}`,
                start_index: this.selectedZone,
                end_index: this.selectedZone,
                effect: effectType,
                duration: config.duration,
                ...config
            }),
            success: () => {
                this.showGestureFeedback(`Zone ${this.selectedZone + 1}: ${effectType}`, '✨');
                this.hapticFeedback('success');
            },
            error: (xhr) => {
                console.error('Zone effect failed:', xhr);
                this.showGestureFeedback('Effect failed', '❌');
            }
        });
    },
    
    colorPaintingActive: false,
    colorPaintingBrushSize: 1,
    colorPaintingCurrentColor: '#00d4ff',
    colorPaintingHistory: [],
    
    toggleColorPainting: function() {
        this.colorPaintingActive = !this.colorPaintingActive;
        if (this.colorPaintingActive) {
            this.showEnhancedGestureFeedback('Color Painting ON - Drag across zones', '🎨');
            this.initColorPaintingMode();
        } else {
            this.showEnhancedGestureFeedback('Color Painting OFF', '🎨');
        }
    },
    
    initColorPaintingMode: function() {
        if (!this.colorPaintingActive) return;
        
        const zoneContainer = document.querySelector('.zone-visualization, .zone-grid');
        if (!zoneContainer) return;
        
        zoneContainer.style.cursor = 'crosshair';
        zoneContainer.addEventListener('mousedown', this.handlePaintStart.bind(this));
        zoneContainer.addEventListener('mousemove', this.handlePaintMove.bind(this));
        zoneContainer.addEventListener('mouseup', this.handlePaintEnd.bind(this));
        zoneContainer.addEventListener('touchstart', this.handlePaintStart.bind(this), { passive: true });
        zoneContainer.addEventListener('touchmove', this.handlePaintMove.bind(this), { passive: false });
        zoneContainer.addEventListener('touchend', this.handlePaintEnd.bind(this));
    },
    
    handlePaintStart: function(e) {
        if (!this.colorPaintingActive) return;
        e.preventDefault();
        const zoneEl = e.target.closest('[data-zone]');
        if (zoneEl) {
            const zoneIndex = parseInt(zoneEl.dataset.zone);
            this.paintZone(zoneIndex);
        }
    },
    
    handlePaintMove: function(e) {
        if (!this.colorPaintingActive || e.buttons !== 1) return;
        e.preventDefault();
        
        const touch = e.touches ? e.touches[0] : e;
        const elements = document.elementsFromPoint(touch.clientX, touch.clientY);
        
        for (const el of elements) {
            const zoneEl = el.closest('[data-zone]');
            if (zoneEl) {
                const zoneIndex = parseInt(zoneEl.dataset.zone);
                const lastPainted = this.colorPaintingHistory[this.colorPaintingHistory.length - 1];
                if (lastPainted !== zoneIndex) {
                    this.paintZone(zoneIndex);
                    this.colorPaintingHistory.push(zoneIndex);
                }
                break;
            }
        }
    },
    
    handlePaintEnd: function() {
        if (!this.colorPaintingActive) return;
        this.colorPaintingHistory = [];
    },
    
    paintZone: function(zoneIndex) {
        const targets = this.multiBulbSelection.length > 0 
            ? this.multiBulbSelection 
            : (this.selectedBulb ? [this.selectedBulb] : []);
        
        if (targets.length === 0) return;
        
        const rgb = this.hexToRgb(this.colorPaintingCurrentColor);
        const hsv = this.rgbToHsv(rgb.r, rgb.g, rgb.b);
        
        $.ajax({
            url: '/api/services/lifx/zones',
            method: 'POST',
            contentType: 'application/json',
            data: JSON.stringify({
                selector: `id:${targets.join(',')}`,
                start_index: zoneIndex,
                end_index: zoneIndex,
                color: `hue:${Math.round(hsv.h * 182)} saturation:${Math.round(hsv.s * 100)}% brightness:${Math.round(hsv.v * 100)}%`,
                duration: 0.3
            }),
            success: () => {
                this.hapticFeedback('light', 0.3);
            }
        });
    },
    
    setColorPaintingColor: function(hexColor) {
        this.colorPaintingCurrentColor = hexColor;
        this.showEnhancedGestureFeedback(`Brush color: ${hexColor}`, '🎨');
    },
    
    setColorPaintingBrushSize: function(size) {
        this.colorPaintingBrushSize = size;
        this.showEnhancedGestureFeedback(`Brush size: ${size}`, '🖌️');
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
        this.initBpmRealtimeDisplay();
        this.initMediaSessionIntegration();
    },
    
    initMediaSessionIntegration: function() {
        if (!('mediaSession' in navigator)) {
            console.log('Media Session API not supported');
            return;
        }
        
        navigator.mediaSession.setActionHandler('play', () => {
            this.handleMediaSessionAction('play');
        });
        
        navigator.mediaSession.setActionHandler('pause', () => {
            this.handleMediaSessionAction('pause');
        });
        
        navigator.mediaSession.setActionHandler('previoustrack', () => {
            this.handleMediaSessionAction('previous');
        });
        
        navigator.mediaSession.setActionHandler('nexttrack', () => {
            this.handleMediaSessionAction('next');
        });
        
        navigator.mediaSession.setActionHandler('stop', () => {
            this.handleMediaSessionAction('stop');
        });
        
        console.log('Media Session API integration initialized');
    },
    
    handleMediaSessionAction: function(action) {
        console.log('Media Session action:', action);
        
        switch(action) {
            case 'play':
                if (typeof MediaPlayer !== 'undefined' && MediaPlayer.togglePlayPause) {
                    MediaPlayer.isPlaying = true;
                    this.mediaPlaybackActive = true;
                    this.updateMediaSessionMetadata();
                }
                break;
            case 'pause':
                if (typeof MediaPlayer !== 'undefined' && MediaPlayer.togglePlayPause) {
                    MediaPlayer.isPlaying = false;
                    this.mediaPlaybackActive = false;
                }
                break;
            case 'previous':
                if (typeof MediaPlayer !== 'undefined' && MediaPlayer.previousTrack) {
                    MediaPlayer.previousTrack();
                    setTimeout(() => this.updateMediaSessionMetadata(), 500);
                }
                break;
            case 'next':
                if (typeof MediaPlayer !== 'undefined' && MediaPlayer.nextTrack) {
                    MediaPlayer.nextTrack();
                    setTimeout(() => this.updateMediaSessionMetadata(), 500);
                }
                break;
            case 'stop':
                this.mediaPlaybackActive = false;
                break;
        }
        
        this.showEnhancedGestureFeedback(`Media: ${action}`, '🎵');
    },
    
    updateMediaSessionMetadata: function() {
        if (!('mediaSession' in navigator)) return;
        
        const trackInfo = typeof MediaPlayer !== 'undefined' ? MediaPlayer.currentTrack : null;
        
        navigator.mediaSession.metadata = new MediaMetadata({
            title: trackInfo?.title || 'Unknown Track',
            artist: trackInfo?.artist || 'Unknown Artist',
            album: trackInfo?.album || 'Unknown Album',
            artwork: [
                { src: '/assets/img/media-placeholder.png', sizes: '96x96', type: 'image/png' },
                { src: '/assets/img/media-placeholder.png', sizes: '128x128', type: 'image/png' },
                { src: '/assets/img/media-placeholder.png', sizes: '192x192', type: 'image/png' },
                { src: '/assets/img/media-placeholder.png', sizes: '256x256', type: 'image/png' },
                { src: '/assets/img/media-placeholder.png', sizes: '384x384', type: 'image/png' },
                { src: '/assets/img/media-placeholder.png', sizes: '512x512', type: 'image/png' }
            ]
        });
        
        navigator.mediaSession.playbackState = this.mediaPlaybackActive ? 'playing' : 'paused';
    },
    
    initBpmRealtimeDisplay: function() {
        const bpmDisplay = document.getElementById('realtime-bpm');
        const bpmValueDisplay = document.getElementById('bpm-value-display');
        
        if (!bpmDisplay && !bpmValueDisplay) return;
        
        setInterval(() => {
            if (this.bpmValue > 0) {
                if (bpmDisplay) bpmDisplay.textContent = Math.round(this.bpmValue);
                if (bpmValueDisplay) bpmValueDisplay.textContent = Math.round(this.bpmValue);
                
                const bpmIndicator = document.getElementById('bpm-realtime-indicator');
                if (bpmIndicator && this.mediaPlaybackActive) {
                    bpmIndicator.classList.add('visible');
                }
            }
        }, 500);
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
            this.audioAnalyzer.fftSize = 512;
            this.audioAnalyzer.smoothingTimeConstant = 0.85;
            
            this.mediaPlaybackActive = true;
            this.initFrequencyBandAnalysis();
            this.monitorBeat();
            this.monitorFrequencyBands();
            console.log('Beat detection initialized successfully with enhanced visualization');
        } catch (e) {
            console.warn('Beat detection not available:', e);
            this.showBeatDetectionFallback();
        }
    },
    
    frequencyBands: {
        subBass: { min: 0, max: 4, value: 0 },
        bass: { min: 4, max: 12, value: 0 },
        lowMid: { min: 12, max: 30, value: 0 },
        mid: { min: 30, max: 60, value: 0 },
        highMid: { min: 60, max: 120, value: 0 },
        treble: { min: 120, max: 255, value: 0 }
    },
    beatHistory: [],
    maxBeatHistory: 8,
    beatPatternDetected: null,
    
    initFrequencyBandAnalysis: function() {
        const visualization = document.getElementById('beat-energy-viz');
        if (!visualization) return;
        
        visualization.innerHTML = `
            <div class="beat-energy-bar" id="band-subBass" title="Sub-Bass"></div>
            <div class="beat-energy-bar" id="band-bass" title="Bass"></div>
            <div class="beat-energy-bar" id="band-lowMid" title="Low-Mid"></div>
            <div class="beat-energy-bar" id="band-mid" title="Mid"></div>
            <div class="beat-energy-bar" id="band-highMid" title="High-Mid"></div>
            <div class="beat-energy-bar" id="band-treble" title="Treble"></div>
        `;
    },
    
    frequencyDataCache: null,
    visualizationFrameCount: 0,
    lastVisualizationUpdate: 0,
    visualizationThrottleMs: 33,
    
    monitorFrequencyBands: function() {
        if (!this.audioAnalyzer) return;
        
        const bufferLength = this.audioAnalyzer.frequencyBinCount;
        this.frequencyDataCache = new Uint8Array(bufferLength);
        const smoothedValues = { subBass: 0, bass: 0, lowMid: 0, mid: 0, highMid: 0, treble: 0 };
        
        const analyzeBands = () => {
            if (!this.mediaPlaybackActive) {
                requestAnimationFrame(analyzeBands);
                return;
            }
            
            this.audioAnalyzer.getByteFrequencyData(this.frequencyDataCache);
            
            const bands = this.frequencyBands;
            const rawData = {
                subBass: this.getBandAverage(this.frequencyDataCache, bands.subBass.min, bands.subBass.max),
                bass: this.getBandAverage(this.frequencyDataCache, bands.bass.min, bands.bass.max),
                lowMid: this.getBandAverage(this.frequencyDataCache, bands.lowMid.min, bands.lowMid.max),
                mid: this.getBandAverage(this.frequencyDataCache, bands.mid.min, bands.mid.max),
                highMid: this.getBandAverage(this.frequencyDataCache, bands.highMid.min, bands.highMid.max),
                treble: this.getBandAverage(this.frequencyDataCache, bands.treble.min, bands.treble.max)
            };
            
            const smoothingFactor = 0.25;
            for (const band in smoothedValues) {
                smoothedValues[band] = smoothedValues[band] + (rawData[band] - smoothedValues[band]) * smoothingFactor;
                bands[band].value = smoothedValues[band];
            }
            
            const now = Date.now();
            if (now - this.lastVisualizationUpdate > this.visualizationThrottleMs) {
                this.updateFrequencyVisualization();
                this.lastVisualizationUpdate = now;
            }
            
            this.visualizationFrameCount++;
            if (this.visualizationFrameCount % 30 === 0) {
                this.analyzeBeatPattern();
            }
            
            requestAnimationFrame(analyzeBands);
        };
        
        analyzeBands();
    },
    
    getBandAverage: function(dataArray, start, end) {
        let sum = 0;
        for (let i = start; i < Math.min(end, dataArray.length); i++) {
            sum += dataArray[i];
        }
        return sum / (Math.min(end, dataArray.length) - start);
    },
    
    updateFrequencyVisualization: function() {
        const bands = this.frequencyBands;
        const bandElements = ['subBass', 'bass', 'lowMid', 'mid', 'highMid', 'treble'];
        const colors = ['#ff0080', '#ff6b6b', '#f39c12', '#00d4ff', '#00ff88', '#9b59b6'];
        
        bandElements.forEach((band, index) => {
            const el = document.getElementById(`band-${band}`);
            if (el) {
                const targetHeight = Math.min(100, (bands[band].value / 255) * 100);
                const currentHeight = parseFloat(el.dataset.currentHeight || '0');
                const smoothedHeight = currentHeight + (targetHeight - currentHeight) * 0.3;
                
                el.dataset.currentHeight = smoothedHeight;
                el.style.height = `${smoothedHeight}%`;
                el.style.background = `linear-gradient(to top, ${colors[index]} 0%, ${colors[index]}88 100%)`;
                
                if (targetHeight > 85) {
                    el.classList.add('peak');
                    if (!el.dataset.peakHeight || targetHeight > parseFloat(el.dataset.peakHeight)) {
                        el.dataset.peakHeight = targetHeight;
                        el.dataset.peakTime = Date.now();
                    }
                } else {
                    el.classList.remove('peak');
                }
                
                let peakEl = el.querySelector('.peak-hold');
                if (!peakEl && targetHeight > 50) {
                    peakEl = document.createElement('div');
                    peakEl.className = 'peak-hold';
                    peakEl.style.cssText = 'position: absolute; top: 0; left: 0; right: 0; height: 3px; background: rgba(255,255,255,0.8); border-radius: 2px; pointer-events: none; transition: top 0.3s ease-out;';
                    el.appendChild(peakEl);
                }
                
                if (peakEl) {
                    const peakHeight = parseFloat(el.dataset.peakHeight || '0');
                    const peakTime = parseInt(el.dataset.peakTime || '0');
                    const decay = (Date.now() - peakTime) / 2000;
                    const decayedPeak = Math.max(0, peakHeight - (decayedPeak * decay));
                    peakEl.style.top = `${100 - decayedPeak}%`;
                    
                    if (decayedPeak <= 0) {
                        el.dataset.peakHeight = '0';
                        if (peakEl.parentNode) peakEl.parentNode.removeChild(peakEl);
                    }
                }
            }
        });
    },
    
    analyzeBeatPattern: function() {
        const bassEnergy = this.frequencyBands.bass.value;
        const threshold = this.beatDetectionSensitivity * 255;
        
        if (bassEnergy > threshold) {
            const now = Date.now();
            this.beatHistory.push(now);
            
            if (this.beatHistory.length > this.maxBeatHistory) {
                this.beatHistory.shift();
            }
            
            if (this.beatHistory.length >= 4) {
                const intervals = [];
                for (let i = 1; i < this.beatHistory.length; i++) {
                    intervals.push(this.beatHistory[i] - this.beatHistory[i-1]);
                }
                
                const avgInterval = intervals.reduce((a, b) => a + b, 0) / intervals.length;
                const consistency = intervals.every(i => Math.abs(i - avgInterval) < 100);
                
                if (consistency && avgInterval > 200 && avgInterval < 1500) {
                    const detectedBPM = Math.round(60000 / avgInterval);
                    if (detectedBPM !== this.beatPatternDetected) {
                        this.beatPatternDetected = detectedBPM;
                        console.log(`Beat pattern detected: ${detectedBPM} BPM`);
                        this.showBeatPatternNotification(detectedBPM);
                    }
                }
            }
        }
    },
    
    showBeatPatternNotification: function(bpm) {
        if (typeof Swal === 'undefined') return;
        
        Swal.fire({
            title: 'Beat Pattern Detected',
            html: `
                <div style="text-align: center;">
                    <i class="fas fa-music" style="font-size: 48px; color: #ff6b6b; margin-bottom: 15px;"></i>
                    <p style="color: #00d4ff; font-size: 24px; font-weight: bold;">${bpm} BPM</p>
                    <p style="color: #adb5bd; margin-top: 10px;">Lighting effects synchronized to detected beat</p>
                </div>
            `,
            timer: 3000,
            showConfirmButton: false,
            background: 'rgba(30, 30, 45, 0.98)',
            backdrop: 'rgba(0, 0, 0, 0.8)'
        });
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
        const bpmHistory = [];
        const maxBpmHistory = 8;
        const energyHistory = [];
        const maxEnergyHistory = 20;
        
        const detectBeat = () => {
            if (!this.mediaPlaybackActive) {
                requestAnimationFrame(detectBeat);
                return;
            }
            
            this.audioAnalyzer.getByteFrequencyData(dataArray);
            
            const bass = dataArray.slice(0, 10).reduce((a, b) => a + b, 0) / 10;
            const mids = dataArray.slice(10, 50).reduce((a, b) => a + b, 0) / 40;
            const treble = dataArray.slice(50, 128).reduce((a, b) => a + b, 0) / 78;
            const totalEnergy = (bass + mids + treble) / 3;
            
            energyHistory.push(totalEnergy);
            if (energyHistory.length > maxEnergyHistory) {
                energyHistory.shift();
            }
            
            const avgEnergy = energyHistory.reduce((a, b) => a + b, 0) / energyHistory.length;
            const variance = energyHistory.reduce((sum, val) => sum + Math.pow(val - avgEnergy, 2), 0) / energyHistory.length;
            const stdDev = Math.sqrt(variance);
            
            const adaptiveThreshold = this.beatDetectionSensitivity * 255 + (stdDev * 0.5);
            const bassEnergy = bass / 255;
            const beatIntensity = bassEnergy * (totalEnergy / 255);
            
            const now = Date.now();
            if (bass > adaptiveThreshold && now - lastBeatTime > this.beatDebounce) {
                const rawBpm = Math.round(60000 / (now - this.lastBeatTime));
                
                if (rawBpm >= 60 && rawBpm <= 200) {
                    bpmHistory.push(rawBpm);
                    if (bpmHistory.length > maxBpmHistory) {
                        bpmHistory.shift();
                    }
                    
                    const sortedBpm = [...bpmHistory].sort((a, b) => a - b);
                    const medianBpm = sortedBpm[Math.floor(sortedBpm.length / 2)];
                    
                    const alpha = 0.3;
                    const smoothedBpm = Math.round(alpha * medianBpm + (1 - alpha) * (this.bpmValue || medianBpm));
                    
                    this.bpmValue = smoothedBpm;
                    this.bpmSmoothed = smoothedBpm;
                    this.lastDetectedBpm = smoothedBpm;
                }
                
                lastBeatTime = now;
                this.lastBeatTime = now;
                this.lastBeatEnergy = beatIntensity;
                this.consecutiveBeats = (this.consecutiveBeats || 0) + 1;
                
                if (this.mediaSyncMode === 'beat') {
                    this.triggerBeatEffectWithIntensity(beatIntensity);
                }
                
                this.updateBpmDisplay();
                this.updateFrequencyVisualization();
            } else if (now - lastBeatTime > 2000) {
                this.consecutiveBeats = 0;
            }
            
            requestAnimationFrame(detectBeat);
        };
        
        detectBeat();
    },
    
    triggerBeatEffectWithIntensity: function(intensity) {
        const targets = this.multiBulbSelection.length > 0 
            ? this.multiBulbSelection 
            : (this.selectedBulb ? [this.selectedBulb] : ['all']);
        
        if (targets.length === 0) return;
        
        const baseBrightness = 70;
        const brightness = baseBrightness + (intensity * 30);
        const duration = 0.05 + (1 - intensity) * 0.05;
        
        switch(this.mediaSyncMode) {
            case 'beat':
                this.applyBeatSyncEffectWithIntensity(targets, brightness, duration, intensity);
                break;
            case 'color':
                this.applyColorSyncEffectWithIntensity(targets, intensity);
                break;
            case 'ambient':
                this.applyAmbientSyncEffect(targets);
                break;
            case 'bass':
                this.applyBassBoostEffect(targets, intensity);
                break;
            case 'spectrum':
                this.applySpectrumSyncEffectWithIntensity(targets, intensity);
                break;
        }
        
        this.showBeatVisualization(intensity);
        this.showBeatFlashEffect(intensity);
        
        if (this.beatFlashEnabled) {
            this.showBeatFlashOverlay(intensity);
        }
        
        if (navigator.vibrate && intensity > 0.8) {
            navigator.vibrate(Math.round(intensity * 50));
        }
    },
    
    applyBeatSyncEffectWithIntensity: function(targets, brightness, duration, intensity) {
        const hueShift = intensity * 30;
        const currentHue = (this.lastColorHue || 0) + hueShift;
        this.lastColorHue = currentHue;
        
        if (typeof sendLifxCommand !== 'undefined') {
            sendLifxCommand('set_state', {
                selector: `id:${targets.join(',')}`,
                brightness: brightness / 100,
                hue: currentHue,
                duration: duration
            });
        }
    },
    
    applyColorSyncEffectWithIntensity: function(targets, intensity) {
        const bassBand = this.frequencyBands.bass.value;
        const trebleBand = this.frequencyBands.treble.value;
        
        const hue = (bassBand / 255) * 360;
        const saturation = 50 + (trebleBand / 255) * 50;
        const brightness = 50 + intensity * 50;
        
        if (typeof sendLifxCommand !== 'undefined') {
            sendLifxCommand('set_color', {
                selector: `id:${targets.join(',')}`,
                color: `hue:${Math.round(hue)},saturation:${Math.round(saturation)}%`,
                brightness: brightness,
                duration: 0.1
            });
        }
    },
    
    applySpectrumSyncEffectWithIntensity: function(targets, intensity) {
        const bandColors = [
            { hue: 0, sat: 100 },
            { hue: 60, sat: 100 },
            { hue: 120, sat: 100 },
            { hue: 180, sat: 100 },
            { hue: 240, sat: 100 },
            { hue: 300, sat: 100 }
        ];
        
        const dominantBand = Math.max(
            this.frequencyBands.subBass.value,
            this.frequencyBands.bass.value,
            this.frequencyBands.lowMid.value,
            this.frequencyBands.mid.value,
            this.frequencyBands.highMid.value,
            this.frequencyBands.treble.value
        );
        
        const colorIndex = Math.floor((dominantBand / 255) * bandColors.length);
        const color = bandColors[colorIndex] || bandColors[0];
        
        if (typeof sendLifxCommand !== 'undefined') {
            sendLifxCommand('set_color', {
                selector: `id:${targets.join(',')}`,
                color: `hue:${color.hue * 182},saturation:${color.sat}%`,
                brightness: 50 + intensity * 50,
                duration: 0.1
            });
        }
    },
    
    triggerBeatEffect: function() {
        const targets = this.multiBulbSelection.length > 0 
            ? this.multiBulbSelection 
            : (this.selectedBulb ? [this.selectedBulb] : ['all']);
        
        if (targets.length === 0) return;
        
        const intensity = Math.min(1.0, this.beatDetectionSensitivity + 0.2);
        const bassEnergy = this.frequencyBands.bass.value / 255;
        const brightness = 70 + bassEnergy * 30;
        
        switch(this.mediaSyncMode) {
            case 'beat':
                this.applyBeatSyncEffect(targets, brightness);
                break;
            case 'color':
                this.applyColorSyncEffect(targets);
                break;
            case 'ambient':
                this.applyAmbientSyncEffect(targets);
                break;
            case 'bass':
                this.applyBassBoostEffect(targets, bassEnergy);
                break;
            case 'spectrum':
                this.applySpectrumSyncEffect(targets);
                break;
        }
        
        this.showBeatVisualization();
        this.showBeatFlashEffect();
        
        if (this.beatFlashEnabled) {
            this.showBeatFlashOverlay();
        }
    },
    
    applyBeatSyncEffect: function(targets, brightness) {
        $.ajax({
            url: '/api/services/lifx/set_state',
            method: 'POST',
            contentType: 'application/json',
            data: JSON.stringify({
                selector: `id:${targets.join(',')}`,
                brightness: Math.round(brightness),
                duration: 0.05
            })
        });
    },
    
    applyColorSyncEffect: function(targets) {
        const bassBand = this.frequencyBands.bass.value;
        const trebleBand = this.frequencyBands.treble.value;
        
        const hue = (bassBand / 255) * 360;
        const saturation = 50 + (trebleBand / 255) * 50;
        
        $.ajax({
            url: '/api/services/lifx/set_color',
            method: 'POST',
            contentType: 'application/json',
            data: JSON.stringify({
                selector: `id:${targets.join(',')}`,
                color: `hue:${Math.round(hue)},saturation:${Math.round(saturation)}%`,
                duration: 0.1
            })
        });
    },
    
    applyAmbientSyncEffect: function(targets) {
        const lowMid = this.frequencyBands.lowMid.value;
        const highMid = this.frequencyBands.highMid.value;
        
        const brightness = 30 + ((lowMid + highMid) / 255) * 40;
        const kelvin = 2700 + (highMid / 255) * 3800;
        
        $.ajax({
            url: '/api/services/lifx/set_state',
            method: 'POST',
            contentType: 'application/json',
            data: JSON.stringify({
                selector: `id:${targets.join(',')}`,
                brightness: Math.round(brightness / 100),
                color: `kelvin:${Math.round(kelvin)}`,
                duration: 0.2
            })
        });
    },
    
    applyBassBoostEffect: function(targets, bassEnergy) {
        const redIntensity = Math.min(255, bassEnergy * 3);
        
        $.ajax({
            url: '/api/services/lifx/set_color',
            method: 'POST',
            contentType: 'application/json',
            data: JSON.stringify({
                selector: `id:${targets.join(',')}`,
                color: `hue:0,saturation:100%,brightness:${30 + bassEnergy * 70}%`,
                duration: 0.05
            })
        });
    },
    
    applySpectrumSyncEffect: function(targets) {
        const bands = this.frequencyBands;
        const totalEnergy = (bands.subBass.value + bands.bass.value + bands.treble.value) / 3;
        
        const hue = (totalEnergy / 255) * 360;
        const brightness = 40 + (totalEnergy / 255) * 60;
        
        $.ajax({
            url: '/api/services/lifx/set_color',
            method: 'POST',
            contentType: 'application/json',
            data: JSON.stringify({
                selector: `id:${targets.join(',')}`,
                color: `hue:${Math.round(hue)},saturation:80%,brightness:${Math.round(brightness)}%`,
                duration: 0.08
            })
        });
    },
    
    showBeatFlashOverlay: function() {
        let overlay = document.querySelector('.lifx-beat-flash');
        if (!overlay) {
            overlay = document.createElement('div');
            overlay.className = 'lifx-beat-flash';
            document.body.appendChild(overlay);
        }
        
        overlay.classList.remove('active');
        void overlay.offsetWidth;
        overlay.classList.add('active');
        
        setTimeout(() => overlay.classList.remove('active'), 150);
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
            if (valueEl) {
                const newValue = this.bpmValue || '--';
                if (valueEl.textContent !== newValue) {
                    valueEl.style.transform = 'scale(1.3)';
                    valueEl.style.transition = 'transform 0.15s ease-out';
                    setTimeout(() => {
                        valueEl.style.transform = 'scale(1)';
                    }, 150);
                }
                valueEl.textContent = newValue;
            }
        }
    },
    
    showBeatFlashEffect: function() {
        let flash = document.querySelector('.beat-flash-effect');
        if (!flash) {
            flash = document.createElement('div');
            flash.className = 'beat-flash-effect';
            flash.innerHTML = '<i class="fas fa-bolt"></i>';
            flash.style.cssText = `
                position: fixed;
                top: 50%;
                left: 50%;
                transform: translate(-50%, -50%);
                font-size: 48px;
                color: #ff6b6b;
                opacity: 0;
                pointer-events: none;
                z-index: 10000;
                transition: all 0.2s ease-out;
            `;
            document.body.appendChild(flash);
            
            requestAnimationFrame(() => {
                flash.style.opacity = '0.8';
                flash.style.transform = 'translate(-50%, -50%) scale(1.2)';
            });
            
            setTimeout(() => {
                flash.style.opacity = '0';
                flash.style.transform = 'translate(-50%, -50%) scale(0.8)';
                setTimeout(() => {
                    if (flash.parentNode) flash.parentNode.removeChild(flash);
                }, 200);
            }, 300);
        } else {
            flash.style.opacity = '0.8';
            flash.style.transform = 'translate(-50%, -50%) scale(1.2)';
            setTimeout(() => {
                flash.style.opacity = '0';
                flash.style.transform = 'translate(-50%, -50%) scale(0.8)';
            }, 300);
        }
    },
    
    updateBpmDisplay: function() {
        const bpmDisplay = document.querySelector('.bpm-display .bpm-value');
        if (bpmDisplay) {
            const newValue = this.bpmValue || '--';
            if (bpmDisplay.textContent !== newValue) {
                bpmDisplay.style.transform = 'scale(1.2)';
                bpmDisplay.style.color = '#ff6b6b';
                setTimeout(() => {
                    bpmDisplay.style.transform = 'scale(1)';
                    bpmDisplay.style.color = '';
                }, 150);
            }
            bpmDisplay.textContent = newValue;
        }
        
        const bpmDisplays = document.querySelectorAll('#realtime-bpm, #bpm-value-display, .bpm-value');
        bpmDisplays.forEach(el => {
            if (el.textContent !== (this.bpmValue || '--')) {
                el.textContent = this.bpmValue || '--';
            }
        });
    },
    
    applyMediaLighting: function(mediaData) {
        const targets = this.multiBulbSelection.length > 0 
            ? this.multiBulbSelection 
            : (this.selectedBulb ? [this.selectedBulb] : ['all']);
        
        if (targets.length === 0) return;
        
        if (mediaData.type === 'beat') {
            const intensity = Math.min(1.0, 0.7 + (this.frequencyBands.bass.value / 255) * 0.3);
            $.ajax({
                url: '/api/services/lifx/set_state',
                method: 'POST',
                contentType: 'application/json',
                data: JSON.stringify({
                    selector: `id:${targets.join(',')}`,
                    brightness: Math.round(70 + (this.frequencyBands.bass.value / 255) * 30),
                    duration: 0.05
                }),
                success: () => {
                    if (this.beatFlashEnabled) {
                        this.showBeatFlashEffect();
                    }
                }
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
    
    mediaSyncModes: {
        'beat': { name: 'Beat Sync', icon: '🥁', description: 'Flash lights on detected beats' },
        'color': { name: 'Color Sync', icon: '🎨', description: 'Match dominant colors' },
        'ambient': { name: 'Ambient', icon: '🌊', description: 'Smooth color transitions' },
        'bass': { name: 'Bass Boost', icon: '🔊', description: 'Emphasize low frequencies' },
        'spectrum': { name: 'Full Spectrum', icon: '🌈', description: 'Use all frequency bands' },
        'pulse': { name: 'Pulse Mode', icon: '💓', description: 'Rhythmic pulsing effect' }
    },
    
    setMediaSyncMode: function(mode) {
        this.mediaSyncMode = mode;
        localStorage.setItem('lifx_media_sync_mode', mode);
        
        document.querySelectorAll('.sync-mode-btn').forEach(btn => {
            btn.classList.remove('active');
            if (btn.dataset.mode === mode) {
                btn.classList.add('active');
            }
        });
        
        const modeInfo = this.mediaSyncModes[mode] || { name: mode, icon: '🎵' };
        this.showEnhancedGestureFeedback(`Sync Mode: ${modeInfo.name}`, modeInfo.icon);
        this.applyMediaSyncModeEffects(mode);
    },
    
    applyMediaSyncModeEffects: function(mode) {
        switch(mode) {
            case 'beat':
                this.beatDetectionSensitivity = 0.7;
                console.log('Beat Sync mode: Standard sensitivity for beat detection');
                break;
            case 'color':
                this.beatDetectionSensitivity = 0.65;
                console.log('Color Sync mode: Moderate sensitivity for color transitions');
                break;
            case 'ambient':
                this.beatDetectionSensitivity = 0.5;
                console.log('Ambient mode: Low sensitivity for subtle effects');
                break;
            case 'bass':
                this.beatDetectionSensitivity = 0.5;
                console.log('Bass Boost mode: Lowered sensitivity for bass emphasis');
                break;
            case 'spectrum':
                this.beatDetectionSensitivity = 0.6;
                console.log('Full Spectrum mode: Balanced sensitivity');
                break;
        }
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
                case 'aurora_borealis':
                    const auroraTime = Date.now() / 1500;
                    hue = (120 + Math.sin(auroraTime) * 60 + Math.sin(auroraTime * 2.3) * 40) % 360;
                    saturation = 60 + Math.sin(auroraTime * 0.7) * 30;
                    brightness = 50 + Math.sin(auroraTime * 1.2) * 25;
                    break;
                case 'candle_flicker':
                    const flickerRand = Math.random();
                    brightness = 30 + flickerRand * 25 + Math.sin(Date.now() / 200) * 10;
                    saturation = 30 + Math.random() * 20;
                    hue = 30 + Math.random() * 15;
                    break;
                case 'ocean_waves':
                    const waveTime = Date.now() / 2500;
                    hue = (180 + Math.sin(waveTime) * 40 + Math.sin(waveTime * 1.5) * 20) % 360;
                    saturation = 50 + Math.sin(waveTime * 0.8) * 30;
                    brightness = 40 + (Math.sin(waveTime) + 1) * 20;
                    break;
                case 'sunset_glow':
                    const sunsetTime = Date.now() / 3000;
                    hue = (20 + Math.sin(sunsetTime * 0.5) * 15) % 360;
                    saturation = 70 + Math.sin(sunsetTime * 0.3) * 20;
                    brightness = 50 + Math.sin(sunsetTime * 0.4) * 20;
                    break;
                case 'neon_sign':
                    const neonCycle = (Date.now() / 100) % 360;
                    hue = neonCycle;
                    saturation = 100;
                    brightness = 80 + Math.sin(neonCycle * 0.1) * 15;
                    break;
                case 'strobe_safe':
                    const strobeTime = Date.now();
                    if (strobeTime % 400 < 200) {
                        brightness = p.brightness;
                        saturation = p.saturation;
                    } else {
                        brightness = p.brightness * 0.3;
                        saturation = p.saturation * 0.5;
                    }
                    break;
                case 'breathing':
                    const breathTime = Date.now() / 4000;
                    brightness = p.brightness * (0.4 + Math.sin(breathTime) * 0.3 + 0.3);
                    saturation = p.saturation * (0.6 + Math.sin(breathTime * 0.5) * 0.2 + 0.2);
                    break;
                case 'twinkle':
                    const twinkleRand = Math.random();
                    if (twinkleRand < 0.05) {
                        brightness = 100;
                        saturation = 80;
                    } else if (twinkleRand < 0.15) {
                        brightness = p.brightness * 0.5;
                        saturation = p.saturation * 0.7;
                    } else {
                        brightness = p.brightness;
                        saturation = p.saturation;
                    }
                    hue = p.hue + Math.random() * 20 - 10;
                    break;
                case 'mystic':
                    const mysticTime = Date.now() / 3000;
                    hue = (p.hue + Math.sin(mysticTime) * 50) % 360;
                    brightness = p.brightness * (0.5 + Math.sin(mysticTime * 2) * 0.25);
                    saturation = p.saturation * (0.7 + Math.sin(mysticTime * 1.5) * 0.3);
                    break;
                case 'dragon':
                    const dragonTime = Date.now();
                    const dragonFlicker = Math.random();
                    if (dragonFlicker < 0.03) {
                        brightness = 100;
                        saturation = 100;
                        hue = 15 + Math.random() * 10;
                    } else {
                        brightness = 60 + Math.random() * 30;
                        saturation = 80 + Math.random() * 20;
                        hue = 10 + Math.random() * 20;
                    }
                    break;
                case 'twinkle':
                    const twinkleTime = Date.now() / 200;
                    const twinkleIndex = Math.floor(twinkleTime) % 10;
                    if (twinkleIndex < 2) {
                        brightness = p.brightness * 1.3;
                    } else {
                        brightness = p.brightness;
                    }
                    saturation = p.saturation * (0.8 + Math.random() * 0.2);
                    break;
                case 'haunted':
                    const hauntedTime = Date.now() / 800;
                    if (Math.random() < 0.1) {
                        brightness = p.brightness * 0.3;
                        hue = 300 + Math.random() * 30;
                    } else {
                        brightness = p.brightness * (0.6 + Math.sin(hauntedTime) * 0.2);
                        hue = p.hue + Math.sin(hauntedTime * 0.5) * 20;
                    }
                    saturation = p.saturation * (0.8 + Math.random() * 0.2);
                    break;
                case 'festive':
                    const festiveColors = [0, 120, 240];
                    const festiveIndex = Math.floor(Date.now() / 500) % 3;
                    hue = festiveColors[festiveIndex];
                    brightness = p.brightness * (0.8 + Math.random() * 0.2);
                    saturation = p.saturation;
                    break;
                case 'celebration':
                    const celebrationTime = Date.now();
                    if (celebrationTime % 600 < 200) {
                        brightness = 100;
                        saturation = 100;
                        hue = Math.random() * 360;
                    } else {
                        brightness = p.brightness;
                        saturation = p.saturation;
                        hue = p.hue;
                    }
                    break;
                case 'spooky':
                    const spookyTime = Date.now() / 1200;
                    brightness = p.brightness * (0.4 + Math.sin(spookyTime) * 0.3);
                    hue = 30 + Math.sin(spookyTime * 0.7) * 15;
                    saturation = 90 + Math.sin(spookyTime * 0.5) * 10;
                    break;
                case 'warm':
                    brightness = p.brightness * (0.7 + Math.sin(Date.now() / 4000) * 0.15);
                    hue = 25 + Math.sin(Date.now() / 3000) * 10;
                    saturation = p.saturation * (0.9 + Math.sin(Date.now() / 5000) * 0.1);
                    break;
                case 'firework':
                    const fireworkTime = Date.now();
                    if (fireworkTime % 1000 < 100) {
                        brightness = 100;
                        saturation = 100;
                        hue = Math.random() * 360;
                    } else {
                        brightness = p.brightness * 0.5;
                        saturation = p.saturation * 0.7;
                    }
                    break;
                case 'irish':
                    const irishTime = Date.now() / 2000;
                    hue = 120 + Math.sin(irishTime) * 30;
                    brightness = p.brightness * (0.8 + Math.sin(irishTime * 2) * 0.2);
                    saturation = p.saturation;
                    break;
                case 'gradient_cycle':
                    const gradientTime = Date.now() / 1500;
                    hue = (p.hue + Math.sin(gradientTime) * 180) % 360;
                    brightness = p.brightness * (0.6 + Math.sin(gradientTime * 0.5) * 0.2);
                    saturation = p.saturation * (0.8 + Math.sin(gradientTime * 0.3) * 0.2);
                    break;
                case 'random_flash':
                    if (Math.random() < 0.08) {
                        brightness = 100;
                        saturation = 100;
                        hue = Math.random() * 360;
                    } else {
                        brightness = p.brightness;
                        saturation = p.saturation;
                        hue = p.hue;
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
                    duration: p.effect === 'sparkle' || p.effect === 'energy' || p.effect === 'twinkle' || p.effect === 'random_flash' ? 0.1 : 0.5
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
            
            const touchpadStored = localStorage.getItem('lifx_touchpad_mode');
            if (touchpadStored === 'true') {
                this.touchpadModeEnabled = true;
            }
            
            const microGestureStored = localStorage.getItem('lifx_micro_gesture');
            if (microGestureStored !== null) {
                this.microGestureEnabled = microGestureStored === 'true';
            }
            
            const favoritesStored = localStorage.getItem('lifx_favorite_scenes');
            if (favoritesStored) {
                this.favoriteScenes = JSON.parse(favoritesStored);
            }
            
            const macrosStored = localStorage.getItem('lifx_gesture_macros');
            if (macrosStored) {
                this.gestureMacros = JSON.parse(macrosStored);
            }
        } catch (e) {
            console.warn('Failed to load LIFX preferences:', e);
        }
    },
    
    initTouchpadMode: function() {
        if (!this.touchpadModeEnabled) return;
        
        document.querySelectorAll('.lifx-bulb-control').forEach(bulbEl => {
            bulbEl.addEventListener('touchmove', (e) => {
                if (!this.touchpadModeEnabled) return;
                e.preventDefault();
                
                const touch = e.touches[0];
                const rect = bulbEl.getBoundingClientRect();
                const x = ((touch.clientX - rect.left) / rect.width) * 100;
                const y = ((touch.clientY - rect.top) / rect.height) * 100;
                
                this.touchpadX = Math.max(0, Math.min(100, x));
                this.touchpadY = Math.max(0, Math.min(100, y));
                
                const hue = (this.touchpadX / 100) * 360;
                const saturation = 100 - (this.touchpadY / 100) * 50;
                
                this.showTouchpadFeedback(hue, saturation);
            }, { passive: false });
            
            bulbEl.addEventListener('touchend', (e) => {
                if (!this.touchpadModeEnabled) return;
                
                const targets = this.multiBulbSelection.length > 0 
                    ? this.multiBulbSelection 
                    : (this.selectedBulb ? [this.selectedBulb] : []);
                
                if (targets.length === 0) return;
                
                const hue = (this.touchpadX / 100) * 360;
                const saturation = 100 - (this.touchpadY / 100) * 50;
                
                $.ajax({
                    url: '/api/services/lifx/set_color',
                    method: 'POST',
                    contentType: 'application/json',
                    data: JSON.stringify({
                        selector: `id:${targets.join(',')}`,
                        color: `hue:${Math.round(hue * 182)} saturation:${Math.round(saturation)}%`,
                        duration: 0.3
                    }),
                    success: () => {
                        this.showGestureFeedback('Color set via touchpad', '🎨');
                        this.hapticFeedback('success');
                    }
                });
            });
        });
        
        console.log('Touchpad mode initialized');
    },
    
    showTouchpadFeedback: function(hue, saturation) {
        let feedback = document.querySelector('.touchpad-feedback');
        if (!feedback) {
            feedback = document.createElement('div');
            feedback.className = 'touchpad-feedback';
            document.body.appendChild(feedback);
        }
        
        feedback.style.cssText = `
            position: fixed;
            pointer-events: none;
            background: hsl(${hue}, ${saturation}%, 50%);
            border: 2px solid white;
            border-radius: 50%;
            width: 60px;
            height: 60px;
            z-index: 10000;
            transition: opacity 0.3s;
        `;
        
        const touch = event.touches?.[0];
        if (touch) {
            feedback.style.left = (touch.clientX - 30) + 'px';
            feedback.style.top = (touch.clientY - 30) + 'px';
            feedback.style.opacity = '1';
        }
        
        setTimeout(() => {
            feedback.style.opacity = '0';
        }, 500);
    },
    
    initGestureMacros: function() {
        const macroArea = document.getElementById('lifx-macro-area');
        if (!macroArea) return;
        
        macroArea.addEventListener('gesture', (e) => {
            const gestureName = e.detail.gesture;
            if (this.gestureMacros[gestureName]) {
                this.executeMacro(this.gestureMacros[gestureName]);
            }
        });
    },
    
    saveGestureMacro: function(name, actions) {
        this.gestureMacros[name] = actions;
        localStorage.setItem('lifx_gesture_macros', JSON.stringify(this.gestureMacros));
        this.showGestureFeedback(`Macro "${name}" saved`, '💾');
    },
    
    executeMacro: function(actions) {
        if (!Array.isArray(actions)) return;
        
        let delay = 0;
        actions.forEach(action => {
            setTimeout(() => {
                if (action.type === 'scene') {
                    this.applyScene(action.value);
                } else if (action.type === 'brightness') {
                    this.adjustBrightness(action.value - this.brightnessLevel);
                } else if (action.type === 'colortemp') {
                    this.adjustColorTemp(action.value);
                } else if (action.type === 'color') {
                    this.applyColorFromHex(action.value, this.selectedBulb || 'all');
                }
            }, delay);
            delay += action.delay || 500;
        });
        
        this.showGestureFeedback('Executing macro', '✨');
        this.hapticFeedback('success');
    },
    
    addToFavoriteScenes: function(sceneName) {
        if (!this.favoriteScenes.includes(sceneName)) {
            this.favoriteScenes.push(sceneName);
            localStorage.setItem('lifx_favorite_scenes', JSON.stringify(this.favoriteScenes));
            this.showGestureFeedback('Added to favorites', '⭐');
        }
    },
    
    removeFromFavoriteScenes: function(sceneName) {
        const index = this.favoriteScenes.indexOf(sceneName);
        if (index > -1) {
            this.favoriteScenes.splice(index, 1);
            localStorage.setItem('lifx_favorite_scenes', JSON.stringify(this.favoriteScenes));
            this.showGestureFeedback('Removed from favorites', '⭐');
        }
    },
    
    addToRecentScenes: function(sceneName) {
        if (!this.recentScenes.includes(sceneName)) {
            this.recentScenes.unshift(sceneName);
            if (this.recentScenes.length > this.maxRecentScenes) {
                this.recentScenes.pop();
            }
        }
    },
    
    showFavoriteScenesPanel: function() {
        if (typeof Swal === 'undefined') {
            alert('Favorite scenes: ' + this.favoriteScenes.join(', '));
            return;
        }
        
        Swal.fire({
            title: '<i class="fas fa-star"></i> Favorite Scenes',
            html: `
                <div style="display: grid; grid-template-columns: repeat(2, 1fr); gap: 10px; padding: 15px;">
                    ${this.favoriteScenes.map(scene => `
                        <button class="btn btn-outline-primary" onclick="LifXTouchControls.applyScene('${scene}'); Swal.close();">
                            <i class="fas fa-palette"></i> ${scene.replace('_', ' ')}
                        </button>
                    `).join('')}
                </div>
                ${this.favoriteScenes.length === 0 ? '<p style="color: #adb5bd;">No favorite scenes yet. Long-press a scene to add it!</p>' : ''}
            `,
            showConfirmButton: false,
            showCloseButton: true
        });
    },
    
    initRadialMenu: function() {
        const radialMenu = document.createElement('div');
        radialMenu.className = 'lifx-radial-menu';
        radialMenu.innerHTML = `
            <div class="radial-menu-item" data-action="brightness" style="transform: rotate(0deg) translate(60px) rotate(0deg);">
                <i class="fas fa-sun"></i>
            </div>
            <div class="radial-menu-item" data-action="colortemp" style="transform: rotate(90deg) translate(60px) rotate(-90deg);">
                <i class="fas fa-thermometer-half"></i>
            </div>
            <div class="radial-menu-item" data-action="scene" style="transform: rotate(180deg) translate(60px) rotate(-180deg);">
                <i class="fas fa-palette"></i>
            </div>
            <div class="radial-menu-item" data-action="power" style="transform: rotate(270deg) translate(60px) rotate(-270deg);">
                <i class="fas fa-power-off"></i>
            </div>
        `;
        radialMenu.style.cssText = `
            position: fixed;
            width: 150px;
            height: 150px;
            border-radius: 50%;
            background: rgba(30, 30, 45, 0.95);
            border: 2px solid rgba(0, 212, 255, 0.3);
            z-index: 9999;
            display: none;
            pointer-events: none;
        `;
        document.body.appendChild(radialMenu);
    },
    
    showRadialMenu: function(x, y) {
        const radialMenu = document.querySelector('.lifx-radial-menu');
        if (!radialMenu) return;
        
        radialMenu.style.display = 'block';
        radialMenu.style.left = (x - 75) + 'px';
        radialMenu.style.top = (y - 75) + 'px';
        radialMenu.style.pointerEvents = 'auto';
        
        this.radialMenuActive = true;
        this.showGestureFeedback('Radial menu active - swipe to select', '🎯');
    },
    
    hideRadialMenu: function() {
        const radialMenu = document.querySelector('.lifx-radial-menu');
        if (radialMenu) {
            radialMenu.style.display = 'none';
            radialMenu.style.pointerEvents = 'none';
        }
        this.radialMenuActive = false;
    },
    
    initMicroGestures: function() {
        if (!this.microGestureEnabled) return;
        
        let startX = 0;
        let startY = 0;
        
        document.addEventListener('touchstart', (e) => {
            startX = e.touches[0].clientX;
            startY = e.touches[0].clientY;
        }, { passive: true });
        
        document.addEventListener('touchend', (e) => {
            if (!this.microGestureEnabled) return;
            
            const endX = e.changedTouches[0].clientX;
            const endY = e.changedTouches[0].clientY;
            
            const deltaX = endX - startX;
            const deltaY = endY - startY;
            
            if (Math.abs(deltaX) < this.microGestureThreshold && Math.abs(deltaY) < this.microGestureThreshold) {
                return;
            }
            
            if (Math.abs(deltaX) > Math.abs(deltaY)) {
                if (deltaX > 0 && deltaX > this.microGestureThreshold) {
                    this.adjustColorTemp(this.quickColorTempStep);
                    this.showGestureFeedback('Warmer (micro)', '☀️');
                } else if (deltaX < 0 && Math.abs(deltaX) > this.microGestureThreshold) {
                    this.adjustColorTemp(-this.quickColorTempStep);
                    this.showGestureFeedback('Cooler (micro)', '❄️');
                }
            } else {
                if (deltaY > 0 && deltaY > this.microGestureThreshold) {
                    this.adjustBrightness(this.quickBrightnessStep);
                    this.showGestureFeedback('Brighter (micro)', '↑');
                } else if (deltaY < 0 && Math.abs(deltaY) > this.microGestureThreshold) {
                    this.adjustBrightness(-this.quickBrightnessStep);
                    this.showGestureFeedback('Dimmer (micro)', '↓');
                }
            }
        });
        
        console.log('Micro-gestures initialized');
    },
    
    initThreeFingerGestures: function() {
        let touchCount = 0;
        let threeFingerStartY = 0;
        
        document.addEventListener('touchstart', (e) => {
            touchCount = e.touches.length;
            if (touchCount === 3) {
                threeFingerStartY = e.touches[0].clientY;
            }
        }, { passive: true });
        
        document.addEventListener('touchend', (e) => {
            if (touchCount === 3) {
                const endY = e.changedTouches[0].clientY;
                const deltaY = endY - threeFingerStartY;
                
                if (deltaY > 50) {
                    this.showFavoriteScenesPanel();
                    this.showGestureFeedback('Favorites panel', '⭐');
                } else if (deltaY < -50) {
                    this.showTouchSensitivityPanel();
                    this.showGestureFeedback('Touch settings', '⚙️');
                }
            }
            touchCount = 0;
        });
        
        console.log('Three-finger gestures initialized');
    },
    
    initQuickSceneSwipes: function() {
        let quickSceneStartX = 0;
        let quickSceneActive = false;
        
        document.addEventListener('touchstart', (e) => {
            if (e.touches[0].clientY > window.innerHeight - 100) {
                quickSceneActive = true;
                quickSceneStartX = e.touches[0].clientX;
            }
        }, { passive: true });
        
        document.addEventListener('touchend', (e) => {
            if (!quickSceneActive) return;
            
            const deltaX = e.changedTouches[0].clientX - quickSceneStartX;
            
            if (Math.abs(deltaX) > 100) {
                if (deltaX > 0) {
                    this.previousScene();
                } else {
                    this.nextScene();
                }
                this.showGestureFeedback('Scene changed', '🎨');
            }
            
            quickSceneActive = false;
        });
        
        console.log('Quick scene swipes initialized');
    },
    
    initAdvancedGestureModes: function() {
        let touchCount = 0;
        let multiTouchStartTime = 0;
        let multiTouchStartPositions = [];
        
        document.addEventListener('touchstart', (e) => {
            touchCount = e.touches.length;
            multiTouchStartTime = Date.now();
            multiTouchStartPositions = Array.from(e.touches).map(t => ({ x: t.clientX, y: t.clientY }));
            
            if (touchCount === 3) {
                this.threeFingerSwipeActive = true;
            } else if (touchCount === 4) {
                this.fourFingerSwipeActive = true;
            }
        }, { passive: true });
        
        document.addEventListener('touchmove', (e) => {
            if (!this.threeFingerSwipeActive && !this.fourFingerSwipeActive) return;
            
            const touches = Array.from(e.touches);
            if (touches.length >= 3) {
                const currentPositions = touches.map(t => ({ x: t.clientX, y: t.clientY }));
                
                if (this.threeFingerSwipeActive && multiTouchStartPositions.length >= 3) {
                    const avgStartX = multiTouchStartPositions.reduce((s, p) => s + p.x, 0) / 3;
                    const avgStartY = multiTouchStartPositions.reduce((s, p) => s + p.y, 0) / 3;
                    const avgCurrentX = currentPositions.reduce((s, p) => s + p.x, 0) / 3;
                    const avgCurrentY = currentPositions.reduce((s, p) => s + p.y, 0) / 3;
                    
                    const deltaX = avgCurrentX - avgStartX;
                    const deltaY = avgCurrentY - avgStartY;
                    
                    if (Math.abs(deltaX) > 80 || Math.abs(deltaY) > 80) {
                        if (deltaY < -80) {
                            this.activatePartyMode();
                            this.threeFingerSwipeActive = false;
                        } else if (deltaY > 80) {
                            this.activateCalmMode();
                            this.threeFingerSwipeActive = false;
                        } else if (deltaX < -80) {
                            this.applyScene('focus');
                            this.threeFingerSwipeActive = false;
                        } else if (deltaX > 80) {
                            this.applyScene('relax');
                            this.threeFingerSwipeActive = false;
                        }
                    }
                }
            }
        }, { passive: true });
        
        document.addEventListener('touchend', (e) => {
            touchCount = e.touches.length;
            if (touchCount < 3) {
                this.threeFingerSwipeActive = false;
            }
            if (touchCount < 4) {
                this.fourFingerSwipeActive = false;
            }
        });
        
        console.log('Advanced gesture modes initialized');
    },
    
    initMediaCenterIntegration: function() {
        this.mediaPlaybackActive = false;
        this.mediaSyncMode = 'beat';
        this.beatDetectionSensitivity = 0.7;
        this.bpmValue = 0;
        
        const mediaContainer = document.getElementById('media-center-integration');
        if (mediaContainer) {
            mediaContainer.innerHTML = `
                <div class="media-sync-status" id="media-sync-status">
                    <div class="status-indicator ${this.lifxMediaSyncEnabled ? 'active' : ''}"></div>
                    <span class="status-text">${this.lifxMediaSyncEnabled ? 'Media Sync Active' : 'Media Sync Off'}</span>
                </div>
                <div class="quick-media-controls" id="quick-media-controls">
                    <button class="media-btn" onclick="LifXTouchControls.activatePartyMode()">
                        <i class="fas fa-party-horn"></i> Party
                    </button>
                    <button class="media-btn" onclick="LifXTouchControls.activateCalmMode()">
                        <i class="fas fa-spa"></i> Calm
                    </button>
                    <button class="media-btn" onclick="LifXTouchControls.toggleColorCycle()">
                        <i class="fas fa-sync"></i> Cycle
                    </button>
                    <button class="media-btn" onclick="LifXTouchControls.toggleLightPainting()">
                        <i class="fas fa-palette"></i> Paint
                    </button>
                </div>
            `;
        }
        
        console.log('Media center integration initialized');
    },
    
    showMediaSyncPanel: function() {
        const panel = document.getElementById('media-sync-panel');
        if (panel) {
            panel.classList.add('visible');
            this.updateMediaSyncPanelUI();
            this.syncPanelVisible = true;
        }
    },
    
    hideMediaSyncPanel: function() {
        const panel = document.getElementById('media-sync-panel');
        if (panel) {
            panel.classList.remove('visible');
            this.syncPanelVisible = false;
        }
        
        // Mobile-specific cleanup
        if (window.innerWidth <= 768) {
            this.closeMediaSyncPanelMobile();
        }
    },
    
    closeMediaSyncPanelMobile: function() {
        const panel = document.getElementById('media-sync-panel');
        if (panel) {
            panel.classList.remove('visible');
            this.syncPanelVisible = false;
        }
        
        // Reset any active media sync states
        this.mediaSyncActive = false;
        
        // Hide mobile controls if visible
        const mobileControls = document.getElementById('media-center-mobile-controls');
        if (mobileControls) {
            mobileControls.classList.remove('show');
        }
        
        // Clear any active BPM indicators
        const bpmIndicator = document.getElementById('bpm-realtime-indicator');
        if (bpmIndicator) {
            bpmIndicator.classList.remove('visible');
        }
        
        console.log('Media sync panel closed (mobile)');
    },
    
    updateMediaSyncPanelUI: function() {
        document.querySelectorAll('.sync-mode-btn').forEach(btn => {
            btn.classList.toggle('active', btn.dataset.mode === this.mediaSyncMode);
        });
        
        const sensitivitySlider = document.getElementById('beat-sensitivity-slider');
        const sensitivityValue = document.getElementById('beat-sensitivity-value');
        if (sensitivitySlider && sensitivityValue) {
            sensitivitySlider.value = Math.round(this.beatDetectionSensitivity * 100);
            sensitivityValue.textContent = Math.round(this.beatDetectionSensitivity * 100) + '%';
        }
        
        this.updateBpmDisplay();
    },
    
    setBeatSensitivity: function(value) {
        this.beatDetectionSensitivity = Math.max(0.3, Math.min(1.0, value / 100));
        localStorage.setItem('lifx_beat_sensitivity', this.beatDetectionSensitivity);
        
        const sensitivityValue = document.getElementById('beat-sensitivity-value');
        if (sensitivityValue) {
            sensitivityValue.textContent = value + '%';
        }
        
        this.showGestureFeedback(`Sensitivity: ${value}%`, '📊');
    },
    
    applySyncPreset: function(presetName) {
        const presets = {
            movie: { mode: 'ambient', sensitivity: 50 },
            party: { mode: 'beat', sensitivity: 80 },
            relax: { mode: 'ambient', sensitivity: 40 },
            gaming: { mode: 'beat', sensitivity: 75 },
            custom: { mode: this.mediaSyncMode, sensitivity: Math.round(this.beatDetectionSensitivity * 100) }
        };
        
        const preset = presets[presetName];
        if (preset) {
            this.setMediaSyncMode(preset.mode);
            this.setBeatSensitivity(preset.sensitivity);
            this.showGestureFeedback(`Preset: ${presetName}`, '✨');
        }
    },
    
    toggleMediaSync: function() {
        this.lifxMediaSyncEnabled = !this.lifxMediaSyncEnabled;
        localStorage.setItem('lifx_media_sync_enabled', this.lifxMediaSyncEnabled);
        
        const icon = document.getElementById('media-sync-icon');
        if (icon) {
            icon.classList.toggle('fa-lightbulb');
            icon.classList.toggle('fa-lightbulb-slash');
        }
        
        this.showGestureFeedback(
            this.lifxMediaSyncEnabled ? 'Media Sync ON' : 'Media Sync OFF',
            this.lifxMediaSyncEnabled ? '💡' : '🌑'
        );
    },
    
    updateBpmDisplay: function() {
        const bpmDisplays = document.querySelectorAll('#realtime-bpm, #bpm-value-display, .bpm-value');
        bpmDisplays.forEach(el => {
            el.textContent = this.bpmValue || '--';
        });
    },
    
    updateFrequencyVisualization: function() {
        if (!this.audioAnalyzer) return;
        
        const bufferLength = this.audioAnalyzer.frequencyBinCount;
        const dataArray = new Uint8Array(bufferLength);
        this.audioAnalyzer.getByteFrequencyData(dataArray);
        
        const bands = this.frequencyBands;
        bands.subBass.value = this.getBandAverage(dataArray, bands.subBass.min, bands.subBass.max);
        bands.bass.value = this.getBandAverage(dataArray, bands.bass.min, bands.bass.max);
        bands.lowMid.value = this.getBandAverage(dataArray, bands.lowMid.min, bands.lowMid.max);
        bands.mid.value = this.getBandAverage(dataArray, bands.mid.min, bands.mid.max);
        bands.highMid.value = this.getBandAverage(dataArray, bands.highMid.min, bands.highMid.max);
        bands.treble.value = this.getBandAverage(dataArray, bands.treble.min, bands.treble.max);
        
        const bandElements = ['subBass', 'bass', 'lowMid', 'mid', 'highMid', 'treble'];
        const colors = ['#ff0080', '#ff6b6b', '#f39c12', '#00d4ff', '#00ff88', '#9b59b6'];
        
        bandElements.forEach((band, index) => {
            const el = document.getElementById(`band-${band}`);
            if (el) {
                const height = Math.min(100, (bands[band].value / 255) * 100);
                el.style.height = `${height}%`;
                el.style.background = `linear-gradient(to top, ${colors[index]} 0%, ${colors[index]}88 100%)`;
                
                if (height > 85) {
                    el.classList.add('peak');
                } else {
                    el.classList.remove('peak');
                }
            }
        });
    },
    
    activatePartyMode: function() {
        const partyColors = ['#ff0080', '#00ff88', '#00d4ff', '#ff6b6b', '#ffe66d'];
        const bulbs = document.querySelectorAll('.lifx-bulb-control');
        if (bulbs.length === 0) return;
        
        bulbs.forEach((bulb, index) => {
            const color = partyColors[index % partyColors.length];
            const rgb = this.hexToRgb(color);
            if (rgb) {
                const hsl = this.rgbToHsl(rgb[0], rgb[1], rgb[2]);
                if (typeof sendLifxCommand !== 'undefined') {
                    sendLifxCommand('set_color', {
                        bulb_id: bulb.dataset.bulbId,
                        hue: hsl[0] * 360,
                        saturation: 100,
                        brightness: 100,
                        duration: 0.3
                    });
                }
            }
        });
        
        this.colorCycleActive = true;
        this.startColorCycle(partyColors);
        this.showEnhancedGestureFeedback('Party Mode! 🎉', '🎊');
    },
    
    activateCalmMode: function() {
        const calmColors = ['#4ecdc4', '#95a5a6', '#87CEEB', '#DDA0DD'];
        const bulbs = document.querySelectorAll('.lifx-bulb-control');
        if (bulbs.length === 0) return;
        
        bulbs.forEach((bulb, index) => {
            const color = calmColors[index % calmColors.length];
            const rgb = this.hexToRgb(color);
            if (rgb) {
                const hsl = this.rgbToHsl(rgb[0], rgb[1], rgb[2]);
                if (typeof sendLifxCommand !== 'undefined') {
                    sendLifxCommand('set_color', {
                        bulb_id: bulb.dataset.bulbId,
                        hue: hsl[0] * 360,
                        saturation: 30,
                        brightness: 40,
                        duration: 0.5
                    });
                }
            }
        });
        
        this.colorCycleActive = false;
        this.showEnhancedGestureFeedback('Calm Mode 🧘', '🌿');
    },
    
    toggleColorCycle: function() {
        this.colorCycleActive = !this.colorCycleActive;
        if (this.colorCycleActive) {
            this.startColorCycle();
        } else {
            this.stopColorCycle();
        }
    },
    
    startColorCycle: function(colors = null) {
        const cycleColors = colors || ['#ff0000', '#ff8800', '#ffff00', '#00ff00', '#0088ff', '#0000ff', '#ff00ff'];
        let colorIndex = 0;
        
        this.colorCycleInterval = setInterval(() => {
            if (!this.colorCycleActive) {
                this.stopColorCycle();
                return;
            }
            
            const color = cycleColors[colorIndex % cycleColors.length];
            const rgb = this.hexToRgb(color);
            if (rgb) {
                const hsl = this.rgbToHsl(rgb[0], rgb[1], rgb[2]);
                if (typeof sendLifxCommand !== 'undefined') {
                    sendLifxCommand('set_color_all', {
                        hue: hsl[0] * 360,
                        saturation: 100,
                        brightness: 80,
                        duration: 1.0
                    });
                }
            }
            colorIndex++;
        }, 2000);
    },
    
    stopColorCycle: function() {
        if (this.colorCycleInterval) {
            clearInterval(this.colorCycleInterval);
            this.colorCycleInterval = null;
        }
        this.colorCycleActive = false;
    },
    
    randomScene: function() {
        const availableScenes = this.scenes.filter(s => 
            !['goodnight', 'rainbow', 'fireplace'].includes(s)
        );
        const randomScene = availableScenes[Math.floor(Math.random() * availableScenes.length)];
        this.applyScene(randomScene);
    },
    
    toggleLightPainting: function() {
        this.lightPaintingActive = !this.lightPaintingActive;
        
        if (this.lightPaintingActive) {
            this.startLightPainting();
        } else {
            this.stopLightPainting();
        }
    },
    
    startLightPainting: function() {
        const bulbs = document.querySelectorAll('.lifx-bulb-control');
        this.lightPaintingBulbs = Array.from(bulbs).map(b => b.dataset.bulbId);
        
        this.lightPaintingInterval = setInterval(() => {
            if (!this.lightPaintingActive) {
                this.stopLightPainting();
                return;
            }
            
            const randomBulb = this.lightPaintingBulbs[Math.floor(Math.random() * this.lightPaintingBulbs.length)];
            const randomHue = Math.random() * 360;
            
            if (typeof sendLifxCommand !== 'undefined') {
                sendLifxCommand('set_color', {
                    bulb_id: randomBulb,
                    hue: randomHue,
                    saturation: 100,
                    brightness: 100,
                    duration: 0.1
                });
            }
        }, 100);
        
        this.showEnhancedGestureFeedback('Light Painting ON - draw with light!', '🎨');
    },
    
    stopLightPainting: function() {
        if (this.lightPaintingInterval) {
            clearInterval(this.lightPaintingInterval);
            this.lightPaintingInterval = null;
        }
        this.lightPaintingActive = false;
        this.showEnhancedGestureFeedback('Light Painting OFF', '✨');
    },
    
    applyScene: function(sceneName, duration = 0.5, transitionEffect = 'smooth') {
        this.currentScene = sceneName;
        this.addToRecentScenes(sceneName);
        
        const transitionEffects = {
            'smooth': { duration: duration, power: 'on' },
            'fade': { duration: duration * 1.5, power: 'on' },
            'instant': { duration: 0, power: 'on' },
            'pulse': { duration: duration, power: 'on', effect: 'pulse' },
            'morph': { duration: duration * 2, power: 'on', effect: 'morph' }
        };
        
        const effectSettings = transitionEffects[transitionEffect] || transitionEffects.smooth;
        
        if (typeof sendLifxCommand !== 'undefined') {
            sendLifxCommand('apply_scene', {
                scene: sceneName,
                duration: effectSettings.duration,
                power: effectSettings.power,
                effect: effectSettings.effect || null
            });
        }
        
        if (this.enhancedSceneTransitions) {
            this.applySceneTransitionEffect(sceneName, transitionEffect);
        }
        
        this.showEnhancedGestureFeedback(`Scene: ${sceneName}`, '🎨');
    },
    
    applySceneTransitionEffect: function(sceneName, effect) {
        const sceneData = this.getSceneData(sceneName);
        if (!sceneData) return;
        
        const { hue, saturation, brightness, kelvin } = sceneData;
        
        if (effect === 'pulse') {
            this.scenePulseEffect(hue, saturation, brightness, kelvin);
        } else if (effect === 'morph') {
            this.sceneMorphEffect(hue, saturation, brightness, kelvin);
        } else if (effect === 'fade') {
            this.sceneFadeEffect(hue, saturation, brightness, kelvin);
        }
    },
    
    scenePulseEffect: function(hue, saturation, brightness, kelvin) {
        let pulseCount = 0;
        const maxPulses = 3;
        const baseBrightness = brightness;
        
        const pulseInterval = setInterval(() => {
            pulseCount++;
            const pulseBrightness = pulseCount % 2 === 0 ? baseBrightness * 0.3 : baseBrightness;
            
            if (typeof sendLifxCommand !== 'undefined') {
                sendLifxCommand('set_state', {
                    brightness: pulseBrightness / 100,
                    duration: 0.2
                });
            }
            
            if (pulseCount >= maxPulses * 2) {
                clearInterval(pulseInterval);
                if (typeof sendLifxCommand !== 'undefined') {
                    sendLifxCommand('set_state', {
                        brightness: baseBrightness / 100,
                        duration: 0.3
                    });
                }
            }
        }, 200);
    },
    
    sceneMorphEffect: function(targetHue, targetSaturation, targetBrightness, targetKelvin) {
        const steps = 20;
        const currentHue = this.lastColorHue || 0;
        const currentBrightness = this.brightnessLevel;
        
        let step = 0;
        const morphInterval = setInterval(() => {
            step++;
            const progress = step / steps;
            const easeProgress = this.easeInOutCubic(progress);
            
            const currentH = currentHue + (targetHue - currentHue) * easeProgress;
            const currentB = currentBrightness + (targetBrightness - currentBrightness) * easeProgress;
            
            if (typeof sendLifxCommand !== 'undefined') {
                sendLifxCommand('set_state', {
                    hue: currentH,
                    brightness: currentB / 100,
                    duration: 0.1
                });
            }
            
            if (step >= steps) {
                clearInterval(morphInterval);
            }
        }, 50);
    },
    
    sceneFadeEffect: function(hue, saturation, brightness, kelvin) {
        const steps = 30;
        let step = 0;
        
        const fadeInterval = setInterval(() => {
            step++;
            const progress = step / steps;
            const easeProgress = this.easeOutQuad(progress);
            const currentBrightness = brightness * easeProgress;
            
            if (typeof sendLifxCommand !== 'undefined') {
                sendLifxCommand('set_state', {
                    brightness: currentBrightness / 100,
                    duration: 0.1
                });
            }
            
            if (step >= steps) {
                clearInterval(fadeInterval);
            }
        }, 50);
    },
    
    easeInOutCubic: function(x) {
        return x < 0.5 ? 4 * x * x * x : 1 - Math.pow(-2 * x + 2, 3) / 2;
    },
    
    easeOutQuad: function(x) {
        return 1 - (1 - x) * (1 - x);
    },
    
    getSceneData: function(sceneName) {
        const scenePresets = {
            'relax': { hue: 5800, saturation: 15000, brightness: 26214, kelvin: 2700 },
            'focus': { hue: 19000, saturation: 8000, brightness: 52428, kelvin: 5000 },
            'energize': { hue: 41000, saturation: 20000, brightness: 65535, kelvin: 6500 },
            'night': { hue: 5800, saturation: 10000, brightness: 13107, kelvin: 2000 },
            'party': { hue: 43680, saturation: 65535, brightness: 65535, kelvin: 5500 },
            'movie': { hue: 3640, saturation: 19660, brightness: 22937, kelvin: 2200 },
            'gaming': { hue: 50960, saturation: 52428, brightness: 58982, kelvin: 5500 },
            'romance': { hue: 60000, saturation: 25000, brightness: 32767, kelvin: 3000 },
            'reading': { hue: 19000, saturation: 5000, brightness: 45875, kelvin: 4500 },
            'meditation': { hue: 50960, saturation: 19660, brightness: 22937, kelvin: 2400 }
        };
        
        return scenePresets[sceneName] || scenePresets.relax;
    },
    
    applySceneFromName: function(sceneName) {
        this.applyScene(sceneName);
    },
    
    recordGestureSuccess: function() {
        if (this.adaptiveSensitivityEnabled) {
            this.gestureSuccessCount++;
            this.updateGestureAccuracy();
        }
    },
    
    recordGestureFailure: function() {
        if (this.adaptiveSensitivityEnabled) {
            this.gestureFailCount++;
            this.updateGestureAccuracy();
        }
    },
    
    updateGestureAccuracy: function() {
        const total = this.gestureSuccessCount + this.gestureFailCount;
        if (total < 5) return;
        
        this.gestureAccuracyScore = Math.round((this.gestureSuccessCount / total) * 100);
        
        if (total >= 10 && this.adaptiveSensitivityEnabled) {
            this.autoAdjustSensitivity();
        }
    },
    
    autoAdjustSensitivity: function() {
        const accuracy = this.gestureAccuracyScore;
        const currentLevel = this.touchSensitivity;
        
        if (accuracy < 70 && currentLevel !== 'low') {
            this.touchSensitivity = currentLevel === 'high' ? 'medium' : 'low';
            this.applySensitivitySettings();
            this.showEnhancedGestureFeedback('Reduced sensitivity for better accuracy', '🎯');
            this.saveGestureSensitivity();
        } else if (accuracy > 90 && currentLevel !== 'high') {
            this.touchSensitivity = currentLevel === 'low' ? 'medium' : 'high';
            this.applySensitivitySettings();
            this.showEnhancedGestureFeedback('Increased sensitivity for faster response', '⚡');
            this.saveGestureSensitivity();
        }
        
        this.gestureSuccessCount = 0;
        this.gestureFailCount = 0;
    },
    
    calculateTouchVelocity: function(currentX, currentY, timestamp) {
        const lastX = this.lastTouchCoordinates.x;
        const lastY = this.lastTouchCoordinates.y;
        const lastTime = this.gestureStartTime;
        
        const deltaX = currentX - lastX;
        const deltaY = currentY - lastY;
        const deltaTime = timestamp - lastTime;
        
        if (deltaTime > 0) {
            const distance = Math.sqrt(deltaX * deltaX + deltaY * deltaY);
            const velocity = distance / deltaTime;
            
            this.touchVelocityHistory.push(velocity);
            if (this.touchVelocityHistory.length > this.maxVelocityHistory) {
                this.touchVelocityHistory.shift();
            }
            
            const avgVelocity = this.touchVelocityHistory.reduce((a, b) => a + b, 0) / this.touchVelocityHistory.length;
            this.touchVelocity = avgVelocity;
            this.lastGestureVelocity = avgVelocity;
            
            if (Math.abs(deltaX) > Math.abs(deltaY)) {
                this.touchDirection = deltaX > 0 ? 'right' : 'left';
            } else {
                this.touchDirection = deltaY > 0 ? 'down' : 'up';
            }
        }
        
        this.lastTouchCoordinates = { x: currentX, y: currentY };
    },
    
    getVelocityBasedThreshold: function(baseThreshold) {
        const velocityFactor = Math.max(0.5, Math.min(1.5, 1.0 - (this.touchVelocity - 0.5)));
        return baseThreshold * velocityFactor;
    },
    
    resetGestureAccuracy: function() {
        this.gestureSuccessCount = 0;
        this.gestureFailCount = 0;
        this.gestureAccuracyScore = 100;
        this.touchVelocityHistory = [];
    },
    
    getGestureAccuracyScore: function() {
        return this.gestureAccuracyScore;
    },
    
    isHighVelocityGesture: function() {
        return this.touchVelocity > 1.5;
    },
    
    shouldIgnoreGesture: function() {
        if (!this.adaptiveSensitivityEnabled) return false;
        const accuracy = this.gestureAccuracyScore;
        return accuracy < 50 && this.touchVelocity < 0.3;
    },
    
    initHapticFeedback: function() {
        if (!navigator.vibrate) {
            console.log('Haptic feedback not supported on this device');
            return;
        }
        this.hapticEnabled = true;
        console.log('Haptic feedback enabled');
    },
    
    initVoiceShortcuts: function() {
        if (!('webkitSpeechRecognition' in window) && !('SpeechRecognition' in window)) {
            return;
        }
        const SpeechRecognition = window.SpeechRecognition || window.webkitSpeechRecognition;
        this.voiceRecognition = new SpeechRecognition();
        this.voiceRecognition.continuous = false;
        this.voiceRecognition.interimResults = false;
        this.voiceRecognition.lang = 'en-US';
        this.voiceRecognition.onresult = (event) => {
            const command = event.results[0][0].transcript.toLowerCase();
            this.processVoiceCommand(command);
        };
        console.log('Voice shortcuts enabled');
    },
    
    processVoiceCommand: function(command) {
        const commands = {
            'lights on': () => this.powerAll('on'),
            'lights off': () => this.powerAll('off'),
            'brighter': () => this.adjustBrightness(20),
            'dimmer': () => this.adjustBrightness(-20),
            'warmer': () => this.adjustColorTemp(300),
            'cooler': () => this.adjustColorTemp(-300),
            'party mode': () => this.activatePartyMode(),
            'calm mode': () => this.activateCalmMode(),
            'rainbow': () => this.applyScene('rainbow'),
            'relax': () => this.applyScene('relax'),
            'focus': () => this.applyScene('focus'),
            'movie': () => this.applyScene('movie')
        };
        
        for (const [keyword, action] of Object.entries(commands)) {
            if (command.includes(keyword)) {
                action();
                this.showEnhancedGestureFeedback(`Voice: ${keyword}`, '🎤');
                return;
            }
        }
    },
    
    startVoiceCommand: function() {
        if (this.voiceRecognition) {
            this.voiceRecognition.start();
            this.showEnhancedGestureFeedback('Listening...', '🎤');
        }
    },
    
    initQuickActions: function() {
        this.quickActions = [
            { name: 'All On', icon: 'fa-power-off', action: () => this.powerAll('on'), color: '#00ff88' },
            { name: 'All Off', icon: 'fa-power-off', action: () => this.powerAll('off'), color: '#ff6b6b' },
            { name: 'Party', icon: 'fa-party-horn', action: () => this.activatePartyMode(), color: '#ff0080' },
            { name: 'Calm', icon: 'fa-spa', action: () => this.activateCalmMode(), color: '#4ecdc4' },
            { name: 'Rainbow', icon: 'fa-rainbow', action: () => this.applyScene('rainbow'), color: '#00d4ff' },
            { name: 'Voice', icon: 'fa-microphone', action: () => this.startVoiceCommand(), color: '#ffe66d' }
        ];
        console.log('Quick actions initialized');
    },
    
    showQuickActionsPanel: function() {
        if (typeof Swal === 'undefined') return;
        Swal.fire({
            title: '<i class="fas fa-bolt"></i> Quick Actions',
            html: `
                <div style="display: grid; grid-template-columns: repeat(3, 1fr); gap: 15px; padding: 20px;">
                    ${this.quickActions.map(action => `
                        <button class="quick-action-btn" 
                                onclick="LifXTouchControls.hideQuickActionsPanel(); ${action.action.toString().includes('()') ? action.action.toString() : 'LifXTouchControls.' + action.name.toLowerCase().replace(' ', '') + '()'}"
                                style="background: ${action.color}20; border: 2px solid ${action.color}; border-radius: 15px; padding: 20px; cursor: pointer; transition: all 0.2s;">
                            <i class="fas ${action.icon}" style="font-size: 32px; color: ${action.color};"></i>
                            <div style="color: ${action.color}; margin-top: 10px; font-weight: bold;">${action.name}</div>
                        </button>
                    `).join('')}
                </div>
            `,
            showConfirmButton: false,
            showCloseButton: true,
            width: '500px'
        });
    },
    
    hideQuickActionsPanel: function() {
        if (typeof Swal !== 'undefined') Swal.close();
    },
    
    initGestureShortcuts: function() {
        this.gestureShortcuts = {
            'doubleTap': () => this.togglePower(),
            'longPress': () => this.showQuickActionsPanel(),
            'threeFingerUp': () => this.showFavoriteScenesPanel(),
            'threeFingerDown': () => this.showTouchSensitivityPanel(),
            'edgeSwipeLeft': () => this.showQuickScenesPanel(),
            'edgeSwipeRight': () => this.showMediaControls()
        };
        console.log('Gesture shortcuts initialized');
    },
    
    mediaSyncActive: false,
    circadianModeActive: false,
    mediaSyncInterval: null,
    audioContext: null,
    analyser: null,
    dataArray: null,
    
    toggleMediaSync: function() {
        this.mediaSyncActive = !this.mediaSyncActive;
        if (this.mediaSyncActive) {
            this.startMediaSync();
            localStorage.setItem('lifx_media_sync_active', 'true');
        } else {
            this.stopMediaSync();
            localStorage.setItem('lifx_media_sync_active', 'false');
        }
    },
    
    startMediaSync: function() {
        if (!this.mediaSyncActive) return;
        
        const syncBeat = () => {
            if (!this.mediaSyncActive) return;
            
            $.ajax({
                url: '/api/services/lifx/set_state',
                method: 'POST',
                contentType: 'application/json',
                data: JSON.stringify({
                    selector: 'all',
                    brightness: 0.3 + Math.random() * 0.7,
                    duration: 0.1
                })
            });
        };
        
        this.mediaSyncInterval = setInterval(syncBeat, 500);
        this.showEnhancedGestureFeedback('Media Sync Started', '🎵');
    },
    
    stopMediaSync: function() {
        this.mediaSyncActive = false;
        if (this.mediaSyncInterval) {
            clearInterval(this.mediaSyncInterval);
            this.mediaSyncInterval = null;
        }
        if (this.audioContext) {
            this.audioContext.close();
            this.audioContext = null;
        }
        this.showEnhancedGestureFeedback('Media Sync Stopped', '🔇');
    },
    
    initMediaSync: function() {
        const saved = localStorage.getItem('lifx_media_sync_active');
        if (saved === 'true') {
            this.mediaSyncActive = true;
            this.startMediaSync();
        }
    },
    
    toggleCircadianMode: function() {
        this.circadianModeActive = !this.circadianModeActive;
        if (this.circadianModeActive) {
            this.startCircadianMode();
            localStorage.setItem('lifx_circadian_active', 'true');
        } else {
            this.stopCircadianMode();
            localStorage.setItem('lifx_circadian_active', 'false');
        }
    },
    
    startCircadianMode: function() {
        if (!this.circadianModeActive) return;
        
        const adjustForTime = () => {
            if (!this.circadianModeActive) return;
            
            const now = new Date();
            const hour = now.getHours();
            const minute = now.getMinutes();
            const time = hour + minute / 60;
            
            let brightness, kelvin;
            
            if (time >= 6 && time < 9) {
                brightness = 0.8;
                kelvin = 4000;
            } else if (time >= 9 && time < 12) {
                brightness = 1.0;
                kelvin = 5500;
            } else if (time >= 12 && time < 17) {
                brightness = 0.9;
                kelvin = 5000;
            } else if (time >= 17 && time < 21) {
                brightness = 0.6;
                kelvin = 3500;
            } else if (time >= 21 && time < 23) {
                brightness = 0.4;
                kelvin = 2700;
            } else {
                brightness = 0.2;
                kelvin = 2200;
            }
            
            $.ajax({
                url: '/api/services/lifx/set_state',
                method: 'POST',
                contentType: 'application/json',
                data: JSON.stringify({
                    selector: 'all',
                    brightness: brightness,
                    duration: 0.5
                }),
                success: () => {
                    $.ajax({
                        url: '/api/services/lifx/set_color',
                        method: 'POST',
                        contentType: 'application/json',
                        data: JSON.stringify({
                            selector: 'all',
                            color: `kelvin:${kelvin}`
                        })
                    });
                }
            });
        };
        
        adjustForTime();
        setInterval(adjustForTime, 60000);
        this.showEnhancedGestureFeedback('Circadian Mode Started', '🕐');
    },
    
    stopCircadianMode: function() {
        this.circadianModeActive = false;
        this.showEnhancedGestureFeedback('Circadian Mode Stopped', '⏸');
    },
    
    setupEdgeGestures: function() {
        let edgeTouchStart = null;
        const edgeThreshold = 30;
        
        document.addEventListener('touchstart', (e) => {
            const touch = e.touches[0];
            if (touch.clientX < edgeThreshold) {
                edgeTouchStart = { x: touch.clientX, y: touch.clientY, fromLeft: true };
            } else if (touch.clientX > window.innerWidth - edgeThreshold) {
                edgeTouchStart = { x: touch.clientX, y: touch.clientY, fromLeft: false };
            }
        }, { passive: true });
        
        document.addEventListener('touchmove', (e) => {
            if (!edgeTouchStart) return;
            const touch = e.touches[0];
            const deltaX = touch.clientX - edgeTouchStart.x;
            
            if (edgeTouchStart.fromLeft && deltaX > 100) {
                this.toggleMediaSync();
                edgeTouchStart = null;
            } else if (!edgeTouchStart.fromLeft && deltaX < -100) {
                this.toggleCircadianMode();
                edgeTouchStart = null;
            }
        }, { passive: true });
        
        document.addEventListener('touchend', () => {
            edgeTouchStart = null;
        });
    },
    
    setupVoiceControlIntegration: function() {
        if (!('webkitSpeechRecognition' in window) && !('SpeechRecognition' in window)) {
            console.log('Voice control not supported');
            return;
        }
        
        const SpeechRecognition = window.SpeechRecognition || window.webkitSpeechRecognition;
        this.voiceRecognition = new SpeechRecognition();
        this.voiceRecognition.continuous = false;
        this.voiceRecognition.interimResults = false;
        
        this.voiceRecognition.onresult = (event) => {
            const transcript = event.results[0][0].transcript.toLowerCase();
            this.processVoiceCommand(transcript);
        };
        
        this.voiceRecognition.onerror = (event) => {
            console.error('Voice recognition error:', event.error);
            this.showEnhancedGestureFeedback('Voice error', '❌');
        };
        
        this.voiceControlEnabled = true;
        console.log('Voice control initialized');
    },
    
    processVoiceCommand: function(command) {
        const commands = {
            'lights on': () => this.powerAll('on'),
            'lights off': () => this.powerAll('off'),
            'brighter': () => this.adjustBrightness(10),
            'dimmer': () => this.adjustBrightness(-10),
            'relax': () => this.applyScene('relax'),
            'focus': () => this.applyScene('focus'),
            'party': () => this.applyScene('party'),
            'movie': () => this.applyScene('movie'),
            'night': () => this.applyScene('night'),
            'rainbow': () => this.startRainbowCycle(),
            'warm': () => this.adjustColorTemp(-500),
            'cool': () => this.adjustColorTemp(500)
        };
        
        for (const [keyword, action] of Object.entries(commands)) {
            if (command.includes(keyword)) {
                action();
                this.showEnhancedGestureFeedback(`Voice: ${keyword}`, '🎤');
                return;
            }
        }
    },
    
    initMediaCenterIntegration: function() {
        this.mediaPlaybackActive = false;
        this.beatDetectionSensitivity = 0.7;
        this.mediaSyncTargets = [];
        console.log('Media center integration initialized');
    },
    
    showMediaControls: function() {
        if (typeof Swal === 'undefined') return;
        
        Swal.fire({
            title: '<i class="fas fa-music"></i> Media Center Controls',
            html: `
                <div style="padding: 20px;">
                    <div class="media-control-section" style="margin-bottom: 20px;">
                        <h5 style="color: #00d4ff; margin-bottom: 10px;">Media Sync</h5>
                        <button class="btn ${this.mediaSyncActive ? 'btn-success' : 'btn-secondary'}" 
                                onclick="LifXTouchControls.toggleMediaSync(); Swal.close();"
                                style="width: 100%; padding: 12px;">
                            <i class="fas ${this.mediaSyncActive ? 'fa-check' : 'fa-toggle-off'}"></i>
                            ${this.mediaSyncActive ? 'Media Sync Active' : 'Enable Media Sync'}
                        </button>
                    </div>
                    
                    <div class="media-control-section" style="margin-bottom: 20px;">
                        <h5 style="color: #00d4ff; margin-bottom: 10px;">Circadian Mode</h5>
                        <button class="btn ${this.circadianModeActive ? 'btn-success' : 'btn-secondary'}" 
                                onclick="LifXTouchControls.toggleCircadianMode(); Swal.close();"
                                style="width: 100%; padding: 12px;">
                            <i class="fas ${this.circadianModeActive ? 'fa-check' : 'fa-toggle-off'}"></i>
                            ${this.circadianModeActive ? 'Circadian Active' : 'Enable Circadian'}
                        </button>
                    </div>
                    
                    <div class="media-control-section">
                        <h5 style="color: #00d4ff; margin-bottom: 10px;">Quick Actions</h5>
                        <div style="display: grid; grid-template-columns: repeat(2, 1fr); gap: 10px;">
                            <button class="btn btn-outline-info" onclick="LifXTouchControls.applyScene('movie'); Swal.close();">
                                <i class="fas fa-film"></i> Movie
                            </button>
                            <button class="btn btn-outline-info" onclick="LifXTouchControls.applyScene('gaming'); Swal.close();">
                                <i class="fas fa-gamepad"></i> Gaming
                            </button>
                            <button class="btn btn-outline-info" onclick="LifXTouchControls.applyScene('party'); Swal.close();">
                                <i class="fas fa-music"></i> Party
                            </button>
                            <button class="btn btn-outline-info" onclick="LifXTouchControls.startRainbowCycle(); Swal.close();">
                                <i class="fas fa-rainbow"></i> Rainbow
                            </button>
                        </div>
                    </div>
                </div>
            `,
            showConfirmButton: false,
            showCloseButton: true,
            width: '500px'
        });
    },
    
    activatePartyMode: function() {
        this.applyScene('party');
        this.showEnhancedGestureFeedback('Party Mode', '🎉');
    },
    
    activateCalmMode: function() {
        this.applyScene('relax');
        this.showEnhancedGestureFeedback('Calm Mode', '🧘');
    },
    
    toggleColorCycle: function() {
        if (this.colorCycleActive) {
            this.stopRainbowCycle();
        } else {
            this.startRainbowCycle();
        }
    },
    
    toggleLightPainting: function() {
        this.lightPaintingActive = !this.lightPaintingActive;
        if (this.lightPaintingActive) {
            this.showEnhancedGestureFeedback('Light Painting ON', '🎨');
        } else {
            this.showEnhancedGestureFeedback('Light Painting OFF', '⬛');
        }
    },
    
    initTouchZones: function() {
        const zones = {
            topLeft: { top: 0, bottom: window.innerHeight * 0.33, left: 0, right: window.innerWidth * 0.33, action: 'zone1' },
            topCenter: { top: 0, bottom: window.innerHeight * 0.25, left: window.innerWidth * 0.25, right: window.innerWidth * 0.75, action: 'brightness' },
            topRight: { top: 0, bottom: window.innerHeight * 0.33, left: window.innerWidth * 0.67, right: window.innerWidth, action: 'zone2' },
            bottomLeft: { top: window.innerHeight * 0.67, bottom: window.innerHeight, left: 0, right: window.innerWidth * 0.33, action: 'zone3' },
            bottomCenter: { top: window.innerHeight * 0.75, bottom: window.innerHeight, left: window.innerWidth * 0.25, right: window.innerWidth * 0.75, action: 'media' },
            bottomRight: { top: window.innerHeight * 0.67, bottom: window.innerHeight, left: window.innerWidth * 0.67, right: window.innerWidth, action: 'zone4' }
        };
        this.touchZones = zones;
        console.log('Touch zones initialized:', Object.keys(zones).length, 'zones');
    },
    
    initMultiSelectGesture: function() {
        let isDragging = false;
        let selectionBox = null;
        let startX, startY;
        
        document.addEventListener('mousedown', (e) => {
            if (e.target.closest('.lifx-bulb-control') && e.ctrlKey) {
                isDragging = true;
                startX = e.clientX;
                startY = e.clientY;
                
                selectionBox = document.createElement('div');
                selectionBox.className = 'lifx-selection-box';
                selectionBox.style.cssText = `
                    position: fixed;
                    border: 2px dashed #00d4ff;
                    background: rgba(0, 212, 255, 0.1);
                    pointer-events: none;
                    z-index: 9999;
                `;
                document.body.appendChild(selectionBox);
            }
        });
        
        document.addEventListener('mousemove', (e) => {
            if (!isDragging || !selectionBox) return;
            
            const currentX = e.clientX;
            const currentY = e.clientY;
            const left = Math.min(startX, currentX);
            const top = Math.min(startY, currentY);
            const width = Math.abs(currentX - startX);
            const height = Math.abs(currentY - startY);
            
            selectionBox.style.left = left + 'px';
            selectionBox.style.top = top + 'px';
            selectionBox.style.width = width + 'px';
            selectionBox.style.height = height + 'px';
            
            document.querySelectorAll('.lifx-bulb-control').forEach(bulb => {
                const rect = bulb.getBoundingClientRect();
                if (rect.left < currentX && rect.right > left &&
                    rect.top < currentY && rect.bottom > top) {
                    bulb.classList.add('multi-selected');
                }
            });
        });
        
        document.addEventListener('mouseup', () => {
            if (selectionBox) {
                selectionBox.remove();
                selectionBox = null;
            }
            isDragging = false;
        });
        
        console.log('Multi-select gesture initialized');
    },
    
    initTouchTrail: function() {
        if (!this.touchTrailEnabled) return;
        
        document.addEventListener('touchmove', (e) => {
            const touch = e.touches[0];
            this.touchTrailPoints.push({ x: touch.clientX, y: touch.clientY, time: Date.now() });
            
            if (this.touchTrailPoints.length > this.maxTrailPoints) {
                this.touchTrailPoints.shift();
            }
            
            this.renderTouchTrail();
        }, { passive: true });
        
        document.addEventListener('touchend', () => {
            setTimeout(() => {
                this.touchTrailPoints = [];
                this.renderTouchTrail();
            }, 300);
        });
        
        console.log('Touch trail initialized');
    },
    
    renderTouchTrail: function() {
        let trailContainer = document.querySelector('.lifx-touch-trail-container');
        if (!trailContainer) {
            trailContainer = document.createElement('div');
            trailContainer.className = 'lifx-touch-trail-container';
            trailContainer.style.cssText = `
                position: fixed;
                pointer-events: none;
                z-index: 9998;
                top: 0;
                left: 0;
                width: 100%;
                height: 100%;
            `;
            document.body.appendChild(trailContainer);
        }
        
        trailContainer.innerHTML = '';
        
        for (let i = 0; i < this.touchTrailPoints.length; i++) {
            const point = this.touchTrailPoints[i];
            const age = Date.now() - point.time;
            const opacity = 1 - (age / 500);
            
            if (opacity > 0) {
                const dot = document.createElement('div');
                dot.style.cssText = `
                    position: absolute;
                    left: ${point.x - 5}px;
                    top: ${point.y - 5}px;
                    width: 10px;
                    height: 10px;
                    background: radial-gradient(circle, rgba(0, 212, 255, ${opacity}), transparent);
                    border-radius: 50%;
                    transform: scale(${opacity});
                `;
                trailContainer.appendChild(dot);
            }
        }
    },
    
    activateWaveEffect: function() {
        const targets = this.multiBulbSelection.length > 0 
            ? this.multiBulbSelection 
            : (this.selectedBulb ? [this.selectedBulb] : []);
        
        if (targets.length === 0) {
            console.log('No bulbs selected for wave effect');
            return;
        }
        
        let delay = 0;
        const baseHue = this.lastColorHue;
        
        targets.forEach((bulbId, index) => {
            setTimeout(() => {
                const hue = (baseHue + index * 30) % 360;
                $.ajax({
                    url: '/api/services/lifx/set_color',
                    method: 'POST',
                    contentType: 'application/json',
                    data: JSON.stringify({
                        selector: `id:${bulbId}`,
                        color: `hue:${hue},saturation:80%,brightness:70%`,
                        duration: 0.5
                    })
                });
            }, delay);
            delay += 200;
        });
        
        this.showEnhancedGestureFeedback('Wave Effect', '🌊');
        console.log('Wave effect activated for', targets.length, 'bulbs');
    },
    
    toggleDiscoMode: function() {
        this.discoModeActive = !this.discoModeActive;
        
        if (this.discoModeActive) {
            this.startDiscoEffect();
            this.showEnhancedGestureFeedback('Disco Mode ON', '💃');
        } else {
            this.stopDiscoEffect();
            this.showEnhancedGestureFeedback('Disco Mode OFF', '⬛');
        }
    },
    
    startDiscoEffect: function() {
        const targets = this.multiBulbSelection.length > 0 
            ? this.multiBulbSelection 
            : (this.selectedBulb ? [this.selectedBulb] : ['all']);
        
        let hue = 0;
        const discoInterval = () => {
            if (!this.discoModeActive) return;
            
            hue = (hue + 60) % 360;
            const brightness = 70 + Math.random() * 30;
            
            targets.forEach(selector => {
                $.ajax({
                    url: '/api/services/lifx/set_color',
                    method: 'POST',
                    contentType: 'application/json',
                    data: JSON.stringify({
                        selector: selector === 'all' ? 'all' : `id:${selector}`,
                        color: `hue:${hue},saturation:100%,brightness:${brightness}%,`,
                        duration: 0.2
                    })
                });
            });
            
            this.discoTimer = setTimeout(discoInterval, 200);
        };
        
        discoInterval();
        console.log('Disco effect started');
    },
    
    stopDiscoEffect: function() {
        if (this.discoTimer) {
            clearTimeout(this.discoTimer);
            this.discoTimer = null;
        }
        console.log('Disco effect stopped');
    },
    
    showBrightnessFeedback: function(brightness) {
        let feedback = document.querySelector('.lifx-brightness-feedback');
        if (!feedback) {
            feedback = document.createElement('div');
            feedback.className = 'lifx-brightness-feedback';
            feedback.style.cssText = `
                position: fixed;
                top: 50%;
                left: 50%;
                transform: translate(-50%, -50%);
                background: rgba(0, 0, 0, 0.8);
                border-radius: 20px;
                padding: 20px 40px;
                z-index: 10000;
                display: flex;
                flex-direction: column;
                align-items: center;
                gap: 10px;
                animation: fadeIn 0.2s ease;
            `;
            document.body.appendChild(feedback);
        }
        
        feedback.innerHTML = `
            <i class="fas fa-sun" style="font-size: 48px; color: #ffd700;"></i>
            <span style="color: white; font-size: 32px; font-weight: bold;">${brightness}%</span>
            <div style="width: 200px; height: 10px; background: rgba(255,255,255,0.2); border-radius: 5px; overflow: hidden;">
                <div style="width: ${brightness}%; height: 100%; background: linear-gradient(90deg, #ffd700, #ff8c00);"></div>
            </div>
        `;
        
        if (this.brightnessFeedbackTimeout) {
            clearTimeout(this.brightnessFeedbackTimeout);
        }
        
        this.brightnessFeedbackTimeout = setTimeout(() => {
            if (feedback.parentNode) {
                feedback.remove();
            }
        }, 1000);
    },
    
    getFirstSelectedBulb: function() {
        if (this.multiBulbSelection && this.multiBulbSelection.length > 0) {
            return this.multiBulbSelection[0];
        }
        return null;
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
    
    // Initialize enhanced gesture features
    LifXTouchControls.initTouchpadMode();
    LifXTouchControls.initMicroGestures();
    LifXTouchControls.initThreeFingerGestures();
    LifXTouchControls.initQuickSceneSwipes();
    LifXTouchControls.initRadialMenu();
    LifXTouchControls.initGestureMacros();
    
    // Initialize new advanced gesture modes
    LifXTouchControls.initAdvancedGestureModes();
    LifXTouchControls.initMediaCenterIntegration();
    LifXTouchControls.initMediaSync();
    
    // Initialize quick actions and gesture shortcuts
    LifXTouchControls.initQuickActions();
    LifXTouchControls.initGestureShortcuts();
    
    // Expose global functions for voice and accessibility
    window.startLifxVoiceCommand = () => LifXTouchControls.initVoiceControl();
    window.setLifxAccessibilityMode = (enabled) => LifXTouchControls.setAccessibilityMode(enabled);
    window.saveLifxZonePreset = (name) => LifXTouchControls.saveZonePreset(name);
    window.loadLifxZonePreset = (name) => LifXTouchControls.loadZonePreset(name);
    window.queueLifxEffect = (effect, duration) => LifXTouchControls.queueEffect(effect, duration);
    window.showLifxFavoriteScenes = () => LifXTouchControls.showFavoriteScenesPanel();
    window.saveLifxGestureMacro = (name, actions) => LifXTouchControls.saveGestureMacro(name, actions);
    
    // Expose media sync functions
    window.showMediaSyncPanel = () => LifXTouchControls.showMediaSyncPanel();
    window.hideMediaSyncPanel = () => LifXTouchControls.hideMediaSyncPanel();
    window.setMediaSyncMode = (mode) => LifXTouchControls.setMediaSyncMode(mode);
    window.setBeatSensitivity = (value) => LifXTouchControls.setBeatSensitivity(value);
    window.applySyncPreset = (preset) => LifXTouchControls.applySyncPreset(preset);
    window.toggleMediaSync = () => LifXTouchControls.toggleMediaSync();
    
    // Expose new mode functions
    window.activateLifxPartyMode = () => LifXTouchControls.activatePartyMode();
    window.activateLifxCalmMode = () => LifXTouchControls.activateCalmMode();
    window.toggleLifxColorCycle = () => LifXTouchControls.toggleColorCycle();
    window.toggleLifxLightPainting = () => LifXTouchControls.toggleLightPainting();
    window.randomLifxScene = () => LifXTouchControls.randomScene();
    window.showLifxQuickActions = () => LifXTouchControls.showQuickActionsPanel();
    window.startLifxVoiceShortcut = () => LifXTouchControls.startVoiceCommand();
    
    console.log('LIFX Touch Controls initialized with enhanced features');
});

// Export for external use
if (typeof module !== 'undefined' && module.exports) {
    module.exports = LifXTouchControls;
}
