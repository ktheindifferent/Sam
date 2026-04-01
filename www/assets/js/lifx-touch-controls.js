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
    
    enable: function(showTutorial = false) {
        if (this.enabled) return;
        
        this.enabled = true;
        console.log('LIFX Touch Controls enabled');
        
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
        
        // Add tap handlers for bulb selection
        document.addEventListener('click', (e) => {
            const bulbEl = e.target.closest('.lifx-bulb-control, .lifx-bulb-card');
            if (bulbEl) {
                const bulbId = bulbEl.getAttribute('data-bulb-id');
                if (e.ctrlKey || e.metaKey) {
                    // Multi-select with Ctrl/Cmd
                    this.toggleBulbSelection(bulbId);
                } else {
                    this.selectBulb(bulbId);
                }
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
        this.brightnessLevel = parseInt(brightness);
        this.colorTempLevel = parseInt(kelvin);
        
        $.ajax({
            url: '/api/services/lifx/set_state',
            method: 'POST',
            contentType: 'application/json',
            data: JSON.stringify({
                selector: `id:${bulbId}`,
                brightness: this.brightnessLevel,
                duration: 0.5
            }),
            success: () => {
                $.ajax({
                    url: '/api/services/lifx/set_color',
                    method: 'POST',
                    contentType: 'application/json',
                    data: JSON.stringify({
                        selector: `id:${bulbId}`,
                        color: `kelvin:${this.colorTempLevel}`
                    }),
                    success: () => {
                        this.updateBulbVisual(bulbId);
                        showNotification('Settings applied', 'success');
                    }
                });
            }
        });
    },
    
    togglePower: function() {
        const bulbId = this.selectedBulb || this.getFirstSelectedBulb();
        if (!bulbId) return;
        
        const bulbEl = document.querySelector(`.lifx-bulb-control[data-bulb-id="${bulbId}"]`);
        const isOn = bulbEl?.classList.contains('power-on');
        
        $.ajax({
            url: '/api/services/lifx/set_state',
            method: 'POST',
            contentType: 'application/json',
            data: JSON.stringify({
                selector: `id:${bulbId}`,
                power: isOn ? 'off' : 'on',
                duration: 0.3
            }),
            success: () => {
                this.updateBulbVisual(bulbId);
                showNotification(`Bulb ${isOn ? 'turned off' : 'turned on'}`, 'info');
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
    
    adjustBrightness: function(delta) {
        if (!this.selectedBulb) return;
        
        this.brightnessLevel = Math.max(0, Math.min(100, this.brightnessLevel + delta));
        
        // Send command to LIFX API
        $.ajax({
            url: '/api/services/lifx/set_state',
            method: 'POST',
            contentType: 'application/json',
            data: JSON.stringify({
                selector: `id:${this.selectedBulb}`,
                brightness: this.brightnessLevel,
                duration: 0.3
            }),
            success: (response) => {
                console.log('Brightness adjusted:', this.brightnessLevel);
                this.updateBulbVisual(this.selectedBulb);
            },
            error: (err) => {
                console.error('Failed to adjust brightness:', err);
            }
        });
    },
    
    adjustColorTemp: function(delta) {
        if (!this.selectedBulb) return;
        
        this.colorTempLevel = Math.max(1500, Math.min(9000, this.colorTempLevel + delta));
        
        // Send command to LIFX API
        $.ajax({
            url: '/api/services/lifx/set_color',
            method: 'POST',
            contentType: 'application/json',
            data: JSON.stringify({
                selector: `id:${this.selectedBulb}`,
                color: `kelvin:${this.colorTempLevel}`
            }),
            success: (response) => {
                console.log('Color temperature adjusted:', this.colorTempLevel);
                this.updateBulbVisual(this.selectedBulb);
            },
            error: (err) => {
                console.error('Failed to adjust color temp:', err);
            }
        });
    },
    
    nextScene: function() {
        const scenes = ['relax', 'focus', 'energize', 'night'];
        const currentIndex = scenes.indexOf(this.currentScene || 'relax');
        this.currentScene = scenes[(currentIndex + 1) % scenes.length];
        this.applyScene(this.currentScene);
    },
    
    previousScene: function() {
        const scenes = ['relax', 'focus', 'energize', 'night'];
        const currentIndex = scenes.indexOf(this.currentScene || 'relax');
        this.currentScene = scenes[(currentIndex - 1 + scenes.length) % scenes.length];
        this.applyScene(this.currentScene);
    },
    
    applyScene: function(scene) {
        if (!this.selectedBulb) return;
        
        const sceneSettings = {
            relax: { brightness: 40, kelvin: 2700 },
            focus: { brightness: 80, kelvin: 5000 },
            energize: { brightness: 100, kelvin: 6500 },
            night: { brightness: 20, kelvin: 2000 }
        };
        
        const settings = sceneSettings[scene];
        if (settings) {
            this.brightnessLevel = settings.brightness;
            this.colorTempLevel = settings.kelvin;
            
            $.ajax({
                url: '/api/services/lifx/set_state',
                method: 'POST',
                contentType: 'application/json',
                data: JSON.stringify({
                    selector: `id:${this.selectedBulb}`,
                    brightness: settings.brightness,
                    duration: 0.5
                }),
                success: () => {
                    $.ajax({
                        url: '/api/services/lifx/set_color',
                        method: 'POST',
                        contentType: 'application/json',
                        data: JSON.stringify({
                            selector: `id:${this.selectedBulb}`,
                            color: `kelvin:${settings.kelvin}`
                        }),
                        success: () => {
                            console.log('Scene applied:', scene);
                            this.updateBulbVisual(this.selectedBulb);
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
            
            // Visual feedback
            bulbEl.classList.add('touch-updated');
            setTimeout(() => bulbEl.classList.remove('touch-updated'), 300);
        }
    },
    
    disable: function() {
        this.enabled = false;
        this.selectedBulb = null;
        document.querySelectorAll('.lifx-bulb-control').forEach(el => {
            el.classList.remove('touch-feedback');
            el.removeAttribute('data-lifx-touch');
        });
        console.log('LIFX Touch Controls disabled');
    }
};

// Initialize when DOM is ready
document.addEventListener('DOMContentLoaded', function() {
    // Auto-enable if touch device detected
    if (typeof is_touch_enabled === 'function' && is_touch_enabled()) {
        LifXTouchControls.enable();
        console.log('Touch device detected - LIFX touch controls auto-enabled');
    }
});

// Export for external use
if (typeof module !== 'undefined' && module.exports) {
    module.exports = LifXTouchControls;
}
