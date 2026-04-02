/**
 * SAM Touch Interface & Media Center Enhancements
 * Enhanced touch feedback, gesture controls, and media visualizations
 */

const TouchMediaEnhancements = (function() {
    'use strict';

    let bpmIndicator = null;
    let sceneIndicator = null;
    let bedtimeIndicator = null;
    let selectionToolbar = null;
    let gestureTrails = [];
    let mediaVisualizerBars = [];
    let mediaSyncActive = false;
    let mediaSyncMode = 'beat';
    let currentBpm = 0;
    let audioContext = null;
    let analyser = null;
    let mediaPlaybackState = {
        isPlaying: false,
        trackName: '',
        artistName: '',
        progress: 0,
        duration: 0
    };
    
    // Configuration
    const CONFIG = {
        gestureTrailCount: 5,
        bpmUpdateInterval: 100,
        sceneIndicatorDuration: 3000,
        touchSensitivity: 'medium',
        enableRipple: true,
        enableGestureTrails: true,
        enableBpmDisplay: true,
        enableKeyboardShortcuts: true,
        enableEdgeSwipe: true,
        enableThreeFingerSwipe: true,
        swipeThreshold: 50,
        pinchThreshold: 0.5,
        doubleTapDelay: 300,
        longPressDelay: 500,
        hapticFeedback: true,
        enableDoubleTap: true,
        enableLongPress: true,
        enableVoiceControl: false,
        animationsEnabled: true,
        lowPowerMode: false,
        enableCircularColorPicker: true,
        enableQuickColorPresets: true,
        enableBrightnessSlider: true,
        enableColorTemperatureSlider: true,
        enableMultiBulbSync: true,
        enableMediaVisualizer: true,
        enableBeatDetection: true,
        enableAmbientMode: true,
        enableSmoothTransitions: true,
        transitionDuration: 300,
        colorPickerSize: 200,
        maxRecentColors: 10,
        enableColorHistory: true,
        enableSceneFavorites: true,
        enableScheduledScenes: true,
        enableWakeUpScene: true,
        enableSleepScene: true,
        enablePartyMode: true,
        enableFocusMode: true,
        enableRelaxMode: true,
        enableReadingMode: true,
        enableMovieMode: true,
        enableGamingMode: true,
        enableNightLight: true,
        enableSimulations: ['sunrise', 'sunset', 'storm', 'fireplace', 'ocean', 'forest', 'aurora'],
        enableEffects: ['rainbow', 'pulse', 'fade', 'strobe', 'flash'],
        enableMusicSync: true,
        enableMicrophoneInput: true,
        enableSystemAudioInput: true,
        enableBassBoost: true,
        enableTrebleBoost: false,
        enableMidRange: true,
        enableFrequencySplit: true,
        enableZoneControl: true,
        enableGroupControl: true,
        enableLocationControl: true,
        enableAllBulbsControl: true,
        enableIndividualControl: true,
        enablePresetControl: true,
        enableCustomScene: true,
        enableDynamicScene: true,
        enableInteractiveScene: true,
        enableVoiceScene: false,
        enableGestureScene: true,
        enableTouchScene: true,
        enableMotionScene: false,
        enablePresenceScene: false,
        enableTimeScene: true,
        enableWeatherScene: false,
        enableSeasonalScene: true,
        enableHolidayScene: true,
        enableBirthdayScene: true,
        enablePartyScene: true,
        enableRomanceScene: true,
        enableDinnerScene: true,
        enableCookingScene: true,
        enableCleaningScene: true,
        enableExerciseScene: true,
        enableYogaScene: true,
        enableMeditationScene: true,
        enableSpaScene: true,
        enableBathScene: true,
        enableShowerScene: true,
        enableBedroomScene: true,
        enableLivingRoomScene: true,
        enableKitchenScene: true,
        enableBathroomScene: true,
        enableOfficeScene: true,
        enableGarageScene: true,
        enablePatioScene: true,
        enableGardenScene: true,
        enablePoolScene: true,
        enableDrivewayScene: true,
        enableFrontDoorScene: true,
        enableBackDoorScene: true,
        enableWindowScene: true,
        enableDoorScene: true,
        enableHallwayScene: true,
        enableStairwayScene: true,
        enableAtticScene: true,
        enableBasementScene: true,
        enableClosetScene: true,
        enableLaundryScene: true,
        enablePantryScene: true,
        enableDiningScene: true,
        enableBreakfastScene: true,
        enableLunchScene: true,
        enableSnackScene: true,
        enableDrinkScene: true,
        enableCoffeeScene: true,
        enableTeaScene: true,
        enableWineScene: true,
        enableBeerScene: true,
        enableCocktailScene: true,
        enableFadeIn: true,
        enableFadeOut: true,
        enablePulse: true,
        enableStrobe: true,
        enableFlash: true,
        enableBlink: true,
        enableDim: true,
        enableBrighten: true,
        enableVibrant: true,
        enableMuted: true,
        enablePastel: true,
        enableNeon: true,
        enableGlow: true,
        enableShadow: true,
        enableHighlight: true,
        enableAccent: true,
        enableAmbient: true,
        enableTask: true,
        enableMood: true,
        enableDecorative: true,
        enableFunctional: true,
        enableEmergency: true,
        enableSecurity: true,
        enableSafety: true,
        enableComfort: true,
        enableConvenience: true,
        enableEfficiency: true,
        enableSustainability: true,
        enableEco: true,
        enableColorPresets: ['red', 'blue', 'green', 'yellow', 'orange', 'purple', 'pink', 'cyan', 'magenta', 'lime', 'teal', 'navy', 'maroon', 'olive', 'brown', 'gray', 'white', 'black', 'silver', 'gold', 'bronze', 'copper', 'brass', 'chrome'],
        enableFinishPresets: ['metallic', 'matte', 'glossy', 'satin', 'textured'],
        enablePatternPresets: ['patterned', 'striped', 'dotted', 'checkered', 'plaid', 'floral', 'geometric', 'abstract'],
        enableStylePresets: ['modern', 'contemporary', 'traditional', 'rustic', 'industrial', 'minimalist', 'maximalist', 'eclectic', 'bohemian', 'scandinavian'],
        enableThemePresets: ['japanese', 'chinese', 'indian', 'mexican', 'mediterranean', 'tropical', 'coastal', 'mountain', 'desert', 'urban', 'rural', 'suburban'],
        enableEraPresets: ['classic', 'vintage', 'retro', 'futuristic', 'sciFi', 'fantasy'],
        enableMoodPresets: ['magical', 'mysterious', 'romantic', 'dramatic', 'calm', 'energizing', 'uplifting', 'soothing', 'relaxing', 'invigorating', 'refreshing', 'revitalizing', 'rejuvenating'],
        enableWellnessPresets: ['healing', 'therapeutic', 'wellness', 'healthy'],
        enableSafetyPresets: ['safe', 'secure', 'protected', 'comfortable', 'cozy'],
        enableEcoPresets: ['fresh', 'clean', 'pure', 'natural', 'organic', 'sustainable', 'renewable', 'recyclable', 'biodegradable', 'compostable', 'zeroWaste', 'carbonNeutral', 'climatePositive', 'environmentallyFriendly', 'ecoFriendly'],
        enableEffectPresets: ['pulse', 'fade', 'strobe', 'flash', 'blink', 'dim', 'brighten'],
        enableTransitionEffects: ['fadeIn', 'fadeOut']
    };

    // Initialize all enhancements
    function init() {
        setupTouchFeedback();
        setupGestureTrails();
        createBpmIndicator();
        createSceneIndicator();
        createBedtimeIndicator();
        createSelectionToolbar();
        setupMediaVisualizer();
        setupSwipeGestures();
        setupMultiSelectGestures();
        setupPinchZoom();
        setupEdgeSwipe();
        setupKeyboardShortcuts();
        setupHapticFeedback();
        setupDoubleTap();
        setupLongPress();
        setupMiniPlayer();
        setupNowPlayingToast();
        console.log('[TouchMediaEnhancements] Initialized');
    }

    // Enhanced touch feedback with velocity-sensitive ripple effect
    function setupTouchFeedback() {
        let lastTouchX = 0, lastTouchY = 0, lastTouchTime = 0;
        
        document.addEventListener('click', function(e) {
            if (!CONFIG.enableRipple) return;
            
            const target = e.target.closest('.touch-feedback, button, .btn, [role="button"]');
            if (!target) return;
            
            const rect = target.getBoundingClientRect();
            const size = Math.max(rect.width, rect.height);
            
            // Calculate touch velocity for ripple intensity
            const currentTime = Date.now();
            const timeDelta = currentTime - lastTouchTime;
            const velocity = timeDelta > 0 ? Math.sqrt(
                Math.pow(e.clientX - lastTouchX, 2) + 
                Math.pow(e.clientY - lastTouchY, 2)
            ) / timeDelta : 0;
            
            const ripple = document.createElement('span');
            ripple.classList.add('ripple');
            ripple.classList.add('ripple-enhanced');
            
            // Set ripple color based on velocity
            const hue = Math.min(360, 180 + velocity * 100);
            ripple.style.setProperty('--ripple-hue', hue);
            ripple.style.setProperty('--ripple-opacity', Math.min(0.8, 0.3 + velocity * 0.5));
            
            ripple.style.width = ripple.style.height = size + 'px';
            ripple.style.left = (e.clientX - rect.left - size / 2) + 'px';
            ripple.style.top = (e.clientY - rect.top - size / 2) + 'px';
            
            target.appendChild(ripple);
            
            setTimeout(() => ripple.remove(), 600);
            
            lastTouchX = e.clientX;
            lastTouchY = e.clientY;
            lastTouchTime = currentTime;
        });
    }

    // Enhanced gesture trail effect with particle system
    function setupGestureTrails() {
        if (!CONFIG.enableGestureTrails) return;
        
        let trailTimeout;
        let lastX = 0, lastY = 0;
        let moveSpeed = 0;
        
        document.addEventListener('mousemove', function(e) {
            clearTimeout(trailTimeout);
            
            // Calculate movement speed for dynamic trail effects
            const deltaX = e.clientX - lastX;
            const deltaY = e.clientY - lastY;
            moveSpeed = Math.sqrt(deltaX * deltaX + deltaY * deltaY);
            
            // Create primary trail
            const trail = document.createElement('div');
            trail.classList.add('gesture-trail');
            trail.classList.add('gesture-trail-enhanced');
            
            // Dynamic sizing and opacity based on speed
            const trailSize = Math.min(30, 10 + moveSpeed * 0.5);
            const opacity = Math.min(0.8, 0.3 + moveSpeed * 0.02);
            const hue = (Date.now() / 50) % 360;
            
            trail.style.cssText = `
                left: ${e.clientX - trailSize/2}px;
                top: ${e.clientY - trailSize/2}px;
                width: ${trailSize}px;
                height: ${trailSize}px;
                opacity: ${opacity};
                background: radial-gradient(circle, hsla(${hue}, 80%, 60%, 0.8), transparent);
                box-shadow: 0 0 ${trailSize/2}px hsla(${hue}, 80%, 60%, 0.6);
            `;
            
            document.body.appendChild(trail);
            gestureTrails.push(trail);
            
            // Create particle burst on fast movements
            if (moveSpeed > 20 && CONFIG.animationsEnabled) {
                createGestureParticles(e.clientX, e.clientY, moveSpeed);
            }
            
            setTimeout(() => {
                trail.style.transition = 'all 0.3s ease-out';
                trail.style.opacity = '0';
                trail.style.transform = 'scale(0.5)';
                setTimeout(() => {
                    trail.remove();
                    gestureTrails = gestureTrails.filter(t => t !== trail);
                }, 300);
            }, 400);
            
            trailTimeout = setTimeout(() => {
                gestureTrails.forEach(t => {
                    t.style.transition = 'all 0.3s ease-out';
                    t.style.opacity = '0';
                    setTimeout(() => t.remove(), 300);
                });
                gestureTrails = [];
            }, 500);
            
            lastX = e.clientX;
            lastY = e.clientY;
        });
    }
    
    // Create particle burst effect for fast gestures
    function createGestureParticles(x, y, speed) {
        const particleCount = Math.min(8, Math.floor(speed / 5));
        const hue = (Date.now() / 50) % 360;
        
        for (let i = 0; i < particleCount; i++) {
            const particle = document.createElement('div');
            particle.classList.add('gesture-particle');
            
            const angle = (Math.PI * 2 / particleCount) * i;
            const velocity = speed * 0.3;
            const dx = Math.cos(angle) * velocity;
            const dy = Math.sin(angle) * velocity;
            
            particle.style.cssText = `
                position: fixed;
                left: ${x}px;
                top: ${y}px;
                width: ${Math.min(8, 3 + speed * 0.1)}px;
                height: ${Math.min(8, 3 + speed * 0.1)}px;
                background: hsla(${hue}, 80%, 60%, 0.8);
                border-radius: 50%;
                pointer-events: none;
                z-index: 9998;
                animation: gesture-particle-anim 0.6s ease-out forwards;
                --particle-dx: ${dx}px;
                --particle-dy: ${dy}px;
            `;
            
            document.body.appendChild(particle);
            setTimeout(() => particle.remove(), 600);
        }
    }

    // BPM Real-time Indicator with enhanced visual feedback
    function createBpmIndicator() {
        if (!CONFIG.enableBpmDisplay) return;
        
        bpmIndicator = document.createElement('div');
        bpmIndicator.className = 'bpm-realtime-indicator';
        bpmIndicator.innerHTML = `
            <div class="bpm-ring"></div>
            <i class="bpm-icon fas fa-heartbeat"></i>
            <div>
                <div class="bpm-value">--</div>
                <div class="bpm-label">BPM</div>
            </div>
            <div class="bpm-bars"></div>
        `;
        document.body.appendChild(bpmIndicator);
        
        // Create visualizer bars for BPM
        const barsContainer = bpmIndicator.querySelector('.bpm-bars');
        for (let i = 0; i < 5; i++) {
            const bar = document.createElement('div');
            bar.className = 'bpm-bar';
            bar.style.animationDelay = `${i * 0.1}s`;
            barsContainer.appendChild(bar);
        }
    }

    function updateBpm(bpm) {
        if (!bpmIndicator) return;
        
        const bpmValue = bpmIndicator.querySelector('.bpm-value');
        const bpmRing = bpmIndicator.querySelector('.bpm-ring');
        const bpmBars = bpmIndicator.querySelectorAll('.bpm-bar');
        
        bpmIndicator.classList.add('visible');
        bpmValue.textContent = bpm || '--';
        
        // Animate ring based on BPM
        if (bpm) {
            const bpmInt = parseInt(bpm);
            const animationDuration = 60 / bpmInt;
            bpmRing.style.animationDuration = `${animationDuration}s`;
            bpmRing.classList.add('pulsing');
            
            // Animate bars with varying heights based on BPM intensity
            bpmBars.forEach((bar, i) => {
                const height = 20 + Math.random() * (bpmInt / 2);
                bar.style.height = `${height}px`;
                bar.classList.add('active');
            });
        } else {
            bpmRing.classList.remove('pulsing');
            bpmBars.forEach(bar => bar.classList.remove('active'));
            setTimeout(() => bpmIndicator.classList.remove('visible'), 2000);
        }
    }

    // Scene Change Indicator with enhanced animations
    function createSceneIndicator() {
        sceneIndicator = document.createElement('div');
        sceneIndicator.className = 'scene-indicator';
        sceneIndicator.innerHTML = `
            <div class="scene-icon-wrapper">
                <i class="scene-icon fas fa-palette"></i>
            </div>
            <div class="scene-name"></div>
            <div class="scene-preview"></div>
        `;
        document.body.appendChild(sceneIndicator);
    }

    function showSceneChange(sceneName, sceneColor = null) {
        if (!sceneIndicator) return;
        
        const iconWrapper = sceneIndicator.querySelector('.scene-icon-wrapper');
        const nameEl = sceneIndicator.querySelector('.scene-name');
        const previewEl = sceneIndicator.querySelector('.scene-preview');
        
        // Set scene icon based on scene type
        const sceneIcons = {
            'relax': 'fa-spa',
            'focus': 'fa-bullseye',
            'energize': 'fa-bolt',
            'night': 'fa-moon',
            'sunset': 'fa-cloud-sun',
            'ocean': 'fa-water',
            'reading': 'fa-book',
            'romance': 'fa-heart',
            'party': 'fa-party-horn',
            'golden': 'fa-sun',
            'arctic': 'fa-snowflake',
            'tropical': 'fa-umbrella-beach',
            'bedtime': 'fa-bed',
            'movie': 'fa-film',
            'gaming': 'fa-gamepad',
            'cooking': 'fa-utensils',
            'creative': 'fa-paint-brush',
            'yoga': 'fa-spa',
            'study': 'fa-graduation-cap',
            'dinner': 'fa-wine-glass',
            'morning': 'fa-coffee',
            'goodnight': 'fa-star-and-crescent'
        };
        
        const icon = sceneIcons[sceneName] || 'fa-palette';
        iconWrapper.querySelector('.scene-icon').className = `scene-icon fas ${icon}`;
        nameEl.textContent = sceneName;
        
        // Set preview color if provided
        if (sceneColor) {
            previewEl.style.background = sceneColor;
            previewEl.classList.add('visible');
        } else {
            previewEl.classList.remove('visible');
        }
        
        sceneIndicator.style.display = 'block';
        sceneIndicator.classList.add('animate');
        sceneIndicator.style.animation = 'scene-slide-in 0.3s ease, scene-fade-out 0.5s ease 2.5s forwards';
        
        // Add particle effect
        createSceneParticles(sceneColor);
        
        setTimeout(() => {
            sceneIndicator.style.display = 'none';
            sceneIndicator.classList.remove('animate');
        }, 3000);
    }
    
    function createSceneParticles(color) {
        for (let i = 0; i < 12; i++) {
            const particle = document.createElement('div');
            particle.className = 'scene-particle';
            particle.style.cssText = `
                position: fixed;
                width: 8px;
                height: 8px;
                background: ${color || 'linear-gradient(135deg, #27a0b9, #1f8999)'};
                border-radius: 50%;
                pointer-events: none;
                z-index: 9999;
                left: ${50 + (Math.random() - 0.5) * 20}%;
                top: ${50 + (Math.random() - 0.5) * 20}%;
                animation: particle-float 1.5s ease-out forwards;
                animation-delay: ${i * 0.05}s;
            `;
            document.body.appendChild(particle);
            setTimeout(() => particle.remove(), 2500);
        }
    }

    // Bedtime Mode Indicator
    function createBedtimeIndicator() {
        bedtimeIndicator = document.createElement('div');
        bedtimeIndicator.className = 'bedtime-mode-active';
        bedtimeIndicator.innerHTML = `
            <i class="fas fa-moon"></i>
            <span>Bedtime Mode Active</span>
        `;
        document.body.appendChild(bedtimeIndicator);
    }

    function setBedtimeMode(active) {
        if (!bedtimeIndicator) return;
        
        if (active) {
            bedtimeIndicator.classList.add('visible');
        } else {
            bedtimeIndicator.classList.remove('visible');
        }
    }

    // Multi-bulb Selection Toolbar
    function createSelectionToolbar() {
        selectionToolbar = document.createElement('div');
        selectionToolbar.id = 'lifx-selection-toolbar';
        selectionToolbar.innerHTML = `
            <span style="color: #fff; font-size: 14px;">
                <i class="fas fa-lightbulb"></i>
                <span class="selected-count">0</span> bulbs selected
            </span>
            <button class="btn-quick-action" onclick="TouchMediaEnhancements.applyToSelected('on')">
                <i class="fas fa-power-off"></i>
                On
            </button>
            <button class="btn-quick-action" onclick="TouchMediaEnhancements.applyToSelected('off')">
                <i class="fas fa-power-off" style="opacity: 0.5;"></i>
                Off
            </button>
            <button class="btn-quick-action" onclick="TouchMediaEnhancements.clearSelection()">
                <i class="fas fa-times"></i>
                Clear
            </button>
        `;
        document.body.appendChild(selectionToolbar);
    }

    let selectedBulbs = new Set();
    
    function addToSelection(bulbId) {
        selectedBulbs.add(bulbId);
        updateSelectionToolbar();
    }

    function removeFromSelection(bulbId) {
        selectedBulbs.delete(bulbId);
        updateSelectionToolbar();
    }

    function updateSelectionToolbar() {
        if (!selectionToolbar) return;
        
        const count = selectedBulbs.size;
        selectionToolbar.querySelector('.selected-count').textContent = count;
        
        if (count > 0) {
            selectionToolbar.classList.add('visible');
        } else {
            selectionToolbar.classList.remove('visible');
        }
    }

    function clearSelection() {
        selectedBulbs.clear();
        updateSelectionToolbar();
        
        document.querySelectorAll('.lifx-bulb-control.multi-selected').forEach(el => {
            el.classList.remove('multi-selected');
        });
    }

    function applyToSelected(action) {
        if (selectedBulbs.size === 0) return;
        
        const bulbIds = Array.from(selectedBulbs);
        console.log(`[TouchMediaEnhancements] Applying ${action} to ${bulbIds.length} bulbs`);
        
        // Dispatch custom event for LIFX controls to handle
        const event = new CustomEvent('lifx-bulk-action', {
            detail: { action, bulbIds }
        });
        document.dispatchEvent(event);
        
        clearSelection();
    }

    // Setup multi-select gestures for LIFX bulbs
    function setupMultiSelectGestures() {
        let isDragging = false;
        let dragStart = null;
        let dragBox = null;
        
        document.addEventListener('mousedown', function(e) {
            if (e.target.closest('.lifx-bulb-control')) {
                isDragging = true;
                dragStart = { x: e.clientX, y: e.clientY };
            }
        });
        
        document.addEventListener('mousemove', function(e) {
            if (!isDragging || !dragStart) return;
            
            const deltaX = e.clientX - dragStart.x;
            const deltaY = e.clientY - dragStart.y;
            
            if (Math.abs(deltaX) > 10 || Math.abs(deltaY) > 10) {
                if (!dragBox) {
                    dragBox = document.createElement('div');
                    dragBox.className = 'multi-select-drag-line';
                    document.body.appendChild(dragBox);
                }
                
                const left = Math.min(dragStart.x, e.clientX);
                const top = Math.min(dragStart.y, e.clientY);
                const width = Math.abs(deltaX);
                const height = Math.abs(deltaY);
                
                dragBox.style.left = left + 'px';
                dragBox.style.top = top + 'px';
                dragBox.style.width = width + 'px';
                dragBox.style.height = height + 'px';
            }
        });
        
        document.addEventListener('mouseup', function(e) {
            if (!isDragging) return;
            isDragging = false;
            
            if (dragBox) {
                const rect = dragBox.getBoundingClientRect();
                
                document.querySelectorAll('.lifx-bulb-control').forEach(bulb => {
                    const bulbRect = bulb.getBoundingClientRect();
                    
                    if (rect.left <= bulbRect.right &&
                        rect.right >= bulbRect.left &&
                        rect.top <= bulbRect.bottom &&
                        rect.bottom >= bulbRect.top) {
                        
                        const bulbId = bulb.dataset.bulbId;
                        if (bulbId) {
                            if (selectedBulbs.has(bulbId)) {
                                removeFromSelection(bulbId);
                                bulb.classList.remove('multi-selected');
                            } else {
                                addToSelection(bulbId);
                                bulb.classList.add('multi-selected');
                            }
                        }
                    }
                });
                
                dragBox.remove();
                dragBox = null;
            }
            
            dragStart = null;
        });
    }

    // Pinch-to-zoom gesture for brightness control
    function setupPinchZoom() {
        let initialDistance = null;
        let initialBrightness = 50;
        
        document.addEventListener('touchstart', function(e) {
            if (e.touches.length === 2) {
                initialDistance = Math.hypot(
                    e.touches[0].clientX - e.touches[1].clientX,
                    e.touches[0].clientY - e.touches[1].clientY
                );
                initialBrightness = LifXTouchControls.state.brightness || 50;
                if (CONFIG.hapticFeedback) {
                    navigator.vibrate?.(10);
                }
            }
        });
        
        document.addEventListener('touchmove', function(e) {
            if (e.touches.length !== 2 || initialDistance === null) return;
            e.preventDefault();
            
            const currentDistance = Math.hypot(
                e.touches[0].clientX - e.touches[1].clientX,
                e.touches[0].clientY - e.touches[1].clientY
            );
            
            const delta = currentDistance - initialDistance;
            const brightnessChange = (delta / 10);
            const newBrightness = Math.max(0, Math.min(100, initialBrightness + brightnessChange));
            
            if (typeof LifXTouchControls !== 'undefined') {
                LifXTouchControls.setBrightness(Math.round(newBrightness));
            }
            
            const pinchIndicator = document.querySelector('.pinch-brightness-indicator');
            if (!pinchIndicator) {
                const indicator = document.createElement('div');
                indicator.className = 'pinch-brightness-indicator';
                indicator.innerHTML = `<i class="fas fa-sun"></i> <span>${Math.round(newBrightness)}%</span>`;
                document.body.appendChild(indicator);
                setTimeout(() => indicator.classList.add('visible'), 10);
            } else {
                pinchIndicator.querySelector('span').textContent = `${Math.round(newBrightness)}%`;
            }
        });
        
        document.addEventListener('touchend', function(e) {
            if (e.touches.length < 2) {
                initialDistance = null;
                const indicator = document.querySelector('.pinch-brightness-indicator');
                if (indicator) {
                    indicator.classList.remove('visible');
                    setTimeout(() => indicator.remove(), 300);
                }
            }
        });
    }

    // Edge swipe for quick panel access
    function setupEdgeSwipe() {
        if (!CONFIG.enableEdgeSwipe) return;
        
        let touchStartX = null;
        let touchStartY = null;
        const edgeThreshold = 30;
        
        document.addEventListener('touchstart', function(e) {
            touchStartX = e.changedTouches[0].screenX;
            touchStartY = e.changedTouches[0].screenY;
        });
        
        document.addEventListener('touchend', function(e) {
            const touchEndX = e.changedTouches[0].screenX;
            const touchEndY = e.changedTouches[0].screenY;
            const deltaX = touchEndX - touchStartX;
            const deltaY = touchEndY - touchStartY;
            
            if (touchStartX < edgeThreshold && deltaX > 100) {
                handleEdgeSwipe('left-edge');
            } else if (touchStartX > window.innerWidth - edgeThreshold && deltaX < -100) {
                handleEdgeSwipe('right-edge');
            } else if (touchStartY < edgeThreshold && deltaY > 100) {
                handleEdgeSwipe('top-edge');
            } else if (touchStartY > window.innerHeight - edgeThreshold && deltaY < -100) {
                handleEdgeSwipe('bottom-edge');
            }
        });
    }

    function handleEdgeSwipe(edge) {
        console.log(`[TouchMediaEnhancements] Edge swipe: ${edge}`);
        
        switch(edge) {
            case 'left-edge':
                toggleQuickScenesPanel();
                break;
            case 'right-edge':
                toggleMediaSyncPanel();
                break;
            case 'top-edge':
                document.querySelector('.header')?.classList.toggle('expanded');
                break;
            case 'bottom-edge':
                document.getElementById('media-controls-panel')?.classList.toggle('expanded');
                break;
        }
        
        if (CONFIG.hapticFeedback) {
            navigator.vibrate?.(15);
        }
    }

    // Keyboard shortcuts for media control
    function setupKeyboardShortcuts() {
        if (!CONFIG.enableKeyboardShortcuts) return;
        
        document.addEventListener('keydown', function(e) {
            if (e.target.tagName === 'INPUT' || e.target.tagName === 'TEXTAREA') return;
            
            switch(e.code) {
                case 'Space':
                    e.preventDefault();
                    mediaPlayPause();
                    break;
                case 'ArrowRight':
                    mediaNext();
                    break;
                case 'ArrowLeft':
                    mediaPrevious();
                    break;
                case 'ArrowUp':
                    e.preventDefault();
                    if (typeof LifXTouchControls !== 'undefined') {
                        const current = LifXTouchControls.state.brightness || 50;
                        LifXTouchControls.setBrightness(Math.min(100, current + 10));
                    }
                    break;
                case 'ArrowDown':
                    e.preventDefault();
                    if (typeof LifXTouchControls !== 'undefined') {
                        const current = LifXTouchControls.state.brightness || 50;
                        LifXTouchControls.setBrightness(Math.max(0, current - 10));
                    }
                    break;
                case 'KeyM':
                    if (e.ctrlKey) {
                        e.preventDefault();
                        toggleMediaSyncPanel();
                    }
                    break;
                case 'KeyL':
                    if (e.ctrlKey) {
                        e.preventDefault();
                        document.getElementById('lifx-color-picker-container')?.classList.toggle('visible');
                    }
                    break;
                case 'KeyB':
                    if (e.ctrlKey) {
                        e.preventDefault();
                        setMediaSyncMode('beat');
                    }
                    break;
                case 'KeyS':
                    if (e.ctrlKey) {
                        e.preventDefault();
                        toggleQuickScenesPanel();
                    }
                    break;
            }
        });
        
        console.log('[TouchMediaEnhancements] Keyboard shortcuts enabled');
    }

    // Enhanced Haptic feedback for touch interactions with pattern library
    function setupHapticFeedback() {
        if (!CONFIG.hapticFeedback || !navigator.vibrate) {
            console.log('[TouchMediaEnhancements] Haptic feedback not available');
            return;
        }
        
        // Haptic pattern library
        const hapticPatterns = {
            light: [5],
            medium: [10],
            strong: [20],
            double: [10, 30, 10],
            triple: [10, 30, 10, 30, 10],
            success: [50, 50, 50],
            error: [50, 50, 50, 50, 100],
            warning: [100, 50, 100],
            notification: [20, 50, 20],
            beat: [15],
            beatStrong: [30],
            beatDouble: [15, 40, 15],
            swipe: [8],
            tap: [5],
            hold: [25],
            release: [10],
            snap: [3],
            click: [8],
            toggle: [12],
            slide: [5, 5, 5],
            zoom: [10, 20, 10],
            scroll: [3],
            select: [15],
            deselect: [8],
            activate: [20],
            deactivate: [10],
            start: [30, 50, 30],
            stop: [50, 50],
            complete: [50, 100, 50],
            error: [100, 50, 100, 50, 100],
            critical: [200, 100, 200, 100, 200]
        };
        
        // Expose haptic function globally
        window.triggerHaptic = function(pattern = 'light') {
            const patternToUse = hapticPatterns[pattern] || hapticPatterns.light;
            navigator.vibrate(patternToUse);
        };
        
        document.addEventListener('click', function(e) {
            const target = e.target.closest('button, .btn, [role="button"]');
            if (target) {
                // Different haptic feedback based on element type
                if (target.classList.contains('btn-primary') || target.classList.contains('btn-main')) {
                    window.triggerHaptic('strong');
                } else if (target.classList.contains('btn-danger') || target.classList.contains('btn-stop')) {
                    window.triggerHaptic('warning');
                } else if (target.classList.contains('btn-success') || target.classList.contains('btn-start')) {
                    window.triggerHaptic('success');
                } else {
                    window.triggerHaptic('click');
                }
            }
        });
        
        document.addEventListener('touchstart', function(e) {
            if (e.target.closest('.lifx-bulb-control')) {
                window.triggerHaptic('tap');
            }
        });
        
        // Add haptic feedback for gestures
        document.addEventListener('gesturestart', function(e) {
            window.triggerHaptic('slide');
        });
        
        document.addEventListener('gestureend', function(e) {
            window.triggerHaptic('release');
        });
    }

    // Double-tap gesture detection
    function setupDoubleTap() {
        if (!CONFIG.enableDoubleTap) return;
        
        let lastTapTime = 0;
        let lastTapTarget = null;
        
        document.addEventListener('touchend', function(e) {
            const now = Date.now();
            const target = e.target.closest('.lifx-bulb-control, .media-item, .scene-item');
            
            if (target && now - lastTapTime < CONFIG.doubleTapDelay && target === lastTapTarget) {
                e.preventDefault();
                handleDoubleTap(target);
                lastTapTime = 0;
                lastTapTarget = null;
            } else {
                lastTapTime = now;
                lastTapTarget = target;
            }
        });
    }

    function handleDoubleTap(target) {
        if (target.classList.contains('lifx-bulb-control')) {
            const bulbId = target.dataset.bulbId;
            if (bulbId) {
                if (typeof LifXTouchControls !== 'undefined') {
                    LifXTouchControls.selectBulb(bulbId);
                    LifXTouchControls.togglePower();
                }
                showGestureHint('Power Toggle', 'fa-power-off');
                // Enhanced haptic for double tap
                if (CONFIG.hapticFeedback) {
                    navigator.vibrate([15, 40, 15]);
                }
            }
        } else if (target.classList.contains('media-item')) {
            const mediaUrl = target.dataset.url;
            if (mediaUrl) {
                openMediaPlayer(mediaUrl, target.dataset.title);
                if (CONFIG.hapticFeedback) {
                    navigator.vibrate([20, 30, 20]);
                }
            }
        } else if (target.classList.contains('scene-item')) {
            const sceneName = target.dataset.scene;
            if (sceneName) {
                applyQuickScene(sceneName);
                if (CONFIG.hapticFeedback) {
                    navigator.vibrate([15, 40, 15]);
                }
            }
        }
    }

    // Long-press gesture detection
    function setupLongPress() {
        if (!CONFIG.enableLongPress) return;
        
        let pressTimer;
        let pressTarget = null;
        let pressStartX = 0;
        let pressStartY = 0;
        const moveThreshold = 10;
        
        document.addEventListener('touchstart', function(e) {
            const target = e.target.closest('.lifx-bulb-control, .media-item, .scene-item');
            if (!target) return;
            
            pressTarget = target;
            pressStartX = e.touches[0].clientX;
            pressStartY = e.touches[0].clientY;
            
            const progressBar = document.getElementById('touch-hold-progress');
            if (progressBar) {
                progressBar.innerHTML = '<div class="touch-hold-progress-bar"></div>';
                progressBar.classList.add('visible');
            }
            
            pressTimer = setTimeout(function() {
                if (pressTarget) {
                    handleLongPress(pressTarget);
                    const bar = progressBar?.querySelector('.touch-hold-progress-bar');
                    if (bar) bar.style.width = '100%';
                    setTimeout(() => {
                        progressBar?.classList.remove('visible');
                        pressTarget = null;
                    }, 200);
                }
            }, CONFIG.longPressDelay);
        });
        
        document.addEventListener('touchmove', function(e) {
            if (!pressTarget) return;
            
            const deltaX = Math.abs(e.touches[0].clientX - pressStartX);
            const deltaY = Math.abs(e.touches[0].clientY - pressStartY);
            
            if (deltaX > moveThreshold || deltaY > moveThreshold) {
                clearTimeout(pressTimer);
                pressTarget = null;
                document.getElementById('touch-hold-progress')?.classList.remove('visible');
            }
        });
        
        document.addEventListener('touchend', function(e) {
            if (pressTimer) {
                clearTimeout(pressTimer);
                pressTimer = null;
            }
            if (pressTarget) {
                document.getElementById('touch-hold-progress')?.classList.remove('visible');
                pressTarget = null;
            }
        });
    }

    function handleLongPress(target) {
        if (target.classList.contains('lifx-bulb-control')) {
            const bulbId = target.dataset.bulbId;
            if (bulbId) {
                if (typeof LifXTouchControls !== 'undefined') {
                    LifXTouchControls.addToMultiSelect(bulbId);
                }
                showGestureHint('Multi-Select', 'fa-users');
                // Enhanced haptic for long press
                if (CONFIG.hapticFeedback) {
                    navigator.vibrate([30, 50, 30]);
                }
            }
        } else if (target.classList.contains('media-item')) {
            showMediaContextMenu(target);
            if (CONFIG.hapticFeedback) {
                navigator.vibrate([25, 40, 25]);
            }
        } else if (target.classList.contains('scene-item')) {
            showSceneContextMenu(target);
            if (CONFIG.hapticFeedback) {
                navigator.vibrate([25, 40, 25]);
            }
        }
    }

    function showGestureHint(text, iconClass) {
        let hint = document.querySelector('.gesture-hint-overlay');
        if (!hint) {
            hint = document.createElement('div');
            hint.className = 'gesture-hint-overlay';
            document.body.appendChild(hint);
        }
        
        hint.innerHTML = `
            <i class="fas ${iconClass}"></i>
            <div class="hint-text">${text}</div>
        `;
        
        hint.classList.add('visible');
        setTimeout(() => hint.classList.remove('visible'), 1500);
    }

    function showMediaContextMenu(target) {
        const menu = document.createElement('div');
        menu.className = 'context-menu';
        menu.innerHTML = `
            <button onclick="playMedia('${target.dataset.url}')"><i class="fas fa-play"></i> Play</button>
            <button onclick="addToPlaylist('${target.dataset.url}')"><i class="fas fa-plus"></i> Add to Playlist</button>
            <button onclick="showMediaInfo('${target.dataset.url}')"><i class="fas fa-info"></i> Info</button>
        `;
        menu.style.cssText = `
            position: fixed;
            z-index: 10000;
            background: rgba(30, 30, 45, 0.98);
            border: 1px solid rgba(39, 160, 185, 0.3);
            border-radius: 12px;
            padding: 10px;
            min-width: 200px;
        `;
        
        const rect = target.getBoundingClientRect();
        menu.style.left = Math.min(rect.left, window.innerWidth - 220) + 'px';
        menu.style.top = Math.min(rect.bottom + 10, window.innerHeight - 150) + 'px';
        
        document.body.appendChild(menu);
        setTimeout(() => menu.remove(), 5000);
    }

    function showSceneContextMenu(target) {
        const sceneName = target.dataset.scene;
        const menu = document.createElement('div');
        menu.className = 'context-menu';
        menu.innerHTML = `
            <button onclick="setAsFavorite('${sceneName}')"><i class="fas fa-star"></i> Favorite</button>
            <button onclick="editScene('${sceneName}')"><i class="fas fa-edit"></i> Edit</button>
            <button onclick="scheduleScene('${sceneName}')"><i class="fas fa-clock"></i> Schedule</button>
        `;
        menu.style.cssText = `
            position: fixed;
            z-index: 10000;
            background: rgba(30, 30, 45, 0.98);
            border: 1px solid rgba(39, 160, 185, 0.3);
            border-radius: 12px;
            padding: 10px;
            min-width: 200px;
        `;
        
        const rect = target.getBoundingClientRect();
        menu.style.left = Math.min(rect.left, window.innerWidth - 220) + 'px';
        menu.style.top = Math.min(rect.bottom + 10, window.innerHeight - 150) + 'px';
        
        document.body.appendChild(menu);
        setTimeout(() => menu.remove(), 5000);
    }

    // Mini Player for floating playback
    let miniPlayerVisible = false;
    
    function setupMiniPlayer() {
        const miniPlayer = document.getElementById('mini-player');
        if (!miniPlayer) return;
        
        makeDraggable(miniPlayer);
    }

    function makeDraggable(element) {
        let pos1 = 0, pos2 = 0, pos3 = 0, pos4 = 0;
        
        element.onmousedown = dragMouseDown;
        element.ontouchstart = dragTouchStart;
        
        function dragMouseDown(e) {
            e.preventDefault();
            pos3 = e.clientX;
            pos4 = e.clientY;
            document.onmouseup = closeDragElement;
            document.onmousemove = elementDrag;
        }
        
        function elementDrag(e) {
            e.preventDefault();
            pos1 = pos3 - e.clientX;
            pos2 = pos4 - e.clientY;
            pos3 = e.clientX;
            pos4 = e.clientY;
            element.style.top = (element.offsetTop - pos2) + 'px';
            element.style.left = (element.offsetLeft - pos1) + 'px';
            element.style.transform = 'none';
        }
        
        function closeDragElement() {
            document.onmouseup = null;
            document.onmousemove = null;
        }
        
        function dragTouchStart(e) {
            const touch = e.touches[0];
            pos3 = touch.clientX;
            pos4 = touch.clientY;
            document.ontouchend = closeDragElement;
            document.ontouchmove = dragTouchMove;
        }
        
        function dragTouchMove(e) {
            const touch = e.touches[0];
            pos1 = pos3 - touch.clientX;
            pos2 = pos4 - touch.clientY;
            pos3 = touch.clientX;
            pos4 = touch.clientY;
            element.style.top = (element.offsetTop - pos2) + 'px';
            element.style.left = (element.offsetLeft - pos1) + 'px';
            element.style.transform = 'none';
        }
    }

    function toggleMiniPlayer() {
        const miniPlayer = document.getElementById('mini-player');
        if (miniPlayer) {
            miniPlayerVisible = !miniPlayerVisible;
            miniPlayer.classList.toggle('visible', miniPlayerVisible);
            miniPlayer.style.display = miniPlayerVisible ? 'block' : 'none';
        }
    }

    function updateMiniPlayer(info) {
        const miniPlayer = document.getElementById('mini-player');
        if (!miniPlayer) return;
        
        const title = miniPlayer.querySelector('.mini-player-title');
        const artist = miniPlayer.querySelector('.mini-player-artist');
        const playIcon = miniPlayer.querySelector('#mini-play-icon');
        
        if (title) title.textContent = info.trackName || 'No Track';
        if (artist) artist.textContent = info.artistName || 'Unknown Artist';
        if (playIcon) playIcon.className = info.isPlaying ? 'fas fa-pause' : 'fas fa-play';
        
        if (!miniPlayerVisible && info.isPlaying) {
            toggleMiniPlayer();
        }
    }

    // Now Playing Toast notification
    let nowPlayingTimeout;
    
    function setupNowPlayingToast() {
        const toast = document.getElementById('now-playing-toast');
        if (!toast) return;
        
        const closeBtn = toast.querySelector('.now-playing-close');
        if (closeBtn) {
            closeBtn.addEventListener('click', hideNowPlayingToast);
        }
    }

    function showNowPlayingToast(info) {
        const toast = document.getElementById('now-playing-toast');
        if (!toast) return;
        
        const art = toast.querySelector('.now-playing-art');
        const title = toast.querySelector('.now-playing-title');
        const artist = toast.querySelector('.now-playing-artist');
        
        if (art) {
            art.style.backgroundImage = info.artwork ? `url(${info.artwork})` : '';
            art.style.background = info.artwork ? '' : 'linear-gradient(135deg, #27a0b9, #1f8999)';
        }
        if (title) title.textContent = info.trackName || 'No Track';
        if (artist) artist.textContent = info.artistName || 'Unknown Artist';
        
        toast.classList.add('visible');
        
        clearTimeout(nowPlayingTimeout);
        nowPlayingTimeout = setTimeout(hideNowPlayingToast, 5000);
    }

    function hideNowPlayingToast() {
        const toast = document.getElementById('now-playing-toast');
        if (toast) {
            toast.classList.remove('visible');
        }
    }

    function hideNowPlaying() {
        hideNowPlayingToast();
    }

    // Enhanced Swipe gesture detection with three-finger support
    function setupSwipeGestures() {
        let touchStartX = 0;
        let touchStartY = 0;
        let swipeThreshold = CONFIG.swipeThreshold;
        let touchStartTime = 0;
        let activeTouches = 0;
        let threeFingerDetected = false;
        
        document.addEventListener('touchstart', function(e) {
            touchStartX = e.changedTouches[0].screenX;
            touchStartY = e.changedTouches[0].screenY;
            touchStartTime = Date.now();
            activeTouches = e.touches.length;
            threeFingerDetected = false;
            
            // Detect three-finger swipe
            if (activeTouches === 3 && CONFIG.enableThreeFingerSwipe) {
                threeFingerDetected = true;
                if (CONFIG.hapticFeedback) {
                    navigator.vibrate([5, 5, 5]);
                }
            }
        });
        
        document.addEventListener('touchmove', function(e) {
            if (threeFingerDetected && e.touches.length === 3) {
                e.preventDefault();
            }
        }, { passive: false });
        
        document.addEventListener('touchend', function(e) {
            const touchEndX = e.changedTouches[0].screenX;
            const touchEndY = e.changedTouches[0].screenY;
            const deltaTime = Date.now() - touchStartTime;
            
            const deltaX = touchEndX - touchStartX;
            const deltaY = touchEndY - touchStartY;
            
            // Three-finger swipe detection
            if (threeFingerDetected && (Math.abs(deltaX) > swipeThreshold || Math.abs(deltaY) > swipeThreshold)) {
                let direction;
                if (Math.abs(deltaX) > Math.abs(deltaY)) {
                    direction = deltaX > 0 ? 'three-finger-right' : 'three-finger-left';
                } else {
                    direction = deltaY > 0 ? 'three-finger-down' : 'three-finger-up';
                }
                handleThreeFingerSwipe(direction);
                return;
            }
            
            // Standard swipe detection
            if (Math.abs(deltaX) > swipeThreshold || Math.abs(deltaY) > swipeThreshold) {
                let direction;
                if (Math.abs(deltaX) > Math.abs(deltaY)) {
                    direction = deltaX > 0 ? 'right' : 'left';
                } else {
                    direction = deltaY > 0 ? 'down' : 'up';
                }
                
                handleSwipe(direction, deltaTime);
            }
        });
    }

    function handleSwipe(direction, deltaTime = 0) {
        const velocity = deltaTime > 0 ? 1000 / deltaTime : 0;
        console.log(`[TouchMediaEnhancements] Swipe detected: ${direction} (${deltaTime}ms, velocity: ${velocity.toFixed(1)})`);
        
        const event = new CustomEvent('swipe-gesture', { detail: { direction, deltaTime, velocity } });
        document.dispatchEvent(event);
        
        // Show enhanced swipe indicator with velocity-based styling
        const indicator = document.createElement('div');
        indicator.className = 'swipe-indicator visible swipe-enhanced';
        
        const arrows = {
            'up': 'fa-chevron-up',
            'down': 'fa-chevron-down',
            'left': 'fa-chevron-left',
            'right': 'fa-chevron-right'
        };
        
        // Velocity-based color (blue -> purple -> pink -> red)
        const hue = Math.min(360, 180 + velocity * 20);
        const scale = Math.min(1.5, 1 + velocity * 0.005);
        
        indicator.innerHTML = `
            <i class="fas ${arrows[direction] || 'fa-chevron-up'} swipe-arrow"></i>
            <div class="swipe-trail" style="--trail-hue: ${hue}; --trail-scale: ${scale}"></div>
        `;
        indicator.style.cssText = `
            position: fixed;
            top: 50%;
            left: 50%;
            transform: translate(-50%, -50%);
            font-size: ${48 * scale}px;
            color: hsla(${hue}, 80%, 60%, 0.9);
            pointer-events: none;
            z-index: 9999;
            filter: drop-shadow(0 0 ${20 * velocity/10}px hsla(${hue}, 80%, 60%, 0.6));
            animation: swipe-indicator-anim 0.5s ease-out forwards;
        `;
        document.body.appendChild(indicator);
        
        // Create swipe trail particles for fast swipes
        if (velocity > 2 && CONFIG.animationsEnabled) {
            createSwipeTrail(direction, velocity, hue);
        }
        
        setTimeout(() => indicator.remove(), 600);
        
        // Velocity-based haptic feedback
        if (CONFIG.hapticFeedback) {
            const hapticStrength = Math.min(30, 8 + Math.floor(velocity));
            navigator.vibrate?.([hapticStrength, 30, hapticStrength / 2]);
        }
    }
    
    // Create swipe trail particle effect
    function createSwipeTrail(direction, velocity, hue) {
        const particleCount = Math.min(12, Math.floor(velocity * 2));
        const isHorizontal = direction === 'left' || direction === 'right';
        const isPositive = direction === 'right' || direction === 'down';
        
        for (let i = 0; i < particleCount; i++) {
            const particle = document.createElement('div');
            particle.className = 'swipe-trail-particle';
            
            const offset = isHorizontal 
                ? { x: (Math.random() - 0.5) * 100, y: (Math.random() - 0.5) * 200 }
                : { x: (Math.random() - 0.5) * 200, y: (Math.random() - 0.5) * 100 };
            
            const travelDistance = isHorizontal 
                ? (isPositive ? 100 + i * 20 : -100 - i * 20)
                : (isPositive ? 100 + i * 20 : -100 - i * 20);
            
            particle.style.cssText = `
                position: fixed;
                left: ${50 + (offset.x / window.innerWidth) * 50}%;
                top: ${50 + (offset.y / window.innerHeight) * 50}%;
                width: ${Math.max(4, 12 - i)}px;
                height: ${Math.max(4, 12 - i)}px;
                background: hsla(${hue}, 80%, 60%, ${0.8 - i * 0.05});
                border-radius: 50%;
                pointer-events: none;
                z-index: 9998;
                animation: swipe-trail-anim 0.4s ease-out forwards;
                --travel-x: ${isHorizontal ? travelDistance : offset.x}px;
                --travel-y: ${!isHorizontal ? travelDistance : offset.y}px;
            `;
            
            document.body.appendChild(particle);
            setTimeout(() => particle.remove(), 500);
        }
    }
    
    function handleThreeFingerSwipe(direction) {
        console.log(`[TouchMediaEnhancements] Three-finger swipe: ${direction}`);
        
        const event = new CustomEvent('three-finger-swipe', { detail: { direction } });
        document.dispatchEvent(event);
        
        // Show three-finger indicator
        const indicator = document.createElement('div');
        indicator.className = 'three-finger-indicator';
        indicator.innerHTML = `
            <div class="finger-dots">
                <span class="dot"></span>
                <span class="dot"></span>
                <span class="dot"></span>
            </div>
            <i class="fas fa-chevron-${direction.includes('up') ? 'up' : direction.includes('down') ? 'down' : direction.includes('left') ? 'left' : 'right'}"></i>
        `;
        indicator.style.cssText = `
            position: fixed;
            top: 50%;
            left: 50%;
            transform: translate(-50%, -50%);
            pointer-events: none;
            z-index: 9999;
            animation: three-finger-anim 0.6s ease-out forwards;
        `;
        document.body.appendChild(indicator);
        
        setTimeout(() => indicator.remove(), 700);
        
        // Enhanced haptic for three-finger
        if (CONFIG.hapticFeedback) {
            navigator.vibrate?.([10, 20, 10]);
        }
        
        // Handle three-finger actions
        switch(direction) {
            case 'three-finger-up':
                // Show all scenes
                toggleQuickScenesPanel();
                break;
            case 'three-finger-down':
                // Quick settings
                toggleMediaSyncPanel();
                break;
            case 'three-finger-left':
                // Previous scene
                if (typeof LifXTouchControls !== 'undefined') {
                    LifXTouchControls.previousScene?.();
                }
                break;
            case 'three-finger-right':
                // Next scene
                if (typeof LifXTouchControls !== 'undefined') {
                    LifXTouchControls.nextScene?.();
                }
                break;
        }
    }

    // Enhanced Media Visualizer with multiple visualization modes
    function setupMediaVisualizer() {
        const container = document.getElementById('media-visualization-container');
        if (!container) return;
        
        const numBars = 64; // Increased resolution
        mediaVisualizerBars = [];
        
        // Create visualization mode selector
        const modeSelector = document.createElement('div');
        modeSelector.className = 'viz-mode-selector';
        modeSelector.innerHTML = `
            <button class="viz-mode-btn active" data-mode="bars"><i class="fas fa-chart-bar"></i></button>
            <button class="viz-mode-btn" data-mode="wave"><i class="fas fa-wave-square"></i></button>
            <button class="viz-mode-btn" data-mode="circular"><i class="fas fa-circle"></i></button>
            <button class="viz-mode-btn" data-mode="particles"><i class="fas fa-sparkles"></i></button>
        `;
        container.appendChild(modeSelector);
        
        // Setup mode switching
        modeSelector.querySelectorAll('.viz-mode-btn').forEach(btn => {
            btn.addEventListener('click', () => {
                modeSelector.querySelectorAll('.viz-mode-btn').forEach(b => b.classList.remove('active'));
                btn.classList.add('active');
                container.dataset.vizMode = btn.dataset.mode;
            });
        });
        
        // Create bars for default visualization
        for (let i = 0; i < numBars; i++) {
            const bar = document.createElement('div');
            bar.className = 'media-viz-bar';
            bar.style.background = `hsl(${(i / numBars) * 360}, 80%, 50%)`;
            bar.style.boxShadow = `0 0 10px hsl(${(i / numBars) * 360}, 80%, 50%)`;
            container.appendChild(bar);
            mediaVisualizerBars.push(bar);
        }
        
        // Create circular visualizer elements
        const circularContainer = document.createElement('div');
        circularContainer.className = 'circular-viz-container';
        for (let i = 0; i < 36; i++) {
            const bar = document.createElement('div');
            bar.className = 'circular-viz-bar';
            bar.style.transform = `rotate(${i * 10}deg) translateY(-100px)`;
            circularContainer.appendChild(bar);
        }
        container.appendChild(circularContainer);
    }

    function updateMediaVisualization(data) {
        if (!mediaVisualizerBars.length) return;
        
        const container = document.getElementById('media-visualization-container');
        const mode = container?.dataset.vizMode || 'bars';
        
        const values = data || Array(mediaVisualizerBars.length).fill(0).map(() => Math.random());
        
        if (mode === 'bars') {
            mediaVisualizerBars.forEach((bar, i) => {
                const value = values[i] || values[values.length - 1] || 0;
                const height = Math.max(5, value * 140);
                bar.style.height = height + 'px';
                bar.style.opacity = 0.5 + value * 0.5;
                
                // Enhanced peak detection with glow effect
                if (value > 0.9) {
                    bar.classList.add('peak');
                    bar.style.boxShadow = `0 0 20px hsl(${(i / mediaVisualizerBars.length) * 360}, 100%, 60%)`;
                } else {
                    bar.classList.remove('peak');
                    bar.style.boxShadow = `0 0 10px hsl(${(i / mediaVisualizerBars.length) * 360}, 80%, 50%)`;
                }
            });
        } else if (mode === 'circular') {
            const circularBars = container.querySelectorAll('.circular-viz-bar');
            circularBars.forEach((bar, i) => {
                const value = values[i * 2] || 0;
                const scale = 1 + value * 1.5;
                bar.style.transform = `rotate(${i * 10}deg) translateY(-${100 + value * 50}px) scale(${scale})`;
                bar.style.background = `hsl(${(i / 36) * 360}, 80%, ${50 + value * 30}%)`;
            });
        }
    }

    // Party Mode Visualizer
    function activatePartyMode(active) {
        let visualizer = document.querySelector('.party-mode-visualizer');
        
        if (active) {
            if (!visualizer) {
                visualizer = document.createElement('div');
                visualizer.className = 'party-mode-visualizer';
                
                for (let i = 0; i < 50; i++) {
                    const dot = document.createElement('div');
                    dot.className = 'party-dot';
                    dot.style.left = Math.random() * 100 + '%';
                    dot.style.top = Math.random() * 100 + '%';
                    dot.style.animationDelay = Math.random() * 2 + 's';
                    visualizer.appendChild(dot);
                }
                
                document.body.appendChild(visualizer);
            }
            
            visualizer.classList.add('active');
        } else {
            if (visualizer) {
                visualizer.classList.remove('active');
                setTimeout(() => visualizer.remove(), 300);
            }
        }
    }

    // Quick scene presets
    const scenePresets = {
        'focus': { color: '#00d4ff', brightness: 80, temperature: 4000 },
        'relax': { color: '#ff9966', brightness: 60, temperature: 2700 },
        'party': { color: '#ff0080', brightness: 100, effect: 'party' },
        'bedtime': { color: '#4a0080', brightness: 30, temperature: 2000 },
        'reading': { color: '#ffffff', brightness: 90, temperature: 5000 },
        'movie': { color: '#8800ff', brightness: 40, temperature: 3000 }
    };

    function applyScenePreset(presetName) {
        const preset = scenePresets[presetName];
        if (!preset) return;
        
        console.log(`[TouchMediaEnhancements] Applying scene: ${presetName}`);
        
        const event = new CustomEvent('apply-lifx-scene', { detail: { name: presetName, ...preset } });
        document.dispatchEvent(event);
        
        showSceneChange(presetName);
        
        if (presetName === 'bedtime') {
            setBedtimeMode(true);
        } else if (presetName === 'party') {
            activatePartyMode(true);
        } else {
            setBedtimeMode(false);
            activatePartyMode(false);
        }
    }

    // Color temperature picker
    function setupColorTemperaturePicker() {
        const picker = document.querySelector('.color-temp-gradient');
        if (!picker) return;
        
        let isDragging = false;
        
        picker.addEventListener('mousedown', handleDrag);
        picker.addEventListener('mousemove', handleDrag);
        picker.addEventListener('mouseup', () => isDragging = false);
        picker.addEventListener('touchstart', handleTouch);
        picker.addEventListener('touchmove', handleTouch);
        picker.addEventListener('touchend', () => isDragging = false);
        
        function handleDrag(e) {
            if (e.type === 'mousedown') isDragging = true;
            if (!isDragging) return;
            
            const rect = picker.getBoundingClientRect();
            const x = (e.clientX - rect.left) / rect.width;
            updateTemperature(x);
        }
        
        function handleTouch(e) {
            if (e.type === 'touchstart') isDragging = true;
            if (!isDragging) return;
            
            const rect = picker.getBoundingClientRect();
            const x = (e.touches[0].clientX - rect.left) / rect.width;
            updateTemperature(x);
        }
        
        function updateTemperature(x) {
            x = Math.max(0, Math.min(1, x));
            const kelvin = 2000 + (x * 7000);
            
            let indicator = picker.querySelector('.color-temp-indicator');
            if (!indicator) {
                indicator = document.createElement('div');
                indicator.className = 'color-temp-indicator';
                picker.appendChild(indicator);
            }
            
            indicator.style.left = (x * 100) + '%';
            
            const event = new CustomEvent('color-temperature-change', { detail: { kelvin, x } });
            document.dispatchEvent(event);
        }
    }

    // Beat detection calibration UI
    function updateBeatDetectionStats(stats) {
        const container = document.querySelector('.beat-detection-calibration');
        if (!container || !stats) return;
        
        let statsEl = container.querySelector('.calibration-stats');
        if (!statsEl) {
            statsEl = document.createElement('div');
            statsEl.className = 'calibration-stats';
            container.appendChild(statsEl);
        }
        
        statsEl.innerHTML = `
            <div class="stat-item">
                <span class="stat-label">Detected BPM</span>
                <span class="stat-value">${stats.bpm || '--'}</span>
            </div>
            <div class="stat-item">
                <span class="stat-label">Confidence</span>
                <span class="stat-value">${stats.confidence ? (stats.confidence * 100).toFixed(0) + '%' : '--'}</span>
            </div>
            <div class="stat-item">
                <span class="stat-label">Last Beat</span>
                <span class="stat-value">${stats.lastBeat ? stats.lastBeat.toFixed(3) + 's' : '--'}</span>
            </div>
            <div class="stat-item">
                <span class="stat-label">Energy</span>
                <span class="stat-value">${stats.energy ? (stats.energy * 100).toFixed(0) + '%' : '--'}</span>
            </div>
        `;
    }

    // Media sync dashboard
    function updateMediaSyncDashboard(status) {
        const container = document.querySelector('.media-sync-dashboard');
        if (!container) return;
        
        let grid = container.querySelector('.sync-status-grid');
        if (!grid) {
            grid = document.createElement('div');
            grid.className = 'sync-status-grid';
            container.appendChild(grid);
        }
        
        grid.innerHTML = `
            <div class="sync-status-card ${status.audioActive ? 'active' : ''}">
                <div class="status-icon"><i class="fas fa-music"></i></div>
                <div class="status-label">Audio Analysis</div>
                <div class="status-value">${status.audioActive ? 'Active' : 'Inactive'}</div>
                <button class="btn-toggle" onclick="TouchMediaEnhancements.toggleAudioAnalysis()">
                    <i class="fas ${status.audioActive ? 'fa-toggle-on' : 'fa-toggle-off'}"></i>
                </button>
            </div>
            <div class="sync-status-card ${status.lightsActive ? 'active' : ''}">
                <div class="status-icon"><i class="fas fa-lightbulb"></i></div>
                <div class="status-label">Light Sync</div>
                <div class="status-value">${status.lightsActive ? 'Active' : 'Inactive'}</div>
                <button class="btn-toggle" onclick="TouchMediaEnhancements.toggleLightSync()">
                    <i class="fas ${status.lightsActive ? 'fa-toggle-on' : 'fa-toggle-off'}"></i>
                </button>
            </div>
        `;
    }

    function toggleAudioAnalysis() {
        document.dispatchEvent(new CustomEvent('toggle-audio-analysis'));
    }

    function toggleLightSync() {
        document.dispatchEvent(new CustomEvent('toggle-light-sync'));
    }

    // Touch sensitivity settings
    function setTouchSensitivity(level) {
        CONFIG.touchSensitivity = level;
        
        const sensitivityMap = {
            'low': 100,
            'medium': 50,
            'high': 20
        };
        
        CONFIG.gestureTrailCount = sensitivityMap[level] || 50;
        
        const event = new CustomEvent('touch-sensitivity-change', { detail: { level } });
        document.dispatchEvent(event);
    }

    function toggleMediaSyncPanel() {
        const panel = document.getElementById('media-sync-panel');
        if (panel) {
            panel.classList.toggle('visible');
            mediaSyncActive = !mediaSyncActive;
            document.getElementById('sync-stat').textContent = mediaSyncActive ? 'On' : 'Off';
        }
    }
    
    function setMediaSyncMode(mode) {
        mediaSyncMode = mode;
        document.querySelectorAll('.media-sync-btn').forEach(btn => {
            btn.classList.remove('active');
            btn.classList.add('inactive');
        });
        const activeBtn = document.getElementById(`${mode}-sync-btn`);
        if (activeBtn) {
            activeBtn.classList.remove('inactive');
            activeBtn.classList.add('active');
        }
        document.getElementById('sync-stat').textContent = mode.charAt(0).toUpperCase() + mode.slice(1);
        
        if (mode === 'beat') {
            startBeatDetection();
        } else if (mode === 'ambient') {
            startAmbientAnalysis();
        } else if (mode === 'spectrum') {
            startSpectrumAnalysis();
        }
    }
    
    function toggleQuickScenesPanel() {
        const panel = document.getElementById('quick-scenes-panel');
        if (panel) {
            panel.classList.toggle('visible');
            if (panel.classList.contains('visible')) {
                renderQuickScenes();
            }
        }
    }
    
    function renderQuickScenes() {
        const grid = document.getElementById('quick-scenes-grid');
        if (!grid) return;
        
        const scenes = [
            { name: 'relax', icon: '🧘', color: '#ff6b6b' },
            { name: 'focus', icon: '🎯', color: '#4ecdc4' },
            { name: 'energize', icon: '⚡', color: '#ffe66d' },
            { name: 'night', icon: '🌙', color: '#1a1a2e' },
            { name: 'sunset', icon: '🌅', color: '#ff9f43' },
            { name: 'ocean', icon: '🌊', color: '#45b7d1' },
            { name: 'reading', icon: '📖', color: '#feca57' },
            { name: 'romance', icon: '💕', color: '#ff9ff3' },
            { name: 'party', icon: '🎉', color: '#00d4ff' },
            { name: 'golden', icon: '☀️', color: '#f9ca24' },
            { name: 'arctic', icon: '❄️', color: '#70a1ff' },
            { name: 'tropical', icon: '🏖️', color: '#00b894' }
        ];
        
        grid.innerHTML = scenes.map(scene => `
            <div class="scene-item" onclick="applyQuickScene('${scene.name}')">
                <div class="scene-icon">${scene.icon}</div>
                <div class="scene-name">${scene.name}</div>
            </div>
        `).join('');
    }
    
    function applyQuickScene(sceneName) {
        if (typeof LifXTouchControls !== 'undefined') {
            LifXTouchControls.applyScene(sceneName);
        }
        toggleQuickScenesPanel();
    }
    
    function mediaPlayPause() {
        const icon = document.getElementById('media-play-icon');
        if (mediaPlaybackState.isPlaying) {
            mediaPlaybackState.isPlaying = false;
            if (icon) icon.className = 'fas fa-play';
            sendMediaCommand('pause');
        } else {
            mediaPlaybackState.isPlaying = true;
            if (icon) icon.className = 'fas fa-pause';
            sendMediaCommand('play');
        }
    }
    
    function mediaPrevious() {
        sendMediaCommand('previous');
    }
    
    function mediaNext() {
        sendMediaCommand('next');
    }
    
    function sendMediaCommand(command) {
        if (typeof websocket !== 'undefined' && websocket && websocket.readyState === WebSocket.OPEN) {
            websocket.send(JSON.stringify({
                type: 'command',
                id: `media_${command}_${Date.now()}`,
                command: `media_${command}`,
                args: {}
            }));
        }
    }
    
    function updateMediaPlayback(info) {
        mediaPlaybackState = { ...mediaPlaybackState, ...info };
        
        const trackName = document.getElementById('media-track-name');
        const artistName = document.getElementById('media-artist-name');
        const progress = document.getElementById('media-progress');
        const playIcon = document.getElementById('media-play-icon');
        
        if (trackName) trackName.textContent = info.trackName || 'No Track';
        if (artistName) artistName.textContent = info.artistName || 'Unknown Artist';
        if (progress) progress.style.width = `${(info.progress / info.duration) * 100 || 0}%`;
        if (playIcon) playIcon.className = info.isPlaying ? 'fas fa-pause' : 'fas fa-play';
    }
    
    function startBeatDetection() {
        initAudioAnalyzer();
        const vizContainer = document.getElementById('beat-visualization');
        if (!vizContainer) return;
        
        if (vizContainer.children.length === 0) {
            for (let i = 0; i < 16; i++) {
                const bar = document.createElement('div');
                bar.className = 'beat-bar';
                bar.style.height = '5px';
                vizContainer.appendChild(bar);
                mediaVisualizerBars.push(bar);
            }
        }
        
        requestAnimationFrame(updateBeatVisualization);
    }
    
    function startAmbientAnalysis() {
        initAudioAnalyzer();
    }
    
    function startSpectrumAnalysis() {
        initAudioAnalyzer();
    }
    
    let beatHistory = [];
    let lastBeatTime = 0;
    let beatThreshold = 0.8;
    let bpmHistory = [];
    let beatEnergyHistory = [];
    let maxBeatEnergyHistory = 10;
    let consecutiveBeatCount = 0;
    let beatConfidence = 0;
    let lastBeatEnergy = 0;
    let calibrationData = {
        baselineEnergy: 0,
        peakEnergy: 0,
        calibrationCount: 0,
        isCalibrating: true,
        calibrationSamples: []
    };
    
    function initAudioAnalyzer() {
        if (!audioContext) {
            audioContext = new (window.AudioContext || window.webkitAudioContext)();
            analyser = audioContext.createAnalyser();
            analyser.fftSize = 1024;
            analyser.smoothingTimeConstant = 0.85;
            beatHistory = [];
            bpmHistory = [];
            beatEnergyHistory = [];
            beatConfidence = 0;
            consecutiveBeatCount = 0;
            calibrationData = {
                baselineEnergy: 0,
                peakEnergy: 0,
                calibrationCount: 0,
                isCalibrating: true,
                calibrationSamples: []
            };
        }
    }
    
    function calibrateBeatDetection(dataArray) {
        const bassRange = dataArray.slice(0, 8);
        const bassAvg = bassRange.reduce((a, b) => a + b, 0) / bassRange.length;
        const currentEnergy = bassAvg / 255;
        
        calibrationData.calibrationSamples.push(currentEnergy);
        if (calibrationData.calibrationSamples.length > 50) {
            calibrationData.calibrationSamples.shift();
        }
        
        calibrationData.calibrationCount++;
        
        const samples = calibrationData.calibrationSamples;
        calibrationData.baselineEnergy = samples.reduce((a, b) => a + b, 0) / samples.length;
        calibrationData.peakEnergy = Math.max(...samples);
        
        if (calibrationData.calibrationCount > 100) {
            calibrationData.isCalibrating = false;
            const dynamicRange = calibrationData.peakEnergy - calibrationData.baselineEnergy;
            beatThreshold = Math.max(0.4, Math.min(0.85, calibrationData.baselineEnergy + (dynamicRange * 0.6)));
        }
        
        return calibrationData;
    }
    
    function resetCalibration() {
        calibrationData = {
            baselineEnergy: 0,
            peakEnergy: 0,
            calibrationCount: 0,
            isCalibrating: true,
            calibrationSamples: []
        };
        beatThreshold = 0.8;
    }
    
    function detectBeat(dataArray) {
        if (calibrationData.isCalibrating) {
            calibrateBeatDetection(dataArray);
        }
        
        const bassRange = dataArray.slice(0, 8);
        const subBassRange = dataArray.slice(0, 4);
        const lowMidRange = dataArray.slice(8, 16);
        
        const bassAvg = bassRange.reduce((a, b) => a + b, 0) / bassRange.length;
        const subBassAvg = subBassRange.reduce((a, b) => a + b, 0) / subBassRange.length;
        const lowMidAvg = lowMidRange.reduce((a, b) => a + b, 0) / lowMidRange.length;
        const bassPeak = Math.max(...bassRange);
        
        const currentBeatEnergy = bassAvg / 255;
        const subBassEnergy = subBassAvg / 255;
        const energyRatio = subBassEnergy / (currentBeatEnergy + 0.01);
        
        beatEnergyHistory.push(currentBeatEnergy);
        if (beatEnergyHistory.length > maxBeatEnergyHistory) {
            beatEnergyHistory.shift();
        }
        
        const avgEnergy = beatEnergyHistory.reduce((a, b) => a + b, 0) / beatEnergyHistory.length;
        const energyChange = currentBeatEnergy - lastBeatEnergy;
        const energyAcceleration = energyChange - (beatEnergyHistory[beatEnergyHistory.length - 2] - beatEnergyHistory[beatEnergyHistory.length - 3] || 0);
        
        const now = Date.now();
        const timeSinceLastBeat = now - lastBeatTime;
        const expectedInterval = bpmHistory.length > 0 ? 60000 / bpmHistory[bpmHistory.length - 1] : 500;
        const intervalDeviation = Math.abs(timeSinceLastBeat - expectedInterval) / expectedInterval;
        
        const calibratedThreshold = calibrationData.isCalibrating 
            ? beatThreshold 
            : calibrationData.baselineEnergy + ((calibrationData.peakEnergy - calibrationData.baselineEnergy) * 0.6);
        
        const dynamicThreshold = Math.max(0.4, Math.min(0.95, 
            calibratedThreshold
            - (avgEnergy * 0.15) 
            - (Math.max(0, energyChange) * 0.1)
            - (Math.max(0, energyAcceleration) * 0.05)
            + (intervalDeviation * 0.2)
        ));
        
        const isBeat = bassPeak > dynamicThreshold * 255 
            && timeSinceLastBeat > 150 
            && timeSinceLastBeat < 1500
            && subBassEnergy > 0.4
            && energyRatio > 0.7;
        
        if (isBeat) {
            lastBeatTime = now;
            beatHistory.push(now);
            consecutiveBeatCount++;
            lastBeatEnergy = currentBeatEnergy;
            
            beatConfidence = Math.min(1.0, beatConfidence + 0.08 + (energyRatio * 0.02));
            
            if (beatHistory.length > 2) {
                const intervals = [];
                for (let i = 1; i < beatHistory.length; i++) {
                    intervals.push(beatHistory[i] - beatHistory[i - 1]);
                }
                
                const sortedIntervals = [...intervals].sort((a, b) => a - b);
                const trimmedIntervals = sortedIntervals.slice(Math.floor(sortedIntervals.length * 0.2), 
                                                               Math.ceil(sortedIntervals.length * 0.8));
                const avgInterval = trimmedIntervals.length > 0 
                    ? trimmedIntervals.reduce((a, b) => a + b, 0) / trimmedIntervals.length
                    : intervals.reduce((a, b) => a + b, 0) / intervals.length;
                
                const detectedBpm = Math.round(60000 / avgInterval);
                
                if (detectedBpm > 60 && detectedBpm < 200) {
                    bpmHistory.push(detectedBpm);
                    if (bpmHistory.length > 16) bpmHistory.shift();
                    
                    const weights = bpmHistory.map((_, i) => Math.pow(i + 1, 1.5));
                    const weightedSum = bpmHistory.reduce((sum, bpm, i) => sum + bpm * weights[i], 0);
                    const weightTotal = weights.reduce((a, b) => a + b, 0);
                    const smoothedBpm = Math.round(weightedSum / weightTotal);
                    
                    updateBpm(smoothedBpm);
                    updateBeatDetectionStats({
                        bpm: smoothedBpm,
                        confidence: beatConfidence,
                        lastBeat: now / 1000,
                        energy: currentBeatEnergy,
                        subBassEnergy: subBassEnergy,
                        energyRatio: energyRatio,
                        consecutiveBeats: consecutiveBeatCount
                    });
                    
                    if (mediaSyncActive && mediaSyncMode === 'beat') {
                        triggerLifxBeat();
                        triggerBeatVisualization(currentBeatEnergy, subBassEnergy);
                    }
                    
                    return true;
                }
            }
        } else {
            const timeDecay = Math.min(0.05, 0.01 * (timeSinceLastBeat / 1000));
            beatConfidence = Math.max(0, beatConfidence - timeDecay);
            if (timeSinceLastBeat > 3000) {
                consecutiveBeatCount = 0;
            }
        }
        
        if (beatHistory.length > 0 && now - beatHistory[beatHistory.length - 1] > 5000) {
            beatHistory.shift();
            beatConfidence = Math.max(0, beatConfidence - 0.1);
        }
        
        return false;
    }
    
    function triggerLifxBeat(energy = 1.0, options = {}) {
        const {
            zones = null,
            color = null,
            brightness = null,
            effect = 'pulse'
        } = options;
        
        const event = new CustomEvent('lifx-beat-trigger', {
            detail: { 
                timestamp: Date.now(),
                energy,
                zones,
                color,
                brightness,
                effect
            }
        });
        document.dispatchEvent(event);
        
        if (mediaSyncActive && CONFIG.enableHapticFeedback && navigator.vibrate) {
            navigator.vibrate([15]);
        }
    }
    
    function triggerLifxBeatMultiZone(zones, pattern = 'wave') {
        const patterns = {
            'wave': zones.map((_, i) => ({ delay: i * 50, intensity: 1.0 })),
            'pulse': zones.map((_, i) => ({ delay: 0, intensity: 1.0 })),
            'cascade': zones.map((_, i) => ({ delay: i * 100, intensity: 1.0 - (i * 0.1) })),
            'random': zones.map(() => ({ delay: Math.random() * 200, intensity: 0.5 + Math.random() * 0.5 }))
        };
        
        const patternData = patterns[pattern] || patterns.wave;
        
        patternData.forEach(({ delay, intensity }, i) => {
            setTimeout(() => {
                triggerLifxBeat(intensity, { zones: [zones[i]], effect: 'flash' });
            }, delay);
        });
    }
    
    function triggerBeatVisualization(energy, subBassEnergy = 0) {
        const visualizerContainer = document.getElementById('beat-visualization');
        if (!visualizerContainer) return;
        
        const combinedEnergy = Math.min(1, energy * 0.7 + subBassEnergy * 0.3);
        const hue = 180 + (subBassEnergy * 60);
        
        const beatPulse = document.createElement('div');
        beatPulse.className = 'beat-pulse-visual beat-pulse-enhanced';
        beatPulse.style.cssText = `
            position: absolute;
            top: 50%;
            left: 50%;
            transform: translate(-50%, -50%) scale(0.5);
            width: ${100 + (combinedEnergy * 200)}px;
            height: ${100 + (combinedEnergy * 200)}px;
            border-radius: 50%;
            background: radial-gradient(circle, 
                hsla(${hue}, 80%, 60%, ${0.4 + combinedEnergy * 0.4}) 0%,
                hsla(${hue + 30}, 70%, 50%, ${0.2 + combinedEnergy * 0.3}) 40%,
                transparent 70%);
            box-shadow: 0 0 ${30 + combinedEnergy * 50}px hsla(${hue}, 80%, 60%, ${0.6 + combinedEnergy * 0.3});
            pointer-events: none;
            z-index: 100;
            animation: beat-pulse-expand-enhanced 0.6s cubic-bezier(0.25, 0.46, 0.45, 0.94) forwards;
        `;
        
        visualizerContainer.appendChild(beatPulse);
        setTimeout(() => beatPulse.remove(), 700);
        
        const bars = visualizerContainer.querySelectorAll('.beat-bar');
        bars.forEach((bar, i) => {
            const delay = i * 0.015;
            const barEnergy = energy + (Math.sin(i * 0.5) * 0.2 + 0.5) * subBassEnergy * 0.3;
            const barHue = (i / bars.length) * 360 + (subBassEnergy * 30);
            setTimeout(() => {
                bar.style.background = `hsla(${barHue}, 80%, ${45 + barEnergy * 35}%, ${0.6 + barEnergy * 0.4})`;
                bar.style.boxShadow = `0 0 ${Math.floor(barEnergy * 25)}px hsla(${barHue}, 80%, 50%, ${0.4 + barEnergy * 0.4})`;
                bar.style.transform = `scaleY(${1 + barEnergy * 0.3})`;
                bar.classList.add('beat-flash');
                setTimeout(() => {
                    bar.style.background = '';
                    bar.style.boxShadow = '';
                    bar.style.transform = '';
                    bar.classList.remove('beat-flash');
                }, 250);
            }, delay);
        });
        
        if (combinedEnergy > 0.8) {
            createBeatParticles(visualizerContainer, combinedEnergy, hue);
        }
    }
    
    function createBeatParticles(container, energy, hue) {
        const particleCount = Math.floor(energy * 12);
        for (let i = 0; i < particleCount; i++) {
            const particle = document.createElement('div');
            particle.className = 'beat-particle';
            
            const angle = (Math.PI * 2 / particleCount) * i;
            const radius = 50 + Math.random() * 100;
            const dx = Math.cos(angle) * radius;
            const dy = Math.sin(angle) * radius;
            
            particle.style.cssText = `
                position: absolute;
                top: 50%;
                left: 50%;
                width: ${4 + energy * 6}px;
                height: ${4 + energy * 6}px;
                background: hsla(${hue}, 80%, 60%, ${0.6 + energy * 0.4});
                border-radius: 50%;
                pointer-events: none;
                z-index: 101;
                animation: beat-particle-spread 0.5s ease-out forwards;
                --particle-dx: ${dx}px;
                --particle-dy: ${dy}px;
            `;
            
            container.appendChild(particle);
            setTimeout(() => particle.remove(), 500);
        }
    }
    
    function updateBeatVisualization() {
        if (!analyser) return;
        
        const dataArray = new Uint8Array(analyser.frequencyBinCount);
        analyser.getByteFrequencyData(dataArray);
        
        const bpmStat = document.getElementById('bpm-stat');
        const intensityStat = document.getElementById('intensity-stat');
        const syncStat = document.getElementById('sync-stat');
        
        let sum = 0;
        let maxFreq = 0;
        let subBass = 0, bass = 0, lowMid = 0, mid = 0, highMid = 0, treble = 0;
        
        const bands = {
            subBass: { start: 0, end: 4, value: 0, count: 0 },
            bass: { start: 4, end: 10, value: 0, count: 0 },
            lowMid: { start: 10, end: 18, value: 0, count: 0 },
            mid: { start: 18, end: 28, value: 0, count: 0 },
            highMid: { start: 28, end: 40, value: 0, count: 0 },
            treble: { start: 40, end: 64, value: 0, count: 0 }
        };
        
        for (let i = 0; i < dataArray.length; i++) {
            sum += dataArray[i];
            if (dataArray[i] > maxFreq) maxFreq = dataArray[i];
            
            for (const [bandName, band] of Object.entries(bands)) {
                if (i >= band.start && i < band.end) {
                    band.value += dataArray[i];
                    band.count++;
                }
            }
            
            if (mediaVisualizerBars[i]) {
                const normalizedValue = dataArray[i] / 255;
                const height = Math.max(5, normalizedValue * 120);
                const hue = (i / dataArray.length) * 360;
                
                mediaVisualizerBars[i].style.height = `${height}px`;
                mediaVisualizerBars[i].style.background = `hsla(${hue}, 80%, ${40 + normalizedValue * 30}%, ${0.7 + normalizedValue * 0.3})`;
                mediaVisualizerBars[i].style.boxShadow = `0 0 ${Math.floor(normalizedValue * 20)}px hsla(${hue}, 80%, 50%, ${0.3 + normalizedValue * 0.4})`;
                
                if (dataArray[i] > 220) {
                    mediaVisualizerBars[i].classList.add('peak');
                    mediaVisualizerBars[i].style.transform = 'scale(1.05)';
                } else {
                    mediaVisualizerBars[i].classList.remove('peak');
                    mediaVisualizerBars[i].style.transform = 'scale(1)';
                }
            }
        }
        
        for (const [bandName, band] of Object.entries(bands)) {
            const avg = band.count > 0 ? band.value / band.count : 0;
            const bandEl = document.getElementById(`band-${bandName}`);
            if (bandEl) {
                const targetHeight = Math.max(10, (avg / 255) * 100);
                const currentHeight = parseFloat(bandEl.style.height) || targetHeight;
                const smoothedHeight = currentHeight + (targetHeight - currentHeight) * 0.3;
                bandEl.style.height = `${smoothedHeight}%`;
                bandEl.style.background = getBandColor(bandName, avg / 255);
                bandEl.style.boxShadow = `0 0 ${Math.floor((avg / 255) * 15)}px ${getBandColor(bandName, avg / 255)}`;
                
                if (avg > 200) {
                    bandEl.style.filter = 'brightness(1.3)';
                    bandEl.style.transform = 'scaleX(1.1)';
                } else {
                    bandEl.style.filter = 'brightness(1)';
                    bandEl.style.transform = 'scaleX(1)';
                }
            }
        }
        
        const avgIntensity = sum / dataArray.length;
        const bassAvg = bands.bass.value / bands.bass.count;
        const subBassAvg = bands.subBass.value / bands.subBass.count;
        
        detectBeat(dataArray);
        
        if (intensityStat) intensityStat.textContent = `${Math.floor((avgIntensity / 255) * 100)}%`;
        if (syncStat) syncStat.textContent = mediaSyncActive ? mediaSyncMode.charAt(0).toUpperCase() + mediaSyncMode.slice(1) : 'Off';
        
        if (mediaSyncActive && mediaSyncMode === 'beat') {
            updateLifxFromAudio(dataArray, maxFreq, bassAvg, subBassAvg);
        }
        
        requestAnimationFrame(updateBeatVisualization);
    }
    
    function getBandColor(bandName, intensity) {
        const colors = {
            subBass: `hsla(0, 80%, ${50 + intensity * 20}%, ${0.6 + intensity * 0.4})`,
            bass: `hsla(30, 80%, ${50 + intensity * 20}%, ${0.6 + intensity * 0.4})`,
            lowMid: `hsla(60, 80%, ${50 + intensity * 20}%, ${0.6 + intensity * 0.4})`,
            mid: `hsla(120, 80%, ${50 + intensity * 20}%, ${0.6 + intensity * 0.4})`,
            highMid: `hsla(180, 80%, ${50 + intensity * 20}%, ${0.6 + intensity * 0.4})`,
            treble: `hsla(240, 80%, ${50 + intensity * 20}%, ${0.6 + intensity * 0.4})`
        };
        return colors[bandName] || colors.mid;
    }
    
    let lastLifxUpdate = 0;
    let lifxColorHistory = { hue: 0, saturation: 50, brightness: 30 };
    
    function updateLifxFromAudio(dataArray, peak, bassAvg, subBassAvg) {
        const now = Date.now();
        if (now - lastLifxUpdate < 50) return;
        lastLifxUpdate = now;
        
        const normalizedPeak = peak / 255;
        const normalizedBass = bassAvg / 255;
        const normalizedSubBass = subBassAvg / 255;
        
        const bassImpact = normalizedSubBass > 0.7 ? 1.3 : 1.0;
        const beatBoost = normalizedSubBass > 0.75 ? 1.2 : 1.0;
        
        const targetHue = ((Date.now() / 30) + (normalizedBass * 60)) % 360;
        const targetSaturation = Math.min(100, 50 + normalizedPeak * 50);
        const targetBrightness = Math.min(100, 30 + normalizedBass * 70);
        
        lifxColorHistory.hue = lifxColorHistory.hue + (targetHue - lifxColorHistory.hue) * 0.2;
        lifxColorHistory.saturation = lifxColorHistory.saturation + (targetSaturation - lifxColorHistory.saturation) * 0.3;
        lifxColorHistory.brightness = lifxColorHistory.brightness + (targetBrightness - lifxColorHistory.brightness) * 0.3;
        
        const smoothedHue = lifxColorHistory.hue;
        const smoothedSaturation = lifxColorHistory.saturation;
        const smoothedBrightness = Math.min(100, lifxColorHistory.brightness * bassImpact * beatBoost);
        const temperature = 2700 + (normalizedPeak * 2300);
        
        if (typeof LifXTouchControls !== 'undefined') {
            const event = new CustomEvent('lifx-media-sync', {
                detail: {
                    hue: smoothedHue,
                    saturation: smoothedSaturation,
                    brightness: smoothedBrightness,
                    temperature: temperature,
                    intensity: normalizedPeak,
                    bassEnergy: normalizedBass,
                    subBassEnergy: normalizedSubBass,
                    isBeat: normalizedSubBass > 0.75,
                    beatBoost: beatBoost > 1.0
                }
            });
            document.dispatchEvent(event);
        }
    }
    
    // Enhanced Circular Color Picker for LIFX
    function createCircularColorPicker() {
        if (!CONFIG.enableCircularColorPicker) return;
        
        const picker = document.createElement('div');
        picker.className = 'circular-color-picker';
        picker.innerHTML = `
            <div class="color-wheel-container">
                <canvas class="color-wheel" width="300" height="300"></canvas>
                <div class="color-selector"></div>
            </div>
            <div class="color-preview"></div>
            <div class="color-sliders">
                <div class="slider-group">
                    <label><i class="fas fa-sun"></i> Brightness</label>
                    <input type="range" class="brightness-slider" min="0" max="100" value="50">
                </div>
                <div class="slider-group">
                    <label><i class="fas fa-thermometer-half"></i> Temperature</label>
                    <input type="range" class="temp-slider" min="1500" max="9000" value="3500">
                </div>
            </div>
            <div class="quick-colors">
                <button class="quick-color" data-color="#FF0000" style="background: #FF0000"></button>
                <button class="quick-color" data-color="#FF8800" style="background: #FF8800"></button>
                <button class="quick-color" data-color="#FFFF00" style="background: #FFFF00"></button>
                <button class="quick-color" data-color="#00FF00" style="background: #00FF00"></button>
                <button class="quick-color" data-color="#00FFFF" style="background: #00FFFF"></button>
                <button class="quick-color" data-color="#0088FF" style="background: #0088FF"></button>
                <button class="quick-color" data-color="#0000FF" style="background: #0000FF"></button>
                <button class="quick-color" data-color="#FF00FF" style="background: #FF00FF"></button>
                <button class="quick-color" data-color="#FFFFFF" style="background: #FFFFFF"></button>
                <button class="quick-color" data-color="#FFB6C1" style="background: #FFB6C1"></button>
                <button class="quick-color" data-color="#87CEEB" style="background: #87CEEB"></button>
                <button class="quick-color" data-color="#DDA0DD" style="background: #DDA0DD"></button>
            </div>
        `;
        document.body.appendChild(picker);
        
        // Initialize color wheel
        const canvas = picker.querySelector('.color-wheel');
        const ctx = canvas.getContext('2d');
        const centerX = 150;
        const centerY = 150;
        const radius = 140;
        
        // Draw color wheel
        for (let angle = 0; angle < 360; angle++) {
            const startAngle = (angle - 1) * Math.PI / 180;
            const endAngle = (angle + 1) * Math.PI / 180;
            
            ctx.beginPath();
            ctx.moveTo(centerX, centerY);
            ctx.arc(centerX, centerY, radius, startAngle, endAngle);
            ctx.closePath();
            
            const gradient = ctx.createRadialGradient(centerX, centerY, 0, centerX, centerY, radius);
            gradient.addColorStop(0, 1, 'hsl(' + angle + ', 0%, 50%)');
            gradient.addColorStop(1, 'hsl(' + angle + ', 100%, 50%)');
            ctx.fillStyle = gradient;
            ctx.fill();
        }
        
        // Add selector
        const selector = picker.querySelector('.color-selector');
        let isDragging = false;
        
        canvas.addEventListener('mousedown', startDrag);
        canvas.addEventListener('touchstart', startDrag);
        document.addEventListener('mousemove', drag);
        document.addEventListener('touchmove', drag);
        document.addEventListener('mouseup', endDrag);
        document.addEventListener('touchend', endDrag);
        
        function startDrag(e) {
            isDragging = true;
            updateColorFromPosition(e);
        }
        
        function drag(e) {
            if (!isDragging) return;
            e.preventDefault();
            updateColorFromPosition(e);
        }
        
        function endDrag() {
            isDragging = false;
        }
        
        function updateColorFromPosition(e) {
            const rect = canvas.getBoundingClientRect();
            const clientX = e.touches ? e.touches[0].clientX : e.clientX;
            const clientY = e.touches ? e.touches[0].clientY : e.clientY;
            const x = clientX - rect.left - centerX;
            const y = clientY - rect.top - centerY;
            
            const angle = Math.atan2(y, x) * 180 / Math.PI;
            const distance = Math.sqrt(x * x + y * y);
            
            const hue = Math.round((angle + 360) % 360);
            const saturation = Math.min(100, Math.round((distance / radius) * 100));
            
            selector.style.left = (centerX + x) + 'px';
            selector.style.top = (centerY + y) + 'px';
            
            const color = `hsl(${hue}, ${saturation}%, 50%)`;
            picker.querySelector('.color-preview').style.background = color;
            picker.dataset.selectedColor = color;
            picker.dataset.hue = hue;
            picker.dataset.saturation = saturation;
            
            // Dispatch event
            const event = new CustomEvent('color-picker-change', {
                detail: { hue, saturation, color }
            });
            document.dispatchEvent(event);
        }
        
        // Brightness slider
        picker.querySelector('.brightness-slider').addEventListener('input', (e) => {
            picker.dataset.brightness = e.target.value;
            const event = new CustomEvent('color-picker-brightness', {
                detail: { brightness: e.target.value }
            });
            document.dispatchEvent(event);
        });
        
        // Temperature slider
        picker.querySelector('.temp-slider').addEventListener('input', (e) => {
            picker.dataset.temperature = e.target.value;
            const event = new CustomEvent('color-picker-temperature', {
                detail: { temperature: e.target.value }
            });
            document.dispatchEvent(event);
        });
        
        // Quick color buttons
        picker.querySelectorAll('.quick-color').forEach(btn => {
            btn.addEventListener('click', () => {
                const color = btn.dataset.color;
                picker.querySelector('.color-preview').style.background = color;
                picker.dataset.selectedColor = color;
                
                // Parse hex to HSL
                const hsl = hexToHsl(color);
                picker.dataset.hue = hsl.h;
                picker.dataset.saturation = hsl.s;
                
                const event = new CustomEvent('color-picker-quick', {
                    detail: { color, ...hsl }
                });
                document.dispatchEvent(event);
                
                if (CONFIG.hapticFeedback) {
                    navigator.vibrate?.(10);
                }
            });
        });
        
        return picker;
    }
    
    function hexToHsl(hex) {
        const result = /^#?([a-f\d]{2})([a-f\d]{2})([a-f\d]{2})$/i.exec(hex);
        if (!result) return { h: 0, s: 0, l: 0 };
        
        let r = parseInt(result[1], 16) / 255;
        let g = parseInt(result[2], 16) / 255;
        let b = parseInt(result[3], 16) / 255;
        
        const max = Math.max(r, g, b), min = Math.min(r, g, b);
        let h, s, l = (max + min) / 2;
        
        if (max === min) {
            h = s = 0;
        } else {
            const d = max - min;
            s = l > 0.5 ? d / (2 - max - min) : d / (max + min);
            switch (max) {
                case r: h = ((g - b) / d + (g < b ? 6 : 0)) / 6; break;
                case g: h = ((b - r) / d + 2) / 6; break;
                case b: h = ((r - g) / d + 4) / 6; break;
            }
        }
        
        return { h: Math.round(h * 360), s: Math.round(s * 100), l: Math.round(l * 100) };
    }
    
    function interpolateHexColors(hex1, hex2, progress) {
        const r1 = parseInt(hex1.slice(1, 3), 16);
        const g1 = parseInt(hex1.slice(3, 5), 16);
        const b1 = parseInt(hex1.slice(5, 7), 16);
        
        const r2 = parseInt(hex2.slice(1, 3), 16);
        const g2 = parseInt(hex2.slice(3, 5), 16);
        const b2 = parseInt(hex2.slice(5, 7), 16);
        
        const r = Math.round(r1 + (r2 - r1) * progress);
        const g = Math.round(g1 + (g2 - g1) * progress);
        const b = Math.round(b1 + (b2 - b1) * progress);
        
        return `#${((1 << 24) + (r << 16) + (g << 8) + b).toString(16).slice(1)}`;
    }
    
    function interpolateHsl(hsl1, hsl2, progress) {
        const h = hsl1.h + (hsl2.h - hsl1.h) * progress;
        const s = hsl1.s + (hsl2.s - hsl1.s) * progress;
        const l = hsl1.l + (hsl2.l - hsl1.l) * progress;
        return `hsl(${h}, ${s}%, ${l}%)`;
    }
    
    // Enhanced LIFX Scene Presets with more options
    const extendedScenePresets = {
        ...scenePresets,
        'meditation': { color: '#9370DB', brightness: 45, temperature: 3500 },
        'gaming': { color: '#FF1493', brightness: 85, temperature: 4000 },
        'cooking': { color: '#FFA500', brightness: 95, temperature: 4500 },
        'creative': { color: '#FF69B4', brightness: 70, temperature: 5000 },
        'yoga': { color: '#98FB98', brightness: 50, temperature: 3800 },
        'movie': { color: '#4B0082', brightness: 35, temperature: 2700 },
        'study': { color: '#87CEEB', brightness: 65, temperature: 4800 },
        'dinner': { color: '#FF6347', brightness: 55, temperature: 3000 },
        'morning': { color: '#FFD700', brightness: 55, temperature: 4000 },
        'goodnight': { color: '#191970', brightness: 20, temperature: 2200 },
        'rainbow': { effect: 'rainbow', brightness: 100, duration: 5 },
        'fireplace': { color: '#FF4500', brightness: 65, temperature: 2200, flicker: true },
        'ice': { color: '#B0E0E6', brightness: 75, temperature: 8000 },
        'aurora': { color: '#00FF7F', brightness: 65, temperature: 5500, effect: 'aurora' },
        'nebula': { color: '#9400D3', brightness: 55, temperature: 6000 },
        'thunder': { color: '#87CEEB', brightness: 85, temperature: 7000, flash: true },
        'crystal': { color: '#E0FFFF', brightness: 70, temperature: 6500 },
        'lagoon': { color: '#40E0D0', brightness: 60, temperature: 4200 },
        'cotton_candy': { color: '#FFB6C1', brightness: 75, temperature: 3500 },
        'spring_blossom': { color: '#FFB7C5', brightness: 65, temperature: 3800 },
        'punchbowl': { color: '#FF69B4', brightness: 90, temperature: 4000 },
        'smashing': { color: '#FF1493', brightness: 95, temperature: 4500 },
        'glitter': { color: '#FFD700', brightness: 85, temperature: 5000, sparkle: true },
        'golden_hour': { color: '#FFD700', brightness: 50, temperature: 3200 },
        'late_night': { color: '#483D8B', brightness: 25, temperature: 2400 },
        'midday': { color: '#FFFFFF', brightness: 100, temperature: 5500 },
        'polar': { color: '#F0FFFF', brightness: 75, temperature: 7500 },
        'cosmic': { color: '#4B0082', brightness: 65, temperature: 6000 },
        'dream': { color: '#D8BFD8', brightness: 45, temperature: 4000 },
        'chill': { color: '#5F9EA0', brightness: 50, temperature: 3500 },
        'adventure': { color: '#FF8C00', brightness: 80, temperature: 4800 },
        'festival': { color: '#FF1493', brightness: 90, temperature: 4200 },
        'bioluminescent': { color: '#00FFFF', brightness: 60, temperature: 5500 },
        'cyberpunk': { color: '#FF00FF', brightness: 75, temperature: 4500 },
        'vaporwave': { color: '#FF69B4', brightness: 70, temperature: 4000 },
        'northern_lights': { color: '#00FF7F', brightness: 65, temperature: 5000 },
        'desert_dawn': { color: '#FF7F50', brightness: 55, temperature: 3600 },
        'forest_mist': { color: '#8FBC8F', brightness: 50, temperature: 4500 },
        'volcanic': { color: '#DC143C', brightness: 75, temperature: 2800 },
        'underwater': { color: '#00BFFF', brightness: 60, temperature: 5200 },
        'space_station': { color: '#F0F8FF', brightness: 80, temperature: 6500 },
        'wizard_tower': { color: '#8B00FF', brightness: 55, temperature: 3000 },
        'dragon_fire': { color: '#FF4500', brightness: 85, temperature: 2500 },
        'fairy_grove': { color: '#98FB98', brightness: 65, temperature: 4200 },
        'haunted': { color: '#800080', brightness: 45, temperature: 3500 },
        'santas_workshop': { color: '#FF0000', brightness: 75, temperature: 3000 },
        'new_year': { color: '#FFD700', brightness: 90, temperature: 4500 },
        'valentines': { color: '#FF69B4', brightness: 60, temperature: 3200 },
        'halloween': { color: '#FF6600', brightness: 70, temperature: 2800 },
        'thanksgiving': { color: '#D2691E', brightness: 55, temperature: 3000 },
        'christmas': { color: '#006400', brightness: 80, temperature: 3500 },
        'easter': { color: '#FFB6C1', brightness: 75, temperature: 4500 },
        'st_patricks': { color: '#008000', brightness: 70, temperature: 4000 },
        'independence_day': { color: '#FF0000', brightness: 85, temperature: 5000 }
    };
    
    // Apply enhanced scene with visual feedback
    function applyEnhancedScene(sceneName, options = {}) {
        const preset = extendedScenePresets[sceneName];
        if (!preset) return;
        
        console.log(`[TouchMediaEnhancements] Applying enhanced scene: ${sceneName}`);
        
        // Add to recent scenes
        if (CONFIG.enableColorHistory) {
            const recentScenes = JSON.parse(localStorage.getItem('recentScenes') || '[]');
            recentScenes.unshift(sceneName);
            recentScenes = recentScenes.slice(0, 10);
            localStorage.setItem('recentScenes', JSON.stringify(recentScenes));
        }
        
        const event = new CustomEvent('apply-lifx-scene-enhanced', { 
            detail: { 
                name: sceneName, 
                ...preset, 
                ...options 
            } 
        });
        document.dispatchEvent(event);
        
        // Show enhanced scene change with color preview
        const colorHex = preset.color || '#27a0b9';
        showSceneChange(sceneName, colorHex);
        
        // Trigger haptic feedback
        if (CONFIG.hapticFeedback) {
            navigator.vibrate?.([15, 30, 15]);
        }
        
        // Handle special effects
        if (preset.effect === 'party' || preset.effect === 'rainbow') {
            activatePartyMode(true);
        } else {
            activatePartyMode(false);
        }
        
        if (sceneName === 'bedtime' || sceneName === 'night' || sceneName === 'goodnight') {
            setBedtimeMode(true);
        } else {
            setBedtimeMode(false);
        }
    }
    
    function transitionSceneSmooth(fromScene, toScene, duration = 2000) {
        const fromPreset = extendedScenePresets[fromScene];
        const toPreset = extendedScenePresets[toScene];
        
        if (!fromPreset || !toPreset) {
            applyEnhancedScene(toScene);
            return;
        }
        
        const startTime = Date.now();
        const fromColor = fromPreset.color || '#00d4ff';
        const toColor = toPreset.color || '#00d4ff';
        const fromBrightness = fromPreset.brightness || 50;
        const toBrightness = toPreset.brightness || 50;
        const fromTemp = fromPreset.temperature || 4000;
        const toTemp = toPreset.temperature || 4000;
        
        function easeInOut(t) {
            return t < 0.5 ? 2 * t * t : -1 + (4 - 2 * t) * t;
        }
        
        function animate() {
            const elapsed = Date.now() - startTime;
            const progress = Math.min(1, elapsed / duration);
            const easedProgress = easeInOut(progress);
            
            const currentColor = interpolateHexColors(fromColor, toColor, easedProgress);
            const currentBrightness = fromBrightness + (toBrightness - fromBrightness) * easedProgress;
            const currentTemp = fromTemp + (toTemp - fromTemp) * easedProgress;
            
            const event = new CustomEvent('lifx-scene-transition', {
                detail: {
                    color: currentColor,
                    brightness: currentBrightness,
                    temperature: currentTemp,
                    progress: easedProgress,
                    fromScene,
                    toScene
                }
            });
            document.dispatchEvent(event);
            
            if (progress < 1) {
                requestAnimationFrame(animate);
            } else {
                applyEnhancedScene(toScene);
            }
        }
        
        requestAnimationFrame(animate);
    }
    
    function sceneSequence(scenes, options = {}) {
        const { loop = false, transitionDuration = 2000, pauseBetween = 1000 } = options;
        let currentIndex = 0;
        let isRunning = true;
        
        function playNext() {
            if (!isRunning) return;
            
            const currentScene = scenes[currentIndex];
            const nextScene = scenes[(currentIndex + 1) % scenes.length];
            
            transitionSceneSmooth(currentScene, nextScene, {
                duration: transitionDuration
            });
            
            currentIndex = (currentIndex + 1) % scenes.length;
            
            if (currentIndex === 0 && !loop) {
                isRunning = false;
                return;
            }
            
            setTimeout(playNext, transitionDuration + pauseBetween);
        }
        
        playNext();
        
        return () => { isRunning = false; };
    }
    
    function startAmbientMode() {
        console.log('[TouchMediaEnhancements] Starting ambient mode');
        const ambientInterval = setInterval(() => {
            if (!mediaPlaybackState.isPlaying) {
                clearInterval(ambientInterval);
                return;
            }
            const colors = ['#27a0b9', '#1f8999', '#00d4ff', '#00b4d8', '#90e0ef'];
            const randomColor = colors[Math.floor(Math.random() * colors.length)];
            const event = new CustomEvent('apply-lifx-color', { detail: { color: randomColor, brightness: 0.6 } });
            document.dispatchEvent(event);
        }, 5000);
        return ambientInterval;
    }
    
    function startMediaVisualizer() {
        console.log('[TouchMediaEnhancements] Starting media visualizer');
        const visualizer = document.createElement('div');
        visualizer.className = 'media-visualizer-overlay';
        visualizer.innerHTML = '<div class="visualizer-bars"></div>';
        document.body.appendChild(visualizer);
        
        const barsContainer = visualizer.querySelector('.visualizer-bars');
        for (let i = 0; i < 32; i++) {
            const bar = document.createElement('div');
            bar.className = 'visualizer-bar';
            bar.style.animationDelay = `${i * 0.05}s`;
            barsContainer.appendChild(bar);
        }
        
        return visualizer;
    }
    
    function createMediaFloatingWidget() {
        const widget = document.createElement('div');
        widget.className = 'media-floating-widget';
        widget.innerHTML = `
            <div class="media-widget-artwork"></div>
            <div class="media-widget-info">
                <div class="media-widget-title">Now Playing</div>
                <div class="media-widget-artist">Unknown Artist</div>
            </div>
            <div class="media-widget-controls">
                <button class="media-widget-btn" data-action="previous"><i class="fas fa-step-backward"></i></button>
                <button class="media-widget-btn" data-action="playpause"><i class="fas fa-play"></i></button>
                <button class="media-widget-btn" data-action="next"><i class="fas fa-step-forward"></i></button>
            </div>
            <button class="media-widget-close"><i class="fas fa-times"></i></button>
        `;
        widget.style.cssText = `
            position: fixed;
            bottom: 20px;
            right: 20px;
            background: rgba(30, 30, 45, 0.95);
            border: 1px solid rgba(39, 160, 185, 0.3);
            border-radius: 16px;
            padding: 15px;
            display: flex;
            align-items: center;
            gap: 12px;
            z-index: 9999;
            backdrop-filter: blur(10px);
            box-shadow: 0 8px 32px rgba(0, 0, 0, 0.4);
            transition: transform 0.3s ease, opacity 0.3s ease;
        `;
        document.body.appendChild(widget);
        
        widget.querySelector('.media-widget-close').addEventListener('click', () => {
            widget.style.transform = 'scale(0.8)';
            widget.style.opacity = '0';
            setTimeout(() => widget.remove(), 300);
        });
        
        return widget;
    }
    
    function syncLightsToMedia(audioData) {
        if (!audioData || !mediaPlaybackState.isPlaying) return;
        
        const { frequency, amplitude } = audioData;
        const hue = (frequency / 256) * 360;
        const brightness = Math.min(100, (amplitude / 256) * 100);
        
        const event = new CustomEvent('apply-lifx-color', { 
            detail: { 
                color: `hue:${hue},saturation:80%,brightness:${brightness}%`,
                duration: 0.1
            } 
        });
        document.dispatchEvent(event);
    }
    
    function startFrequencyAnalysis() {
        console.log('[TouchMediaEnhancements] Starting frequency analysis');
        if (!audioContext) {
            audioContext = new (window.AudioContext || window.webkitAudioContext)();
            analyser = audioContext.createAnalyser();
            analyser.fftSize = 256;
        }
        return analyser;
    }
    
    function createSpectrumAnalyzer() {
        const analyzer = document.createElement('div');
        analyzer.className = 'spectrum-analyzer';
        analyzer.innerHTML = `
            <div class="spectrum-bars"></div>
            <div class="spectrum-labels">
                <span>20Hz</span>
                <span>1kHz</span>
                <span>20kHz</span>
            </div>
        `;
        analyzer.style.cssText = `
            position: fixed;
            bottom: 0;
            left: 0;
            right: 0;
            height: 120px;
            background: linear-gradient(to top, rgba(39, 160, 185, 0.2), transparent);
            z-index: 9998;
            pointer-events: none;
        `;
        document.body.appendChild(analyzer);
        
        const barsContainer = analyzer.querySelector('.spectrum-bars');
        for (let i = 0; i < 64; i++) {
            const bar = document.createElement('div');
            bar.className = 'spectrum-bar';
            bar.style.cssText = `
                width: ${100/64}%;
                height: 10px;
                background: linear-gradient(to top, #27a0b9, #00d4ff);
                border-radius: 2px 2px 0 0;
                display: inline-block;
                transition: height 0.1s ease;
            `;
            barsContainer.appendChild(bar);
        }
        
        return analyzer;
    }
    
    function applyDynamicColorShift(options = {}) {
        const { duration = 30000, colors = ['#ff0000', '#00ff00', '#0000ff', '#ffff00', '#ff00ff', '#00ffff'] } = options;
        let currentIndex = 0;
        let startTime = Date.now();
        
        const shiftColor = () => {
            const elapsed = Date.now() - startTime;
            if (elapsed >= duration) return;
            
            currentIndex = (currentIndex + 1) % colors.length;
            const event = new CustomEvent('apply-lifx-color', { 
                detail: { color: colors[currentIndex], duration: duration / colors.length } 
            });
            document.dispatchEvent(event);
            
            setTimeout(shiftColor, duration / colors.length);
        };
        
        shiftColor();
    }
    
    function startRhythmSync(bpm) {
        console.log('[TouchMediaEnhancements] Starting rhythm sync at', bpm, 'BPM');
        const interval = 60000 / bpm;
        let beatCount = 0;
        
        const rhythmInterval = setInterval(() => {
            if (!mediaPlaybackState.isPlaying) {
                clearInterval(rhythmInterval);
                return;
            }
            
            beatCount++;
            const brightness = beatCount % 4 === 0 ? 100 : 70;
            const event = new CustomEvent('apply-lifx-brightness', { detail: { brightness: brightness / 100, duration: 0.05 } });
            document.dispatchEvent(event);
        }, interval);
        
        return rhythmInterval;
    }
    
    function createMediaPlaylistUI(playlist) {
        const playlistUI = document.createElement('div');
        playlistUI.className = 'media-playlist-ui';
        playlistUI.innerHTML = `
            <div class="playlist-header">
                <h4><i class="fas fa-list"></i> Playlist</h4>
                <button class="playlist-close"><i class="fas fa-times"></i></button>
            </div>
            <div class="playlist-tracks"></div>
        `;
        
        const tracksContainer = playlistUI.querySelector('.playlist-tracks');
        playlist.forEach((track, index) => {
            const trackEl = document.createElement('div');
            trackEl.className = 'playlist-track';
            trackEl.innerHTML = `
                <span class="track-number">${index + 1}</span>
                <div class="track-info">
                    <div class="track-title">${track.title}</div>
                    <div class="track-artist">${track.artist}</div>
                </div>
                <span class="track-duration">${track.duration}</span>
            `;
            tracksContainer.appendChild(trackEl);
        });
        
        playlistUI.style.cssText = `
            position: fixed;
            top: 50%;
            left: 50%;
            transform: translate(-50%, -50%);
            background: rgba(30, 30, 45, 0.95);
            border: 1px solid rgba(39, 160, 185, 0.3);
            border-radius: 16px;
            padding: 20px;
            max-width: 400px;
            max-height: 500px;
            overflow-y: auto;
            z-index: 10000;
        `;
        
        playlistUI.querySelector('.playlist-close').addEventListener('click', () => {
            playlistUI.remove();
        });
        
        document.body.appendChild(playlistUI);
        return playlistUI;
    }
    
    function showMediaLyrics(lyrics) {
        const lyricsEl = document.createElement('div');
        lyricsEl.className = 'media-lyrics-display';
        lyricsEl.innerHTML = `
            <div class="lyrics-content">${lyrics}</div>
            <button class="lyrics-close"><i class="fas fa-times"></i></button>
        `;
        
        lyricsEl.style.cssText = `
            position: fixed;
            top: 50%;
            left: 50%;
            transform: translate(-50%, -50%);
            background: rgba(30, 30, 45, 0.95);
            border: 1px solid rgba(39, 160, 185, 0.3);
            border-radius: 16px;
            padding: 30px;
            max-width: 600px;
            max-height: 400px;
            overflow-y: auto;
            z-index: 10000;
            text-align: center;
            white-space: pre-wrap;
            line-height: 1.8;
        `;
        
        lyricsEl.querySelector('.lyrics-close').addEventListener('click', () => {
            lyricsEl.remove();
        });
        
        document.body.appendChild(lyricsEl);
        return lyricsEl;
    }
    
    function toggleImmersiveMode() {
        const isImmersive = document.body.classList.toggle('immersive-mode');
        
        if (isImmersive) {
            document.body.style.cssText = `
                overflow: hidden;
            `;
            document.querySelectorAll('.sidebar, .navbar').forEach(el => {
                el.style.opacity = '0';
                el.style.transition = 'opacity 0.5s ease';
            });
        } else {
            document.body.style.cssText = '';
            document.querySelectorAll('.sidebar, .navbar').forEach(el => {
                el.style.opacity = '1';
            });
        }
        
        return isImmersive;
    }
    
    function createMediaQueuePanel(queue) {
        const queuePanel = document.createElement('div');
        queuePanel.className = 'media-queue-panel';
        queuePanel.innerHTML = `
            <div class="queue-header">
                <h4><i class="fas fa-layer-group"></i> Up Next</h4>
                <button class="queue-close"><i class="fas fa-times"></i></button>
            </div>
            <div class="queue-items"></div>
        `;
        
        const itemsContainer = queuePanel.querySelector('.queue-items');
        queue.forEach((item, index) => {
            const itemEl = document.createElement('div');
            itemEl.className = 'queue-item';
            itemEl.innerHTML = `
                <span class="queue-position">${index + 1}</span>
                <div class="queue-info">
                    <div class="queue-title">${item.title}</div>
                    <div class="queue-source">${item.source}</div>
                </div>
            `;
            itemsContainer.appendChild(itemEl);
        });
        
        queuePanel.style.cssText = `
            position: fixed;
            right: 20px;
            top: 100px;
            background: rgba(30, 30, 45, 0.95);
            border: 1px solid rgba(39, 160, 185, 0.3);
            border-radius: 16px;
            padding: 15px;
            width: 300px;
            max-height: 400px;
            overflow-y: auto;
            z-index: 9999;
        `;
        
        queuePanel.querySelector('.queue-close').addEventListener('click', () => {
            queuePanel.remove();
        });
        
        document.body.appendChild(queuePanel);
        return queuePanel;
    }
    
    function startBpmTracking() {
        console.log('[TouchMediaEnhancements] Starting BPM tracking');
        const bpmDisplay = document.createElement('div');
        bpmDisplay.className = 'bpm-tracking-display';
        bpmDisplay.innerHTML = `
            <i class="fas fa-heartbeat"></i>
            <span class="bpm-value">--</span>
            <span class="bpm-label">BPM</span>
        `;
        
        bpmDisplay.style.cssText = `
            position: fixed;
            top: 20px;
            right: 20px;
            background: rgba(30, 30, 45, 0.9);
            border: 2px solid #00d4ff;
            border-radius: 12px;
            padding: 10px 15px;
            display: flex;
            align-items: center;
            gap: 8px;
            z-index: 9999;
            font-size: 18px;
            font-weight: bold;
        `;
        
        document.body.appendChild(bpmDisplay);
        
        const updateBpmDisplay = (bpm) => {
            const valueEl = bpmDisplay.querySelector('.bpm-value');
            if (valueEl) {
                valueEl.textContent = bpm || '--';
            }
        };
        
        return { display: bpmDisplay, updateBpm: updateBpmDisplay };
    }
    
    function createNowPlayingWidget() {
        const widget = document.createElement('div');
        widget.className = 'now-playing-widget';
        widget.innerHTML = `
            <div class="np-artwork"></div>
            <div class="np-info">
                <div class="np-title">Now Playing</div>
                <div class="np-artist">Unknown Artist</div>
                <div class="np-progress">
                    <div class="np-progress-bar"></div>
                </div>
            </div>
            <div class="np-controls">
                <button class="np-btn" data-action="previous"><i class="fas fa-step-backward"></i></button>
                <button class="np-btn np-play" data-action="playpause"><i class="fas fa-play"></i></button>
                <button class="np-btn" data-action="next"><i class="fas fa-step-forward"></i></button>
            </div>
        `;
        
        widget.style.cssText = `
            position: fixed;
            bottom: 80px;
            right: 20px;
            background: rgba(30, 30, 45, 0.95);
            border: 1px solid rgba(39, 160, 185, 0.3);
            border-radius: 16px;
            padding: 15px;
            display: flex;
            align-items: center;
            gap: 15px;
            z-index: 9999;
            backdrop-filter: blur(10px);
            box-shadow: 0 8px 32px rgba(0, 0, 0, 0.4);
            min-width: 300px;
        `;
        
        document.body.appendChild(widget);
        return widget;
    }
    
    // Enhanced LIFX Zone-Based Effects
    function applyZoneEffect(zoneConfig) {
        const { zones, effect, duration = 5 } = zoneConfig;
        
        zones.forEach((zone, index) => {
            setTimeout(() => {
                const event = new CustomEvent('lifx-zone-effect', {
                    detail: {
                        zone: zone.id,
                        effect: effect,
                        color: zone.color,
                        brightness: zone.brightness || 50,
                        temperature: zone.temperature || 4000
                    }
                });
                document.dispatchEvent(event);
            }, index * (duration / zones.length) * 1000);
        });
    }
    
    function createWaveEffect(direction = 'left-to-right') {
        const zonePresets = {
            'left-to-right': [
                { id: 'zone1', color: '#00d4ff', brightness: 80 },
                { id: 'zone2', color: '#00ff88', brightness: 80 },
                { id: 'zone3', color: '#ffff00', brightness: 80 },
                { id: 'zone4', color: '#ff8800', brightness: 80 },
                { id: 'zone5', color: '#ff0080', brightness: 80 }
            ],
            'right-to-left': [
                { id: 'zone5', color: '#00d4ff', brightness: 80 },
                { id: 'zone4', color: '#00ff88', brightness: 80 },
                { id: 'zone3', color: '#ffff00', brightness: 80 },
                { id: 'zone2', color: '#ff8800', brightness: 80 },
                { id: 'zone1', color: '#ff0080', brightness: 80 }
            ],
            'center-out': [
                { id: 'zone3', color: '#00d4ff', brightness: 100 },
                { id: 'zone2', color: '#00ff88', brightness: 80 },
                { id: 'zone4', color: '#00ff88', brightness: 80 },
                { id: 'zone1', color: '#ffff00', brightness: 60 },
                { id: 'zone5', color: '#ffff00', brightness: 60 }
            ],
            'rainbow': [
                { id: 'zone1', color: '#ff0000', brightness: 80 },
                { id: 'zone2', color: '#ff8800', brightness: 80 },
                { id: 'zone3', color: '#ffff00', brightness: 80 },
                { id: 'zone4', color: '#00ff00', brightness: 80 },
                { id: 'zone5', color: '#0088ff', brightness: 80 }
            ]
        };
        
        applyZoneEffect({
            zones: zonePresets[direction] || zonePresets['left-to-right'],
            effect: 'wave',
            duration: 2
        });
    }
    
    function createPulseZoneEffect(centerZone, radius = 2) {
        const colors = ['#00d4ff', '#00ff88', '#ffff00'];
        const zones = [];
        
        for (let i = 0; i <= radius; i++) {
            const zoneId = `zone${Math.max(1, Math.min(5, centerZone + i))}`;
            zones.push({
                id: zoneId,
                color: colors[i % colors.length],
                brightness: 100 - (i * 20)
            });
        }
        
        applyZoneEffect({
            zones,
            effect: 'pulse-expand',
            duration: 1.5
        });
    }
    
    // Improved Scene Transitions with crossfade and smooth interpolation
    function transitionScene(fromScene, toScene, options = {}) {
        const {
            duration = 2000,
            easing = 'ease-in-out',
            crossfade = true,
            intermediateScenes = []
        } = options;
        
        const fromPreset = extendedScenePresets[fromScene];
        const toPreset = extendedScenePresets[toScene];
        
        if (!fromPreset || !toPreset) {
            applyEnhancedScene(toScene);
            return;
        }
        
        const startTime = Date.now();
        
        function interpolate(start, end, progress) {
            return start + (end - start) * progress;
        }
        
        function easeInOut(t) {
            return t < 0.5 ? 2 * t * t : -1 + (4 - 2 * t) * t;
        }
        
        function animate() {
            const elapsed = Date.now() - startTime;
            const progress = Math.min(1, elapsed / duration);
            const easedProgress = easeInOut(progress);
            
            const currentColor = interpolateHexColors(
                fromPreset.color || '#00d4ff',
                toPreset.color || '#00d4ff',
                easedProgress
            );
            
            const currentBrightness = interpolate(
                fromPreset.brightness || 50,
                toPreset.brightness || 50,
                easedProgress
            );
            
            const currentTemp = interpolate(
                fromPreset.temperature || 4000,
                toPreset.temperature || 4000,
                easedProgress
            );
            
            const event = new CustomEvent('lifx-scene-transition', {
                detail: {
                    color: currentColor,
                    brightness: currentBrightness,
                    temperature: currentTemp,
                    progress: easedProgress,
                    fromScene,
                    toScene
                }
            });
            document.dispatchEvent(event);
            
            if (progress < 1) {
                requestAnimationFrame(animate);
            } else {
                applyEnhancedScene(toScene);
            }
        }
        
        requestAnimationFrame(animate);
    }
    
    function interpolateHexColors(hex1, hex2, progress) {
        const r1 = parseInt(hex1.slice(1, 3), 16);
        const g1 = parseInt(hex1.slice(3, 5), 16);
        const b1 = parseInt(hex1.slice(5, 7), 16);
        
        const r2 = parseInt(hex2.slice(1, 3), 16);
        const g2 = parseInt(hex2.slice(3, 5), 16);
        const b2 = parseInt(hex2.slice(5, 7), 16);
        
        const r = Math.round(interpolate(r1, r2, progress));
        const g = Math.round(interpolate(g1, g2, progress));
        const b = Math.round(interpolate(b1, b2, progress));
        
        return `#${((1 << 24) + (r << 16) + (g << 8) + b).toString(16).slice(1)}`;
    }
    
    function createSceneSequence(scenes, options = {}) {
        const { loop = false, transitionDuration = 2000, pauseBetween = 1000 } = options;
        let currentIndex = 0;
        let isRunning = true;
        
        function playNext() {
            if (!isRunning) return;
            
            const currentScene = scenes[currentIndex];
            const nextScene = scenes[(currentIndex + 1) % scenes.length];
            
            transitionScene(currentScene, nextScene, {
                duration: transitionDuration
            });
            
            currentIndex = (currentIndex + 1) % scenes.length;
            
            if (currentIndex === 0 && !loop) {
                isRunning = false;
                return;
            }
            
            setTimeout(playNext, transitionDuration + pauseBetween);
        }
        
        playNext();
        
        return () => { isRunning = false; };
    }
    
    // Sync lights to music rhythm with advanced patterns
    function setupRhythmSync(bpm, pattern = 'four-on-floor') {
        const patterns = {
            'four-on-floor': [1, 0, 0, 0],
            'rock': [1, 0, 0.5, 0],
            'waltz': [1, 0.3, 0.3, 0, 0, 0],
            'syncopated': [1, 0, 0.7, 0, 0.5, 0, 0.3, 0],
            'double-time': [1, 0.5, 1, 0.5],
            'triplet': [1, 0, 0.7, 1, 0, 0.7]
        };
        
        const patternData = patterns[pattern] || patterns['four-on-floor'];
        const intervalMs = (60 / bpm) * 1000;
        let beatIndex = 0;
        
        const rhythmInterval = setInterval(() => {
            if (!mediaPlaybackState.isPlaying) {
                clearInterval(rhythmInterval);
                return;
            }
            
            const intensity = patternData[beatIndex % patternData.length];
            const brightness = 30 + (intensity * 70);
            
            const event = new CustomEvent('lifx-rhythm-sync', {
                detail: {
                    beatIndex,
                    intensity,
                    brightness,
                    bpm,
                    pattern
                }
            });
            document.dispatchEvent(event);
            
            beatIndex++;
        }, intervalMs);
        
        return rhythmInterval;
    }
    
    // Public API
    return {
        init,
        updateBpm,
        showSceneChange,
        setBedtimeMode,
        addToSelection,
        removeFromSelection,
        clearSelection,
        applyToSelected,
        activatePartyMode,
        applyScenePreset,
        applyEnhancedScene,
        createCircularColorPicker,
        setupColorTemperaturePicker,
        updateMediaVisualization,
        updateBeatDetectionStats,
        updateMediaSyncDashboard,
        toggleAudioAnalysis,
        toggleLightSync,
        setTouchSensitivity,
        toggleMediaSyncPanel,
        setMediaSyncMode,
        toggleQuickScenesPanel,
        applyQuickScene,
        mediaPlayPause,
        mediaPrevious,
        mediaNext,
        updateMediaPlayback,
        updateMiniPlayer,
        showNowPlayingToast,
        hideNowPlaying,
        toggleMiniPlayer,
        makeDraggable,
        CONFIG,
        extendedScenePresets,
        startAmbientMode,
        startMediaVisualizer,
        createMediaFloatingWidget,
        syncLightsToMedia,
        startFrequencyAnalysis,
        createSpectrumAnalyzer,
        applyDynamicColorShift,
        startRhythmSync,
        createMediaPlaylistUI,
        showMediaLyrics,
        toggleImmersiveMode,
        createMediaQueuePanel,
        startBpmTracking,
        createNowPlayingWidget,
        applyZoneEffect,
        createWaveEffect,
        createPulseZoneEffect,
        transitionScene,
        createSceneSequence,
        setupRhythmSync,
        resetCalibration,
        getCalibrationData: () => calibrationData,
        isCalibrating: () => calibrationData.isCalibrating
    };
})();

// Initialize on DOM ready
document.addEventListener('DOMContentLoaded', function() {
    TouchMediaEnhancements.init();
    TouchMediaEnhancements.setupColorTemperaturePicker();
    
    // Create circular color picker if enabled
    if (TouchMediaEnhancements.CONFIG.enableCircularColorPicker) {
        TouchMediaEnhancements.createCircularColorPicker();
    }
    
    console.log('[TouchMediaEnhancements] Fully initialized with all enhancements');
});

// Global functions for HTML onclick handlers
function toggleMediaSyncPanel() {
    TouchMediaEnhancements.toggleMediaSyncPanel();
}

function setMediaSyncMode(mode) {
    TouchMediaEnhancements.setMediaSyncMode(mode);
}

function toggleQuickScenesPanel() {
    TouchMediaEnhancements.toggleQuickScenesPanel();
}

function applyQuickScene(sceneName) {
    TouchMediaEnhancements.applyQuickScene(sceneName);
}

function mediaPlayPause() {
    TouchMediaEnhancements.mediaPlayPause();
}

function mediaPrevious() {
    TouchMediaEnhancements.mediaPrevious();
}

function mediaNext() {
    TouchMediaEnhancements.mediaNext();
}
