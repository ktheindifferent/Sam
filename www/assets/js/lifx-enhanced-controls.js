/**
 * SAM LIFX Enhanced Touch Controls
 * Advanced lighting control with scenes, effects, and automation
 */

(function() {
    'use strict';

    const LIFXEnhancedControls = {
        config: {
            pollingInterval: 5000,
            enableAnimations: true,
            enableTransitions: true,
            defaultTransitionDuration: 0.3,
            enableVoiceControl: true,
            enablePresenceSimulation: true,
            enableSunriseAlarm: true,
            enableSunsetDimming: true,
            enableNightLight: true,
            enableKidsMode: false,
            enablePartyStrobe: false,
            enableRelaxMode: true,
            enableFocusMode: true,
            enableReadingMode: true,
            enableMovieMode: true,
            enableGamingMode: true,
            enableCookingMode: false,
            enableBedtimeRoutine: true,
            enableMorningRoutine: true,
            enableAwayMode: false,
            enableWelcomeHome: true,
            enableGoodnightRoutine: true,
            enableWakeUpRoutine: true,
            maxBrightness: 100,
            minBrightness: 5,
            maxColorTemperature: 9000,
            minColorTemperature: 1500,
            enableSchedules: true,
            enableTimers: true,
            enableCountdown: true,
            enableRandomization: false,
            enableEnergySaving: false,
            enableAdaptiveLighting: true,
            enableCircadianRhythm: true,
            enableBiophilicLighting: true,
            enableHumanCentric: true,
            enableTunableWhite: true,
            enableFullColor: true,
            enableMultizone: true,
            enableTileEffects: true,
            enableBeamEffects: true,
            enableLumenEffects: true,
            enableCandleEffect: true,
            enableFireplaceEffect: true,
            enableRainbowEffect: true,
            enableColorCycleEffect: true,
            enableStrobeEffect: false,
            enablePoliceEffect: false,
            enableAmbulanceEffect: false,
            enableFiretruckEffect: false,
            enableDiscoEffect: false,
            enableRaveEffect: false,
            enableChaseEffect: true,
            enableWaveEffect: true,
            enablePulseEffect: true,
            enableBreathEffect: true,
            enableFadeEffect: true,
            enableBlinkEffect: true,
            enableFlashEffect: true,
            enableAlertEffect: true,
            enableNotificationLights: true,
            enableDoorbellSync: true,
            enablePhoneCallAlert: true,
            enableMessageAlert: true,
            enableEmailAlert: false,
            enableCalendarAlert: true,
            enableWeatherAlert: true,
            enableSecurityAlert: true,
            enableSmokeAlarmSync: true,
            enableMotionSync: true,
            enableDoorSync: true,
            enableWindowSync: true,
            enableCameraSync: true,
            enableAlarmSync: true
        },

        state: {
            lights: [],
            groups: [],
            scenes: [],
            effects: [],
            schedules: [],
            rooms: [],
            zones: [],
            favorites: [],
            recentColors: [],
            customScenes: [],
            activeEffect: null,
            isAnimating: false,
            brightnessLevel: 100,
            colorTemperature: 4000,
            currentHue: 0,
            currentSaturation: 0,
            connectionStatus: 'disconnected',
            lastSyncTime: 0,
            presenceSimulated: false,
            sunriseActive: false,
            sunsetActive: false,
            nightLightActive: false,
            kidsModeActive: false,
            partyModeActive: false,
            relaxModeActive: false,
            focusModeActive: false,
            readingModeActive: false,
            movieModeActive: false,
            gamingModeActive: false,
            cookingModeActive: false,
            bedtimeActive: false,
            morningActive: false,
            awayModeActive: false,
            welcomeHomeActive: false,
            goodnightActive: false,
            wakeUpActive: false,
            energySavingActive: false,
            adaptiveLightingActive: true,
            circadianActive: false,
            biophilicActive: false,
            humanCentricActive: false,
            candleEffectActive: false,
            fireplaceEffectActive: false,
            rainbowEffectActive: false,
            colorCycleActive: false,
            chaseActive: false,
            waveActive: false,
            pulseActive: false,
            breathActive: false,
            fadeActive: false,
            selectedLights: [],
            multiSelectMode: false,
            editMode: false,
            onboardingComplete: false
        },

        colorPresets: [
            { name: 'Pure White', hex: '#FFFFFF', hsb: [0, 0, 100], kelvin: 5500 },
            { name: 'Warm White', hex: '#F5E6D3', hsb: [45, 15, 96], kelvin: 2700 },
            { name: 'Cool White', hex: '#E8F4F8', hsb: [195, 10, 97], kelvin: 6500 },
            { name: 'Daylight', hex: '#F0F8FF', hsb: [210, 5, 100], kelvin: 5000 },
            { name: 'Soft White', hex: '#FFF8DC', hsb: [45, 13, 100], kelvin: 3000 },
            { name: 'Bright White', hex: '#FFFFFF', hsb: [0, 0, 100], kelvin: 4000 },
            { name: 'Tungsten', hex: '#FFE4C4', hsb: [30, 23, 100], kelvin: 2800 },
            { name: 'Fluorescent', hex: '#F0FFFF', hsb: [180, 6, 100], kelvin: 4200 },
            { name: 'Halogen', hex: '#FFF5E6', hsb: [30, 10, 100], kelvin: 3200 },
            { name: 'Incandescent', hex: '#FFEFD5', hsb: [35, 17, 100], kelvin: 2500 },
            { name: 'LED Cool', hex: '#E0FFFF', hsb: [180, 12, 100], kelvin: 6000 },
            { name: 'LED Warm', hex: '#FFF0E0', hsb: [30, 12, 100], kelvin: 3000 },
            { name: 'Sunrise', hex: '#FF7F50', hsb: [15, 69, 100], kelvin: 2000 },
            { name: 'Sunset', hex: '#FF6347', hsb: [10, 72, 100], kelvin: 2200 },
            { name: 'Noon', hex: '#FFFACD', hsb: [50, 20, 100], kelvin: 5500 },
            { name: 'Midnight', hex: '#191970', hsb: [240, 78, 44], kelvin: 4000 },
            { name: 'Golden Hour', hex: '#FFD700', hsb: [50, 100, 100], kelvin: 3000 },
            { name: 'Blue Hour', hex: '#4169E1', hsb: [225, 73, 88], kelvin: 8000 },
            { name: 'Twilight', hex: '#6495ED', hsb: [215, 58, 93], kelvin: 7000 },
            { name: 'Dawn', hex: '#87CEEB', hsb: [200, 44, 92], kelvin: 6000 },
            { name: 'Dusk', hex: '#483D8B', hsb: [248, 56, 55], kelvin: 5000 },
            { name: 'Moonlight', hex: '#F0F8FF', hsb: [210, 6, 100], kelvin: 4000 },
            { name: 'Starlight', hex: '#E6E6FA', hsb: [240, 10, 98], kelvin: 5000 },
            { name: 'Candlelight', hex: '#FFE5B4', hsb: [35, 29, 100], kelvin: 1800 },
            { name: 'Firelight', hex: '#FF4500', hsb: [15, 100, 100], kelvin: 2000 },
            { name: 'Campfire', hex: '#FF6347', hsb: [10, 72, 100], kelvin: 2200 },
            { name: 'Bonfire', hex: '#FF4500', hsb: [15, 100, 100], kelvin: 2000 },
            { name: 'Ember', hex: '#DC143C', hsb: [355, 73, 86], kelvin: 1800 },
            { name: 'Lava', hex: '#CF1020', hsb: [355, 84, 81], kelvin: 1500 },
            { name: 'Magma', hex: '#B22222', hsb: [0, 81, 70], kelvin: 1600 }
        ],

        moodPresets: [
            { name: 'Relax', icon: '🧘', hue: 5800, saturation: 15000, brightness: 40, kelvin: 2700, description: 'Calm and soothing' },
            { name: 'Focus', icon: '🎯', hue: 19000, saturation: 8000, brightness: 80, kelvin: 5000, description: 'Enhance concentration' },
            { name: 'Energize', icon: '⚡', hue: 41000, saturation: 20000, brightness: 100, kelvin: 6500, description: 'Boost your energy' },
            { name: 'Night', icon: '🌙', hue: 5800, saturation: 10000, brightness: 20, kelvin: 2000, description: 'Gentle night light' },
            { name: 'Reading', icon: '📚', hue: 19000, saturation: 5000, brightness: 70, kelvin: 4500, description: 'Perfect for books' },
            { name: 'Romance', icon: '💕', hue: 60000, saturation: 25000, brightness: 50, kelvin: 3000, description: 'Romantic ambiance' },
            { name: 'Party', icon: '🎉', hue: 43680, saturation: 65535, brightness: 100, kelvin: 5500, description: 'Party time!' },
            { name: 'Sunset', icon: '🌅', hue: 7098, saturation: 40000, brightness: 60, kelvin: 2500, description: 'Warm sunset glow' },
            { name: 'Arctic', icon: '❄️', hue: 32760, saturation: 15000, brightness: 80, kelvin: 7000, description: 'Cool arctic light' },
            { name: 'Golden', icon: '☀️', hue: 8000, saturation: 30000, brightness: 70, kelvin: 3200, description: 'Golden sunshine' },
            { name: 'Ocean', icon: '🌊', hue: 34580, saturation: 42598, brightness: 75, kelvin: 4000, description: 'Ocean breeze' },
            { name: 'Tropical', icon: '🏝️', hue: 27300, saturation: 65535, brightness: 72, kelvin: 3800, description: 'Tropical paradise' },
            { name: 'Meditation', icon: '🧘', hue: 50960, saturation: 19660, brightness: 35, kelvin: 2400, description: 'Deep meditation' },
            { name: 'Gaming', icon: '🎮', hue: 50960, saturation: 52428, brightness: 90, kelvin: 5500, description: 'Game on!' },
            { name: 'Movie', icon: '🎬', hue: 3640, saturation: 19660, brightness: 35, kelvin: 2200, description: 'Cinema experience' },
            { name: 'Morning', icon: '🌄', hue: 9100, saturation: 32767, brightness: 85, kelvin: 5500, description: 'Fresh morning' },
            { name: 'Goodnight', icon: '😴', hue: 43680, saturation: 6553, brightness: 10, kelvin: 2000, description: 'Sleep tight' },
            { name: 'Rainbow', icon: '🌈', hue: 0, saturation: 65535, brightness: 80, kelvin: 4000, description: 'All the colors' },
            { name: 'Fireplace', icon: '🔥', hue: 5460, saturation: 52428, brightness: 60, kelvin: 2000, description: 'Cozy fire' },
            { name: 'Ice', icon: '🧊', hue: 36400, saturation: 32767, brightness: 70, kelvin: 8000, description: 'Cool as ice' },
            { name: 'Aurora', icon: '🌌', hue: 32760, saturation: 45875, brightness: 75, kelvin: 6000, description: 'Northern lights' },
            { name: 'Nebula', icon: '🌠', hue: 50960, saturation: 52428, brightness: 70, kelvin: 4500, description: 'Cosmic wonder' },
            { name: 'Thunder', icon: '⛈️', hue: 5460, saturation: 39321, brightness: 90, kelvin: 5000, description: 'Stormy weather' },
            { name: 'Crystal', icon: '💎', hue: 34580, saturation: 26214, brightness: 80, kelvin: 7500, description: 'Crystal clear' },
            { name: 'Cyberpunk', icon: '🤖', hue: 30940, saturation: 52428, brightness: 90, kelvin: 4500, description: 'Future city' },
            { name: 'Vaporwave', icon: '🌴', hue: 58240, saturation: 39321, brightness: 80, kelvin: 4000, description: 'Aesthetic vibes' },
            { name: 'Halloween', icon: '🎃', hue: 5460, saturation: 52428, brightness: 75, kelvin: 2800, description: 'Spooky season' },
            { name: 'Christmas', icon: '🎄', hue: 5800, saturation: 45875, brightness: 85, kelvin: 3500, description: 'Holiday cheer' },
            { name: 'Beach', icon: '🏖️', hue: 18200, saturation: 32767, brightness: 80, kelvin: 5000, description: 'Beach day' },
            { name: 'Forest', icon: '🌲', hue: 25480, saturation: 39321, brightness: 65, kelvin: 4200, description: 'Forest walk' },
            { name: 'Yoga', icon: '🧘', hue: 25480, saturation: 26214, brightness: 60, kelvin: 3800, description: 'Find your zen' },
            { name: 'Cooking', icon: '🍳', hue: 5460, saturation: 32767, brightness: 90, kelvin: 4500, description: 'Chef mode' },
            { name: 'Creative', icon: '🎨', hue: 58240, saturation: 45875, brightness: 80, kelvin: 5000, description: 'Get creative' },
            { name: 'Dinner', icon: '🍽️', hue: 6000, saturation: 26214, brightness: 50, kelvin: 3000, description: 'Dinner party' },
            { name: 'Spa', icon: '💆', hue: 32760, saturation: 19660, brightness: 40, kelvin: 3500, description: 'Spa day' },
            { name: 'Festival', icon: '🎪', hue: 27300, saturation: 58982, brightness: 100, kelvin: 4200, description: 'Festival vibes' },
            { name: 'Zen', icon: '☯️', hue: 10920, saturation: 13107, brightness: 55, kelvin: 3500, description: 'Inner peace' },
            { name: 'Serenity', icon: '🕊️', hue: 20020, saturation: 19660, brightness: 65, kelvin: 4000, description: 'Peaceful calm' },
            { name: 'Inspire', icon: '💡', hue: 54600, saturation: 32767, brightness: 75, kelvin: 5000, description: 'Get inspired' },
            { name: 'Dream', icon: '💭', hue: 49140, saturation: 26214, brightness: 50, kelvin: 3800, description: 'Sweet dreams' },
            { name: 'Hope', icon: '🌟', hue: 5800, saturation: 39321, brightness: 70, kelvin: 4200, description: 'Shine bright' },
            { name: 'Joy', icon: '😊', hue: 8190, saturation: 45875, brightness: 85, kelvin: 4500, description: 'Pure joy' },
            { name: 'Love', icon: '❤️', hue: 65520, saturation: 52428, brightness: 60, kelvin: 3000, description: 'Spread love' },
            { name: 'Peace', icon: '☮️', hue: 21840, saturation: 19660, brightness: 55, kelvin: 3600, description: 'World peace' },
            { name: 'Gratitude', icon: '🙏', hue: 5460, saturation: 26214, brightness: 65, kelvin: 3400, description: 'Be thankful' },
            { name: 'Confidence', icon: '💪', hue: 32760, saturation: 45875, brightness: 80, kelvin: 5000, description: 'Feel confident' },
            { name: 'Motivation', icon: '🚀', hue: 16380, saturation: 52428, brightness: 90, kelvin: 5500, description: 'Get motivated' },
            { name: 'Celebration', icon: '🥳', hue: 32760, saturation: 58982, brightness: 95, kelvin: 4800, description: 'Celebrate!' },
            { name: 'Comfort', icon: '🛋️', hue: 5460, saturation: 19660, brightness: 45, kelvin: 2800, description: 'Get comfy' },
            { name: 'Cozy', icon: '🧣', hue: 6552, saturation: 26214, brightness: 50, kelvin: 3000, description: 'Cozy vibes' }
        ],

        effectPresets: [
            { name: 'Candle Flicker', type: 'candle', duration: 0, description: 'Realistic candle flickering' },
            { name: 'Fireplace', type: 'fireplace', duration: 0, description: 'Warm fire simulation' },
            { name: 'Rainbow Cycle', type: 'rainbow', duration: 10, description: 'Smooth rainbow transition' },
            { name: 'Color Pulse', type: 'pulse', duration: 2, description: 'Pulsing color effect' },
            { name: 'Breathing', type: 'breath', duration: 3, description: 'Gentle breathing effect' },
            { name: 'Color Chase', type: 'chase', duration: 1, description: 'Colors chasing around' },
            { name: 'Ocean Wave', type: 'wave', duration: 2, description: 'Wave-like motion' },
            { name: 'Fade In/Out', type: 'fade', duration: 4, description: 'Smooth fade effect' },
            { name: 'Strobe', type: 'strobe', duration: 0.1, description: 'Fast strobe effect', warning: true },
            { name: 'Police', type: 'police', duration: 0.2, description: 'Police light pattern', warning: true },
            { name: 'Ambulance', type: 'ambulance', duration: 0.3, description: 'Ambulance pattern', warning: true },
            { name: 'Firetruck', type: 'firetruck', duration: 0.3, description: 'Firetruck pattern', warning: true },
            { name: 'Disco', type: 'disco', duration: 0.5, description: 'Disco party mode', warning: true },
            { name: 'Random Colors', type: 'random', duration: 3, description: 'Random color changes' },
            { name: 'Sunrise', type: 'sunrise', duration: 300, description: 'Gradual sunrise simulation' },
            { name: 'Sunset', type: 'sunset', duration: 300, description: 'Gradual sunset simulation' },
            { name: 'Aurora', type: 'aurora', duration: 5, description: 'Northern lights effect' },
            { name: 'Nebula', type: 'nebula', duration: 8, description: 'Cosmic nebula effect' },
            { name: 'Lightning', type: 'lightning', duration: 0, description: 'Random lightning flashes' },
            { name: 'TV Simulator', type: 'tv', duration: 0, description: 'Simulate TV flicker' },
            { name: 'Occupancy', type: 'occupancy', duration: 0, description: 'Simulate presence' },
            { name: 'Alarm', type: 'alarm', duration: 0.5, description: 'Alert notification' },
            { name: 'Doorbell', type: 'doorbell', duration: 1, description: 'Doorbell chime' },
            { name: 'Phone Call', type: 'phone', duration: 0.5, description: 'Incoming call alert' },
            { name: 'Message', type: 'message', duration: 0.3, description: 'New message alert' },
            { name: 'Email', type: 'email', duration: 0.3, description: 'Email notification' },
            { name: 'Calendar', type: 'calendar', duration: 1, description: 'Calendar reminder' },
            { name: 'Weather', type: 'weather', duration: 2, description: 'Weather alert' },
            { name: 'Security', type: 'security', duration: 0.2, description: 'Security alert' },
            { name: 'Motion', type: 'motion', duration: 0.5, description: 'Motion detected' },
            { name: 'Door Open', type: 'door', duration: 0.5, description: 'Door opened' },
            { name: 'Window Open', type: 'window', duration: 0.5, description: 'Window opened' }
        ],

        routines: {
            morning: {
                name: 'Good Morning',
                time: '07:00',
                actions: [
                    { delay: 0, brightness: 10, color: { hue: 5800, saturation: 10000, kelvin: 2000 } },
                    { delay: 60, brightness: 30, color: { hue: 9100, saturation: 20000, kelvin: 3500 } },
                    { delay: 120, brightness: 60, color: { hue: 18200, saturation: 25000, kelvin: 4500 } },
                    { delay: 180, brightness: 80, color: { hue: 21840, saturation: 15000, kelvin: 5500 } }
                ]
            },
            bedtime: {
                name: 'Bedtime',
                time: '22:00',
                actions: [
                    { delay: 0, brightness: 50, color: { hue: 5460, saturation: 20000, kelvin: 3000 } },
                    { delay: 60, brightness: 30, color: { hue: 5800, saturation: 15000, kelvin: 2700 } },
                    { delay: 120, brightness: 15, color: { hue: 6000, saturation: 10000, kelvin: 2400 } },
                    { delay: 180, brightness: 5, color: { hue: 5800, saturation: 5000, kelvin: 2000 } }
                ]
            },
            away: {
                name: 'Away Mode',
                trigger: 'away',
                actions: [
                    { delay: 0, random: true, brightness: { min: 30, max: 70 }, duration: 300 }
                ]
            },
            welcomeHome: {
                name: 'Welcome Home',
                trigger: 'home',
                actions: [
                    { delay: 0, brightness: 70, color: { hue: 18200, saturation: 20000, kelvin: 4000 } }
                ]
            },
            goodnight: {
                name: 'Goodnight',
                trigger: 'goodnight',
                actions: [
                    { delay: 0, brightness: 0 }
                ]
            }
        },

        init() {
            this.setupLightDiscovery();
            this.setupColorPicker();
            this.setupSceneSelector();
            this.setupEffectControls();
            this.setupRoomManagement();
            this.setupGroupControls();
            this.setupScheduleManager();
            this.setupRoutineManager();
            this.setupBrightnessControls();
            this.setupTemperatureControls();
            this.setupMultiSelect();
            this.setupTouchGestures();
            this.setupVoiceCommands();
            this.setupPresenceSimulation();
            this.setupEnergyMonitoring();
            this.setupNotificationSync();
            this.setupAutomationRules();
            this.setupQuickActions();
            this.setupFavorites();
            this.setupRecentColors();
            this.setupCustomScenes();
            this.setupOnboarding();
            this.setupSettings();
            this.setupHelpTips();
            console.log('[LIFXEnhancedControls] Initialized');
        },

        setupLightDiscovery() {
            const refreshBtn = document.getElementById('lifx-refresh-lights');
            if (refreshBtn) {
                refreshBtn.addEventListener('click', () => this.discoverLights());
            }
            
            this.discoverLights();
        },

        async discoverLights() {
            try {
                const response = await fetch('/api/services/lifx/list', {
                    method: 'GET',
                    headers: { 'Content-Type': 'application/json' }
                });
                
                if (response.ok) {
                    const data = await response.json();
                    this.state.lights = data.lights || [];
                    this.state.groups = data.groups || [];
                    this.renderLightsList();
                    this.state.connectionStatus = 'connected';
                    this.showToast(`Found ${this.state.lights.length} lights`, 'success');
                } else {
                    this.state.connectionStatus = 'error';
                    this.showToast('Failed to discover lights', 'error');
                }
            } catch (error) {
                console.error('[LIFXEnhancedControls] Discovery error:', error);
                this.state.connectionStatus = 'disconnected';
                this.showToast('LIFX service unavailable', 'error');
            }
        },

        renderLightsList() {
            const container = document.getElementById('lifx-lights-list');
            if (!container) return;
            
            container.innerHTML = '';
            
            if (this.state.lights.length === 0) {
                container.innerHTML = `
                    <div class="lifx-empty-state">
                        <div class="lifx-empty-icon">💡</div>
                        <p>No lights found</p>
                        <small>Make sure your LIFX lights are powered on</small>
                    </div>
                `;
                return;
            }
            
            this.state.lights.forEach(light => {
                const lightCard = document.createElement('div');
                lightCard.className = `lifx-light-card ${light.power ? 'on' : 'off'}`;
                lightCard.dataset.id = light.id;
                lightCard.innerHTML = `
                    <div class="light-toggle" onclick="LIFXEnhancedControls.toggleLight('${light.id}')">
                        <div class="toggle-indicator ${light.power ? 'active' : ''}"></div>
                    </div>
                    <div class="light-info">
                        <h4>${light.label || 'Unnamed Light'}</h4>
                        <small>${light.product_name || 'LIFX Light'}</small>
                    </div>
                    <div class="light-controls">
                        <button class="light-settings-btn" onclick="LIFXEnhancedControls.openLightSettings('${light.id}')">
                            ⚙️
                        </button>
                    </div>
                    <div class="light-preview" style="background: hsl(${light.color.hue / 65535 * 360}, ${light.color.saturation / 65535 * 100}%, ${light.color.brightness / 65535 * 100}%)"></div>
                `;
                container.appendChild(lightCard);
            });
        },

        toggleLight(lightId) {
            const light = this.state.lights.find(l => l.id === lightId);
            if (!light) return;
            
            fetch('/api/services/lifx/toggle', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ selector: lightId })
            }).then(() => {
                light.power = !light.power;
                this.renderLightsList();
            });
        },

        openLightSettings(lightId) {
            const light = this.state.lights.find(l => l.id === lightId);
            if (!light) return;
            
            this.state.selectedLights = [lightId];
            this.showLightControlPanel(light);
        },

        showLightControlPanel(light) {
            const panel = document.getElementById('lifx-control-panel');
            if (!panel) return;
            
            panel.classList.add('visible');
            panel.innerHTML = `
                <div class="panel-header">
                    <h3>${light.label || 'Light Settings'}</h3>
                    <button class="close-btn" onclick="LIFXEnhancedControls.closeControlPanel()">✕</button>
                </div>
                <div class="panel-content">
                    <div class="color-picker-section">
                        <div id="lifx-color-wheel"></div>
                    </div>
                    <div class="sliders-section">
                        <div class="slider-group">
                            <label>Brightness</label>
                            <input type="range" min="0" max="100" value="${light.color.brightness / 655.35}" 
                                oninput="LIFXEnhancedControls.updateBrightness(this.value)">
                        </div>
                        <div class="slider-group">
                            <label>Temperature</label>
                            <input type="range" min="1500" max="9000" value="${light.color.kelvin}" 
                                oninput="LIFXEnhancedControls.updateTemperature(this.value)">
                        </div>
                    </div>
                    <div class="preset-section">
                        <h4>Quick Presets</h4>
                        <div class="preset-grid">
                            ${this.moodPresets.slice(0, 12).map(preset => `
                                <button class="preset-btn" onclick="LIFXEnhancedControls.applyPreset('${preset.name}')">
                                    ${preset.icon} ${preset.name}
                                </button>
                            `).join('')}
                        </div>
                    </div>
                </div>
            `;
        },

        closeControlPanel() {
            const panel = document.getElementById('lifx-control-panel');
            if (panel) panel.classList.remove('visible');
        },

        setupColorPicker() {
            const colorInput = document.getElementById('lifx-color-input');
            if (colorInput) {
                colorInput.addEventListener('input', (e) => {
                    this.setColorFromHex(e.target.value);
                });
            }
        },

        setColorFromHex(hex) {
            const rgb = parseInt(hex.slice(1), 16);
            const r = (rgb >> 16) & 0xff;
            const g = (rgb >> 8) & 0xff;
            const b = rgb & 0xff;
            
            const max = Math.max(r, g, b) / 255;
            const min = Math.min(r, g, b) / 255;
            const delta = max - min;
            
            let hue = 0;
            if (delta !== 0) {
                switch (max) {
                    case r: hue = ((g - b) / delta + (g < b ? 6 : 0)) * 60; break;
                    case g: hue = ((b - r) / delta + 2) * 60; break;
                    case b: hue = ((r - g) / delta + 4) * 60; break;
                }
            }
            
            const saturation = max === 0 ? 0 : (delta / max) * 100;
            const brightness = max * 100;
            
            this.setLightColor(hue * 65535 / 360, saturation * 655.35, brightness * 655.35);
        },

        setupSceneSelector() {
            const container = document.getElementById('lifx-scenes-grid');
            if (!container) return;
            
            container.innerHTML = `
                <div class="scene-category">
                    <h4>Mood Scenes</h4>
                    <div class="scene-scroll">
                        ${this.moodPresets.map(scene => `
                            <button class="scene-card" onclick="LIFXEnhancedControls.applyScene('${scene.name}')">
                                <span class="scene-icon">${scene.icon}</span>
                                <span class="scene-name">${scene.name}</span>
                            </button>
                        `).join('')}
                    </div>
                </div>
            `;
        },

        applyScene(sceneName) {
            const scene = this.moodPresets.find(s => s.name === sceneName);
            if (!scene) return;
            
            this.setLightColor(scene.hue, scene.saturation, scene.brightness * 655.35, scene.kelvin);
            this.showToast(`Applied ${scene.name} scene`, 'success');
        },

        setupEffectControls() {
            const container = document.getElementById('lifx-effects-list');
            if (!container) return;
            
            container.innerHTML = `
                ${this.effectPresets.map(effect => `
                    <div class="effect-item ${effect.warning ? 'warning' : ''}">
                        <div class="effect-info">
                            <span class="effect-name">${effect.name}</span>
                            <small>${effect.description}</small>
                        </div>
                        <button class="effect-btn" onclick="LIFXEnhancedControls.startEffect('${effect.type}')">
                            ${this.state.activeEffect === effect.type ? 'Stop' : 'Start'}
                        </button>
                    </div>
                `).join('')}
            `;
        },

        startEffect(effectType) {
            if (this.state.activeEffect === effectType) {
                this.stopEffect();
                return;
            }
            
            fetch('/api/services/lifx/effect', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ 
                    effect: effectType,
                    selector: 'all'
                })
            }).then(() => {
                this.state.activeEffect = effectType;
                this.showToast(`Started ${effectType} effect`, 'success');
                this.renderEffectsList();
            });
        },

        stopEffect() {
            fetch('/api/services/lifx/effect', {
                method: 'DELETE',
                headers: { 'Content-Type': 'application/json' }
            }).then(() => {
                this.state.activeEffect = null;
                this.showToast('Effect stopped', 'info');
                this.renderEffectsList();
            });
        },

        renderEffectsList() {
            const buttons = document.querySelectorAll('.effect-btn');
            buttons.forEach(btn => {
                const effectName = btn.parentElement.dataset.effect;
                btn.textContent = this.state.activeEffect === effectName ? 'Stop' : 'Start';
            });
        },

        setupRoomManagement() {
            this.state.rooms = JSON.parse(localStorage.getItem('lifxRooms') || '[]');
        },

        setupGroupControls() {
            const groupSelect = document.getElementById('lifx-group-select');
            if (groupSelect) {
                groupSelect.addEventListener('change', (e) => {
                    this.selectGroup(e.target.value);
                });
            }
        },

        selectGroup(groupId) {
            if (groupId === 'all') {
                this.state.selectedLights = this.state.lights.map(l => l.id);
            } else {
                const group = this.state.groups.find(g => g.id === groupId);
                this.state.selectedLights = group?.lights || [];
            }
            this.showToast(`Selected ${this.state.selectedLights.length} lights`, 'info');
        },

        setupScheduleManager() {
            this.state.schedules = JSON.parse(localStorage.getItem('lifxSchedules') || '[]');
        },

        setupRoutineManager() {
            const routineSelect = document.getElementById('lifx-routine-select');
            if (routineSelect) {
                routineSelect.innerHTML = Object.entries(this.routines).map(([key, routine]) => `
                    <option value="${key}">${routine.name}</option>
                `).join('');
                
                routineSelect.addEventListener('change', (e) => {
                    this.runRoutine(e.target.value);
                });
            }
        },

        runRoutine(routineKey) {
            const routine = this.routines[routineKey];
            if (!routine) return;
            
            this.showToast(`Running ${routine.name} routine`, 'info');
            
            routine.actions.forEach((action, index) => {
                setTimeout(() => {
                    if (action.color) {
                        this.setLightColor(
                            action.color.hue || 0,
                            action.color.saturation || 0,
                            action.color.brightness * 655.35,
                            action.color.kelvin
                        );
                    }
                    if (action.brightness !== undefined) {
                        this.setBrightness(action.brightness);
                    }
                }, action.delay * 1000);
            });
        },

        setupBrightnessControls() {
            const brightnessSlider = document.getElementById('lifx-brightness-master');
            if (brightnessSlider) {
                brightnessSlider.addEventListener('input', (e) => {
                    this.setGlobalBrightness(e.target.value);
                });
            }
        },

        setGlobalBrightness(value) {
            this.state.brightnessLevel = parseInt(value);
            
            fetch('/api/services/lifx/set_state', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({
                    selector: 'all',
                    brightness: value / 100,
                    duration: this.config.defaultTransitionDuration
                })
            });
        },

        setBrightness(percent) {
            fetch('/api/services/lifx/set_state', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({
                    selector: 'all',
                    brightness: percent / 100,
                    duration: this.config.defaultTransitionDuration
                })
            });
        },

        setupTemperatureControls() {
            const tempSlider = document.getElementById('lifx-temperature-slider');
            if (tempSlider) {
                tempSlider.addEventListener('input', (e) => {
                    this.setColorTemperature(e.target.value);
                });
            }
        },

        setColorTemperature(kelvin) {
            this.state.colorTemperature = parseInt(kelvin);
            
            fetch('/api/services/lifx/set_state', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({
                    selector: 'all',
                    kelvin: parseInt(kelvin),
                    duration: this.config.defaultTransitionDuration
                })
            });
        },

        updateBrightness(value) {
            const lightId = this.state.selectedLights[0];
            if (!lightId) return;
            
            fetch('/api/services/lifx/set_state', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({
                    selector: lightId,
                    brightness: value / 100,
                    duration: 0.3
                })
            });
        },

        updateTemperature(value) {
            const lightId = this.state.selectedLights[0];
            if (!lightId) return;
            
            fetch('/api/services/lifx/set_state', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({
                    selector: lightId,
                    kelvin: parseInt(value),
                    duration: 0.3
                })
            });
        },

        setupMultiSelect() {
            const multiSelectBtn = document.getElementById('lifx-multi-select-toggle');
            if (multiSelectBtn) {
                multiSelectBtn.addEventListener('click', () => {
                    this.state.multiSelectMode = !this.state.multiSelectMode;
                    multiSelectBtn.classList.toggle('active', this.state.multiSelectMode);
                    this.renderLightsList();
                });
            }
        },

        setupTouchGestures() {
            const lightContainer = document.getElementById('lifx-lights-list');
            if (!lightContainer) return;
            
            let touchStartX = 0;
            let touchStartY = 0;
            
            lightContainer.addEventListener('touchstart', (e) => {
                touchStartX = e.touches[0].clientX;
                touchStartY = e.touches[0].clientY;
            }, { passive: true });
            
            lightContainer.addEventListener('touchend', (e) => {
                const touchEndX = e.changedTouches[0].clientX;
                const touchEndY = e.changedTouches[0].clientY;
                
                const deltaX = touchEndX - touchStartX;
                const deltaY = touchEndY - touchStartY;
                
                if (Math.abs(deltaX) > Math.abs(deltaY) && Math.abs(deltaX) > 50) {
                    if (deltaX > 0) {
                        this.increaseBrightness();
                    } else {
                        this.decreaseBrightness();
                    }
                }
            }, { passive: true });
        },

        increaseBrightness() {
            const newBrightness = Math.min(100, this.state.brightnessLevel + 10);
            this.setGlobalBrightness(newBrightness);
        },

        decreaseBrightness() {
            const newBrightness = Math.max(0, this.state.brightnessLevel - 10);
            this.setGlobalBrightness(newBrightness);
        },

        setupVoiceCommands() {
            if (!this.config.enableVoiceControl || !('webkitSpeechRecognition' in window)) return;
            
            const recognition = new webkitSpeechRecognition();
            recognition.continuous = false;
            recognition.interimResults = false;
            
            recognition.onresult = (event) => {
                const command = event.results[0][0].transcript.toLowerCase();
                this.processVoiceCommand(command);
            };
            
            const voiceBtn = document.getElementById('lifx-voice-btn');
            if (voiceBtn) {
                voiceBtn.addEventListener('click', () => {
                    recognition.start();
                    this.showToast('Listening...', 'info');
                });
            }
        },

        processVoiceCommand(command) {
            if (command.includes('on') || command.includes('turn on')) {
                this.setLightColor(null, null, 65535);
                this.showToast('Lights on', 'success');
            } else if (command.includes('off') || command.includes('turn off')) {
                this.setLightColor(null, null, 0);
                this.showToast('Lights off', 'info');
            } else if (command.includes('brighter')) {
                this.increaseBrightness();
            } else if (command.includes('dimmer') || command.includes('darker')) {
                this.decreaseBrightness();
            } else if (command.includes('relax') || command.includes('relaxing')) {
                this.applyPreset('Relax');
            } else if (command.includes('focus') || command.includes('concentrate')) {
                this.applyPreset('Focus');
            } else if (command.includes('party')) {
                this.applyPreset('Party');
            } else if (command.includes('movie')) {
                this.applyPreset('Movie');
            } else if (command.includes('read') || command.includes('reading')) {
                this.applyPreset('Reading');
            } else if (command.includes('sleep') || command.includes('night')) {
                this.applyPreset('Goodnight');
            } else {
                this.showToast('Command not recognized', 'warning');
            }
        },

        applyPreset(presetName) {
            const preset = this.moodPresets.find(p => p.name === presetName);
            if (preset) {
                this.setLightColor(preset.hue, preset.saturation, preset.brightness * 655.35, preset.kelvin);
                this.showToast(`Applied ${presetName} preset`, 'success');
            }
        },

        setupPresenceSimulation() {
            if (!this.config.enablePresenceSimulation) return;
        },

        setupEnergyMonitoring() {
            const energyDisplay = document.getElementById('lifx-energy-display');
            if (energyDisplay) {
                this.updateEnergyDisplay(energyDisplay);
            }
        },

        updateEnergyDisplay(display) {
            const totalWatts = this.state.lights.reduce((sum, light) => {
                return sum + (light.power ? (light.watts || 11) : 0);
            }, 0);
            
            display.innerHTML = `
                <div class="energy-stat">
                    <span class="energy-value">${totalWatts}W</span>
                    <span class="energy-label">Current Usage</span>
                </div>
            `;
        },

        setupNotificationSync() {
            if (!this.config.enableNotificationLights) return;
            
            if ('Notification' in window) {
                Notification.requestPermission().then(permission => {
                    if (permission === 'granted') {
                        this.listenForNotifications();
                    }
                });
            }
        },

        listenForNotifications() {
            const originalNotification = window.Notification;
            window.Notification = function(title, options) {
                LIFXEnhancedControls.triggerNotificationAlert();
                return new originalNotification(title, options);
            };
        },

        triggerNotificationAlert() {
            if (!this.config.enableNotificationLights) return;
            
            fetch('/api/services/lifx/set_state', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({
                    selector: 'all',
                    color: 'rgb(0,255,255)',
                    brightness: 1,
                    duration: 0.5
                })
            });
            
            setTimeout(() => {
                fetch('/api/services/lifx/set_state', {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify({
                        selector: 'all',
                        brightness: this.state.brightnessLevel / 100,
                        duration: 0.5
                    })
                });
            }, 1000);
        },

        setupAutomationRules() {
            this.state.automationRules = JSON.parse(localStorage.getItem('lifxAutomation') || '[]');
        },

        setupQuickActions() {
            const quickActionsContainer = document.getElementById('lifx-quick-actions');
            if (quickActionsContainer) {
                quickActionsContainer.innerHTML = `
                    <button class="quick-action-btn" onclick="LIFXEnhancedControls.allOn()">
                        <span>💡</span> All On
                    </button>
                    <button class="quick-action-btn" onclick="LIFXEnhancedControls.allOff()">
                        <span>🌑</span> All Off
                    </button>
                    <button class="quick-action-btn" onclick="LIFXEnhancedControls.fullBright()">
                        <span>☀️</span> Full Bright
                    </button>
                    <button class="quick-action-btn" onclick="LIFXEnhancedControls.halfBright()">
                        <span>🌙</span> Half Bright
                    </button>
                    <button class="quick-action-btn" onclick="LIFXEnhancedControls.warmWhite()">
                        <span>🔶</span> Warm White
                    </button>
                    <button class="quick-action-btn" onclick="LIFXEnhancedControls.coolWhite()">
                        <span>🔷</span> Cool White
                    </button>
                `;
            }
        },

        allOn() {
            this.setLightColor(null, null, 65535);
            this.showToast('All lights on', 'success');
        },

        allOff() {
            this.setLightColor(null, null, 0);
            this.showToast('All lights off', 'info');
        },

        fullBright() {
            this.setGlobalBrightness(100);
            this.showToast('Full brightness', 'success');
        },

        halfBright() {
            this.setGlobalBrightness(50);
            this.showToast('Half brightness', 'info');
        },

        warmWhite() {
            this.setLightColor(0, 0, 65535, 2700);
            this.showToast('Warm white', 'success');
        },

        coolWhite() {
            this.setLightColor(0, 0, 65535, 6500);
            this.showToast('Cool white', 'success');
        },

        setupFavorites() {
            this.state.favorites = JSON.parse(localStorage.getItem('lifxFavorites') || '[]');
        },

        setupRecentColors() {
            this.state.recentColors = JSON.parse(localStorage.getItem('lifxRecentColors') || '[]');
        },

        setupCustomScenes() {
            this.state.customScenes = JSON.parse(localStorage.getItem('lifxCustomScenes') || '[]');
        },

        setupOnboarding() {
            this.state.onboardingComplete = localStorage.getItem('lifxOnboardingComplete') === 'true';
            
            if (!this.state.onboardingComplete) {
                this.showOnboarding();
            }
        },

        showOnboarding() {
            const onboarding = document.createElement('div');
            onboarding.className = 'lifx-onboarding';
            onboarding.innerHTML = `
                <div class="onboarding-content">
                    <h2>Welcome to LIFX Control</h2>
                    <p>Control your smart lights with ease</p>
                    <div class="onboarding-steps">
                        <div class="step">
                            <span class="step-icon">💡</span>
                            <span>Discover your lights automatically</span>
                        </div>
                        <div class="step">
                            <span class="step-icon">🎨</span>
                            <span>Choose from preset scenes or create custom colors</span>
                        </div>
                        <div class="step">
                            <span class="step-icon">⚡</span>
                            <span>Apply effects and automate your lighting</span>
                        </div>
                    </div>
                    <button class="onboarding-complete-btn" onclick="LIFXEnhancedControls.completeOnboarding()">
                        Get Started
                    </button>
                </div>
            `;
            document.body.appendChild(onboarding);
        },

        completeOnboarding() {
            localStorage.setItem('lifxOnboardingComplete', 'true');
            this.state.onboardingComplete = true;
            document.querySelector('.lifx-onboarding')?.remove();
            this.discoverLights();
        },

        setupSettings() {
            const settingsBtn = document.getElementById('lifx-settings-btn');
            if (settingsBtn) {
                settingsBtn.addEventListener('click', () => {
                    this.showSettings();
                });
            }
        },

        showSettings() {
            const settings = document.createElement('div');
            settings.className = 'lifx-settings-modal';
            settings.innerHTML = `
                <div class="settings-content">
                    <h3>LIFX Settings</h3>
                    <div class="setting-item">
                        <label>Transition Duration</label>
                        <input type="range" min="0.1" max="5" step="0.1" value="${this.config.defaultTransitionDuration}"
                            onchange="LIFXEnhancedControls.config.defaultTransitionDuration = parseFloat(this.value)">
                    </div>
                    <div class="setting-item">
                        <label>Enable Animations</label>
                        <input type="checkbox" ${this.config.enableAnimations ? 'checked' : ''}
                            onchange="LIFXEnhancedControls.config.enableAnimations = this.checked">
                    </div>
                    <div class="setting-item">
                        <label>Enable Voice Control</label>
                        <input type="checkbox" ${this.config.enableVoiceControl ? 'checked' : ''}
                            onchange="LIFXEnhancedControls.config.enableVoiceControl = this.checked">
                    </div>
                    <button class="settings-close" onclick="this.closest('.lifx-settings-modal').remove()">Close</button>
                </div>
            `;
            document.body.appendChild(settings);
        },

        setupHelpTips() {
            const helpBtn = document.getElementById('lifx-help-btn');
            if (helpBtn) {
                helpBtn.addEventListener('click', () => {
                    this.showHelpTips();
                });
            }
        },

        showHelpTips() {
            const tips = [
                'Swipe left/right on lights to adjust brightness',
                'Long press a light to select multiple',
                'Use voice commands for hands-free control',
                'Create custom scenes for your favorite colors',
                'Schedule lights to turn on/off automatically'
            ];
            
            const helpModal = document.createElement('div');
            helpModal.className = 'lifx-help-modal';
            helpModal.innerHTML = `
                <div class="help-content">
                    <h3>Help & Tips</h3>
                    <ul>
                        ${tips.map(tip => `<li>${tip}</li>`).join('')}
                    </ul>
                    <button class="help-close" onclick="this.closest('.lifx-help-modal').remove()">Got it!</button>
                </div>
            `;
            document.body.appendChild(helpModal);
        },

        setLightColor(hue, saturation, brightness, kelvin) {
            const body = { selector: 'all' };
            
            if (hue !== null) body.hue = hue;
            if (saturation !== null) body.saturation = saturation;
            if (brightness !== null) body.brightness = brightness / 65535;
            if (kelvin !== null) body.kelvin = kelvin;
            
            body.duration = this.config.defaultTransitionDuration;
            
            fetch('/api/services/lifx/set_state', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify(body)
            });
        },

        showToast(message, type = 'info') {
            const toast = document.createElement('div');
            toast.className = `lifx-toast lifx-toast-${type}`;
            toast.textContent = message;
            toast.style.cssText = `
                position: fixed;
                bottom: 20px;
                right: 20px;
                background: rgba(30, 30, 45, 0.95);
                border: 1px solid rgba(39, 160, 185, 0.3);
                border-radius: 8px;
                padding: 12px 20px;
                color: #fff;
                z-index: 9999;
                animation: lifx-toast-slide 0.3s ease;
            `;
            document.body.appendChild(toast);
            
            setTimeout(() => {
                toast.style.animation = 'lifx-toast-fade 0.3s ease forwards';
                setTimeout(() => toast.remove(), 300);
            }, 3000);
        }
    };

    window.LIFXEnhancedControls = LIFXEnhancedControls;

    document.addEventListener('DOMContentLoaded', () => {
        LIFXEnhancedControls.init();
    });
})();
