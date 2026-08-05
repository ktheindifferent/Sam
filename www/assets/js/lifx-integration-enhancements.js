/**
 * LIFX Integration Enhancements
 * Advanced lighting effects, scenes, and automation
 * Copyright 2021-2026 The Open Sam Foundation (OSF)
 */

(function() {
    'use strict';

    const LIFXEnhancements = {
        config: {
            enableAdvancedEffects: true,
            enableCircadianRhythm: true,
            enableMediaSync: true,
            enableVoiceControl: false,
            enablePresenceSimulation: false,
            defaultTransitionDuration: 0.5,
            maxSceneFavorites: 10,
            enableColorFlow: true,
            enableZoneControl: true,
            enableSchedules: true,
            enableGeofencing: false,
            enableWeatherSync: false,
            enableMusicVisualizer: true,
            beatDetectionSensitivity: 0.7,
            ambientLightSync: false,
            enableEnergySaving: false,
            autoOffTimeout: 0
        },

        state: {
            discoveredBulbs: [],
            activeScenes: [],
            favoriteScenes: [],
            activeEffects: [],
            circadianMode: false,
            mediaSyncActive: false,
            colorFlowActive: false,
            partyModeActive: false,
            breathingLightActive: false,
            currentKelvin: 3500,
            currentBrightness: 50,
            currentHue: 180,
            currentSaturation: 100,
            zoneStates: {},
            scheduledTasks: [],
            energyUsage: 0,
            lastActivityTime: Date.now(),
            presenceState: 'away',
            weatherCondition: 'clear',
            audioAnalyzer: null,
            beatHistory: [],
            colorFlowIndex: 0,
            effectInterval: null
        },

        scenes: {
            relax: { hue: 200, saturation: 50, kelvin: 2700, brightness: 40 },
            focus: { hue: 180, saturation: 30, kelvin: 5000, brightness: 80 },
            energize: { hue: 100, saturation: 80, kelvin: 6000, brightness: 100 },
            night: { hue: 240, saturation: 20, kelvin: 2000, brightness: 15 },
            sunset: { hue: 30, saturation: 70, kelvin: 2500, brightness: 50 },
            ocean: { hue: 200, saturation: 60, kelvin: 4000, brightness: 60 },
            reading: { hue: 180, saturation: 20, kelvin: 4500, brightness: 70 },
            romance: { hue: 330, saturation: 60, kelvin: 2200, brightness: 35 },
            party: { hue: 280, saturation: 90, kelvin: 4000, brightness: 100 },
            golden: { hue: 45, saturation: 65, kelvin: 3000, brightness: 65 },
            arctic: { hue: 190, saturation: 40, kelvin: 6500, brightness: 75 },
            tropical: { hue: 120, saturation: 75, kelvin: 3500, brightness: 80 },
            meditation: { hue: 220, saturation: 30, kelvin: 2700, brightness: 30 },
            gaming: { hue: 270, saturation: 85, kelvin: 4000, brightness: 90 },
            cooking: { hue: 180, saturation: 25, kelvin: 5500, brightness: 95 },
            creative: { hue: 310, saturation: 70, kelvin: 4200, brightness: 85 },
            yoga: { hue: 160, saturation: 35, kelvin: 3500, brightness: 50 },
            movie: { hue: 200, saturation: 40, kelvin: 2500, brightness: 25 },
            study: { hue: 190, saturation: 25, kelvin: 5200, brightness: 75 },
            dinner: { hue: 35, saturation: 55, kelvin: 2800, brightness: 55 },
            morning: { hue: 50, saturation: 45, kelvin: 4800, brightness: 70 },
            goodnight: { hue: 240, saturation: 15, kelvin: 1800, brightness: 10 },
            aurora: { hue: 140, saturation: 80, kelvin: 4000, brightness: 70 },
            nebula: { hue: 260, saturation: 75, kelvin: 3800, brightness: 65 },
            crystal: { hue: 180, saturation: 50, kelvin: 7000, brightness: 80 },
            cotton_candy: { hue: 320, saturation: 60, kelvin: 3500, brightness: 60 },
            spring_blossom: { hue: 340, saturation: 65, kelvin: 4000, brightness: 70 },
            punchbowl: { hue: 180, saturation: 85, kelvin: 4200, brightness: 90 },
            smashing: { hue: 200, saturation: 90, kelvin: 4500, brightness: 95 },
            glimmer: { hue: 280, saturation: 70, kelvin: 3800, brightness: 75 }
        },

        effects: {
            pulse: { name: 'Pulse', duration: 1000, type: 'brightness' },
            breathe: { name: 'Breathe', duration: 2000, type: 'brightness_smooth' },
            colorCycle: { name: 'Color Cycle', duration: 5000, type: 'hue' },
            flame: { name: 'Flame', duration: 150, type: 'flicker' },
            strobe: { name: 'Strobe', duration: 100, type: 'power' },
            police: { name: 'Police', duration: 500, type: 'alternating' },
            rainbow: { name: 'Rainbow', duration: 3000, type: 'rainbow' },
            wave: { name: 'Wave', duration: 800, type: 'wave' },
            sparkle: { name: 'Sparkle', duration: 200, type: 'sparkle' },
            thunder: { name: 'Thunder', duration: 2000, type: 'thunder' },
            fireplace: { name: 'Fireplace', duration: 100, type: 'fireplace' },
            candle: { name: 'Candle', duration: 150, type: 'candle' },
            sunrise: { name: 'Sunrise', duration: 300000, type: 'sunrise' },
            sunset: { name: 'Sunset', duration: 300000, type: 'sunset' },
            disco: { name: 'Disco', duration: 400, type: 'disco' },
            beatSync: { name: 'Beat Sync', duration: 0, type: 'beat' }
        },

        circadianSchedule: {
            morning: { time: '06:00', kelvin: 4800, brightness: 60, hue: 50 },
            midday: { time: '12:00', kelvin: 6500, brightness: 90, hue: 180 },
            afternoon: { time: '15:00', kelvin: 5500, brightness: 80, hue: 170 },
            evening: { time: '18:00', kelvin: 3500, brightness: 50, hue: 30 },
            night: { time: '21:00', kelvin: 2200, brightness: 20, hue: 240 },
            sleep: { time: '23:00', kelvin: 1800, brightness: 5, hue: 240 }
        },

        init() {
            this.loadSavedPreferences();
            this.setupWebSocketListener();
            this.initCircadianRhythm();
            this.initMediaSync();
            this.initScheduledTasks();
            this.setupVoiceControl();
            console.log('[LIFXEnhancements] Initialized');
        },

        setupWebSocketListener() {
            if (window.WebSocket) {
                const wsProtocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
                const isLocalHost = window.location.hostname === 'localhost' ||
                    window.location.hostname === '127.0.0.1' ||
                    window.location.hostname === '::1';
                const wsUrl = isLocalHost
                    ? `${wsProtocol}//${window.location.hostname}:8080/ws`
                    : `${wsProtocol}//${window.location.host}/ws`;
                try {
                    const ws = new WebSocket(wsUrl);
                    ws.onopen = () => {
                        console.log('[LIFXEnhancements] WebSocket connected');
                        this.subscribeToLIFXUpdates();
                    };
                    ws.onmessage = (event) => {
                        try {
                            const data = JSON.parse(event.data);
                            this.handleWebSocketMessage(data);
                        } catch (e) {
                            console.error('[LIFXEnhancements] Parse error:', e);
                        }
                    };
                    ws.onclose = () => {
                        console.log('[LIFXEnhancements] WebSocket disconnected');
                        setTimeout(() => this.setupWebSocketListener(), 3000);
                    };
                } catch (e) {
                    console.error('[LIFXEnhancements] WebSocket setup failed:', e);
                }
            }
        },

        subscribeToLIFXUpdates() {
            const ws = this.getWebSocket();
            if (ws && ws.readyState === WebSocket.OPEN) {
                ws.send(JSON.stringify({
                    type: 'subscribe',
                    channels: ['lifx', 'services']
                }));
            }
        },

        handleWebSocketMessage(data) {
            if (data.type === 'lifx_update') {
                this.updateBulbStates(data.bulbs);
            } else if (data.type === 'service_status' && data.service === 'lifx') {
                this.updateServiceStatus(data.status);
            }
        },

        updateBulbStates(bulbs) {
            this.state.discoveredBulbs = bulbs;
            this.notifyBulbUpdate();
        },

        updateServiceStatus(status) {
            console.log('[LIFXEnhancements] Service status:', status);
        },

        getWebSocket() {
            return window.samWebSocket || null;
        },

        notifyBulbUpdate() {
            const event = new CustomEvent('lifx_bulbs_updated', {
                detail: { bulbs: this.state.discoveredBulbs }
            });
            document.dispatchEvent(event);
        },

        applyScene(sceneName, selector = 'all', duration = this.config.defaultTransitionDuration) {
            const scene = this.scenes[sceneName];
            if (!scene) {
                console.error('[LIFXEnhancements] Unknown scene:', sceneName);
                return;
            }

            this.state.activeScenes.push(sceneName);
            this.sendLIFXCommand('set_state', {
                selector: selector,
                hue: scene.hue,
                saturation: scene.saturation,
                kelvin: scene.kelvin,
                brightness: scene.brightness / 100,
                duration: duration
            });

            this.showSceneNotification(sceneName);
        },

        applyCustomColor(hue, saturation, kelvin, brightness, selector = 'all', duration = this.config.defaultTransitionDuration) {
            this.state.currentHue = hue;
            this.state.currentSaturation = saturation;
            this.state.currentKelvin = kelvin;
            this.state.currentBrightness = brightness;

            this.sendLIFXCommand('set_state', {
                selector: selector,
                hue: hue,
                saturation: saturation,
                kelvin: kelvin,
                brightness: brightness / 100,
                duration: duration
            });
        },

        togglePower(selector = 'all') {
            this.sendLIFXCommand('toggle_power', { selector: selector });
        },

        powerOn(selector = 'all') {
            this.sendLIFXCommand('set_power', { selector: selector, power: 'on' });
        },

        powerOff(selector = 'all') {
            this.sendLIFXCommand('set_power', { selector: selector, power: 'off' });
        },

        adjustBrightness(delta, selector = 'all') {
            const newBrightness = Math.max(0, Math.min(100, this.state.currentBrightness + delta));
            this.state.currentBrightness = newBrightness;
            this.sendLIFXCommand('set_brightness', {
                selector: selector,
                brightness: newBrightness / 100,
                duration: this.config.defaultTransitionDuration
            });
        },

        setBrightness(level, selector = 'all') {
            this.state.currentBrightness = level;
            this.sendLIFXCommand('set_brightness', {
                selector: selector,
                brightness: level / 100,
                duration: this.config.defaultTransitionDuration
            });
        },

        adjustColorTemp(delta) {
            const newKelvin = Math.max(1500, Math.min(9000, this.state.currentKelvin + delta));
            this.state.currentKelvin = newKelvin;
            this.sendLIFXCommand('set_color', {
                selector: 'all',
                kelvin: newKelvin,
                duration: this.config.defaultTransitionDuration
            });
        },

        startEffect(effectName, selector = 'all') {
            const effect = this.effects[effectName];
            if (!effect) {
                console.error('[LIFXEnhancements] Unknown effect:', effectName);
                return;
            }

            this.state.activeEffects.push(effectName);

            switch (effect.type) {
                case 'brightness':
                    this.startPulseEffect(selector, effect.duration);
                    break;
                case 'brightness_smooth':
                    this.startBreatheEffect(selector, effect.duration);
                    break;
                case 'hue':
                    this.startColorCycleEffect(selector, effect.duration);
                    break;
                case 'flicker':
                    this.startFlameEffect(selector);
                    break;
                case 'rainbow':
                    this.startRainbowEffect(selector, effect.duration);
                    break;
                case 'beat':
                    this.startBeatSyncEffect(selector);
                    break;
                default:
                    console.log('[LIFXEnhancements] Effect type not implemented:', effect.type);
            }

            this.showEffectNotification(effectName);
        },

        stopEffect(effectName) {
            const index = this.state.activeEffects.indexOf(effectName);
            if (index > -1) {
                this.state.activeEffects.splice(index, 1);
            }

            if (this.state.effectInterval) {
                clearInterval(this.state.effectInterval);
                this.state.effectInterval = null;
            }
        },

        startPulseEffect(selector, duration) {
            let brightness = this.state.currentBrightness;
            let increasing = true;

            this.state.effectInterval = setInterval(() => {
                if (increasing) {
                    brightness = Math.min(100, brightness + 10);
                    if (brightness >= 100) increasing = false;
                } else {
                    brightness = Math.max(0, brightness - 10);
                    if (brightness <= 20) increasing = true;
                }

                this.setBrightness(brightness, selector);
            }, duration / 10);
        },

        startBreatheEffect(selector, duration) {
            let phase = 0;
            const baseBrightness = this.state.currentBrightness;

            this.state.effectInterval = setInterval(() => {
                phase += 0.1;
                const modulation = Math.sin(phase) * 30;
                this.setBrightness(baseBrightness + modulation, selector);
            }, duration / 20);
        },

        startColorCycleEffect(selector, duration) {
            let hue = this.state.currentHue;

            this.state.effectInterval = setInterval(() => {
                hue = (hue + 5) % 360;
                this.sendLIFXCommand('set_color', {
                    selector: selector,
                    hue: hue,
                    saturation: this.state.currentSaturation,
                    kelvin: this.state.currentKelvin,
                    brightness: this.state.currentBrightness / 100,
                    duration: duration / 1000
                });
            }, duration / 72);
        },

        startRainbowEffect(selector, duration) {
            const colors = [0, 60, 120, 180, 240, 300];
            let index = 0;

            this.state.effectInterval = setInterval(() => {
                this.sendLIFXCommand('set_color', {
                    selector: selector,
                    hue: colors[index],
                    saturation: 100,
                    kelvin: 4000,
                    brightness: 100,
                    duration: duration / 1000
                });
                index = (index + 1) % colors.length;
            }, duration / 6);
        },

        startFlameEffect(selector) {
            const flicker = () => {
                const brightness = 70 + Math.random() * 30;
                const kelvin = 1800 + Math.random() * 400;
                this.sendLIFXCommand('set_state', {
                    selector: selector,
                    brightness: brightness / 100,
                    kelvin: kelvin,
                    duration: 0.1
                });
            };

            this.state.effectInterval = setInterval(flicker, 150);
        },

        startBeatSyncEffect(selector) {
            if (!this.state.audioAnalyzer) {
                this.setupAudioAnalyzer();
            }

            const detectBeat = () => {
                if (this.state.audioAnalyzer) {
                    const energy = this.getAudioEnergy();
                    if (energy > this.config.beatDetectionSensitivity) {
                        this.flashOnBeat(selector);
                    }
                }
            };

            this.state.effectInterval = setInterval(detectBeat, 100);
        },

        setupAudioAnalyzer() {
            try {
                const audioContext = new (window.AudioContext || window.webkitAudioContext)();
                this.state.audioAnalyzer = audioContext.createAnalyser();
                this.state.audioAnalyzer.fftSize = 256;
            } catch (e) {
                console.error('[LIFXEnhancements] Audio analyzer setup failed:', e);
            }
        },

        getAudioEnergy() {
            if (!this.state.audioAnalyzer) return 0;
            const dataArray = new Uint8Array(this.state.audioAnalyzer.frequencyBinCount);
            this.state.audioAnalyzer.getByteFrequencyData(dataArray);
            const sum = dataArray.reduce((a, b) => a + b, 0);
            return (sum / dataArray.length) / 255;
        },

        flashOnBeat(selector) {
            const originalBrightness = this.state.currentBrightness;
            this.setBrightness(100, selector);
            setTimeout(() => {
                this.setBrightness(originalBrightness, selector);
            }, 100);
        },

        initCircadianRhythm() {
            if (!this.config.enableCircadianRhythm) return;

            const checkSchedule = () => {
                const now = new Date();
                const currentTime = `${String(now.getHours()).padStart(2, '0')}:${String(now.getMinutes()).padStart(2, '0')}`;

                for (const [key, schedule] of Object.entries(this.circadianSchedule)) {
                    if (currentTime === schedule.time) {
                        this.applyCircadianSetting(key, schedule);
                    }
                }
            };

            setInterval(checkSchedule, 60000);
            console.log('[LIFXEnhancements] Circadian rhythm monitoring started');
        },

        applyCircadianSetting(name, schedule) {
            this.sendLIFXCommand('set_state', {
                selector: 'all',
                hue: schedule.hue,
                kelvin: schedule.kelvin,
                brightness: schedule.brightness / 100,
                duration: 60
            });
            console.log(`[LIFXEnhancements] Applied circadian setting: ${name}`);
        },

        toggleCircadianMode() {
            this.state.circadianMode = !this.state.circadianMode;
            this.config.enableCircadianRhythm = this.state.circadianMode;
            if (this.state.circadianMode) {
                this.initCircadianRhythm();
            }
            return this.state.circadianMode;
        },

        initMediaSync() {
            if (!this.config.enableMediaSync) return;

            document.addEventListener('media_playback_start', () => {
                this.state.mediaSyncActive = true;
                this.startEffect('beatSync');
            });

            document.addEventListener('media_playback_stop', () => {
                this.state.mediaSyncActive = false;
                this.stopEffect('beatSync');
            });
        },

        toggleMediaSync() {
            this.config.enableMediaSync = !this.config.enableMediaSync;
            if (this.config.enableMediaSync) {
                this.initMediaSync();
            }
            return this.config.enableMediaSync;
        },

        initScheduledTasks() {
            if (!this.config.enableSchedules) return;

            const savedTasks = localStorage.getItem('lifx_scheduled_tasks');
            if (savedTasks) {
                try {
                    this.state.scheduledTasks = JSON.parse(savedTasks);
                    this.state.scheduledTasks.forEach(task => {
                        this.scheduleTask(task);
                    });
                } catch (e) {
                    console.error('[LIFXEnhancements] Failed to load scheduled tasks:', e);
                }
            }
        },

        scheduleTask(task) {
            const now = new Date();
            const taskTime = new Date();
            const [hours, minutes] = task.time.split(':');
            taskTime.setHours(parseInt(hours), parseInt(minutes), 0, 0);

            if (taskTime <= now) {
                taskTime.setDate(taskTime.getDate() + 1);
            }

            const delay = taskTime - now;
            const timeoutId = setTimeout(() => {
                this.executeScheduledTask(task);
                if (task.recurring) {
                    this.scheduleTask(task);
                }
            }, delay);

            task.timeoutId = timeoutId;
        },

        executeScheduledTask(task) {
            console.log('[LIFXEnhancements] Executing scheduled task:', task.name);
            if (task.action === 'scene') {
                this.applyScene(task.scene);
            } else if (task.action === 'power') {
                task.power === 'on' ? this.powerOn() : this.powerOff();
            } else if (task.action === 'brightness') {
                this.setBrightness(task.level);
            }

            const event = new CustomEvent('lifx_scheduled_task', { detail: { task: task } });
            document.dispatchEvent(event);
        },

        addScheduledTask(task) {
            this.state.scheduledTasks.push(task);
            localStorage.setItem('lifx_scheduled_tasks', JSON.stringify(this.state.scheduledTasks));
            this.scheduleTask(task);
        },

        removeScheduledTask(taskId) {
            const index = this.state.scheduledTasks.findIndex(t => t.id === taskId);
            if (index > -1) {
                const task = this.state.scheduledTasks[index];
                if (task.timeoutId) {
                    clearTimeout(task.timeoutId);
                }
                this.state.scheduledTasks.splice(index, 1);
                localStorage.setItem('lifx_scheduled_tasks', JSON.stringify(this.state.scheduledTasks));
            }
        },

        setupVoiceControl() {
            if (!this.config.enableVoiceControl || !('webkitSpeechRecognition' in window)) {
                return;
            }

            const recognition = new webkitSpeechRecognition();
            recognition.continuous = true;
            recognition.interimResults = false;

            recognition.onresult = (event) => {
                const transcript = event.results[event.results.length - 1][0].transcript.toLowerCase();
                this.processVoiceCommand(transcript);
            };

            recognition.start();
            console.log('[LIFXEnhancements] Voice control enabled');
        },

        processVoiceCommand(command) {
            const commands = {
                'lights on': () => this.powerOn(),
                'lights off': () => this.powerOff(),
                'brighter': () => this.adjustBrightness(10),
                'dimmer': () => this.adjustBrightness(-10),
                'maximum brightness': () => this.setBrightness(100),
                'minimum brightness': () => this.setBrightness(10),
                'warmer': () => this.adjustColorTemp(500),
                'cooler': () => this.adjustColorTemp(-500),
                'relax mode': () => this.applyScene('relax'),
                'focus mode': () => this.applyScene('focus'),
                'party mode': () => this.applyScene('party'),
                'night mode': () => this.applyScene('night'),
                'movie mode': () => this.applyScene('movie')
            };

            for (const [keyword, action] of Object.entries(commands)) {
                if (command.includes(keyword)) {
                    action();
                    console.log('[LIFXEnhancements] Voice command executed:', keyword);
                    return;
                }
            }
        },

        sendLIFXCommand(command, params) {
            const ws = this.getWebSocket();
            if (ws && ws.readyState === WebSocket.OPEN) {
                ws.send(JSON.stringify({
                    type: 'command',
                    command: command,
                    args: params
                }));
            } else {
                this.sendHTTPCommand(command, params);
            }
        },

        sendHTTPCommand(command, params) {
            fetch(`/api/services/lifx/${command}`, {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify(params)
            }).catch(e => console.error('[LIFXEnhancements] HTTP command failed:', e));
        },

        showSceneNotification(sceneName) {
            const event = new CustomEvent('lifx_scene_changed', { detail: { scene: sceneName } });
            document.dispatchEvent(event);
        },

        showEffectNotification(effectName) {
            const event = new CustomEvent('lifx_effect_started', { detail: { effect: effectName } });
            document.dispatchEvent(event);
        },

        addFavoriteScene(sceneName) {
            if (!this.state.favoriteScenes.includes(sceneName) &&
                this.state.favoriteScenes.length < this.config.maxSceneFavorites) {
                this.state.favoriteScenes.push(sceneName);
                localStorage.setItem('lifx_favorite_scenes', JSON.stringify(this.state.favoriteScenes));
            }
        },

        removeFavoriteScene(sceneName) {
            const index = this.state.favoriteScenes.indexOf(sceneName);
            if (index > -1) {
                this.state.favoriteScenes.splice(index, 1);
                localStorage.setItem('lifx_favorite_scenes', JSON.stringify(this.state.favoriteScenes));
            }
        },

        loadSavedPreferences() {
            const favorites = localStorage.getItem('lifx_favorite_scenes');
            if (favorites) {
                try {
                    this.state.favoriteScenes = JSON.parse(favorites);
                } catch (e) {
                    console.error('[LIFXEnhancements] Failed to load favorites:', e);
                }
            }
        },

        getZoneState(zoneId) {
            return this.state.zoneStates[zoneId] || { power: 'off', brightness: 50 };
        },

        setZoneState(zoneId, state) {
            this.state.zoneStates[zoneId] = state;
            this.sendLIFXCommand('set_zone_state', { zone: zoneId, ...state });
        },

        activatePartyMode() {
            this.state.partyModeActive = true;
            this.startEffect('disco');
            this.applyScene('party');
        },

        deactivatePartyMode() {
            this.state.partyModeActive = false;
            this.stopEffect('disco');
        },

        togglePartyMode() {
            if (this.state.partyModeActive) {
                this.deactivatePartyMode();
            } else {
                this.activatePartyMode();
            }
            return this.state.partyModeActive;
        },

        startBreathingLight() {
            this.state.breathingLightActive = true;
            this.startEffect('breathe');
        },

        stopBreathingLight() {
            this.state.breathingLightActive = false;
            this.stopEffect('breathe');
        },

        toggleBreathingLight() {
            if (this.state.breathingLightActive) {
                this.stopBreathingLight();
            } else {
                this.startBreathingLight();
            }
            return this.state.breathingLightActive;
        },

        getColorFlowState() {
            return {
                active: this.state.colorFlowActive,
                index: this.state.colorFlowIndex,
                direction: 'clockwise'
            };
        },

        startColorFlow() {
            this.state.colorFlowActive = true;
            this.startEffect('colorCycle');
        },

        stopColorFlow() {
            this.state.colorFlowActive = false;
            this.stopEffect('colorCycle');
        },

        setTransitionDuration(duration) {
            this.config.defaultTransitionDuration = duration;
        },

        getEnergyUsage() {
            return this.state.energyUsage;
        },

        updateEnergyUsage(bulbs) {
            let total = 0;
            bulbs.forEach(bulb => {
                if (bulb.power === 'on') {
                    total += (bulb.brightness * 0.15);
                }
            });
            this.state.energyUsage = total;
        }
    };

    LIFXEnhancements.init();
    window.LIFXEnhancements = LIFXEnhancements;
})();
