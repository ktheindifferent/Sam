/**
 * SAM Enhanced Touch Gestures
 * Advanced multi-touch gesture recognition with velocity tracking and edge swipes
 * Copyright 2021-2026 The Open Sam Foundation (OSF)
 */

export class EnhancedGestureRecogniser {
    constructor(options = {}) {
        this.touchStartX = 0;
        this.touchStartY = 0;
        this.touchStartTime = 0;
        this.lastTapTime = 0;
        this.pinchStartDistance = 0;
        this.rotationStartAngle = 0;
        this.isPinching = false;
        this.isRotating = false;
        this.activeTouches = new Map();
        this.longPressTimer = null;
        this.edgeSwipeStart = null;
        this.edgeSwipeDetected = false;
        this.edgeSwipeDirection = null;
        this.velocityX = 0;
        this.velocityY = 0;
        this.lastTouchX = 0;
        this.lastTouchY = 0;
        this.lastTouchTime = 0;
        this.fingerCountHistory = [];
        
        this.config = {
            minSwipeDistance: options.minSwipeDistance || 50,
            maxSwipeTime: options.maxSwipeTime || 300,
            longPressDuration: options.longPressDuration || 500,
            doubleTapInterval: options.doubleTapInterval || 300,
            pinchThreshold: options.pinchThreshold || 10,
            rotationThreshold: options.rotationThreshold || 15,
            edgeThreshold: options.edgeThreshold || 30,
            minVelocity: options.minVelocity || 0.1,
            velocitySmoothing: options.velocitySmoothing || 3
        };
        
        this.gestureCallbacks = {
            swipeLeft: [], swipeRight: [], swipeUp: [], swipeDown: [],
            tap: [], doubleTap: [], longPress: [],
            pinchIn: [], pinchOut: [], rotate: [],
            threeFingerSwipe: [], fourFingerSwipe: [], edgeSwipe: []
        };
        
        this.velocityHistory = [];
    }

    calculateDistance(touches) {
        if (touches.length < 2) return 0;
        const dx = touches[0].clientX - touches[1].clientX;
        const dy = touches[0].clientY - touches[1].clientY;
        return Math.sqrt(dx * dx + dy * dy);
    }

    calculateAngle(touches) {
        if (touches.length < 2) return 0;
        const dx = touches[1].clientX - touches[0].clientX;
        const dy = touches[1].clientY - touches[0].clientY;
        return Math.atan2(dy, dx) * (180 / Math.PI);
    }

    calculateVelocity(currentX, currentY, currentTime) {
        const deltaTime = currentTime - this.lastTouchTime;
        if (deltaTime <= 0) return { x: 0, y: 0 };
        
        const rawVelX = (currentX - this.lastTouchX) / deltaTime;
        const rawVelY = (currentY - this.lastTouchY) / deltaTime;
        
        this.velocityHistory.push({ x: rawVelX, y: rawVelY });
        if (this.velocityHistory.length > this.config.velocitySmoothing) {
            this.velocityHistory.shift();
        }
        
        this.velocityX = this.velocityHistory.reduce((sum, v) => sum + v.x, 0) / this.velocityHistory.length;
        this.velocityY = this.velocityHistory.reduce((sum, v) => sum + v.y, 0) / this.velocityHistory.length;
        
        return { x: this.velocityX, y: this.velocityY };
    }

    isEdgeSwipe(startX, startY, screenWidth, screenHeight) {
        return (
            startX < this.config.edgeThreshold ||
            startX > screenWidth - this.config.edgeThreshold ||
            startY < this.config.edgeThreshold ||
            startY > screenHeight - this.config.edgeThreshold
        );
    }

    onTouchStart(e) {
        const touches = e.touches;
        const timestamp = Date.now();
        
        for (let i = 0; i < touches.length; i++) {
            this.activeTouches.set(touches[i].identifier, {
                x: touches[i].clientX,
                y: touches[i].clientY,
                startTime: timestamp
            });
        }
        
        this.fingerCountHistory.push({ count: touches.length, timestamp });
        if (this.fingerCountHistory.length > 5) this.fingerCountHistory.shift();
        
        if (touches.length === 1) {
            this.touchStartX = touches[0].clientX;
            this.touchStartY = touches[0].clientY;
            this.touchStartTime = timestamp;
            this.lastTouchX = this.touchStartX;
            this.lastTouchY = this.touchStartY;
            this.lastTouchTime = timestamp;
            this.velocityHistory = [];
            
            const screenWidth = window.innerWidth;
            const screenHeight = window.innerHeight;
            if (this.isEdgeSwipe(this.touchStartX, this.touchStartY, screenWidth, screenHeight)) {
                this.edgeSwipeStart = { x: this.touchStartX, y: this.touchStartY };
            }
            
            if (this.longPressTimer) clearTimeout(this.longPressTimer);
            this.longPressTimer = setTimeout(() => {
                this.triggerGesture('longPress', { x: this.touchStartX, y: this.touchStartY });
                this.longPressTimer = null;
            }, this.config.longPressDuration);
        } else if (touches.length === 2) {
            this.pinchStartDistance = this.calculateDistance(touches);
            this.rotationStartAngle = this.calculateAngle(touches);
            this.isPinching = true;
            this.isRotating = true;
            if (this.longPressTimer) clearTimeout(this.longPressTimer);
        }
        
        e.preventDefault();
    }

    onTouchMove(e) {
        const touches = e.touches;
        const timestamp = Date.now();
        
        if (touches.length === 1 && !this.isPinching) {
            const currentX = touches[0].clientX;
            const currentY = touches[0].clientY;
            const deltaX = currentX - this.touchStartX;
            const deltaY = currentY - this.touchStartY;
            
            this.calculateVelocity(currentX, currentY, timestamp);
            this.lastTouchX = currentX;
            this.lastTouchY = currentY;
            this.lastTouchTime = timestamp;
            
            if (Math.abs(deltaX) > 10 || Math.abs(deltaY) > 10) {
                if (this.longPressTimer) {
                    clearTimeout(this.longPressTimer);
                    this.longPressTimer = null;
                }
            }
            
            if (this.edgeSwipeStart && !this.edgeSwipeDetected) {
                const moveFromEdge = this.touchStartX < 30 ? deltaX : -deltaX;
                if (Math.abs(moveFromEdge) > this.config.edgeThreshold) {
                    this.edgeSwipeDetected = true;
                    this.edgeSwipeDirection = deltaX > 0 ? 'right' : 'left';
                }
            }
        } else if (touches.length === 2 && this.isPinching) {
            const distance = this.calculateDistance(touches);
            const delta = distance - this.pinchStartDistance;
            
            if (Math.abs(delta) > this.config.pinchThreshold) {
                this.triggerGesture(delta > 0 ? 'pinchOut' : 'pinchIn', {
                    distance, delta,
                    velocity: Math.abs(delta) / (timestamp - this.touchStartTime)
                });
                this.pinchStartDistance = distance;
            }
            
            const currentAngle = this.calculateAngle(touches);
            const angleDelta = currentAngle - this.rotationStartAngle;
            if (Math.abs(angleDelta) > this.config.rotationThreshold) {
                this.triggerGesture('rotate', { angle: currentAngle, delta: angleDelta });
                this.rotationStartAngle = currentAngle;
            }
        } else if (touches.length >= 3) {
            if (this.fingerCountHistory.every(f => f.count >= 3)) {
                const deltaX = touches[0].clientX - this.touchStartX;
                const deltaY = touches[0].clientY - this.touchStartY;
                if (Math.abs(deltaX) > this.config.minSwipeDistance || Math.abs(deltaY) > this.config.minSwipeDistance) {
                    const gesture = touches.length === 3 ? 'threeFingerSwipe' : 'fourFingerSwipe';
                    let direction = '';
                    if (Math.abs(deltaX) > Math.abs(deltaY)) {
                        direction = deltaX > 0 ? 'right' : 'left';
                    } else {
                        direction = deltaY > 0 ? 'down' : 'up';
                    }
                    this.triggerGesture(gesture, { direction, fingerCount: touches.length });
                    this.fingerCountHistory = [];
                }
            }
        }
        
        e.preventDefault();
    }

    onTouchEnd(e) {
        const timestamp = Date.now();
        
        if (this.longPressTimer) {
            clearTimeout(this.longPressTimer);
            this.longPressTimer = null;
        }
        
        if (e.touches.length === 0 && !this.isPinching) {
            if (this.edgeSwipeDetected) {
                this.triggerGesture('edgeSwipe', {
                    direction: this.edgeSwipeDirection,
                    startX: this.edgeSwipeStart.x,
                    startY: this.edgeSwipeStart.y
                });
                this.edgeSwipeDetected = false;
                this.edgeSwipeDirection = null;
            }
            
            const deltaTime = timestamp - this.touchStartTime;
            if (deltaTime < this.config.maxSwipeTime) {
                if (timestamp - this.lastTapTime < this.config.doubleTapInterval) {
                    this.triggerGesture('doubleTap', { x: this.touchStartX, y: this.touchStartY });
                } else {
                    this.triggerGesture('tap', { x: this.touchStartX, y: this.touchStartY });
                }
                this.lastTapTime = timestamp;
            }
            
            const timeDiff = timestamp - this.touchStartTime;
            const absDeltaX = Math.abs(this.touchStartX - this.touchStartX);
            const absDeltaY = Math.abs(this.touchStartY - this.touchStartY);
            
            if (timeDiff < this.config.maxSwipeTime && 
                (absDeltaX > this.config.minSwipeDistance || absDeltaY > this.config.minSwipeDistance)) {
                let swipeType = 'swipe';
                if (this.fingerCountHistory.some(f => f.count === 3)) {
                    swipeType = 'threeFingerSwipe';
                } else if (this.fingerCountHistory.some(f => f.count === 4)) {
                    swipeType = 'fourFingerSwipe';
                }
                
                if (absDeltaX > absDeltaY) {
                    const direction = this.touchStartX > this.touchStartX ? 'right' : 'left';
                    this.triggerGesture(swipeType, { direction, velocity: Math.abs(this.velocityX) });
                } else {
                    const direction = this.touchStartY > this.touchStartY ? 'down' : 'up';
                    this.triggerGesture(swipeType, { direction, velocity: Math.abs(this.velocityY) });
                }
            }
        } else if (e.touches.length < 2) {
            this.isPinching = false;
            this.isRotating = false;
        }
        
        const endedTouches = Array.from(this.activeTouches.keys()).filter(id => 
            !Array.from(e.touches).some(t => t.identifier === id)
        );
        endedTouches.forEach(id => this.activeTouches.delete(id));
        
        if (e.touches.length === 0) {
            this.fingerCountHistory = [];
        }
    }

    triggerGesture(name, data = {}) {
        const callbacks = this.gestureCallbacks[name] || [];
        callbacks.forEach(cb => cb(data));
    }

    on(gesture, callback) {
        if (this.gestureCallbacks[gesture]) {
            this.gestureCallbacks[gesture].push(callback);
        }
        return this;
    }

    off(gesture, callback) {
        if (this.gestureCallbacks[gesture]) {
            this.gestureCallbacks[gesture] = this.gestureCallbacks[gesture].filter(cb => cb !== callback);
        }
        return this;
    }

    attach(element) {
        element.addEventListener('touchstart', (e) => this.onTouchStart(e), { passive: false });
        element.addEventListener('touchmove', (e) => this.onTouchMove(e), { passive: false });
        element.addEventListener('touchend', (e) => this.onTouchEnd(e), { passive: false });
        element.addEventListener('touchcancel', (e) => this.onTouchEnd(e), { passive: false });
    }

    destroy() {
        if (this.longPressTimer) clearTimeout(this.longPressTimer);
        this.activeTouches.clear();
        this.gestureCallbacks = {};
    }
}

export default EnhancedGestureRecogniser;
