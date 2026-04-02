/**
 * SAM Touch Interface Improvements
 * Enhanced touch responsiveness, gestures, and haptic feedback
 * Copyright 2021-2026 The Open Sam Foundation (OSF)
 */

(function() {
    'use strict';

    const TouchInterface = {
        config: {
            touchSensitivity: 'medium',
            hapticEnabled: true,
            gestureHints: true,
            rippleEffect: true,
            swipeEdgeZone: 30,
            longPressDelay: 400,
            doubleTapDelay: 250,
            swipeMinDistance: 40,
            swipeMaxTime: 300,
            pinchMinDistance: 25,
            touchTrailEnabled: true,
            adaptiveSensitivity: true,
            accessibilityMode: false,
            highContrastHints: false,
            reducedMotionMode: false
        },

        state: {
            isTouchDevice: false,
            lastTouchX: 0,
            lastTouchY: 0,
            lastTouchTime: 0,
            touchVelocity: 0,
            touchDirection: null,
            isLongPress: false,
            longPressTimer: null,
            lastTapTime: 0,
            tapCount: 0,
            touchPoints: [],
            gestureInProgress: false,
            gestureStartX: 0,
            gestureStartY: 0,
            gestureStartTime: 0,
            edgeSwipeZone: null,
            activeTouches: new Map(),
            touchHistory: [],
            maxHistoryLength: 10,
            gestureAccuracyScore: 100,
            consecutiveSuccesses: 0,
            consecutiveFails: 0
        },

        sensitivityLevels: {
            low: { swipeDistance: 80, swipeTime: 400, pinchDistance: 50, longPressDelay: 600 },
            medium: { swipeDistance: 50, swipeTime: 300, pinchDistance: 30, longPressDelay: 400 },
            high: { swipeDistance: 30, swipeTime: 200, pinchDistance: 20, longPressDelay: 300 },
            very_high: { swipeDistance: 15, swipeTime: 150, pinchDistance: 10, longPressDelay: 200 }
        },

        hapticPatterns: {
            light: { duration: 10, intensity: 0.3 },
            medium: { duration: 20, intensity: 0.5 },
            strong: { duration: 40, intensity: 0.8 },
            success: { pattern: [10, 50, 10], intensity: 0.6 },
            error: { pattern: [30, 50, 30], intensity: 0.7 },
            warning: { pattern: [15, 30, 15], intensity: 0.5 },
            gesture: { duration: 15, intensity: 0.4 },
            selection: { duration: 8, intensity: 0.3 }
        },

        init() {
            this.detectTouchDevice();
            this.loadUserPreferences();
            this.setupGlobalTouchListeners();
            this.setupGestureRecognition();
            this.setupTouchFeedback();
            this.setupAdaptiveSensitivity();
            this.setupAccessibilityFeatures();
            console.log('[TouchInterface] Initialized', this.state.isTouchDevice ? '(Touch Device)' : '(Mouse/Keyboard)');
        },

        detectTouchDevice() {
            this.state.isTouchDevice = 'ontouchstart' in window ||
                navigator.maxTouchPoints > 0 ||
                navigator.msMaxTouchPoints > 0;

            if (this.state.isTouchDevice) {
                document.documentElement.classList.add('touch-device');
                this.setupTouchOptimizations();
            } else {
                document.documentElement.classList.add('mouse-device');
            }
        },

        setupTouchOptimizations() {
            document.querySelectorAll('[data-touch-optimized]').forEach(el => {
                el.style.touchAction = 'manipulation';
                el.style.webkitTapHighlightColor = 'transparent';
                el.setAttribute('tabindex', '0');
            });

            const style = document.createElement('style');
            style.textContent = `
                .touch-device * {
                    -webkit-tap-highlight-color: transparent;
                    touch-action: manipulation;
                }
                .touch-feedback {
                    transition: transform 0.12s ease, box-shadow 0.12s ease;
                    will-change: transform;
                }
                .touch-feedback:active {
                    transform: scale(0.95);
                }
                .touch-ripple {
                    position: absolute;
                    border-radius: 50%;
                    background: radial-gradient(circle, rgba(0, 212, 255, 0.6) 0%, transparent 70%);
                    transform: scale(0);
                    animation: ripple-animation 0.4s ease-out;
                    pointer-events: none;
                }
                @keyframes ripple-animation {
                    to { transform: scale(2.5); opacity: 0; }
                }
                .touch-trail {
                    position: fixed;
                    width: 20px;
                    height: 20px;
                    border-radius: 50%;
                    background: radial-gradient(circle, rgba(0, 212, 255, 0.4) 0%, transparent 70%);
                    pointer-events: none;
                    z-index: 9999;
                    animation: trail-fade 0.3s ease-out forwards;
                }
                @keyframes trail-fade {
                    to { transform: scale(0.5); opacity: 0; }
                }
                .gesture-hint {
                    position: fixed;
                    background: rgba(0, 212, 255, 0.9);
                    color: white;
                    padding: 8px 16px;
                    border-radius: 20px;
                    font-size: 14px;
                    font-weight: 500;
                    z-index: 10000;
                    animation: hint-appear 0.2s ease, hint-fade 0.3s ease 0.8s forwards;
                    pointer-events: none;
                }
                @keyframes hint-appear {
                    from { opacity: 0; transform: translateY(10px); }
                    to { opacity: 1; transform: translateY(0); }
                }
                @keyframes hint-fade {
                    to { opacity: 0; }
                }
            `;
            document.head.appendChild(style);
        },

        setupGlobalTouchListeners() {
            document.addEventListener('touchstart', (e) => this.handleTouchStart(e), { passive: true });
            document.addEventListener('touchmove', (e) => this.handleTouchMove(e), { passive: true });
            document.addEventListener('touchend', (e) => this.handleTouchEnd(e), { passive: true });
            document.addEventListener('touchcancel', (e) => this.handleTouchCancel(e), { passive: true });
        },

        handleTouchStart(e) {
            const touch = e.touches[0];
            const now = Date.now();

            this.state.touchPoints.push({
                x: touch.clientX,
                y: touch.clientY,
                time: now,
                identifier: touch.identifier
            });

            this.state.gestureStartX = touch.clientX;
            this.state.gestureStartY = touch.clientY;
            this.state.gestureStartTime = now;
            this.state.lastTouchX = touch.clientX;
            this.state.lastTouchY = touch.clientY;
            this.state.lastTouchTime = now;

            this.checkEdgeSwipe(touch.clientX, touch.clientY);

            if (e.touches.length === 1) {
                this.state.longPressTimer = setTimeout(() => {
                    this.state.isLongPress = true;
                    this.triggerGesture('longPress', { x: touch.clientX, y: touch.clientY });
                    this.hapticFeedback('medium');
                }, this.sensitivityLevels[this.config.touchSensitivity].longPressDelay);

                this.createTouchRipple(touch.clientX, touch.clientY);
            }

            if (e.touches.length > 1) {
                clearTimeout(this.state.longPressTimer);
                this.state.touchPoints = Array.from(e.touches).map(t => ({
                    x: t.clientX,
                    y: t.clientY,
                    time: now,
                    identifier: t.identifier
                }));
            }

            if (this.config.touchTrailEnabled) {
                this.createTouchTrail(touch.clientX, touch.clientY);
            }
        },

        handleTouchMove(e) {
            if (e.touches.length === 0) return;

            const touch = e.touches[0];
            const now = Date.now();
            const deltaX = touch.clientX - this.state.lastTouchX;
            const deltaY = touch.clientY - this.state.lastTouchY;
            const deltaTime = now - this.state.lastTouchTime;

            this.state.touchVelocity = Math.sqrt(deltaX * deltaX + deltaY * deltaY) / Math.max(deltaTime, 1);
            this.state.lastTouchX = touch.clientX;
            this.state.lastTouchY = touch.clientY;
            this.state.lastTouchTime = now;

            const totalDeltaX = touch.clientX - this.state.gestureStartX;
            const totalDeltaY = touch.clientY - this.state.gestureStartY;
            const totalDistance = Math.sqrt(totalDeltaX * totalDeltaX + totalDeltaY * totalDeltaY);
            const sensitivity = this.sensitivityLevels[this.config.touchSensitivity];

            if (totalDistance > sensitivity.swipeDistance && !this.state.gestureInProgress) {
                clearTimeout(this.state.longPressTimer);
                this.state.gestureInProgress = true;

                if (Math.abs(totalDeltaX) > Math.abs(totalDeltaY)) {
                    this.state.touchDirection = totalDeltaX > 0 ? 'right' : 'left';
                } else {
                    this.state.touchDirection = totalDeltaY > 0 ? 'down' : 'up';
                }
            }

            if (this.state.gestureInProgress && this.config.touchTrailEnabled) {
                this.createTouchTrail(touch.clientX, touch.clientY);
            }

            if (e.touches.length === 2 && this.state.touchPoints.length >= 2) {
                const initialDistance = this.getDistance(
                    this.state.touchPoints[0].x, this.state.touchPoints[0].y,
                    this.state.touchPoints[1].x, this.state.touchPoints[1].y
                );
                const currentDistance = this.getDistance(
                    e.touches[0].clientX, e.touches[0].clientY,
                    e.touches[1].clientX, e.touches[1].clientY
                );
                const distanceDelta = Math.abs(currentDistance - initialDistance);

                if (distanceDelta > sensitivity.pinchDistance) {
                    const gesture = currentDistance > initialDistance ? 'pinchOut' : 'pinchIn';
                    this.triggerGesture(gesture, {
                        distance: distanceDelta,
                        x: (e.touches[0].clientX + e.touches[1].clientX) / 2,
                        y: (e.touches[0].clientY + e.touches[1].clientY) / 2
                    });
                }
            }

            e.preventDefault();
        },

        handleTouchEnd(e) {
            clearTimeout(this.state.longPressTimer);
            const now = Date.now();

            if (e.touches.length === 0) {
                if (this.state.gestureInProgress && this.state.touchDirection) {
                    const gesture = 'swipe' + this.state.touchDirection.charAt(0).toUpperCase() + this.state.touchDirection.slice(1);
                    const duration = now - this.state.gestureStartTime;
                    const sensitivity = this.sensitivityLevels[this.config.touchSensitivity];

                    if (duration < sensitivity.swipeTime) {
                        this.triggerGesture(gesture, {
                            distance: Math.sqrt(
                                Math.pow(this.state.lastTouchX - this.state.gestureStartX, 2) +
                                Math.pow(this.state.lastTouchY - this.state.gestureStartY, 2)
                            ),
                            duration: duration,
                            velocity: this.state.touchVelocity,
                            x: this.state.lastTouchX,
                            y: this.state.lastTouchY
                        });
                    }

                    this.state.gestureInProgress = false;
                    this.state.touchDirection = null;
                } else if (!this.state.isLongPress) {
                    const timeSinceLastTap = now - this.state.lastTapTime;

                    if (timeSinceLastTap < this.config.doubleTapDelay) {
                        this.state.tapCount++;
                        if (this.state.tapCount >= 2) {
                            this.triggerGesture('doubleTap', {
                                x: this.state.lastTouchX,
                                y: this.state.lastTouchY
                            });
                            this.state.tapCount = 0;
                        }
                    } else {
                        this.state.tapCount = 1;
                        this.state.lastTapTime = now;
                        setTimeout(() => {
                            if (this.state.tapCount === 1) {
                                this.triggerGesture('tap', {
                                    x: this.state.lastTouchX,
                                    y: this.state.lastTouchY
                                });
                            }
                            this.state.tapCount = 0;
                        }, this.config.doubleTapDelay);
                    }
                }

                this.state.isLongPress = false;
                this.state.touchPoints = [];
            }

            if (e.touches.length === 1 && this.state.touchPoints.length > 1) {
                this.state.touchPoints.shift();
            }
        },

        handleTouchCancel(e) {
            clearTimeout(this.state.longPressTimer);
            this.state.gestureInProgress = false;
            this.state.touchPoints = [];
        },

        setupGestureRecognition() {
            window.gestureHandlers = new Map();

            window.onGesture = (gestureName, handler) => {
                if (!window.gestureHandlers.has(gestureName)) {
                    window.gestureHandlers.set(gestureName, []);
                }
                window.gestureHandlers.get(gestureName).push(handler);
            };

            window.triggerGesture = (gestureName, data) => {
                this.triggerGesture(gestureName, data);
            };
        },

        triggerGesture(gestureName, data) {
            const handlers = window.gestureHandlers?.get(gestureName);
            if (handlers) {
                handlers.forEach(handler => {
                    try {
                        handler(data);
                        this.recordGestureSuccess();
                    } catch (e) {
                        console.error('[TouchInterface] Gesture handler error:', e);
                        this.recordGestureFail();
                    }
                });
            }

            if (this.config.gestureHints) {
                this.showGestureHint(gestureName, data);
            }
        },

        setupTouchFeedback() {
            if (!this.config.rippleEffect) return;

            document.addEventListener('click', (e) => {
                if (e.target.closest('[data-no-ripple]')) return;
                this.createTouchRipple(e.clientX, e.clientY);
            });
        },

        createTouchRipple(x, y) {
            if (!this.config.rippleEffect) return;

            const ripple = document.createElement('div');
            ripple.className = 'touch-ripple';
            ripple.style.left = `${x - 25}px`;
            ripple.style.top = `${y - 25}px`;
            ripple.style.width = '50px';
            ripple.style.height = '50px';
            document.body.appendChild(ripple);

            setTimeout(() => ripple.remove(), 400);
        },

        createTouchTrail(x, y) {
            if (!this.config.touchTrailEnabled) return;

            const trail = document.createElement('div');
            trail.className = 'touch-trail';
            trail.style.left = `${x - 10}px`;
            trail.style.top = `${y - 10}px`;
            document.body.appendChild(trail);

            setTimeout(() => trail.remove(), 300);
        },

        showGestureHint(gestureName, data) {
            const hints = {
                swipeUp: '↑ Swipe Up',
                swipeDown: '↓ Swipe Down',
                swipeLeft: '← Swipe Left',
                swipeRight: '→ Swipe Right',
                doubleTap: '👆 Double Tap',
                longPress: '✋ Long Press',
                pinchOut: '👌 Pinch Out',
                pinchIn: '👌 Pinch In',
                tap: '👆 Tap'
            };

            const hint = document.createElement('div');
            hint.className = 'gesture-hint';
            hint.textContent = hints[gestureName] || gestureName;
            hint.style.left = `${(data?.x || window.innerWidth / 2) - 50}px`;
            hint.style.top = `${(data?.y || window.innerHeight / 2) - 30}px`;
            document.body.appendChild(hint);

            setTimeout(() => hint.remove(), 1200);
        },

        hapticFeedback(type = 'medium') {
            if (!this.config.hapticEnabled || !navigator.vibrate) return;

            const pattern = this.hapticPatterns[type];
            if (!pattern) return;

            if (pattern.pattern) {
                navigator.vibrate(pattern.pattern);
            } else {
                navigator.vibrate(pattern.duration);
            }
        },

        checkEdgeSwipe(x, y) {
            const zone = this.config.swipeEdgeZone;
            if (x < zone) this.state.edgeSwipeZone = 'left';
            else if (x > window.innerWidth - zone) this.state.edgeSwipeZone = 'right';
            else if (y < zone) this.state.edgeSwipeZone = 'top';
            else if (y > window.innerHeight - zone) this.state.edgeSwipeZone = 'bottom';
            else this.state.edgeSwipeZone = null;
        },

        setupAdaptiveSensitivity() {
            if (!this.config.adaptiveSensitivity) return;

            setInterval(() => {
                const successRate = this.state.consecutiveSuccesses /
                    Math.max(1, this.state.consecutiveSuccesses + this.state.consecutiveFails);

                if (successRate < 0.3 && this.config.touchSensitivity !== 'low') {
                    const levels = ['very_high', 'high', 'medium', 'low'];
                    const currentIndex = levels.indexOf(this.config.touchSensitivity);
                    if (currentIndex < levels.length - 1) {
                        this.config.touchSensitivity = levels[currentIndex + 1];
                        console.log('[TouchInterface] Adjusted sensitivity to:', this.config.touchSensitivity);
                    }
                } else if (successRate > 0.9 && this.config.touchSensitivity !== 'very_high') {
                    const levels = ['low', 'medium', 'high', 'very_high'];
                    const currentIndex = levels.indexOf(this.config.touchSensitivity);
                    if (currentIndex > 0) {
                        this.config.touchSensitivity = levels[currentIndex - 1];
                        console.log('[TouchInterface] Adjusted sensitivity to:', this.config.touchSensitivity);
                    }
                }

                this.state.consecutiveSuccesses = 0;
                this.state.consecutiveFails = 0;
            }, 30000);
        },

        recordGestureSuccess() {
            this.state.consecutiveSuccesses++;
            this.state.gestureAccuracyScore = Math.min(100, this.state.gestureAccuracyScore + 2);
        },

        recordGestureFail() {
            this.state.consecutiveFails++;
            this.state.gestureAccuracyScore = Math.max(50, this.state.gestureAccuracyScore - 5);
        },

        setupAccessibilityFeatures() {
            if (this.config.accessibilityMode) {
                document.documentElement.classList.add('accessibility-mode');
                this.config.hapticEnabled = false;
                this.config.touchTrailEnabled = false;
                this.config.gestureHints = false;
            }

            if (this.config.reducedMotionMode) {
                document.documentElement.classList.add('reduced-motion');
                this.config.rippleEffect = false;
                this.config.touchTrailEnabled = false;
            }

            if (this.config.highContrastHints) {
                document.documentElement.classList.add('high-contrast-hints');
            }
        },

        loadUserPreferences() {
            const saved = localStorage.getItem('touchInterfaceConfig');
            if (saved) {
                try {
                    const config = JSON.parse(saved);
                    Object.assign(this.config, config);
                } catch (e) {
                    console.error('[TouchInterface] Failed to load preferences:', e);
                }
            }
        },

        saveUserPreferences() {
            localStorage.setItem('touchInterfaceConfig', JSON.stringify(this.config));
        },

        getDistance(x1, y1, x2, y2) {
            return Math.sqrt(Math.pow(x2 - x1, 2) + Math.pow(y2 - y1, 2));
        },

        setSensitivity(level) {
            if (this.sensitivityLevels[level]) {
                this.config.touchSensitivity = level;
                this.saveUserPreferences();
                console.log('[TouchInterface] Sensitivity set to:', level);
            }
        },

        toggleHaptic() {
            this.config.hapticEnabled = !this.config.hapticEnabled;
            this.saveUserPreferences();
            return this.config.hapticEnabled;
        },

        toggleRipple() {
            this.config.rippleEffect = !this.config.rippleEffect;
            this.saveUserPreferences();
            return this.config.rippleEffect;
        },

        toggleTrails() {
            this.config.touchTrailEnabled = !this.config.touchTrailEnabled;
            this.saveUserPreferences();
            return this.config.touchTrailEnabled;
        }
    };

    TouchInterface.init();
    window.TouchInterface = TouchInterface;
})();
