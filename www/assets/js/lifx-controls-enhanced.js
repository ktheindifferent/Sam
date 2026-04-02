/**
 * SAM LIFX Enhanced Controls
 * Advanced light control with effects, scenes, and media synchronization
 * Copyright 2021-2026 The Open Sam Foundation (OSF)
 */

export class LIFXControlsEnhanced {
    constructor(options = {}) {
        this.config = {
            apiBaseUrl: options.apiBaseUrl || '/api/services/lifx',
            defaultDuration: options.defaultDuration || 0.5,
            maxBrightness: options.maxBrightness || 100,
            minBrightness: options.minBrightness || 0,
            colorTransitionSmooth: options.colorTransitionSmooth ?? true,
            enableEffects: options.enableEffects ?? true,
            effectFrameRate: options.effectFrameRate || 60,
            mediaSyncEnabled: options.mediaSyncEnabled ?? false,
            circadianRhythmEnabled: options.circadianRhythmEnabled ?? false
        };
        
        this.state = {
            bulbs: [],
            selectedBulbs: new Set(),
            activeScene: null,
            activeEffect: null,
            brightness: 50,
            colorTemperature: 4000,
            currentColor: { h: 0, s: 0, b: 50, k: 4000 },
            effectRunning: false,
            effectInterval: null,
            mediaSyncActive: false,
            circadianActive: false,
            lastCommandTime: 0,
            commandQueue: [],
            groupStates: new Map(),
            roomStates: new Map()
        };
        
        this.effects = {
            pulse: this.pulseEffect.bind(this),
            breathe: this.breatheEffect.bind(this),
            rainbow: this.rainbowEffect.bind(this),
            fire: this.fireEffect.bind(this),
            aurora: this.auroraEffect.bind(this),
            strobe: this.strobeEffect.bind(this),
            party: this.partyEffect.bind(this),
            candle: this.candleEffect.bind(this),
            lightning: this.lightningEffect.bind(this),
            sunrise: this.sunriseEffect.bind(this),
            sunset: this.sunsetEffect.bind(this),
            cop: this.copEffect.bind(this),
            emergency: this.emergencyEffect.bind(this),
            colorLoop: this.colorLoopEffect.bind(this)
        };
        
        this.scenes = {
            relax: { h: 5800, s: 15000, b: 40, k: 2700 },
            focus: { h: 19000, s: 8000, b: 80, k: 5000 },
            energize: { h: 41000, s: 20000, b: 100, k: 6500 },
            night: { h: 5800, s: 10000, b: 20, k: 2000 },
            reading: { h: 19000, s: 5000, b: 75, k: 4500 },
            romance: { h: 60000, s: 25000, b: 50, k: 3000 },
            party: { h: 43680, s: 65535, b: 100, k: 5500 },
            movie: { h: 3640, s: 19660, b: 30, k: 2200 },
            gaming: { h: 50960, s: 52428, b: 70, k: 5500 },
            cooking: { h: 5460, s: 32767, b: 90, k: 4500 },
            sleep: { h: 43680, s: 6553, b: 15, k: 2000 },
            wake: { h: 9100, s: 32767, b: 80, k: 5500 },
            meditation: { h: 50960, s: 19660, b: 35, k: 2400 },
            yoga: { h: 25480, s: 26214, b: 45, k: 3800 },
            dinner: { h: 6000, s: 26214, b: 55, k: 3000 }
        };
        
        this.eventCallbacks = {
            bulbsUpdated: [], sceneChanged: [], effectStarted: [], effectStopped: [],
            colorChanged: [], brightnessChanged: [], powerChanged: [], mediaSyncToggled: []
        };
    }

    async fetchBulbs(selector = 'all') {
        try {
            const response = await fetch(`${this.config.apiBaseUrl}/v1/lights/${selector}`);
            const data = await response.json();
            this.state.bulbs = Array.isArray(data) ? data : (data.results || []);
            this.triggerEvent('bulbsUpdated', { bulbs: this.state.bulbs });
            return this.state.bulbs;
        } catch (error) {
            console.error('Failed to fetch bulbs:', error);
            return [];
        }
    }

    async setPower(power, selector = 'all', duration = null) {
        const payload = {
            power,
            duration: duration ?? this.config.defaultDuration
        };
        
        try {
            const response = await fetch(`${this.config.apiBaseUrl}/v1/lights/${selector}/state`, {
                method: 'PUT',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify(payload)
            });
            const result = await response.json();
            this.triggerEvent('powerChanged', { power, selector, results: result.results });
            return result;
        } catch (error) {
            console.error('Failed to set power:', error);
            return null;
        }
    }

    async setColor(color, selector = 'all', duration = null) {
        const payload = { color };
        if (duration !== null) payload.duration = duration;
        else payload.duration = this.config.defaultDuration;
        
        try {
            const response = await fetch(`${this.config.apiBaseUrl}/v1/lights/${selector}/state`, {
                method: 'PUT',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify(payload)
            });
            const result = await response.json();
            this.state.currentColor = this.parseColor(color);
            this.triggerEvent('colorChanged', { color: this.state.currentColor, selector, results: result.results });
            return result;
        } catch (error) {
            console.error('Failed to set color:', error);
            return null;
        }
    }

    async setBrightness(brightness, selector = 'all', duration = null) {
        const level = Math.max(this.config.minBrightness, Math.min(this.config.maxBrightness, brightness));
        const payload = {
            brightness: level / 100,
            duration: duration ?? this.config.defaultDuration
        };
        
        try {
            const response = await fetch(`${this.config.apiBaseUrl}/v1/lights/${selector}/state`, {
                method: 'PUT',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify(payload)
            });
            const result = await response.json();
            this.state.brightness = level;
            this.triggerEvent('brightnessChanged', { brightness: level, selector, results: result.results });
            return result;
        } catch (error) {
            console.error('Failed to set brightness:', error);
            return null;
        }
    }

    async setColorTemperature(kelvin, selector = 'all', duration = null) {
        const payload = {
            color: { kelvin },
            duration: duration ?? this.config.defaultDuration
        };
        
        try {
            const response = await fetch(`${this.config.apiBaseUrl}/v1/lights/${selector}/state`, {
                method: 'PUT',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify(payload)
            });
            const result = await response.json();
            this.state.colorTemperature = kelvin;
            this.triggerEvent('colorChanged', { kelvin, selector, results: result.results });
            return result;
        } catch (error) {
            console.error('Failed to set color temperature:', error);
            return null;
        }
    }

    async applyScene(sceneId, selector = 'all', duration = null) {
        const scene = this.scenes[sceneId];
        if (!scene) {
            console.error('Unknown scene:', sceneId);
            return null;
        }
        
        const payload = {
            color: `hue:${scene.h}`,
            brightness: scene.b / 100,
            kelvin: scene.k,
            duration: duration ?? this.config.defaultDuration
        };
        
        try {
            const response = await fetch(`${this.config.apiBaseUrl}/v1/lights/${selector}/state`, {
                method: 'PUT',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify(payload)
            });
            const result = await response.json();
            this.state.activeScene = sceneId;
            this.triggerEvent('sceneChanged', { scene: sceneId, selector, results: result.results });
            return result;
        } catch (error) {
            console.error('Failed to apply scene:', error);
            return null;
        }
    }

    async startEffect(effectName, selector = 'all', options = {}) {
        if (!this.effects[effectName]) {
            console.error('Unknown effect:', effectName);
            return false;
        }
        
        this.stopEffect();
        this.state.activeEffect = effectName;
        this.state.effectRunning = true;
        
        await this.effects[effectName](selector, options);
        this.triggerEvent('effectStarted', { effect: effectName, selector });
        return true;
    }

    stopEffect() {
        if (this.state.effectInterval) {
            clearInterval(this.state.effectInterval);
            this.state.effectInterval = null;
        }
        this.state.effectRunning = false;
        this.state.activeEffect = null;
        this.triggerEvent('effectStopped', { effect: this.state.activeEffect });
    }

    async pulseEffect(selector, options = {}) {
        const { color = 'red', duration = 0.5, cycles = 3 } = options;
        let count = 0;
        
        const originalState = await this.fetchBulbs(selector);
        
        return new Promise((resolve) => {
            this.state.effectInterval = setInterval(async () => {
                count++;
                if (count > cycles * 2) {
                    await this.setColor(color, selector, duration);
                    clearInterval(this.state.effectInterval);
                    this.state.effectRunning = false;
                    resolve();
                    return;
                }
                
                if (count % 2 === 1) {
                    await this.setColor(color, selector, duration);
                } else {
                    await this.setBrightness(0, selector, duration);
                }
            }, duration * 1000);
        });
    }

    async breatheEffect(selector, options = {}) {
        const { color = '#27a0b9', duration = 1, cycles = 5 } = options;
        let count = 0;
        
        return new Promise((resolve) => {
            const breathe = async () => {
                count++;
                if (count > cycles) {
                    clearInterval(this.state.effectInterval);
                    this.state.effectRunning = false;
                    resolve();
                    return;
                }
                
                for (let i = 0; i <= 100; i += 5) {
                    const brightness = 20 + (i / 100) * 80;
                    await this.setBrightness(brightness, selector, duration / 20);
                    await this.sleep(50);
                }
                for (let i = 100; i >= 0; i -= 5) {
                    const brightness = 20 + (i / 100) * 80;
                    await this.setBrightness(brightness, selector, duration / 20);
                    await this.sleep(50);
                }
            };
            
            this.state.effectInterval = setInterval(breathe, duration * 1000);
            breathe();
        });
    }

    async rainbowEffect(selector, options = {}) {
        const { duration = 2, cycles = 3, saturation = 100 } = options;
        let hue = 0;
        
        return new Promise((resolve) => {
            const totalSteps = (360 * cycles) / 5;
            let step = 0;
            
            this.state.effectInterval = setInterval(async () => {
                step++;
                if (step > totalSteps) {
                    clearInterval(this.state.effectInterval);
                    this.state.effectRunning = false;
                    resolve();
                    return;
                }
                
                hue = (hue + 5) % 360;
                await this.setColor(`hue:${hue * 182},saturation:${saturation / 100}`, selector, 0.1);
            }, (duration * 1000) / totalSteps);
        });
    }

    async fireEffect(selector, options = {}) {
        const { baseBrightness = 60, flickerIntensity = 30 } = options;
        
        return new Promise((resolve) => {
            this.state.effectInterval = setInterval(async () => {
                if (!this.state.effectRunning) {
                    resolve();
                    return;
                }
                
                const flicker = Math.random() * flickerIntensity;
                const brightness = baseBrightness + flicker - (flickerIntensity / 2);
                const hue = 5460 + Math.random() * 2000;
                
                await this.setColor(`hue:${hue},saturation:0.8`, selector, 0.1);
                await this.setBrightness(brightness, selector, 0.1);
            }, 100);
        });
    }

    async auroraEffect(selector, options = {}) {
        const { duration = 3 } = options;
        const colors = ['#00ff88', '#00d4ff', '#27a0b9', '#0088ff', '#8800ff'];
        let index = 0;
        
        return new Promise((resolve) => {
            this.state.effectInterval = setInterval(async () => {
                if (!this.state.effectRunning) {
                    resolve();
                    return;
                }
                
                const color = colors[index % colors.length];
                const nextColor = colors[(index + 1) % colors.length];
                
                await this.setColor(color, selector, duration / 2);
                await this.sleep(duration * 500);
                await this.setColor(nextColor, selector, duration / 2);
                
                index++;
            }, duration * 1000);
        });
    }

    async strobeEffect(selector, options = {}) {
        const { color = 'white', duration = 0.05, cycles = 10 } = options;
        let count = 0;
        
        return new Promise((resolve) => {
            this.state.effectInterval = setInterval(async () => {
                count++;
                if (count > cycles * 2) {
                    clearInterval(this.state.effectInterval);
                    this.state.effectRunning = false;
                    resolve();
                    return;
                }
                
                if (count % 2 === 1) {
                    await this.setColor(color, selector, duration);
                    await this.setBrightness(100, selector, duration);
                } else {
                    await this.setBrightness(0, selector, duration);
                }
            }, duration * 1000);
        });
    }

    async partyEffect(selector, options = {}) {
        const { minDuration = 0.2, maxDuration = 0.5 } = options;
        
        return new Promise((resolve) => {
            this.state.effectInterval = setInterval(async () => {
                if (!this.state.effectRunning) {
                    resolve();
                    return;
                }
                
                const hue = Math.random() * 360 * 182;
                const duration = minDuration + Math.random() * (maxDuration - minDuration);
                
                await this.setColor(`hue:${hue},saturation:1`, selector, duration);
                await this.setBrightness(80 + Math.random() * 20, selector, duration);
            }, 200);
        });
    }

    async candleEffect(selector, options = {}) {
        const { baseBrightness = 50, baseColor = 2200 } = options;
        
        return new Promise((resolve) => {
            this.state.effectInterval = setInterval(async () => {
                if (!this.state.effectRunning) {
                    resolve();
                    return;
                }
                
                const flicker = Math.sin(Date.now() / 500) * 10 + Math.random() * 15;
                const brightness = baseBrightness + flicker;
                const kelvin = baseColor + Math.random() * 200;
                
                await this.setBrightness(Math.max(30, Math.min(70, brightness)), selector, 0.2);
                await this.setColorTemperature(Math.round(kelvin), selector, 0.2);
            }, 200);
        });
    }

    async lightningEffect(selector, options = {}) {
        const { flashCount = 3, pauseBetween = 2000 } = options;
        let flashes = 0;
        
        return new Promise((resolve) => {
            const flash = async () => {
                flashes++;
                if (flashes > flashCount) {
                    flashes = 0;
                    await this.sleep(pauseBetween);
                }
                
                const flashDuration = 50 + Math.random() * 100;
                await this.setColor('#ffffff', selector, 0.05);
                await this.setBrightness(100, selector, 0.05);
                await this.sleep(flashDuration);
                await this.setBrightness(0, selector, 0.05);
            };
            
            this.state.effectInterval = setInterval(flash, 100);
        });
    }

    async sunriseEffect(selector, options = {}) {
        const { duration = 30000 } = options;
        const steps = 60;
        
        return new Promise((resolve) => {
            let step = 0;
            
            this.state.effectInterval = setInterval(async () => {
                step++;
                if (step > steps) {
                    clearInterval(this.state.effectInterval);
                    this.state.effectRunning = false;
                    resolve();
                    return;
                }
                
                const progress = step / steps;
                const brightness = progress * 80;
                const kelvin = 2000 + (progress * 3500);
                const hue = 9100 - (progress * 3000);
                
                await this.setColor(`hue:${Math.round(hue)},saturation:${0.3 * (1 - progress)}`, selector, 0.5);
                await this.setBrightness(brightness, selector, 0.5);
                await this.setColorTemperature(Math.round(kelvin), selector, 0.5);
            }, duration / steps);
        });
    }

    async sunsetEffect(selector, options = {}) {
        const { duration = 30000 } = options;
        const steps = 60;
        
        return new Promise((resolve) => {
            let step = 0;
            
            this.state.effectInterval = setInterval(async () => {
                step++;
                if (step > steps) {
                    clearInterval(this.state.effectInterval);
                    this.state.effectRunning = false;
                    resolve();
                    return;
                }
                
                const progress = step / steps;
                const brightness = 80 * (1 - progress * 0.7);
                const kelvin = 5500 - (progress * 3000);
                const hue = 9100 + (progress * 4000);
                
                await this.setColor(`hue:${Math.round(hue)},saturation:0.4`, selector, 0.5);
                await this.setBrightness(brightness, selector, 0.5);
                await this.setColorTemperature(Math.round(kelvin), selector, 0.5);
            }, duration / steps);
        });
    }

    async copEffect(selector, options = {}) {
        return new Promise((resolve) => {
            this.state.effectInterval = setInterval(async () => {
                if (!this.state.effectRunning) {
                    resolve();
                    return;
                }
                
                await this.setColor('red', selector, 0.1);
                await this.sleep(400);
                await this.setColor('blue', selector, 0.1);
                await this.sleep(400);
            }, 800);
        });
    }

    async emergencyEffect(selector, options = {}) {
        return new Promise((resolve) => {
            this.state.effectInterval = setInterval(async () => {
                if (!this.state.effectRunning) {
                    resolve();
                    return;
                }
                
                await this.setColor('yellow', selector, 0.1);
                await this.sleep(200);
                await this.setBrightness(0, selector, 0.1);
                await this.sleep(200);
                await this.setBrightness(100, selector, 0.1);
                await this.sleep(200);
                await this.setBrightness(0, selector, 0.1);
                await this.sleep(200);
            }, 800);
        });
    }

    async colorLoopEffect(selector, options = {}) {
        const { stepSize = 10, duration = 0.5 } = options;
        let hue = 0;
        
        return new Promise((resolve) => {
            this.state.effectInterval = setInterval(async () => {
                if (!this.state.effectRunning) {
                    resolve();
                    return;
                }
                
                hue = (hue + stepSize) % (360 * 182);
                await this.setColor(`hue:${hue},saturation:1`, selector, duration);
            }, duration * 1000);
        });
    }

    parseColor(color) {
        if (typeof color === 'string') {
            if (color.startsWith('hue:')) {
                const parts = color.split(',');
                const result = {};
                parts.forEach(part => {
                    const [key, value] = part.split(':');
                    result[key] = parseFloat(value);
                });
                return result;
            }
            if (color.startsWith('#')) {
                const hex = color.slice(1);
                const r = parseInt(hex.slice(0, 2), 16) / 255;
                const g = parseInt(hex.slice(2, 4), 16) / 255;
                const b = parseInt(hex.slice(4, 6), 16) / 255;
                return { r, g, b };
            }
        }
        return color;
    }

    sleep(ms) {
        return new Promise(resolve => setTimeout(resolve, ms));
    }

    selectBulb(bulbId) {
        if (this.state.selectedBulbs.has(bulbId)) {
            this.state.selectedBulbs.delete(bulbId);
        } else {
            this.state.selectedBulbs.add(bulbId);
        }
        return Array.from(this.state.selectedBulbs);
    }

    selectAllBulbs() {
        this.state.bulbs.forEach(bulb => this.state.selectedBulbs.add(bulb.id));
        return Array.from(this.state.selectedBulbs);
    }

    clearSelection() {
        this.state.selectedBulbs.clear();
        return [];
    }

    getSelectedBulbs() {
        return Array.from(this.state.selectedBulbs);
    }

    enableMediaSync() {
        this.state.mediaSyncActive = true;
        this.triggerEvent('mediaSyncToggled', { enabled: true });
    }

    disableMediaSync() {
        this.state.mediaSyncActive = false;
        this.stopEffect();
        this.triggerEvent('mediaSyncToggled', { enabled: false });
    }

    toggleMediaSync() {
        if (this.state.mediaSyncActive) {
            this.disableMediaSync();
        } else {
            this.enableMediaSync();
        }
        return this.state.mediaSyncActive;
    }

    syncWithBeat(beatData) {
        if (!this.state.mediaSyncActive) return;
        
        const { energy, bpm } = beatData;
        if (energy > 0.7) {
            this.setBrightness(80 + energy * 20, 'all', 0.05);
        }
    }

    on(event, callback) {
        if (this.eventCallbacks[event]) {
            this.eventCallbacks[event].push(callback);
        }
        return this;
    }

    off(event, callback) {
        if (this.eventCallbacks[event]) {
            this.eventCallbacks[event] = this.eventCallbacks[event].filter(cb => cb !== callback);
        }
        return this;
    }

    triggerEvent(event, data = {}) {
        const callbacks = this.eventCallbacks[event] || [];
        callbacks.forEach(cb => cb(data));
    }

    getState() {
        return { ...this.state };
    }

    getScenes() {
        return Object.keys(this.scenes).map(key => ({
            id: key,
            ...this.scenes[key]
        }));
    }

    getEffects() {
        return Object.keys(this.effects);
    }
}

export default LIFXControlsEnhanced;
