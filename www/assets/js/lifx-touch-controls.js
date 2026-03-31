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
    
    enable: function() {
        if (this.enabled) return;
        
        this.enabled = true;
        console.log('LIFX Touch Controls enabled');
        
        // Add visual indicators for touch-controlled elements
        document.querySelectorAll('.lifx-bulb-control').forEach(el => {
            el.classList.add('touch-feedback');
            el.setAttribute('data-lifx-touch', 'true');
        });
        
        // Register gesture callbacks
        if (typeof onGesture === 'function') {
            // Swipe up/down on bulb card to adjust brightness
            onGesture('swipeUp', (data) => {
                if (this.selectedBulb) {
                    this.adjustBrightness(10);
                    showSwipeHint('Brightness +');
                }
            });
            
            onGesture('swipeDown', (data) => {
                if (this.selectedBulb) {
                    this.adjustBrightness(-10);
                    showSwipeHint('Brightness -');
                }
            });
            
            // Swipe left/right to adjust color temperature
            onGesture('swipeRight', (data) => {
                if (this.selectedBulb) {
                    this.adjustColorTemp(200);
                    showSwipeHint('Warmer');
                }
            });
            
            onGesture('swipeLeft', (data) => {
                if (this.selectedBulb) {
                    this.adjustColorTemp(-200);
                    showSwipeHint('Cooler');
                }
            });
            
            // Pinch to cycle through preset scenes
            onGesture('pinchOut', (data) => {
                if (this.selectedBulb) {
                    this.nextScene();
                    showSwipeHint('Next Scene');
                }
            });
            
            onGesture('pinchIn', (data) => {
                if (this.selectedBulb) {
                    this.previousScene();
                    showSwipeHint('Previous Scene');
                }
            });
        }
        
        // Add tap handlers for bulb selection
        document.addEventListener('click', (e) => {
            const bulbEl = e.target.closest('.lifx-bulb-control');
            if (bulbEl) {
                this.selectBulb(bulbEl.getAttribute('data-bulb-id'));
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
