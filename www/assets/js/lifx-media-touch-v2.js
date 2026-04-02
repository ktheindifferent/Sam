/**
 * SAM LIFX Media Touch Controls V2
 * Enhanced touch interface with media center integration and advanced LIFX controls
 */

(function() {
    'use strict';

    const LIFXMediaTouchV2 = {
        config: {
            touchSensitivity: 'medium',
            enableHapticFeedback: true,
            enableGestureTrails: true,
            enableMediaSync: true,
            beatDetectionThreshold: 0.75,
            colorTransitionDuration: 1000,
            gestureHoldDuration: 500,
            swipeThreshold: 50,
            doubleTapDelay: 300,
            rippleDuration: 400,
            gestureTrailSize: 20,
            enableVelocityRipples: true,
            enableSwipeTrails: true,
            enableMultiSelectDrag: true,
            enableAdaptiveSensitivity: true,
            minSwipeVelocity: 0.3,
            maxTouchHistory: 10,
            gestureTrailDecay: 0.8,
            maxVelocityRipples: 5,
            calibrationSamples: 20,
            beatCalibrationSamples: 30,
            visualizationMode: 'bars',
            hapticPatterns: {
                tap: [10],
                doubleTap: [15, 50, 15],
                longPress: [50, 50, 50],
                swipe: [20],
                beat: [25],
                success: [10, 50, 10, 50, 10],
                gesture: [15, 30, 15],
                calibration: [20, 40, 20, 40, 20]
            }
        },

        state: {
            selectedBulbs: new Set(),
            activeScene: null,
            activeEffect: null,
            mediaSyncActive: false,
            mediaSyncMode: 'beat',
            bpmDetected: 0,
            brightnessLevel: 50,
            colorTemperature: 4000,
            partyModeActive: false,
            bedtimeModeActive: false,
            circadianActive: false,
            lastTouchTime: 0,
            lastTapTime: 0,
            tapCount: 0,
            gestureHistory: [],
            touchHoldProgress: 0,
            isTouchHoldActive: false,
            frequencyData: new Uint8Array(6),
            beatHistory: [],
            adaptiveThreshold: 0.75,
            bpmHistory: [],
            bpmSmoothed: 0,
            lastBeatTime: 0,
            lastBassEnergy: 0,
            touchVelocity: null,
            gestureScale: 1,
            sensitivityCalibrated: false,
            baselineEnergy: 128,
            visualizationMode: 'bars',
            gestureCalibrationData: [],
            touchSensitivityMap: {
                low: { threshold: 0.85, multiplier: 0.7 },
                medium: { threshold: 0.75, multiplier: 1.0 },
                high: { threshold: 0.65, multiplier: 1.3 },
                custom: { threshold: 0.75, multiplier: 1.0 }
            },
            lastGestureTime: 0,
            gestureDebounce: 50,
            calibrationInProgress: false,
            touchAccuracyScore: 100,
            beatCalibrationData: [],
            beatCalibrationInProgress: false,
            beatSensitivity: 0.7,
            visualizationModeUnlocked: false
        },

        scenePresets: [
            { id: 'relax', name: 'Relax', icon: '🧘', hue: 5800, saturation: 15000, brightness: 26214, kelvin: 2700 },
            { id: 'focus', name: 'Focus', icon: '🎯', hue: 19000, saturation: 8000, brightness: 52428, kelvin: 5000 },
            { id: 'energize', name: 'Energize', icon: '⚡', hue: 41000, saturation: 20000, brightness: 65535, kelvin: 6500 },
            { id: 'night', name: 'Night', icon: '🌙', hue: 5800, saturation: 10000, brightness: 13107, kelvin: 2000 },
            { id: 'reading', name: 'Reading', icon: '📚', hue: 19000, saturation: 5000, brightness: 45875, kelvin: 4500 },
            { id: 'romance', name: 'Romance', icon: '💕', hue: 60000, saturation: 25000, brightness: 32767, kelvin: 3000 },
            { id: 'party', name: 'Party', icon: '🎉', hue: 43680, saturation: 65535, brightness: 65535, kelvin: 5500 },
            { id: 'sunset', name: 'Sunset', icon: '🌅', hue: 7098, saturation: 40000, brightness: 39321, kelvin: 2500 },
            { id: 'arctic', name: 'Arctic', icon: '❄️', hue: 32760, saturation: 15000, brightness: 52428, kelvin: 7000 },
            { id: 'golden', name: 'Golden', icon: '☀️', hue: 8000, saturation: 30000, brightness: 45875, kelvin: 3200 },
            { id: 'ocean', name: 'Ocean', icon: '🌊', hue: 34580, saturation: 42598, brightness: 49151, kelvin: 4000 },
            { id: 'tropical', name: 'Tropical', icon: '🏖️', hue: 27300, saturation: 65535, brightness: 47185, kelvin: 3800 },
            { id: 'meditation', name: 'Meditation', icon: '🧘', hue: 50960, saturation: 19660, brightness: 22937, kelvin: 2400 },
            { id: 'gaming', name: 'Gaming', icon: '🎮', hue: 50960, saturation: 52428, brightness: 58982, kelvin: 5500 },
            { id: 'movie', name: 'Movie', icon: '🎬', hue: 3640, saturation: 19660, brightness: 22937, kelvin: 2200 },
            { id: 'morning', name: 'Morning', icon: '🌄', hue: 9100, saturation: 32767, brightness: 55705, kelvin: 5500 },
            { id: 'goodnight', name: 'Goodnight', icon: '😴', hue: 43680, saturation: 6553, brightness: 6553, kelvin: 2000 },
            { id: 'rainbow', name: 'Rainbow', icon: '🌈', hue: 0, saturation: 65535, brightness: 52428, kelvin: 4000 },
            { id: 'fireplace', name: 'Fireplace', icon: '🔥', hue: 5460, saturation: 52428, brightness: 39321, kelvin: 2000 },
            { id: 'ice', name: 'Ice', icon: '🧊', hue: 36400, saturation: 32767, brightness: 45875, kelvin: 8000 },
            { id: 'aurora', name: 'Aurora', icon: '🌌', hue: 32760, saturation: 45875, brightness: 49151, kelvin: 6000 },
            { id: 'nebula', name: 'Nebula', icon: '🌠', hue: 50960, saturation: 52428, brightness: 45875, kelvin: 4500 },
            { id: 'thunder', name: 'Thunder', icon: '⛈️', hue: 5460, saturation: 39321, brightness: 58982, kelvin: 5000 },
            { id: 'crystal', name: 'Crystal', icon: '💎', hue: 34580, saturation: 26214, brightness: 52428, kelvin: 7500 },
            { id: 'cyberpunk', name: 'Cyberpunk', icon: '🤖', hue: 30940, saturation: 52428, brightness: 58982, kelvin: 4500 },
            { id: 'vaporwave', name: 'Vaporwave', icon: '🌴', hue: 58240, saturation: 39321, brightness: 52428, kelvin: 4000 },
            { id: 'halloween', name: 'Halloween', icon: '🎃', hue: 5460, saturation: 52428, brightness: 49151, kelvin: 2800 },
            { id: 'christmas', name: 'Christmas', icon: '🎄', hue: 5800, saturation: 45875, brightness: 55705, kelvin: 3500 },
            { id: 'beach', name: 'Beach', icon: '🏖️', hue: 18200, saturation: 32767, brightness: 52428, kelvin: 5000 },
            { id: 'forest', name: 'Forest', icon: '🌲', hue: 25480, saturation: 39321, brightness: 42598, kelvin: 4200 },
            { id: 'yoga', name: 'Yoga', icon: '🧘', hue: 25480, saturation: 26214, brightness: 39321, kelvin: 3800 },
            { id: 'cooking', name: 'Cooking', icon: '🍳', hue: 5460, saturation: 32767, brightness: 58982, kelvin: 4500 },
            { id: 'creative', name: 'Creative', icon: '🎨', hue: 58240, saturation: 45875, brightness: 52428, kelvin: 5000 },
            { id: 'dinner', name: 'Dinner', icon: '🍽️', hue: 6000, saturation: 26214, brightness: 32767, kelvin: 3000 },
            { id: 'spa', name: 'Spa', icon: '💆', hue: 32760, saturation: 19660, brightness: 26214, kelvin: 3500 },
            { id: 'festival', name: 'Festival', icon: '🎪', hue: 27300, saturation: 58982, brightness: 65535, kelvin: 4200 },
            { id: 'spring', name: 'Spring', icon: '🌸', hue: 27300, saturation: 32767, brightness: 52428, kelvin: 4500 },
            { id: 'autumn', name: 'Autumn', icon: '🍂', hue: 7098, saturation: 45875, brightness: 42598, kelvin: 3200 },
            { id: 'winter', name: 'Winter', icon: '❄️', hue: 32760, saturation: 26214, brightness: 58982, kelvin: 6500 },
            { id: 'summer', name: 'Summer', icon: '☀️', hue: 9100, saturation: 39321, brightness: 65535, kelvin: 5500 },
            { id: 'rain', name: 'Rain', icon: '🌧️', hue: 34580, saturation: 32767, brightness: 32767, kelvin: 5000 },
            { id: 'fog', name: 'Fog', icon: '🌫️', hue: 0, saturation: 6553, brightness: 26214, kelvin: 6000 },
            { id: 'snow', name: 'Snow', icon: '🌨️', hue: 32760, saturation: 13107, brightness: 52428, kelvin: 7000 },
            { id: 'desert', name: 'Desert', icon: '🏜️', hue: 7098, saturation: 26214, brightness: 58982, kelvin: 4000 },
            { id: 'jungle', name: 'Jungle', icon: '🌴', hue: 25480, saturation: 52428, brightness: 45875, kelvin: 4200 },
            { id: 'savanna', name: 'Savanna', icon: '🦁', hue: 9100, saturation: 32767, brightness: 52428, kelvin: 3800 },
            { id: 'canyon', name: 'Canyon', icon: '🏜️', hue: 5460, saturation: 39321, brightness: 45875, kelvin: 3500 },
            { id: 'volcano', name: 'Volcano', icon: '🌋', hue: 5460, saturation: 58982, brightness: 52428, kelvin: 2500 },
            { id: 'geyser', name: 'Geyser', icon: '🌊', hue: 32760, saturation: 19660, brightness: 45875, kelvin: 5500 },
            { id: 'lagoon', name: 'Lagoon', icon: '💧', hue: 34580, saturation: 45875, brightness: 49151, kelvin: 4500 },
            { id: 'reef', name: 'Reef', icon: '🐠', hue: 27300, saturation: 52428, brightness: 52428, kelvin: 4800 },
            { id: 'abyss', name: 'Abyss', icon: '🌊', hue: 50960, saturation: 39321, brightness: 19660, kelvin: 3000 },
            { id: 'galaxy', name: 'Galaxy', icon: '🌌', hue: 50960, saturation: 58982, brightness: 39321, kelvin: 5000 },
            { id: 'pulsar', name: 'Pulsar', icon: '⭐', hue: 43680, saturation: 65535, brightness: 65535, kelvin: 8000 },
            { id: 'comet', name: 'Comet', icon: '☄️', hue: 9100, saturation: 45875, brightness: 58982, kelvin: 5500 },
            { id: 'meteor', name: 'Meteor', icon: '💫', hue: 5460, saturation: 52428, brightness: 65535, kelvin: 6000 },
            { id: 'eclipse', name: 'Eclipse', icon: '🌑', hue: 0, saturation: 0, brightness: 6553, kelvin: 2000 },
            { id: 'solstice', name: 'Solstice', icon: '🌞', hue: 9100, saturation: 52428, brightness: 65535, kelvin: 6000 },
            { id: 'equinox', name: 'Equinox', icon: '⚖️', hue: 27300, saturation: 32767, brightness: 52428, kelvin: 4500 },
            { id: 'dawn', name: 'Dawn', icon: '🌄', hue: 9100, saturation: 26214, brightness: 39321, kelvin: 4000 },
            { id: 'dusk', name: 'Dusk', icon: '🌆', hue: 7098, saturation: 32767, brightness: 32767, kelvin: 3500 },
            { id: 'midnight', name: 'Midnight', icon: '🌃', hue: 50960, saturation: 39321, brightness: 13107, kelvin: 2500 },
            { id: 'noon', name: 'Noon', icon: '☀️', hue: 9100, saturation: 19660, brightness: 65535, kelvin: 6500 },
            { id: 'twilight', name: 'Twilight', icon: '🌇', hue: 50960, saturation: 26214, brightness: 26214, kelvin: 3000 },
            { id: 'starlight', name: 'Starlight', icon: '✨', hue: 32760, saturation: 19660, brightness: 32767, kelvin: 5500 },
            { id: 'moonlight', name: 'Moonlight', icon: '🌙', hue: 32760, saturation: 13107, brightness: 19660, kelvin: 4000 },
            { id: 'sunlight', name: 'Sunlight', icon: '☀️', hue: 9100, saturation: 26214, brightness: 65535, kelvin: 5800 },
            { id: 'candlelight', name: 'Candlelight', icon: '🕯️', hue: 5460, saturation: 32767, brightness: 19660, kelvin: 2000 },
            { id: 'lamplight', name: 'Lamplight', icon: '🏮', hue: 7098, saturation: 26214, brightness: 32767, kelvin: 2700 },
            { id: 'neon', name: 'Neon', icon: '💡', hue: 43680, saturation: 65535, brightness: 58982, kelvin: 5000 },
            { id: 'LED', name: 'LED', icon: '💡', hue: 32760, saturation: 52428, brightness: 65535, kelvin: 6000 },
            { id: 'incandescent', name: 'Incandescent', icon: '💡', hue: 5460, saturation: 19660, brightness: 45875, kelvin: 2700 },
            { id: 'fluorescent', name: 'Fluorescent', icon: '💡', hue: 32760, saturation: 6553, brightness: 58982, kelvin: 6500 },
            { id: 'halogen', name: 'Halogen', icon: '💡', hue: 7098, saturation: 13107, brightness: 52428, kelvin: 3200 },
            { id: 'mercury', name: 'Mercury', icon: '💡', hue: 32760, saturation: 19660, brightness: 58982, kelvin: 5500 },
            { id: 'sodium', name: 'Sodium', icon: '💡', hue: 9100, saturation: 52428, brightness: 45875, kelvin: 2500 },
            { id: 'xenon', name: 'Xenon', icon: '💡', hue: 0, saturation: 6553, brightness: 65535, kelvin: 6000 },
            { id: 'plasma', name: 'Plasma', icon: '⚡', hue: 43680, saturation: 58982, brightness: 65535, kelvin: 7000 },
            { id: 'arc', name: 'Arc', icon: '⚡', hue: 32760, saturation: 52428, brightness: 65535, kelvin: 6500 },
            { id: 'spark', name: 'Spark', icon: '✨', hue: 9100, saturation: 45875, brightness: 65535, kelvin: 5500 },
            { id: 'ember', name: 'Ember', icon: '🔥', hue: 5460, saturation: 45875, brightness: 32767, kelvin: 2200 },
            { id: 'flame', name: 'Flame', icon: '🔥', hue: 7098, saturation: 52428, brightness: 45875, kelvin: 2500 },
            { id: 'inferno', name: 'Inferno', icon: '🔥', hue: 5460, saturation: 65535, brightness: 65535, kelvin: 3000 },
            { id: 'blaze', name: 'Blaze', icon: '🔥', hue: 9100, saturation: 58982, brightness: 58982, kelvin: 2800 },
            { id: 'bonfire', name: 'Bonfire', icon: '🔥', hue: 7098, saturation: 45875, brightness: 52428, kelvin: 2400 },
            { id: 'campfire', name: 'Campfire', icon: '🔥', hue: 5460, saturation: 39321, brightness: 45875, kelvin: 2300 },
            { id: 'hearth', name: 'Hearth', icon: '🏠', hue: 7098, saturation: 32767, brightness: 39321, kelvin: 2600 },
            { id: 'forge', name: 'Forge', icon: '⚒️', hue: 5460, saturation: 52428, brightness: 58982, kelvin: 2800 },
            { id: 'kiln', name: 'Kiln', icon: '🏺', hue: 9100, saturation: 39321, brightness: 52428, kelvin: 3000 },
            { id: 'furnace', name: 'Furnace', icon: '🔥', hue: 7098, saturation: 45875, brightness: 49151, kelvin: 2700 },
            { id: 'concentrate', name: 'Concentrate', icon: '🧠', hue: 18200, saturation: 12000, brightness: 58982, kelvin: 5200 },
            { id: 'chromatic', name: 'Chromatic', icon: '🎨', hue: 21840, saturation: 49152, brightness: 52428, kelvin: 4800 },
            { id: 'magnify', name: 'Magnify', icon: '🔍', hue: 16380, saturation: 19660, brightness: 49151, kelvin: 4600 },
            { id: 'contemplation', name: 'Contemplation', icon: '💭', hue: 49140, saturation: 26214, brightness: 32767, kelvin: 3800 },
            { id: 'faded', name: 'Faded', icon: '👻', hue: 0, saturation: 3276, brightness: 39321, kelvin: 5000 },
            { id: 'calm', name: 'Calm', icon: '🕊️', hue: 34580, saturation: 19660, brightness: 32767, kelvin: 3500 },
            { id: 'serenity', name: 'Serenity', icon: '🌺', hue: 32760, saturation: 26214, brightness: 39321, kelvin: 4000 },
            { id: 'tranquility', name: 'Tranquility', icon: '🪷', hue: 50960, saturation: 13107, brightness: 26214, kelvin: 3200 },
            { id: 'peace', name: 'Peace', icon: '☮️', hue: 18200, saturation: 19660, brightness: 45875, kelvin: 4200 },
            { id: 'harmony', name: 'Harmony', icon: '☯️', hue: 25480, saturation: 32767, brightness: 42598, kelvin: 3800 },
            { id: 'balance', name: 'Balance', icon: '⚖️', hue: 27300, saturation: 26214, brightness: 49151, kelvin: 4500 },
            { id: 'zen', name: 'Zen', icon: '🎋', hue: 32760, saturation: 19660, brightness: 39321, kelvin: 4000 },
            { id: 'mindfulness', name: 'Mindfulness', icon: '🧠', hue: 18200, saturation: 15000, brightness: 52428, kelvin: 5000 },
            { id: 'breathe', name: 'Breathe', icon: '🌬️', hue: 34580, saturation: 13107, brightness: 32767, kelvin: 3800 },
            { id: 'restore', name: 'Restore', icon: '🔄', hue: 18200, saturation: 19660, brightness: 45875, kelvin: 4500 },
            { id: 'rejuvenate', name: 'Rejuvenate', icon: '💧', hue: 34580, saturation: 26214, brightness: 42598, kelvin: 4200 },
            { id: 'refresh', name: 'Refresh', icon: '🍃', hue: 25480, saturation: 32767, brightness: 49151, kelvin: 4800 },
            { id: 'revitalize', name: 'Revitalize', icon: '⚡', hue: 9100, saturation: 39321, brightness: 55705, kelvin: 5500 },
            { id: 'awaken', name: 'Awaken', icon: '👁️', hue: 18200, saturation: 26214, brightness: 52428, kelvin: 5200 },
            { id: 'inspire', name: 'Inspire', icon: '💡', hue: 43680, saturation: 45875, brightness: 58982, kelvin: 5000 },
            { id: 'motivate', name: 'Motivate', icon: '🔥', hue: 7098, saturation: 52428, brightness: 58982, kelvin: 4500 },
            { id: 'empower', name: 'Empower', icon: '💪', hue: 5460, saturation: 45875, brightness: 65535, kelvin: 5000 },
            { id: 'celebrate', name: 'Celebrate', icon: '🎊', hue: 27300, saturation: 58982, brightness: 65535, kelvin: 4800 },
            { id: 'joy', name: 'Joy', icon: '😊', hue: 9100, saturation: 45875, brightness: 58982, kelvin: 4500 },
            { id: 'happiness', name: 'Happiness', icon: '😄', hue: 8000, saturation: 39321, brightness: 55705, kelvin: 4200 },
            { id: 'bliss', name: 'Bliss', icon: '😌', hue: 32760, saturation: 32767, brightness: 45875, kelvin: 4000 },
            { id: 'euphoria', name: 'Euphoria', icon: '🌟', hue: 43680, saturation: 58982, brightness: 65535, kelvin: 5500 },
            { id: 'ecstasy', name: 'Ecstasy', icon: '💫', hue: 50960, saturation: 52428, brightness: 58982, kelvin: 5000 },
            { id: 'rapture', name: 'Rapture', icon: '😍', hue: 60000, saturation: 45875, brightness: 52428, kelvin: 4500 },
            { id: 'delight', name: 'Delight', icon: '🎁', hue: 27300, saturation: 39321, brightness: 52428, kelvin: 4200 },
            { id: 'pleasure', name: 'Pleasure', icon: '🍫', hue: 7098, saturation: 32767, brightness: 42598, kelvin: 3500 },
            { id: 'comfort', name: 'Comfort', icon: '🛋️', hue: 5460, saturation: 26214, brightness: 39321, kelvin: 3000 },
            { id: 'cozy', name: 'Cozy', icon: '🧸', hue: 5460, saturation: 32767, brightness: 32767, kelvin: 2700 },
            { id: 'warmth', name: 'Warmth', icon: '🔆', hue: 7098, saturation: 39321, brightness: 42598, kelvin: 2800 },
            { id: 'hygge', name: 'Hygge', icon: '🕯️', hue: 5460, saturation: 26214, brightness: 26214, kelvin: 2400 },
            { id: 'sanctuary', name: 'Sanctuary', icon: '🏛️', hue: 32760, saturation: 19660, brightness: 32767, kelvin: 3500 },
            { id: 'retreat', name: 'Retreat', icon: '🏡', hue: 25480, saturation: 26214, brightness: 39321, kelvin: 3800 },
            { id: 'haven', name: 'Haven', icon: '🕊️', hue: 34580, saturation: 19660, brightness: 39321, kelvin: 4000 },
            { id: 'oasis', name: 'Oasis', icon: '🌴', hue: 34580, saturation: 32767, brightness: 45875, kelvin: 4200 },
            { id: 'paradise', name: 'Paradise', icon: '🏝️', hue: 27300, saturation: 45875, brightness: 52428, kelvin: 4500 },
            { id: 'utopia', name: 'Utopia', icon: '🌈', hue: 0, saturation: 32767, brightness: 52428, kelvin: 5000 },
            { id: 'eden', name: 'Eden', icon: '🍎', hue: 25480, saturation: 39321, brightness: 45875, kelvin: 4200 },
            { id: 'shangri_la', name: 'Shangri-La', icon: '🏔️', hue: 32760, saturation: 26214, brightness: 42598, kelvin: 4500 },
            { id: 'camelot', name: 'Camelot', icon: '🏰', hue: 50960, saturation: 39321, brightness: 45875, kelvin: 4000 },
            { id: 'avalon', name: 'Avalon', icon: '🌫️', hue: 34580, saturation: 19660, brightness: 32767, kelvin: 3800 },
            { id: 'olympus', name: 'Olympus', icon: '⚡', hue: 0, saturation: 6553, brightness: 65535, kelvin: 6500 },
            { id: 'asgard', name: 'Asgard', icon: '⚔️', hue: 9100, saturation: 45875, brightness: 52428, kelvin: 5000 },
            { id: 'valhalla', name: 'Valhalla', icon: '🛡️', hue: 7098, saturation: 52428, brightness: 58982, kelvin: 4500 },
            { id: 'atlantis', name: 'Atlantis', icon: '🌊', hue: 34580, saturation: 45875, brightness: 42598, kelvin: 4800 },
            { id: 'lemuria', name: 'Lemuria', icon: '🔮', hue: 50960, saturation: 39321, brightness: 39321, kelvin: 4200 },
            { id: 'mu', name: 'Mu', icon: '🌏', hue: 27300, saturation: 32767, brightness: 45875, kelvin: 4500 },
            { id: 'hyperborea', name: 'Hyperborea', icon: '❄️', hue: 32760, saturation: 19660, brightness: 52428, kelvin: 7000 },
            { id: 'golden_age', name: 'Golden Age', icon: '👑', hue: 9100, saturation: 52428, brightness: 65535, kelvin: 5500 },
            { id: 'silver_age', name: 'Silver Age', icon: '🥈', hue: 32760, saturation: 13107, brightness: 58982, kelvin: 6000 },
            { id: 'bronze_age', name: 'Bronze Age', icon: '🥉', hue: 7098, saturation: 39321, brightness: 45875, kelvin: 3500 },
            { id: 'iron_age', name: 'Iron Age', icon: '⛓️', hue: 0, saturation: 6553, brightness: 39321, kelvin: 4000 },
            { id: 'stone_age', name: 'Stone Age', icon: '🪨', hue: 0, saturation: 0, brightness: 32767, kelvin: 4500 },
            { id: 'digital_age', name: 'Digital Age', icon: '💻', hue: 32760, saturation: 52428, brightness: 58982, kelvin: 6500 },
            { id: 'space_age', name: 'Space Age', icon: '🚀', hue: 0, saturation: 0, brightness: 65535, kelvin: 6500 },
            { id: 'atomic', name: 'Atomic', icon: '☢️', hue: 12000, saturation: 65535, brightness: 65535, kelvin: 6000 },
            { id: 'quantum', name: 'Quantum', icon: '⚛️', hue: 43680, saturation: 52428, brightness: 65535, kelvin: 6500 },
            { id: 'cosmic', name: 'Cosmic', icon: '🌌', hue: 50960, saturation: 45875, brightness: 52428, kelvin: 5500 },
            { id: 'universal', name: 'Universal', icon: '♾️', hue: 0, saturation: 0, brightness: 65535, kelvin: 6000 },
            { id: 'infinite', name: 'Infinite', icon: '🔁', hue: 0, saturation: 0, brightness: 65535, kelvin: 5500 },
            { id: 'eternal', name: 'Eternal', icon: '⏳', hue: 50960, saturation: 39321, brightness: 45875, kelvin: 4500 },
            { id: 'timeless', name: 'Timeless', icon: '🕰️', hue: 32760, saturation: 19660, brightness: 39321, kelvin: 4000 },
            { id: 'ancient', name: 'Ancient', icon: '🏺', hue: 7098, saturation: 32767, brightness: 39321, kelvin: 3000 },
            { id: 'primordial', name: 'Primordial', icon: '🌑', hue: 0, saturation: 0, brightness: 19660, kelvin: 2500 },
            { id: 'elemental', name: 'Elemental', icon: '🔥', hue: 0, saturation: 52428, brightness: 58982, kelvin: 3000 },
            { id: 'earth', name: 'Earth', icon: '🌍', hue: 25480, saturation: 39321, brightness: 45875, kelvin: 4000 },
            { id: 'air', name: 'Air', icon: '💨', hue: 32760, saturation: 13107, brightness: 52428, kelvin: 5500 },
            { id: 'fire', name: 'Fire', icon: '🔥', hue: 5460, saturation: 58982, brightness: 65535, kelvin: 2500 },
            { id: 'water', name: 'Water', icon: '💧', hue: 34580, saturation: 32767, brightness: 45875, kelvin: 4500 },
            { id: 'metal', name: 'Metal', icon: '🔩', hue: 0, saturation: 0, brightness: 58982, kelvin: 6000 },
            { id: 'wood', name: 'Wood', icon: '🪵', hue: 7098, saturation: 39321, brightness: 39321, kelvin: 3500 },
            { id: 'aether', name: 'Aether', icon: '✨', hue: 32760, saturation: 19660, brightness: 65535, kelvin: 7000 },
            { id: 'void', name: 'Void', icon: '⚫', hue: 0, saturation: 0, brightness: 6553, kelvin: 2000 },
            { id: 'light', name: 'Light', icon: '💡', hue: 9100, saturation: 6553, brightness: 65535, kelvin: 6500 },
            { id: 'dark', name: 'Dark', icon: '🌑', hue: 0, saturation: 0, brightness: 13107, kelvin: 2000 },
            { id: 'shadow', name: 'Shadow', icon: '🌒', hue: 0, saturation: 0, brightness: 19660, kelvin: 2500 },
            { id: 'twilight_zone', name: 'Twilight Zone', icon: '🌆', hue: 50960, saturation: 32767, brightness: 26214, kelvin: 3200 },
            { id: 'fifth_dimension', name: '5th Dimension', icon: '🌀', hue: 43680, saturation: 45875, brightness: 52428, kelvin: 5000 },
            { id: 'parallel', name: 'Parallel', icon: '↔️', hue: 32760, saturation: 39321, brightness: 45875, kelvin: 4500 },
            { id: 'dimension', name: 'Dimension', icon: '📐', hue: 18200, saturation: 32767, brightness: 52428, kelvin: 5000 },
            { id: 'matrix', name: 'Matrix', icon: '0️⃣', hue: 25480, saturation: 52428, brightness: 45875, kelvin: 4500 },
            { id: 'simulation', name: 'Simulation', icon: '🖥️', hue: 32760, saturation: 45875, brightness: 52428, kelvin: 5500 },
            { id: 'virtual', name: 'Virtual', icon: '🥽', hue: 43680, saturation: 52428, brightness: 58982, kelvin: 5500 },
            { id: 'augmented', name: 'Augmented', icon: '📱', hue: 27300, saturation: 45875, brightness: 58982, kelvin: 5000 },
            { id: 'mixed_reality', name: 'Mixed Reality', icon: '🔀', hue: 32760, saturation: 39321, brightness: 55705, kelvin: 5200 },
            { id: 'extended_reality', name: 'XR', icon: '🌐', hue: 43680, saturation: 45875, brightness: 58982, kelvin: 5500 },
            { id: 'metaverse', name: 'Metaverse', icon: '🪐', hue: 50960, saturation: 52428, brightness: 58982, kelvin: 5000 },
            { id: 'web3', name: 'Web3', icon: '🔗', hue: 32760, saturation: 45875, brightness: 55705, kelvin: 5500 },
            { id: 'blockchain', name: 'Blockchain', icon: '⛓️', hue: 43680, saturation: 39321, brightness: 52428, kelvin: 5000 },
            { id: 'crypto', name: 'Crypto', icon: '🪙', hue: 9100, saturation: 52428, brightness: 58982, kelvin: 5500 },
            { id: 'nft', name: 'NFT', icon: '🖼️', hue: 43680, saturation: 58982, brightness: 65535, kelvin: 5500 },
            { id: 'dao', name: 'DAO', icon: '🗳️', hue: 32760, saturation: 39321, brightness: 52428, kelvin: 5000 },
            { id: 'defi', name: 'DeFi', icon: '💰', hue: 9100, saturation: 45875, brightness: 58982, kelvin: 5000 },
            { id: 'smart_contract', name: 'Smart Contract', icon: '📜', hue: 25480, saturation: 39321, brightness: 52428, kelvin: 4800 }
        ],

        effectPresets: [
            { id: 'pulse', name: 'Pulse', icon: '💓', duration: 5, cycles: 3 },
            { id: 'rainbow', name: 'Rainbow Cycle', icon: '🌈', duration: 10, cycles: 2 },
            { id: 'strobe', name: 'Strobe', icon: '⚡', duration: 3, cycles: 10 },
            { id: 'fireplace', name: 'Fireplace', icon: '🔥', duration: 30, cycles: 1 },
            { id: 'aurora', name: 'Aurora', icon: '🌌', duration: 15, cycles: 1 },
            { id: 'breath', name: 'Breath', icon: '🌬️', duration: 8, cycles: 4 },
            { id: 'color_cycle', name: 'Color Cycle', icon: '🎨', duration: 20, cycles: 1 }
        ],

        mediaPresets: [
            { id: 'spotify', name: 'Spotify', icon: '🎵', service: 'spotify' },
            { id: 'youtube', name: 'YouTube', icon: '📺', service: 'youtube' },
            { id: 'youtube_music', name: 'YouTube Music', icon: '🎵', service: 'youtube-music' },
            { id: 'plex', name: 'Plex', icon: '🎬', service: 'plex' },
            { id: 'jellyfin', name: 'Jellyfin', icon: '🎬', service: 'jellyfin' },
            { id: 'emby', name: 'Emby', icon: '🎭', service: 'emby' },
            { id: 'radio', name: 'Radio', icon: '📻', service: 'radio' },
            { id: 'tidal', name: 'Tidal', icon: '🌊', service: 'tidal' },
            { id: 'apple_music', name: 'Apple Music', icon: '🎶', service: 'apple_music' },
            { id: 'soundcloud', name: 'SoundCloud', icon: '☁️', service: 'soundcloud' },
            { id: 'bandcamp', name: 'Bandcamp', icon: '🎸', service: 'bandcamp' },
            { id: 'deezer', name: 'Deezer', icon: '💙', service: 'deezer' },
            { id: 'qobuz', name: 'Qobuz', icon: '🎼', service: 'qobuz' },
            { id: 'audiobook', name: 'Audiobook', icon: '📚', service: 'audiobook' },
            { id: 'podcast', name: 'Podcast', icon: '🎙️', service: 'podcast' },
            { id: 'twitch', name: 'Twitch', icon: '🎮', service: 'twitch' },
            { id: 'netflix', name: 'Netflix', icon: '🎬', service: 'netflix' },
            { id: 'disney', name: 'Disney+', icon: '✨', service: 'disney' },
            { id: 'hulu', name: 'Hulu', icon: '🟢', service: 'hulu' },
            { id: 'prime', name: 'Prime Video', icon: '📦', service: 'prime' },
            { id: 'hbo', name: 'HBO Max', icon: '🎭', service: 'hbo' },
            { id: 'paramount', name: 'Paramount+', icon: '🏔️', service: 'paramount' },
            { id: 'peacock', name: 'Peacock', icon: '🦚', service: 'peacock' },
            { id: 'crunchyroll', name: 'Crunchyroll', icon: '👺', service: 'crunchyroll' },
            { id: 'funimation', name: 'Funimation', icon: '🎌', service: 'funimation' },
            { id: 'discord', name: 'Discord', icon: '💬', service: 'discord' },
            { id: 'zoom', name: 'Zoom', icon: '📹', service: 'zoom' },
            { id: 'teams', name: 'Teams', icon: '👥', service: 'teams' },
            { id: 'meet', name: 'Meet', icon: '📷', service: 'meet' },
            { id: 'game', name: 'Gaming', icon: '🎮', service: 'game' },
            { id: 'vr', name: 'VR', icon: '🥽', service: 'vr' },
            { id: 'stream', name: 'Streaming', icon: '📡', service: 'stream' },
            { id: 'record', name: 'Recording', icon: '🔴', service: 'record' },
            { id: 'edit', name: 'Editing', icon: '✂️', service: 'edit' },
            { id: 'present', name: 'Presentation', icon: '📊', service: 'present' }
        ],

        syncModePresets: [
            { id: 'beat', name: 'Beat Sync', icon: '💓', description: 'Lights pulse with the beat' },
            { id: 'color', name: 'Color Sync', icon: '🎨', description: 'Colors match the music mood' },
            { id: 'spectrum', name: 'Spectrum', icon: '🌈', description: 'Full frequency visualization' },
            { id: 'ambient', name: 'Ambient', icon: '🌊', description: 'Gentle ambient reactions' },
            { id: 'intense', name: 'Intense', icon: '⚡', description: 'High-energy light show' },
            { id: 'cinema', name: 'Cinema', icon: '🎬', description: 'Movie-optimized sync' },
            { id: 'party', name: 'Party Mode', icon: '🎉', description: 'Full party experience' },
            { id: 'chill', name: 'Chill', icon: '🧘', description: 'Relaxed lighting' },
            { id: 'focus', name: 'Focus', icon: '🎯', description: 'Minimal distraction' },
            { id: 'custom', name: 'Custom', icon: '⚙️', description: 'Your custom settings' }
        ],

        init() {
            this.loadTouchSensitivity();
            this.loadVisualizationPreference();
            this.setupTouchGestures();
            this.setupMediaPlayers();
            this.setupLightGroups();
            this.setupVolumeSliders();
            this.setupBrightnessSliders();
            this.setupSceneSelector();
            this.setupQuickActions();
            this.setupMediaPresets();
            this.setupColorPicker();
            this.setupEffectSelector();
            this.setupZoneControl();
            this.setupGestureHints();
            this.setupCalibrationButton();
            this.setupCleanupHandlers();
            this.syncStatus();
            this.startPeriodicSync();
            console.log('[LIFXMediaTouchV2] Initialized with sensitivity:', this.state.touchSensitivity);
        },

        setupTouchGestures() {
            const touchableElements = document.querySelectorAll('.lifx-bulb-control, .media-control-btn, .scene-btn');
            
            touchableElements.forEach(el => {
                el.addEventListener('touchstart', this.handleTouchStart.bind(this), { passive: true });
                el.addEventListener('touchmove', this.handleTouchMove.bind(this), { passive: true });
                el.addEventListener('touchend', this.handleTouchEnd.bind(this), { passive: true });
                el.addEventListener('click', this.handleTouchClick.bind(this));
            });

            document.addEventListener('gesturestart', this.handleGestureStart.bind(this));
            document.addEventListener('gesturechange', this.handleGestureChange.bind(this));
            document.addEventListener('gestureend', this.handleGestureEnd.bind(this));
        },

        handleTouchStart(e) {
            const target = e.currentTarget;
            if (!target) return;
            
            const touch = e.touches[0];
            if (!touch) return;
            
            const now = Date.now();
            if (now - this.state.lastGestureTime < this.state.gestureDebounce) {
                return;
            }
            this.state.lastGestureTime = now;
            
            target.dataset.touchStartX = touch.clientX;
            target.dataset.touchStartY = touch.clientY;
            target.dataset.touchStartTime = Date.now();
            target.dataset.lastTouchX = touch.clientX;
            target.dataset.lastTouchY = touch.clientY;
            
            target.classList.add('touch-active');
            
            this.triggerHaptic('tap');
            
            this.showEnhancedTouchRipple(e, target);
            this.startTouchHoldTimer(target);
            
            if (this.config.enableVelocityRipples) {
                this.touchVelocity = [];
            }
            
            if (this.state.calibrationInProgress) {
                this.recordCalibrationTouch(touch.clientX, touch.clientY);
            }
        },

        handleTouchMove(e) {
            const target = e.currentTarget;
            if (!target) return;
            
            const touch = e.touches[0];
            if (!touch) return;
            
            const startX = parseFloat(target.dataset.touchStartX || 0);
            const startY = parseFloat(target.dataset.touchStartY || 0);
            const lastX = parseFloat(target.dataset.lastTouchX || touch.clientX);
            const lastY = parseFloat(target.dataset.lastTouchY || touch.clientY);
            
            const deltaX = touch.clientX - startX;
            const deltaY = touch.clientY - startY;
            const instantDeltaX = touch.clientX - lastX;
            const instantDeltaY = touch.clientY - lastY;
            
            target.dataset.lastTouchX = touch.clientX;
            target.dataset.lastTouchY = touch.clientY;
            
            const movementThreshold = this.getMovementThreshold();
            if (Math.abs(deltaX) > movementThreshold || Math.abs(deltaY) > movementThreshold) {
                target.classList.remove('touch-active');
                this.cancelTouchHoldTimer();
            }
            
            if (this.config.enableGestureTrails) {
                this.showEnhancedGestureTrail(touch.clientX, touch.clientY, instantDeltaX, instantDeltaY);
            }
            
            if (this.config.enableVelocityRipples && this.touchVelocity) {
                this.touchVelocity.push({ x: instantDeltaX, y: instantDeltaY, time: Date.now() });
                if (this.touchVelocity.length > 5) {
                    this.touchVelocity.shift();
                }
            }
            
            const velocity = Math.sqrt(instantDeltaX * instantDeltaX + instantDeltaY * instantDeltaY);
            if (this.config.enableSwipeTrails && velocity > 2) {
                this.showSwipeTrail(touch.clientX, touch.clientY, instantDeltaX, instantDeltaY);
            }
            
            this.updateTouchHoldProgress(target, deltaX, deltaY);
            
            if (this.state.calibrationInProgress) {
                this.recordCalibrationTouch(touch.clientX, touch.clientY);
            }
        },

        handleTouchEnd(e) {
            const target = e.currentTarget;
            if (!target) return;
            
            const touch = e.changedTouches[0];
            if (!touch) return;
            
            const startX = parseFloat(target.dataset.touchStartX || 0);
            const startY = parseFloat(target.dataset.touchStartY || 0);
            const startTime = parseFloat(target.dataset.touchStartTime || 0);
            
            const deltaX = touch.clientX - startX;
            const deltaY = touch.clientY - startY;
            const duration = Date.now() - startTime;
            const currentTime = Date.now();
            
            target.classList.remove('touch-active');
            this.cancelTouchHoldTimer();
            
            if (this.state.isTouchHoldActive) {
                this.handleLongPress(target, e);
                this.state.isTouchHoldActive = false;
            } else if (Math.abs(deltaX) > this.config.swipeThreshold || Math.abs(deltaY) > this.config.swipeThreshold) {
                const horizontal = deltaX > 0 ? 'right' : 'left';
                const vertical = deltaY > 0 ? 'down' : 'up';
                this.handleSwipe(target, horizontal, vertical);
                
                if (this.config.enableSwipeTrails) {
                    this.showSwipeTrailEnd(touch.clientX, touch.clientY, horizontal === 'right' || horizontal === 'left' ? deltaX : deltaY);
                }
            } else if (duration < this.config.doubleTapDelay && currentTime - this.state.lastTapTime < this.config.doubleTapDelay) {
                if (this.state.tapCount === 1) {
                    this.handleDoubleTap(target, e);
                    this.state.tapCount = 2;
                    this.state.lastTapTime = currentTime;
                } else if (this.state.tapCount === 2) {
                    this.handleTripleTap(target, e);
                    this.state.tapCount = 0;
                    this.state.lastTapTime = 0;
                } else {
                    this.handleDoubleTap(target, e);
                    this.state.tapCount = 2;
                }
            } else {
                this.state.tapCount = 1;
                this.state.lastTapTime = currentTime;
                
                if (this.config.enableVelocityRipples && this.touchVelocity && this.touchVelocity.length > 2) {
                    const avgVelocity = this.touchVelocity.reduce((sum, v) => sum + Math.sqrt(v.x * v.x + v.y * v.y), 0) / this.touchVelocity.length;
                    if (avgVelocity > 2) {
                        this.showVelocityRipple(target, avgVelocity);
                    }
                }
                this.touchVelocity = null;
            }
        },

        handleTouchClick(e) {
            const target = e.currentTarget;
            const bulbId = target.dataset.bulbId;
            
            if (bulbId) {
                this.toggleBulbSelection(bulbId);
            }
        },

        handleLongPress(target, e) {
            e.preventDefault();
            const bulbId = target.dataset.bulbId;
            
            if (bulbId) {
                this.showBulbContextMenu(bulbId, e);
            }
            
            this.triggerHaptic('longPress');
        },

        handleSwipe(target, horizontal, vertical) {
            const bulbId = target.dataset.bulbId;
            const now = Date.now();
            
            if (bulbId && horizontal === 'right') {
                this.toggleBulbPower(bulbId, 'toggle');
                this.showSwipeHint(horizontal);
                this.triggerHaptic('swipeRight');
                this.recordGesture({ type: 'power', selector: `id:${bulbId}`, previousPower: null });
            } else if (bulbId && horizontal === 'left') {
                this.showBrightnessSlider(bulbId);
                this.showSwipeHint(horizontal);
                this.triggerHaptic('swipeLeft');
            } else if (bulbId && vertical === 'up') {
                this.showColorPicker(bulbId);
                this.showSwipeHint(vertical);
                this.triggerHaptic('swipeUp');
            } else if (bulbId && vertical === 'down') {
                this.showSceneSelector(bulbId);
                this.showSwipeHint(vertical);
                this.triggerHaptic('swipeDown');
            } else {
                this.triggerHaptic('swipe');
            }
            
            this.state.lastGestureTime = now;
        },

        handleGestureStart(e) {
            e.preventDefault();
            this.state.gestureScale = e.scale;
        },

        handleGestureChange(e) {
            e.preventDefault();
            const delta = e.scale - this.state.gestureScale;
            
            if (Math.abs(delta) > 0.1) {
                this.adjustGlobalBrightness(delta > 0 ? 10 : -10);
                this.state.gestureScale = e.scale;
                this.triggerHaptic('ripple');
            }
        },

        handleGestureEnd(e) {
            e.preventDefault();
        },

        touchHoldTimer: null,

        startTouchHoldTimer(target) {
            this.cancelTouchHoldTimer();
            this.state.touchHoldProgress = 0;
            this.state.isTouchHoldActive = true;
            
            const startTime = Date.now();
            const duration = this.config.gestureHoldDuration;
            
            this.touchHoldTimer = setInterval(() => {
                const elapsed = Date.now() - startTime;
                const progress = Math.min(100, (elapsed / duration) * 100);
                this.state.touchHoldProgress = progress;
                
                if (progress >= 100) {
                    this.cancelTouchHoldTimer();
                }
            }, 50);
        },

        cancelTouchHoldTimer() {
            if (this.touchHoldTimer) {
                clearInterval(this.touchHoldTimer);
                this.touchHoldTimer = null;
            }
            this.state.touchHoldProgress = 0;
        },

        updateTouchHoldProgress(target, deltaX, deltaY) {
            if (!this.state.isTouchHoldActive) return;
            
            const movement = Math.sqrt(deltaX * deltaX + deltaY * deltaY);
            if (movement > 20) {
                this.cancelTouchHoldTimer();
                this.state.isTouchHoldActive = false;
            }
        },

        handleDoubleTap(target, e) {
            const bulbId = target.dataset.bulbId;
            
            if (bulbId) {
                this.toggleBulbPower(bulbId, 'toggle');
                this.showTouchFeedback(target, 'Double Tap');
            }
            
            this.triggerHaptic('doubleTap');
        },

        handleTripleTap(target, e) {
            const bulbId = target.dataset.bulbId;
            
            if (bulbId) {
                this.toggleBulbPower(bulbId, 'off');
                this.showTouchFeedback(target, 'Triple Tap - Off');
                this.triggerHaptic('success');
            }
        },

        handlePinch(target, scale) {
            const bulbId = target.dataset.bulbId;
            
            if (bulbId) {
                const brightnessDelta = Math.round((scale - 1) * 50);
                this.adjustBulbBrightness(bulbId, brightnessDelta);
                this.showPinchFeedback(scale);
                this.triggerHaptic('ripple');
            }
        },

        handleRotate(target, rotation) {
            const bulbId = target.dataset.bulbId;
            
            if (bulbId) {
                const hueDelta = Math.round(rotation * 10);
                this.adjustBulbHue(bulbId, hueDelta);
                this.showRotateFeedback(rotation);
                this.triggerHaptic('gesture');
            }
        },

        showPinchFeedback(scale) {
            const brightnessChange = scale > 1 ? 'Brighter' : 'Dimmer';
            this.showTouchFeedback(null, `${brightnessChange} +${Math.abs(Math.round((scale - 1) * 50))}%`);
        },

        showRotateFeedback(rotation) {
            const colorShift = rotation > 0 ? 'Warmer' : 'Cooler';
            this.showTouchFeedback(null, `${colorShift} ${Math.abs(Math.round(rotation * 10))}°`);
        },

        showTripleTapFeedback(e) {
            const feedback = document.createElement('div');
            feedback.className = 'touch-feedback-message triple-tap';
            feedback.textContent = '✕ Power Off';
            feedback.style.cssText = `
                position: fixed;
                left: ${e.clientX}px;
                top: ${e.clientY - 50}px;
                color: #ff6b6b;
                font-weight: bold;
                font-size: 16px;
                text-shadow: 0 0 10px rgba(255, 107, 107, 0.8);
                pointer-events: none;
                opacity: 0;
                transition: opacity 0.3s ease, transform 0.3s ease;
                transform: translateY(20px);
                z-index: 10000;
            `;
            document.body.appendChild(feedback);
            
            setTimeout(() => {
                feedback.style.opacity = '1';
                feedback.style.transform = 'translateY(0)';
            }, 10);
            setTimeout(() => {
                feedback.style.opacity = '0';
                feedback.style.transform = 'translateY(-30px)';
                setTimeout(() => feedback.remove(), 300);
            }, 1000);
        },

        showTouchFeedback(target, message) {
            const feedback = document.createElement('div');
            feedback.className = 'touch-feedback-message';
            feedback.textContent = message;
            feedback.style.position = 'absolute';
            feedback.style.top = '50%';
            feedback.style.left = '50%';
            feedback.style.transform = 'translate(-50%, -50%)';
            feedback.style.color = '#00d4ff';
            feedback.style.fontWeight = 'bold';
            feedback.style.fontSize = '14px';
            feedback.style.textShadow = '0 0 10px rgba(0, 212, 255, 0.8)';
            feedback.style.pointerEvents = 'none';
            feedback.style.opacity = '0';
            feedback.style.transition = 'opacity 0.2s ease';
            
            target.style.position = 'relative';
            target.appendChild(feedback);
            
            setTimeout(() => feedback.style.opacity = '1', 10);
            setTimeout(() => {
                feedback.style.opacity = '0';
                setTimeout(() => feedback.remove(), 200);
            }, 800);
        },

        showEnhancedTouchRipple(e, target) {
            const ripple = document.createElement('span');
            ripple.classList.add('ripple-enhanced');
            
            const rect = target.getBoundingClientRect();
            const size = Math.max(rect.width, rect.height) * 1.5;
            
            const bulbId = target.dataset.bulbId;
            let hue = 180;
            let saturation = 80;
            let lightness = 60;
            
            if (bulbId && this.state.selectedBulbs.has(bulbId)) {
                hue = 0;
                saturation = 70;
                lightness = 55;
            }
            
            const touchSensitivity = this.state.touchSensitivityMap[this.state.touchSensitivity] || this.state.touchSensitivityMap.medium;
            const scale = 1 + (1 - touchSensitivity.multiplier) * 0.3;
            
            ripple.style.cssText = `
                position: absolute;
                width: ${size}px;
                height: ${size}px;
                left: ${e.clientX - rect.left - size / 2}px;
                top: ${e.clientY - rect.top - size / 2}px;
                background: radial-gradient(circle, 
                    hsla(${hue}, ${saturation}%, ${lightness}%, 0.4) 0%, 
                    hsla(${hue}, ${saturation}%, ${lightness}%, 0) 70%);
                border-radius: 50%;
                transform: scale(0);
                animation: ripple-enhanced-anim ${this.config.rippleDuration}ms ease-out forwards;
                pointer-events: none;
                box-shadow: 0 0 20px hsla(${hue}, ${saturation}%, ${lightness}%, 0.6);
                --ripple-hue: ${hue};
                --ripple-opacity: 0.4;
            `;
            
            target.appendChild(ripple);
            
            setTimeout(() => {
                if (ripple.parentNode) ripple.parentNode.removeChild(ripple);
            }, this.config.rippleDuration);
        },
        
        showTouchRipple(e, target) {
            const ripple = document.createElement('span');
            ripple.classList.add('lifx-touch-ripple');
            
            const rect = target.getBoundingClientRect();
            const size = Math.max(rect.width, rect.height);
            ripple.style.width = ripple.style.height = size + 'px';
            ripple.style.left = (e.clientX - rect.left - size / 2) + 'px';
            ripple.style.top = (e.clientY - rect.top - size / 2) + 'px';
            
            const bulbId = target.dataset.bulbId;
            if (bulbId && this.state.selectedBulbs.has(bulbId)) {
                ripple.style.background = 'radial-gradient(circle, rgba(255, 107, 107, 0.8) 0%, rgba(255, 107, 107, 0.4) 40%, transparent 70%)';
            }
            
            target.appendChild(ripple);
            
            setTimeout(() => ripple.remove(), 600);
        },

        showVelocityRipple(target, velocity) {
            const ripple = document.createElement('span');
            ripple.classList.add('velocity-ripple');
            
            const scale = Math.min(3.0, 1 + velocity / 8);
            const duration = Math.max(250, 500 - velocity * 40);
            const hue = Math.min(200, 160 + velocity * 2);
            
            ripple.style.setProperty('--ripple-scale', scale);
            ripple.style.setProperty('--ripple-duration', duration + 'ms');
            ripple.style.setProperty('--ripple-hue', hue);
            
            const rect = target.getBoundingClientRect();
            const size = Math.max(rect.width, rect.height) * 0.8;
            ripple.style.width = ripple.style.height = size + 'px';
            ripple.style.left = '50%';
            ripple.style.top = '50%';
            ripple.style.transform = 'translate(-50%, -50%)';
            ripple.style.background = `radial-gradient(circle, hsla(${hue}, 80%, 60%, 0.7) 0%, hsla(${hue}, 80%, 60%, 0.3) 40%, transparent 70%)`;
            
            target.appendChild(ripple);
            
            setTimeout(() => ripple.remove(), duration);
        },

        showSwipeTrail(x, y, dx, dy) {
            const trail = document.createElement('div');
            trail.classList.add('swipe-trail-particle');
            
            const size = 8 + Math.random() * 6;
            trail.style.width = trail.style.height = size + 'px';
            trail.style.left = (x - size / 2) + 'px';
            trail.style.top = (y - size / 2) + 'px';
            trail.style.background = `radial-gradient(circle, hsla(${180 + Math.random() * 40}, 80%, 60%, 0.6) 0%, transparent 70%)`;
            
            const travelX = dx > 0 ? 20 : -20;
            const travelY = dy > 0 ? 20 : -20;
            trail.style.setProperty('--travel-x', travelX + 'px');
            trail.style.setProperty('--travel-y', travelY + 'px');
            
            document.body.appendChild(trail);
            
            setTimeout(() => trail.remove(), 400);
        },

        showSwipeTrailEnd(x, y, distance) {
            const trail = document.createElement('div');
            trail.classList.add('swipe-trail');
            
            const scale = Math.min(1.5, 0.5 + Math.abs(distance) / 200);
            trail.style.setProperty('--trail-scale', scale);
            trail.style.left = (x - 50) + 'px';
            trail.style.top = (y - 50) + 'px';
            
            document.body.appendChild(trail);
            
            setTimeout(() => trail.remove(), 500);
        },

        showEnhancedGestureTrail(x, y, deltaX, deltaY) {
            const trail = document.createElement('div');
            trail.classList.add('gesture-trail-enhanced');
            
            const velocity = Math.sqrt(deltaX * deltaX + deltaY * deltaY);
            const size = this.config.gestureTrailSize * (1 + Math.min(velocity / 20, 1));
            const opacity = Math.min(0.8, 0.3 + velocity / 30);
            
            const hue = 180 + Math.min(velocity * 2, 40);
            trail.style.cssText = `
                position: fixed;
                width: ${size}px;
                height: ${size}px;
                left: ${x - size / 2}px;
                top: ${y - size / 2}px;
                background: radial-gradient(circle, 
                    hsla(${hue}, 80%, 60%, ${opacity}) 0%, 
                    hsla(${hue}, 80%, 60%, 0) 70%);
                border-radius: 50%;
                pointer-events: none;
                z-index: 9998;
                filter: blur(2px);
                animation: gesture-trail-fade ${0.4 + velocity / 50}s ease-out forwards;
            `;
            
            document.body.appendChild(trail);
            
            setTimeout(() => {
                if (trail.parentNode) trail.parentNode.removeChild(trail);
            }, 500);
        },
        
        showGestureTrail(x, y) {
            const trail = document.createElement('div');
            trail.classList.add('lifx-gesture-trail');
            trail.style.left = (x - 10) + 'px';
            trail.style.top = (y - 10) + 'px';
            
            document.body.appendChild(trail);
            
            setTimeout(() => {
                trail.remove();
            }, 400);
        },

        showSwipeHint(direction) {
            const hint = document.createElement('div');
            hint.className = 'gesture-hint-overlay visible';
            
            const icons = {
                'right': '➡️',
                'left': '⬅️',
                'up': '⬆️',
                'down': '⬇️'
            };
            
            const texts = {
                'right': 'Power Toggle',
                'left': 'Brightness',
                'up': 'Color Picker',
                'down': 'Scenes'
            };
            
            hint.innerHTML = `
                <i class="gesture-icon">${icons[direction] || '👆'}</i>
                <span class="hint-text">${texts[direction] || ''}</span>
            `;
            
            document.body.appendChild(hint);
            
            setTimeout(() => {
                hint.classList.remove('visible');
                setTimeout(() => hint.remove(), 300);
            }, 1000);
        },

        setupSceneSelector() {
            const sceneSelector = document.getElementById('lifx-scene-selector');
            if (!sceneSelector) return;

            sceneSelector.innerHTML = this.scenePresets.map(scene => 
                `<option value="${scene.id}">${scene.icon} ${scene.name}</option>`
            ).join('');

            sceneSelector.addEventListener('change', (e) => {
                this.applyScene(e.target.value);
            });
        },

        setupQuickActions() {
            const quickActionsContainer = document.getElementById('lifx-quick-actions');
            if (!quickActionsContainer) return;

            quickActionsContainer.innerHTML = `
                <div class="quick-actions-grid">
                    <button class="quick-action-btn" data-action="all-off" data-label="All Off" data-icon="💡">
                        <span class="icon">💡</span>
                        <span class="label">All Off</span>
                    </button>
                    <button class="quick-action-btn" data-action="all-on" data-label="All On" data-icon="☀️">
                        <span class="icon">☀️</span>
                        <span class="label">All On</span>
                    </button>
                    <button class="quick-action-btn" data-action="circadian" data-label="Circadian" data-icon="🕐">
                        <span class="icon">🕐</span>
                        <span class="label">Circadian</span>
                    </button>
                    <button class="quick-action-btn" data-action="party" data-action-type="effect" data-label="Party" data-icon="🎉">
                        <span class="icon">🎉</span>
                        <span class="label">Party</span>
                    </button>
                    <button class="quick-action-btn" data-action="fireplace" data-action-type="effect" data-label="Fireplace" data-icon="🔥">
                        <span class="icon">🔥</span>
                        <span class="label">Fireplace</span>
                    </button>
                    <button class="quick-action-btn" data-action="aurora" data-action-type="effect" data-label="Aurora" data-icon="🌌">
                        <span class="icon">🌌</span>
                        <span class="label">Aurora</span>
                    </button>
                </div>
            `;

            quickActionsContainer.querySelectorAll('.quick-action-btn').forEach(btn => {
                btn.addEventListener('click', (e) => {
                    const action = e.currentTarget.dataset.action;
                    const actionType = e.currentTarget.dataset.actionType || 'scene';
                    this.handleQuickAction(action, actionType);
                });
                btn.addEventListener('touchstart', (e) => {
                    e.preventDefault();
                    btn.classList.add('active');
                });
                btn.addEventListener('touchend', (e) => {
                    e.preventDefault();
                    btn.classList.remove('active');
                });
            });
        },

        handleQuickAction(action, actionType) {
            switch(action) {
                case 'all-off':
                    this.setLifxState('all', 'off');
                    break;
                case 'all-on':
                    this.setLifxState('all', 'on');
                    break;
                case 'circadian':
                    this.applyCircadian();
                    break;
                case 'party':
                    this.applyEffect('rainbow', 3, 10);
                    break;
                case 'fireplace':
                    this.applyEffect('fireplace', 1, 5);
                    break;
                case 'aurora':
                    this.applyEffect('aurora', 1, 5);
                    break;
            }
        },

        setupMediaPresets() {
            const mediaPresetsContainer = document.getElementById('media-presets');
            if (!mediaPresetsContainer) return;

            mediaPresetsContainer.innerHTML = `
                <div class="media-presets-grid">
                    ${this.mediaPresets.map(preset => `
                        <button class="media-preset-btn" data-service="${preset.service}">
                            <span class="icon">${preset.icon}</span>
                            <span class="label">${preset.name}</span>
                        </button>
                    `).join('')}
                </div>
            `;

            mediaPresetsContainer.querySelectorAll('.media-preset-btn').forEach(btn => {
                btn.addEventListener('click', (e) => {
                    const service = e.currentTarget.dataset.service;
                    this.launchMediaService(service);
                });
            });
        },

        async launchMediaService(service) {
            try {
                const response = await fetch(`/api/services/${service}/launch`, {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' }
                });
                const data = await response.json();
                this.showToast(`${service} launched!`, 'success');
            } catch (error) {
                this.showToast(`Failed to launch ${service}`, 'error');
            }
        },

        async applyScene(sceneName, duration = 1.0) {
            const preset = this.scenePresets.find(p => p.id === sceneName);
            if (!preset) {
                this.showToast('Unknown scene', 'error');
                return;
            }
            
            try {
                const response = await fetch('/api/services/lifx/scenes', {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify({
                        selector: 'all',
                        scene: sceneName,
                        duration: duration
                    })
                });
                
                if (!response.ok) {
                    throw new Error(`HTTP ${response.status}`);
                }
                
                const data = await response.json();
                if (data.success) {
                    this.state.activeScene = sceneName;
                    this.showToast(`${preset.icon} Scene '${preset.name}' applied!`, 'success');
                    this.showSceneIndicator(sceneName);
                    this.triggerHaptic('scene');
                    this.showSceneTransitionEffect(preset);
                } else {
                    throw new Error(data.error || 'Unknown error');
                }
            } catch (error) {
                console.error('[LIFXMediaTouchV2] Error applying scene:', error);
                this.showToast(`Failed to apply scene: ${error.message}`, 'error');
                this.triggerHaptic('error');
            }
        },
        
        showSceneTransitionEffect(preset) {
            const effectEl = document.createElement('div');
            effectEl.className = 'scene-transition-effect';
            effectEl.style.cssText = `
                position: fixed;
                top: 0;
                left: 0;
                right: 0;
                bottom: 0;
                background: radial-gradient(circle at center, 
                    hsla(${preset.hue / 182.4}, ${preset.saturation / 655.35}%, ${preset.brightness / 655.35}%, 0.4) 0%, 
                    transparent 70%);
                pointer-events: none;
                z-index: 9996;
                animation: scene-transition-fade 1s ease-out forwards;
            `;
            
            if (!document.getElementById('scene-transition-style')) {
                const style = document.createElement('style');
                style.id = 'scene-transition-style';
                style.textContent = `
                    @keyframes scene-transition-fade {
                        0% { opacity: 0; transform: scale(0.8); }
                        50% { opacity: 1; transform: scale(1.1); }
                        100% { opacity: 0; transform: scale(1); }
                    }
                `;
                document.head.appendChild(style);
            }
            
            document.body.appendChild(effectEl);
            setTimeout(() => {
                if (effectEl.parentNode) effectEl.parentNode.removeChild(effectEl);
            }, 1000);
        },

        async applyCircadian() {
            try {
                const response = await fetch('/api/services/lifx/circadian', {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify({
                        selector: 'all',
                        enable: true
                    })
                });
                
                if (!response.ok) {
                    throw new Error(`HTTP ${response.status}`);
                }
                
                const data = await response.json();
                if (data.success) {
                    this.state.circadianActive = true;
                    this.showToast(`🕐 Circadian rhythm applied (${data.time_of_day})`, 'success');
                } else {
                    throw new Error(data.error || 'Unknown error');
                }
            } catch (error) {
                console.error('[LIFXMediaTouchV2] Error applying circadian:', error);
                this.showToast(`Failed to apply circadian: ${error.message}`, 'error');
            }
        },

        async applyEffect(effectName, cycles = 1, duration = 5) {
            const preset = this.effectPresets.find(p => p.id === effectName);
            
            try {
                const response = await fetch('/api/services/lifx/effect', {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify({
                        selector: 'all',
                        effect: effectName,
                        cycles: cycles,
                        duration: duration
                    })
                });
                
                if (!response.ok) {
                    throw new Error(`HTTP ${response.status}`);
                }
                
                const data = await response.json();
                if (data.success) {
                    this.state.activeEffect = effectName;
                    this.showToast(`✨ ${preset ? preset.name : effectName} effect started!`, 'success');
                    this.showEffectIndicator(effectName);
                    this.triggerHaptic('effect');
                    this.showEffectParticles(preset);
                } else {
                    throw new Error(data.error || 'Unknown error');
                }
            } catch (error) {
                console.error('[LIFXMediaTouchV2] Error applying effect:', error);
                this.showToast(`Failed to apply effect: ${error.message}`, 'error');
                this.triggerHaptic('error');
            }
        },
        
        showEffectParticles(preset) {
            const particleCount = 12;
            const icons = {
                'pulse': '💓',
                'rainbow': '🌈',
                'strobe': '⚡',
                'fireplace': '🔥',
                'aurora': '🌌',
                'breath': '🌬️',
                'color_cycle': '🎨'
            };
            const icon = icons[preset?.id] || '✨';
            
            for (let i = 0; i < particleCount; i++) {
                const particle = document.createElement('div');
                particle.className = 'effect-particle';
                const angle = (i / particleCount) * 360;
                const radius = 100;
                particle.style.cssText = `
                    position: fixed;
                    left: 50%;
                    top: 50%;
                    width: 40px;
                    height: 40px;
                    font-size: 24px;
                    display: flex;
                    align-items: center;
                    justify-content: center;
                    pointer-events: none;
                    z-index: 9998;
                    animation: effect-particle-explode 1.2s ease-out forwards;
                    --angle: ${angle}deg;
                    --radius: ${radius}px;
                `;
                particle.textContent = icon;
                
                if (!document.getElementById('effect-particle-style')) {
                    const style = document.createElement('style');
                    style.id = 'effect-particle-style';
                    style.textContent = `
                        @keyframes effect-particle-explode {
                            0% { 
                                opacity: 1; 
                                transform: translate(-50%, -50%) scale(0.5);
                            }
                            100% { 
                                opacity: 0;
                                transform: translate(calc(-50% + cos(var(--angle)) * var(--radius)), calc(-50% + sin(var(--angle)) * var(--radius))) scale(1.2);
                            }
                        }
                    `;
                    document.head.appendChild(style);
                }
                
                document.body.appendChild(particle);
                setTimeout(() => {
                    if (particle.parentNode) particle.parentNode.removeChild(particle);
                }, 1200);
            }
        },

        async setLifxState(selector, power) {
            try {
                const response = await fetch('/api/services/lifx/set_state', {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify({
                        selector: selector,
                        power: power
                    })
                });
                
                if (!response.ok) {
                    throw new Error(`HTTP ${response.status}`);
                }
                
                const data = await response.json();
                if (data.success) {
                    this.showToast(`${power === 'on' ? '💡' : '🌑'} Lights ${power}!`, 'success');
                } else {
                    throw new Error(data.error || 'Unknown error');
                }
            } catch (error) {
                console.error('[LIFXMediaTouchV2] Error setting state:', error);
                this.showToast(`Failed to set state: ${error.message}`, 'error');
            }
        },

        toggleBulbSelection(bulbId) {
            if (this.state.selectedBulbs.has(bulbId)) {
                this.state.selectedBulbs.delete(bulbId);
                document.querySelector(`[data-bulb-id="${bulbId}"]`)?.classList.remove('multi-selected');
            } else {
                this.state.selectedBulbs.add(bulbId);
                document.querySelector(`[data-bulb-id="${bulbId}"]`)?.classList.add('multi-selected');
            }
            this.updateSelectionToolbar();
        },

        updateSelectionToolbar() {
            const toolbar = document.getElementById('lifx-selection-toolbar');
            if (!toolbar) return;
            
            const count = this.state.selectedBulbs.size;
            toolbar.querySelector('.selected-count').textContent = count;
            
            if (count > 0) {
                toolbar.classList.add('visible');
            } else {
                toolbar.classList.remove('visible');
            }
        },

        showSceneIndicator(sceneName) {
            const indicator = document.createElement('div');
            indicator.className = 'scene-indicator visible';
            const scene = this.scenePresets.find(s => s.id === sceneName);
            indicator.innerHTML = `${scene ? scene.icon : '🎨'} ${scene ? scene.name : sceneName}`;
            document.body.appendChild(indicator);
            
            setTimeout(() => {
                indicator.classList.remove('visible');
                setTimeout(() => indicator.remove(), 300);
            }, 3000);
        },

        showEffectIndicator(effectName) {
            const indicator = document.createElement('div');
            indicator.className = 'effect-active-indicator visible';
            const effect = this.effectPresets.find(e => e.id === effectName);
            indicator.innerHTML = `
                <span class="effect-icon">${effect ? effect.icon : '✨'}</span>
                <span class="effect-name">${effect ? effect.name : effectName}</span>
            `;
            document.body.appendChild(indicator);
        },

        showToast(message, type = 'info') {
            const toast = document.createElement('div');
            toast.className = `lifx-toast lifx-toast-${type}`;
            toast.textContent = message;
            document.body.appendChild(toast);
            
            setTimeout(() => {
                toast.classList.add('visible');
            }, 10);
            
            setTimeout(() => {
                toast.classList.remove('visible');
                setTimeout(() => toast.remove(), 300);
            }, 3000);
        },

        syncStatus() {
            fetch('/api/services/lifx/status')
                .then(res => {
                    if (!res.ok) {
                        throw new Error(`HTTP ${res.status}`);
                    }
                    return res.json();
                })
                .then(data => {
                    this.updateLifxStatus(data);
                })
                .catch(err => {
                    console.error('[LIFXMediaTouchV2] LIFX status sync error:', err);
                    this.updateLifxStatus({ connected: false, bulbs_found: 0 });
                });
        },

        updateLifxStatus(data) {
            const statusElement = document.getElementById('lifx-status');
            if (statusElement) {
                statusElement.innerHTML = `
                    <span class="status-dot ${data.connected ? 'connected' : 'disconnected'}"></span>
                    <span>${data.bulbs_found || 0} bulbs found</span>
                `;
            }
        },

        startPeriodicSync() {
            setInterval(() => {
                this.syncStatus();
            }, 5000);
        },

        setupColorPicker() {
            const colorPicker = document.getElementById('lifx-color-picker');
            if (!colorPicker) return;
            
            colorPicker.addEventListener('input', (e) => {
                const color = e.target.value;
                this.applyColorToSelected(color);
            });
        },

        setupEffectSelector() {
            const effectSelector = document.getElementById('lifx-effect-selector');
            if (!effectSelector) return;
            
            effectSelector.innerHTML = this.effectPresets.map(effect => 
                `<option value="${effect.id}">${effect.icon} ${effect.name}</option>`
            ).join('');
            
            effectSelector.addEventListener('change', (e) => {
                const effect = this.effectPresets.find(p => p.id === e.target.value);
                if (effect) {
                    this.applyEffect(effect.id, effect.cycles, effect.duration);
                }
            });
        },

        setupZoneControl() {
            const zoneControl = document.getElementById('lifx-zone-control');
            if (!zoneControl) return;
            
            zoneControl.innerHTML = `
                <div class="zone-control-header">
                    <span class="zone-icon">📍</span>
                    <span class="zone-title">Zone Control</span>
                </div>
                <div class="zone-selection">
                    <button class="zone-btn" data-zone="all">All Zones</button>
                    <button class="zone-btn" data-zone="start">Start</button>
                    <button class="zone-btn" data-zone="middle">Middle</button>
                    <button class="zone-btn" data-zone="end">End</button>
                </div>
                <div class="zone-selection zone-selection-fine">
                    <button class="zone-btn" data-zone="left">Left</button>
                    <button class="zone-btn" data-zone="center-left">Center-L</button>
                    <button class="zone-btn" data-zone="center-right">Center-R</button>
                    <button class="zone-btn" data-zone="right">Right</button>
                </div>
                <div id="lifx-zone-strip" class="zone-strip-container"></div>
            `;
            
            zoneControl.querySelectorAll('.zone-btn').forEach(btn => {
                btn.addEventListener('click', (e) => {
                    const zone = e.currentTarget.dataset.zone;
                    this.applyZoneColor(zone);
                });
                btn.addEventListener('touchstart', (e) => {
                    e.preventDefault();
                    btn.classList.add('active');
                });
                btn.addEventListener('touchend', (e) => {
                    e.preventDefault();
                    btn.classList.remove('active');
                });
            });
            
            this.setupZoneStripVisualization();
        },

        setupGestureHints() {
            const hintsContainer = document.getElementById('lifx-gesture-hints');
            if (!hintsContainer) return;
            
            const isTouchDevice = typeof is_touch_enabled === 'function' && is_touch_enabled();
            if (!isTouchDevice) {
                hintsContainer.innerHTML = `
                    <div class="gesture-hint-item">
                        <span class="gesture-icon">🖱️</span>
                        <span class="gesture-text">Click to select</span>
                    </div>
                    <div class="gesture-hint-item">
                        <span class="gesture-icon">🖱️🖱️</span>
                        <span class="gesture-text">Double-click to toggle power</span>
                    </div>
                    <div class="gesture-hint-item">
                        <span class="gesture-icon">⌨️</span>
                        <span class="gesture-text">Use scene selector for presets</span>
                    </div>
                `;
                return;
            }
            
            const sensitivityInfo = this.state.touchSensitivityMap[this.state.touchSensitivity] || this.state.touchSensitivityMap.medium;
            
            hintsContainer.innerHTML = `
                <div class="gesture-hint-item">
                    <span class="gesture-icon">👆</span>
                    <span class="gesture-text">Tap to select</span>
                </div>
                <div class="gesture-hint-item">
                    <span class="gesture-icon">👆👆</span>
                    <span class="gesture-text">Long press for menu</span>
                </div>
                <div class="gesture-hint-item">
                    <span class="gesture-icon">👉</span>
                    <span class="gesture-text">Swipe right to toggle power</span>
                </div>
                <div class="gesture-hint-item">
                    <span class="gesture-icon">👈</span>
                    <span class="gesture-text">Swipe left for brightness</span>
                </div>
                <div class="gesture-hint-item">
                    <span class="gesture-icon">🤏</span>
                    <span class="gesture-text">Pinch to adjust global brightness</span>
                </div>
                <div class="gesture-hint-item">
                    <span class="gesture-icon">🎯</span>
                    <span class="gesture-text">Sensitivity: ${this.state.touchSensitivity} (${Math.round(sensitivityInfo.multiplier * 100)}%)</span>
                </div>
            `;
        },
        
        setupCalibrationButton() {
            const calibrateBtn = document.getElementById('touch-calibrate-btn');
            if (!calibrateBtn) return;
            
            calibrateBtn.addEventListener('click', () => {
                this.startTouchSensitivityCalibration();
            });
            
            calibrateBtn.addEventListener('touchstart', (e) => {
                e.preventDefault();
                calibrateBtn.classList.add('active');
                this.startTouchSensitivityCalibration();
            });
            
            calibrateBtn.addEventListener('touchend', (e) => {
                e.preventDefault();
                calibrateBtn.classList.remove('active');
            });
        },
        
        setupCleanupHandlers() {
            window.addEventListener('beforeunload', () => {
                this.cancelTouchHoldTimer();
                if (this.audioContext) {
                    this.audioContext.close();
                }
                this.state.mediaSyncActive = false;
            });
            
            document.addEventListener('visibilitychange', () => {
                if (document.hidden) {
                    this.cancelTouchHoldTimer();
                }
            });
        },

        applyColorToSelected(hexColor) {
            if (this.state.selectedBulbs.size === 0) return;
            
            const bulbIds = Array.from(this.state.selectedBulbs);
            
            fetch('/api/services/lifx/set_color', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({
                    selector: bulbIds.map(id => `id:${id}`).join(','),
                    color: hexColor
                })
            }).then(res => res.json())
              .then(data => {
                  if (data.success) {
                      this.showToast(`Color applied to ${bulbIds.length} bulbs`, 'success');
                      this.triggerHaptic('color');
                  }
              });
        },

        applyZoneColor(zone) {
            const zoneRanges = {
                'all': { start: 0, end: 255 },
                'start': { start: 0, end: 85 },
                'middle': { start: 86, end: 170 },
                'end': { start: 171, end: 255 },
                'left': { start: 0, end: 63 },
                'center-left': { start: 64, end: 127 },
                'center-right': { start: 128, end: 191 },
                'right': { start: 192, end: 255 },
                'first-quarter': { start: 0, end: 63 },
                'second-quarter': { start: 64, end: 127 },
                'third-quarter': { start: 128, end: 191 },
                'fourth-quarter': { start: 192, end: 255 }
            };
            
            const range = zoneRanges[zone];
            if (!range) return;
            
            const color = this.getCurrentColor();
            
            fetch('/api/services/lifx/zones', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({
                    selector: 'all',
                    start_index: range.start,
                    end_index: range.end,
                    color: color,
                    duration: 0.5
                })
            }).then(res => res.json())
              .then(data => {
                  if (data.success) {
                      this.showToast(`Zone ${zone} updated`, 'success');
                      this.highlightZoneSegment(zone, range.start, range.end);
                      this.triggerHaptic('zone');
                      this.showZoneTransitionEffect(range.start, range.end, color);
                  }
              });
        },
        
        getCurrentColor() {
            const colorPicker = document.getElementById('lifx-color-picker');
            if (colorPicker && colorPicker.value) {
                return colorPicker.value;
            }
            return '#00d4ff';
        },
        
        highlightZoneSegment(zone, start, end) {
            const segments = document.querySelectorAll('.zone-segment');
            const totalSegments = segments.length;
            
            if (totalSegments === 0) return;
            
            const startIndex = Math.floor((start / 255) * totalSegments);
            const endIndex = Math.floor((end / 255) * totalSegments);
            
            segments.forEach((segment, index) => {
                if (index >= startIndex && index <= endIndex) {
                    segment.classList.add('active');
                    setTimeout(() => segment.classList.remove('active'), 1000);
                }
            });
        },
        
        highlightZoneSegmentByRange(start, end) {
            const segments = document.querySelectorAll('.zone-segment');
            const totalSegments = segments.length;
            
            if (totalSegments === 0) return;
            
            const startIndex = Math.floor((start / 255) * totalSegments);
            const endIndex = Math.floor((end / 255) * totalSegments);
            
            segments.forEach((segment, index) => {
                if (index >= startIndex && index <= endIndex) {
                    segment.classList.add('active');
                    segment.style.boxShadow = '0 0 15px rgba(0, 212, 255, 0.8)';
                    setTimeout(() => {
                        segment.classList.remove('active');
                        segment.style.boxShadow = '';
                    }, 300);
                }
            });
        },
        
        showZoneTransitionEffect(start, end, color) {
            const effectEl = document.createElement('div');
            effectEl.className = 'zone-transition-effect';
            effectEl.style.cssText = `
                position: fixed;
                bottom: 0;
                left: ${(start / 255) * 100}%;
                width: ${((end - start) / 255) * 100}%;
                height: 100vh;
                background: linear-gradient(to top, ${color}40, transparent);
                pointer-events: none;
                z-index: 9997;
                animation: zone-transition-rise 1.5s ease-out forwards;
            `;
            
            if (!document.getElementById('zone-transition-style')) {
                const style = document.createElement('style');
                style.id = 'zone-transition-style';
                style.textContent = `
                    @keyframes zone-transition-rise {
                        0% { transform: scaleY(0); opacity: 0.8; }
                        100% { transform: scaleY(1); opacity: 0; }
                    }
                `;
                document.head.appendChild(style);
            }
            
            document.body.appendChild(effectEl);
            setTimeout(() => {
                if (effectEl.parentNode) effectEl.parentNode.removeChild(effectEl);
            }, 1500);
        },
        
        setupZoneStripVisualization() {
            const zoneStrip = document.getElementById('lifx-zone-strip');
            if (!zoneStrip) return;
            
            const segmentCount = 32;
            let html = '';
            
            for (let i = 0; i < segmentCount; i++) {
                const zoneIndex = Math.floor((i / segmentCount) * 256);
                html += `<div class="zone-segment" data-zone-index="${zoneIndex}" title="Zone ${zoneIndex}">
                    <span class="segment-tooltip">Zone ${zoneIndex}</span>
                </div>`;
            }
            
            zoneStrip.innerHTML = html;
            
            let activeSegment = null;
            let zoneDragTimeout = null;
            
            zoneStrip.querySelectorAll('.zone-segment').forEach(segment => {
                segment.addEventListener('touchstart', (e) => {
                    e.preventDefault();
                    segment.classList.add('touch-active');
                    activeSegment = segment;
                    this.triggerHaptic('tap');
                }, { passive: true });
                
                segment.addEventListener('touchmove', (e) => {
                    e.preventDefault();
                    const touch = e.touches[0];
                    const target = document.elementFromPoint(touch.clientX, touch.clientY);
                    if (target && target.classList.contains('zone-segment') && target !== activeSegment) {
                        if (activeSegment) activeSegment.classList.remove('touch-active');
                        activeSegment = target;
                        activeSegment.classList.add('touch-active');
                        
                        const zoneIndex = parseInt(activeSegment.dataset.zoneIndex);
                        const segmentSize = Math.ceil(256 / segmentCount);
                        const start = zoneIndex;
                        const end = Math.min(255, zoneIndex + segmentSize - 1);
                        
                        if (zoneDragTimeout) clearTimeout(zoneDragTimeout);
                        zoneDragTimeout = setTimeout(() => {
                            this.applyZoneColorToRange(start, end, false);
                        }, 50);
                    }
                }, { passive: true });
                
                segment.addEventListener('touchend', (e) => {
                    e.preventDefault();
                    segment.classList.remove('touch-active');
                    if (zoneDragTimeout) clearTimeout(zoneDragTimeout);
                    activeSegment = null;
                });
                
                segment.addEventListener('click', (e) => {
                    if (activeSegment) return;
                    const zoneIndex = parseInt(e.currentTarget.dataset.zoneIndex);
                    const segmentSize = Math.ceil(256 / segmentCount);
                    const start = zoneIndex;
                    const end = Math.min(255, zoneIndex + segmentSize - 1);
                    
                    this.applyZoneColorToRange(start, end, true);
                });
            });
        },
        
        applyZoneColorToRange(start, end, showFeedback = true) {
            const color = this.getCurrentColor();
            
            fetch('/api/services/lifx/zones', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({
                    selector: 'all',
                    start_index: start,
                    end_index: end,
                    color: color,
                    duration: 0.15
                })
            }).then(res => res.json())
              .then(data => {
                  if (data.success) {
                      this.highlightZoneSegmentByRange(start, end);
                      if (showFeedback) {
                          this.showToast(`Zones ${start}-${end} updated`, 'success');
                          this.triggerHaptic('zone');
                      }
                  }
              })
              .catch(err => {
                  console.warn('[LIFXMediaTouchV2] Zone update failed:', err);
              });
        },

        adjustGlobalBrightness(delta) {
            this.state.brightnessLevel = Math.max(0, Math.min(100, this.state.brightnessLevel + delta));
            
            fetch('/api/services/lifx/set_state', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({
                    selector: 'all',
                    brightness: this.state.brightnessLevel / 100
                })
            });
            
            this.showBrightnessFeedback(this.state.brightnessLevel);
            this.triggerHaptic('brightness', Math.abs(delta) / 20);
        },

        showBrightnessFeedback(level) {
            const feedback = document.createElement('div');
            feedback.className = 'touch-feedback-brightness visible';
            feedback.textContent = `${level}%`;
            document.body.appendChild(feedback);
            
            setTimeout(() => {
                feedback.classList.remove('visible');
                setTimeout(() => feedback.remove(), 300);
            }, 1000);
        },

        showBulbContextMenu(bulbId, e) {
            const menu = document.createElement('div');
            menu.className = 'lifx-context-menu';
            menu.innerHTML = `
                <button class="context-menu-item" data-action="power">Toggle Power</button>
                <button class="context-menu-item" data-action="brightness">Brightness</button>
                <button class="context-menu-item" data-action="color">Color</button>
                <button class="context-menu-item" data-action="scene">Apply Scene</button>
            `;
            
            menu.style.left = e.clientX + 'px';
            menu.style.top = e.clientY + 'px';
            
            document.body.appendChild(menu);
            
            menu.querySelectorAll('.context-menu-item').forEach(item => {
                item.addEventListener('click', (ev) => {
                    const action = ev.currentTarget.dataset.action;
                    this.handleBulbAction(bulbId, action);
                    menu.remove();
                });
            });
            
            setTimeout(() => menu.remove(), 5000);
        },

        handleBulbAction(bulbId, action) {
            switch(action) {
                case 'power':
                    this.toggleBulbPower(bulbId);
                    break;
                case 'brightness':
                    this.showBrightnessSlider(bulbId);
                    break;
                case 'color':
                    this.showColorPicker(bulbId);
                    break;
                case 'scene':
                    this.showSceneSelector(bulbId);
                    break;
            }
        },

        toggleBulbPower(bulbId, state = 'toggle') {
            fetch('/api/services/lifx/set_state', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({
                    selector: `id:${bulbId}`,
                    power: state === 'toggle' ? 'toggle' : state
                })
            });
        },

        showBrightnessSlider(bulbId) {
            const slider = document.createElement('div');
            slider.className = 'lifx-brightness-slider';
            slider.innerHTML = `
                <input type="range" min="0" max="100" value="50" />
            `;
            
            slider.style.position = 'fixed';
            slider.style.left = '50%';
            slider.style.top = '50%';
            slider.style.transform = 'translate(-50%, -50%)';
            
            document.body.appendChild(slider);
            
            slider.querySelector('input').addEventListener('input', (e) => {
                const brightness = e.target.value / 100;
                fetch('/api/services/lifx/set_state', {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify({
                        selector: `id:${bulbId}`,
                        brightness: brightness
                    })
                });
            });
            
            setTimeout(() => slider.remove(), 5000);
        },

        showColorPicker(bulbId) {
            const picker = document.createElement('input');
            picker.type = 'color';
            picker.style.position = 'fixed';
            picker.style.left = '50%';
            picker.style.top = '50%';
            picker.style.transform = 'translate(-50%, -50%)';
            picker.style.zIndex = '10000';
            
            document.body.appendChild(picker);
            
            picker.addEventListener('input', (e) => {
                const color = e.target.value;
                fetch('/api/services/lifx/set_color', {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify({
                        selector: `id:${bulbId}`,
                        color: color
                    })
                });
                picker.remove();
            });
            
            picker.click();
        },

        showSceneSelector(bulbId) {
            const selector = document.createElement('select');
            selector.innerHTML = this.scenePresets.map(scene => 
                `<option value="${scene.id}">${scene.icon} ${scene.name}</option>`
            ).join('');
            
            selector.style.position = 'fixed';
            selector.style.left = '50%';
            selector.style.top = '50%';
            selector.style.transform = 'translate(-50%, -50%)';
            selector.style.zIndex = '10000';
            selector.style.padding = '10px';
            selector.style.fontSize = '16px';
            
            document.body.appendChild(selector);
            
            selector.addEventListener('change', (e) => {
                const sceneId = e.target.value;
                const scene = this.scenePresets.find(s => s.id === sceneId);
                if (scene) {
                    fetch('/api/services/lifx/scenes', {
                        method: 'POST',
                        headers: { 'Content-Type': 'application/json' },
                        body: JSON.stringify({
                            selector: `id:${bulbId}`,
                            scene: sceneId,
                            duration: 0.5
                        })
                    });
                }
                selector.remove();
            });
        },

        async undoLastGesture() {
            if (this.state.gestureHistory.length === 0) {
                this.showToast('No actions to undo', 'info');
                return;
            }
            
            const lastAction = this.state.gestureHistory.pop();
            this.showToast(`Undoing: ${lastAction.type}`, 'info');
            
            try {
                if (lastAction.type === 'color') {
                    await fetch('/api/services/lifx/set_color', {
                        method: 'POST',
                        headers: { 'Content-Type': 'application/json' },
                        body: JSON.stringify({
                            selector: lastAction.selector,
                            color: lastAction.previousColor
                        })
                    });
                } else if (lastAction.type === 'power') {
                    await fetch('/api/services/lifx/set_state', {
                        method: 'POST',
                        headers: { 'Content-Type': 'application/json' },
                        body: JSON.stringify({
                            selector: lastAction.selector,
                            power: lastAction.previousPower ? 'on' : 'off'
                        })
                    });
                } else if (lastAction.type === 'brightness') {
                    await fetch('/api/services/lifx/set_state', {
                        method: 'POST',
                        headers: { 'Content-Type': 'application/json' },
                        body: JSON.stringify({
                            selector: lastAction.selector,
                            brightness: lastAction.previousBrightness
                        })
                    });
                } else if (lastAction.type === 'scene') {
                    await fetch('/api/services/lifx/scenes', {
                        method: 'POST',
                        headers: { 'Content-Type': 'application/json' },
                        body: JSON.stringify({
                            selector: lastAction.selector,
                            scene: lastAction.previousScene,
                            duration: 0.5
                        })
                    });
                }
                this.showToast('Action undone', 'success');
            } catch (error) {
                this.showToast(`Undo failed: ${error.message}`, 'error');
            }
        },

        recordGesture(action) {
            this.state.gestureHistory.push(action);
            if (this.state.gestureHistory.length > this.config.maxTouchHistory) {
                this.state.gestureHistory.shift();
            }
            this.updateUndoButtonState();
        },

        updateUndoButtonState() {
            const undoBtn = document.getElementById('lifx-undo-btn');
            if (!undoBtn) return;
            
            if (this.state.gestureHistory.length > 0) {
                undoBtn.classList.add('visible', 'has-history');
                undoBtn.disabled = false;
            } else {
                undoBtn.classList.remove('visible', 'has-history');
                undoBtn.disabled = true;
            }
        },

        getMovementThreshold() {
            const sensitivity = this.state.touchSensitivityMap[this.state.touchSensitivity] || this.state.touchSensitivityMap.medium;
            return 10 * sensitivity.multiplier;
        },

        recordCalibrationTouch(x, y) {
            this.state.gestureCalibrationData.push({ x, y, time: Date.now() });
            
            if (this.state.gestureCalibrationData.length >= this.config.calibrationSamples) {
                this.completeCalibration();
            }
        },

        startTouchSensitivityCalibration() {
            this.state.calibrationInProgress = true;
            this.state.gestureCalibrationData = [];
            this.state.touchAccuracyScore = 100;
            
            this.showToast('Touch calibration started - tap the centers of bulbs', 'info');
            
            if (this.config.enableHapticFeedback && navigator.vibrate) {
                navigator.vibrate(this.config.hapticPatterns.calibration);
            }
            
            this.showCalibrationOverlay();
        },

        completeCalibration() {
            this.state.calibrationInProgress = false;
            
            const data = this.state.gestureCalibrationData;
            if (data.length < 5) {
                this.showToast('Calibration failed - not enough samples', 'error');
                this.triggerHaptic('error');
                this.hideCalibrationOverlay();
                return;
            }
            
            let totalDeviation = 0;
            for (let i = 1; i < data.length; i++) {
                const dx = data[i].x - data[i-1].x;
                const dy = data[i].y - data[i-1].y;
                totalDeviation += Math.sqrt(dx * dx + dy * dy);
            }
            
            const avgDeviation = totalDeviation / (data.length - 1);
            const accuracy = Math.max(0, Math.min(100, 100 - (avgDeviation - 50) / 2));
            this.state.touchAccuracyScore = Math.round(accuracy);
            
            if (accuracy < 70) {
                this.state.touchSensitivity = 'low';
            } else if (accuracy < 85) {
                this.state.touchSensitivity = 'medium';
            } else {
                this.state.touchSensitivity = 'high';
            }
            
            this.showToast(`Calibration complete! Accuracy: ${Math.round(accuracy)}%`, 'success');
            this.triggerHaptic('success');
            this.hideCalibrationOverlay();
            this.setupGestureHints();
        },

        showCalibrationOverlay() {
            let overlay = document.querySelector('.gesture-calibration-overlay');
            if (!overlay) {
                overlay = document.createElement('div');
                overlay.className = 'gesture-calibration-overlay';
                overlay.innerHTML = `
                    <div style="text-align: center; color: #fff;">
                        <i class="gesture-icon" style="font-size: 48px; color: #27a0b9;">🎯</i>
                        <h3 style="margin: 15px 0 10px;">Touch Calibration</h3>
                        <p style="color: #adb5bd; font-size: 14px;">Tap the center of each bulb to calibrate sensitivity</p>
                        <div class="calibration-progress" style="margin-top: 20px;">
                            <div style="background: rgba(255,255,255,0.1); border-radius: 10px; height: 10px; overflow: hidden;">
                                <div id="calibration-progress-fill" style="background: linear-gradient(90deg, #27a0b9, #00d4ff); height: 100%; width: 0%; transition: width 0.2s ease;"></div>
                            </div>
                            <p id="calibration-progress-text" style="color: #27a0b9; font-size: 12px; margin-top: 8px;">0 / ${this.config.calibrationSamples} samples</p>
                        </div>
                    </div>
                `;
                overlay.style.cssText = `
                    position: fixed;
                    top: 50%;
                    left: 50%;
                    transform: translate(-50%, -50%);
                    background: rgba(30, 30, 45, 0.95);
                    border: 2px solid rgba(39, 160, 185, 0.5);
                    border-radius: 16px;
                    padding: 30px 40px;
                    z-index: 10000;
                    box-shadow: 0 10px 40px rgba(0, 0, 0, 0.5);
                `;
                document.body.appendChild(overlay);
            }
            
            this.updateCalibrationProgress();
        },

        hideCalibrationOverlay() {
            const overlay = document.querySelector('.gesture-calibration-overlay');
            if (overlay) {
                overlay.remove();
            }
        },

        updateCalibrationProgress() {
            const progressFill = document.getElementById('calibration-progress-fill');
            const progressText = document.getElementById('calibration-progress-text');
            
            if (progressFill && progressText) {
                const progress = (this.state.gestureCalibrationData.length / this.config.calibrationSamples) * 100;
                progressFill.style.width = `${progress}%`;
                progressText.textContent = `${this.state.gestureCalibrationData.length} / ${this.config.calibrationSamples} samples`;
            }
        },

        triggerHaptic(pattern, intensity = 1.0) {
            if (!this.config.enableHapticFeedback || !navigator.vibrate) return;
            
            const hapticPatterns = {
                tap: [10],
                doubleTap: [15, 50, 15],
                longPress: [50, 50, 50],
                swipe: [20],
                swipeRight: [12, 20, 8],
                swipeLeft: [8, 20, 12],
                swipeUp: [10, 15, 10],
                swipeDown: [10, 10, 15],
                beat: [25],
                success: [10, 20, 10, 20, 10],
                gesture: [15, 30, 15],
                calibration: [20, 40, 20, 40, 20],
                scene: [15, 25, 15],
                effect: [12, 18, 12, 18],
                color: [10, 15, 10],
                zone: [8, 12, 8],
                brightness: [10, 20, 10],
                error: [25, 20, 25, 20, 25],
                ripple: [8],
                warning: [20, 15, 20]
            };
            
            const basePattern = hapticPatterns[pattern] || hapticPatterns.tap;
            
            if (intensity !== 1.0) {
                const scaledPattern = basePattern.map(duration => Math.round(duration * intensity));
                navigator.vibrate(scaledPattern);
            } else {
                navigator.vibrate(basePattern);
            }
        },

        setTouchSensitivity(level) {
            if (['low', 'medium', 'high', 'custom'].includes(level)) {
                this.state.touchSensitivity = level;
                this.showToast(`Touch sensitivity: ${level}`, 'info');
                this.setupGestureHints();
                
                try {
                    localStorage.setItem('lifx-touch-sensitivity', level);
                } catch (e) {
                    console.warn('[LIFXMediaTouchV2] Failed to save sensitivity:', e);
                }
            }
        },

        loadTouchSensitivity() {
            try {
                const saved = localStorage.getItem('lifx-touch-sensitivity');
                if (saved && ['low', 'medium', 'high', 'custom'].includes(saved)) {
                    this.state.touchSensitivity = saved;
                    const sensitivity = this.state.touchSensitivityMap[saved];
                    console.log(`[LIFXMediaTouchV2] Loaded sensitivity: ${saved} (threshold: ${sensitivity.threshold}, multiplier: ${sensitivity.multiplier})`);
                    return saved;
                }
            } catch (e) {
                console.warn('[LIFXMediaTouchV2] Failed to load sensitivity:', e);
            }
            return 'medium';
        },

        async powerAll(state) {
            const selector = this.state.selectedBulbs.size > 0 
                ? Array.from(this.state.selectedBulbs).map(id => `id:${id}`).join(',')
                : 'all';
            
            try {
                const response = await fetch('/api/services/lifx/set_state', {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify({
                        selector: selector,
                        power: state
                    })
                });
                
                const data = await response.json();
                if (data.success) {
                    this.showToast(`${state === 'on' ? '💡' : '🌑'} ${this.state.selectedBulbs.size > 0 ? this.state.selectedBulbs.size + ' bulbs' : 'All lights'} turned ${state}!`, 'success');
                    this.clearMultiSelection();
                }
            } catch (error) {
                this.showToast(`Failed: ${error.message}`, 'error');
            }
        },

        clearMultiSelection() {
            this.state.selectedBulbs.clear();
            document.querySelectorAll('.lifx-bulb-control.multi-selected').forEach(el => {
                el.classList.remove('multi-selected');
            });
            this.updateSelectionToolbar();
        },

        showGroupManagementPanel() {
            Swal.fire({
                title: 'Manage Bulb Groups',
                html: `
                    <div style="text-align: left;">
                        <p>Create and manage groups of LIFX bulbs for batch control.</p>
                        <div class="form-group">
                            <label>Group Name</label>
                            <input type="text" id="group-name" class="form-control" placeholder="Living Room">
                        </div>
                        <div class="form-group">
                            <label>Select Bulbs</label>
                            <div id="group-bulb-selector" style="max-height: 200px; overflow-y: auto;">
                                ${this.getBulbSelectorHTML()}
                            </div>
                        </div>
                    </div>
                `,
                confirmButtonText: 'Create Group',
                showCancelButton: true,
                cancelButtonText: 'Cancel'
            }).then((result) => {
                if (result.isConfirmed) {
                    this.createLightGroup();
                }
            });
        },

        getBulbSelectorHTML() {
            return `<p style="color: #adb5bd; text-align: center;">Bulb selection coming soon...</p>`;
        },

        createLightGroup() {
            const groupName = document.getElementById('group-name')?.value;
            if (!groupName) {
                this.showToast('Please enter a group name', 'error');
                return;
            }
            this.showToast(`Group "${groupName}" created!`, 'success');
        },

        adjustBrightnessBatch(delta) {
            if (this.state.selectedBulbs.size === 0) {
                this.showToast('Select bulbs first', 'warning');
                return;
            }
            
            const newBrightness = Math.max(0, Math.min(100, this.state.brightnessLevel + delta));
            this.state.brightnessLevel = newBrightness;
            
            fetch('/api/services/lifx/set_state', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({
                    selector: Array.from(this.state.selectedBulbs).map(id => `id:${id}`).join(','),
                    brightness: newBrightness / 100
                })
            }).then(res => res.json())
              .then(data => {
                  if (data.success) {
                      this.showToast(`Brightness: ${newBrightness}%`, 'success');
                  }
              });
        },

        setVisualizationMode(mode) {
            this.state.visualizationMode = mode;
            const vizContainer = document.querySelector('.frequency-viz');
            if (!vizContainer) return;
            
            vizContainer.className = `frequency-viz visualization-${mode}`;
            
            switch(mode) {
                case 'bars':
                    vizContainer.style.flexDirection = 'row';
                    vizContainer.style.alignItems = 'flex-end';
                    break;
                case 'wave':
                    vizContainer.style.flexDirection = 'row';
                    vizContainer.style.alignItems = 'center';
                    break;
                case 'circular':
                    vizContainer.style.display = 'flex';
                    vizContainer.style.flexWrap = 'wrap';
                    vizContainer.style.justifyContent = 'center';
                    break;
            }
            
            this.showToast(`Visualization: ${mode}`, 'info');
            this.saveVisualizationPreference(mode);
        },
        
        saveVisualizationPreference(mode) {
            try {
                localStorage.setItem('lifx-viz-mode', mode);
            } catch (e) {
                console.warn('[LIFXMediaTouchV2] Failed to save viz preference:', e);
            }
        },
        
        loadVisualizationPreference() {
            try {
                const saved = localStorage.getItem('lifx-viz-mode');
                if (saved && ['bars', 'wave', 'circular'].includes(saved)) {
                    this.state.visualizationMode = saved;
                    return saved;
                }
            } catch (e) {
                console.warn('[LIFXMediaTouchV2] Failed to load viz preference:', e);
            }
            return 'bars';
        },

        setupMediaPlayers() {
            this.mediaPlayers = {
                spotify: null,
                youtube: null,
                plex: null,
                tidal: null,
                apple_music: null,
                radio: null
            };
            
            this.connectSpotify();
            this.setupMediaSyncButton();
            this.setupNowPlayingDisplay();
        },
        
        async connectSpotify() {
            try {
                const response = await fetch('/api/services/spotify/status');
                const data = await response.json();
                if (data.connected) {
                    this.mediaPlayers.spotify = data;
                    this.startMediaPlaybackMonitor();
                }
            } catch (error) {
                console.warn('Spotify not available');
            }
        },
        
        startMediaPlaybackMonitor() {
            setInterval(async () => {
                try {
                    const response = await fetch('/api/services/spotify/now-playing');
                    const data = await response.json();
                    if (data.track) {
                        this.updateMediaDisplay(data.track);
                    }
                } catch (error) {
                    // Silent fail - media may not be playing
                }
            }, 5000);
        },
        
        updateMediaDisplay(track) {
            const trackName = document.getElementById('media-track-name');
            const artistName = document.getElementById('media-artist-name');
            if (trackName) trackName.textContent = track.name || 'No Track';
            if (artistName) artistName.textContent = track.artist || 'Unknown Artist';
        },
        
        setupMediaSyncButton() {
            const syncBtn = document.getElementById('media-sync-toggle');
            if (!syncBtn) return;
            
            syncBtn.addEventListener('click', () => {
                this.toggleMediaSync();
                syncBtn.classList.toggle('active', this.state.mediaSyncActive);
            });
            
            this.setupMediaSyncModes();
        },
        
        setupMediaSyncModes() {
            const modeButtons = document.querySelectorAll('.media-sync-mode-btn');
            modeButtons.forEach(btn => {
                btn.addEventListener('click', (e) => {
                    const mode = e.currentTarget.dataset.mode;
                    this.setMediaSyncMode(mode);
                    modeButtons.forEach(b => b.classList.remove('active'));
                    e.currentTarget.classList.add('active');
                });
            });
        },
        
        setMediaSyncMode(mode) {
            this.state.mediaSyncMode = mode;
            this.showToast(`Media sync mode: ${mode}`, 'info');
            
            switch(mode) {
                case 'beat':
                    this.startBeatDetection();
                    break;
                case 'ambient':
                    this.startAmbientAnalysis();
                    break;
                case 'spectrum':
                    this.startSpectrumAnalysis();
                    break;
                case 'off':
                    this.disableMediaSync();
                    break;
            }
        },
        
        toggleMediaSync() {
            if (this.state.mediaSyncActive) {
                this.disableMediaSync();
                this.showToast('Media sync disabled', 'info');
            } else {
                this.enableMediaSync();
                this.showToast('Media sync enabled - lights will pulse to the beat!', 'success');
            }
        },

        setupLightGroups() {
            // Initialize light group management
            this.lightGroups = new Map();
        },

        setupVolumeSliders() {
            // Setup volume control sliders for media
            const volumeSliders = document.querySelectorAll('.media-volume-slider');
            volumeSliders.forEach(slider => {
                slider.addEventListener('input', (e) => {
                    this.setVolume(e.target.value);
                });
            });
        },

        setupBrightnessSliders() {
            // Setup brightness sliders
            const brightnessSliders = document.querySelectorAll('.lifx-brightness-slider-input');
            brightnessSliders.forEach(slider => {
                slider.addEventListener('input', (e) => {
                    this.setGlobalBrightness(e.target.value);
                });
            });
        },

        setVolume(level) {
            // Implement volume control via WebSocket
            if (window.websocket && websocket.readyState === WebSocket.OPEN) {
                websocket.send(JSON.stringify({
                    type: 'command',
                    id: `volume_${Date.now()}`,
                    command: 'set_volume',
                    args: { level: parseInt(level) }
                }));
            }
        },

        setGlobalBrightness(level) {
            this.state.brightnessLevel = parseInt(level);
            fetch('/api/services/lifx/set_state', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({
                    selector: 'all',
                    brightness: level / 100
                })
            });
        },

        enableMediaSync() {
            this.state.mediaSyncActive = true;
            this.startBeatDetection();
        },

        disableMediaSync() {
            this.state.mediaSyncActive = false;
            if (this.audioContext) {
                try {
                    this.audioContext.suspend();
                } catch (error) {
                    console.warn('[LIFXMediaTouchV2] Audio context suspend failed:', error);
                }
            }
            if (this.analyser) {
                this.analyser.disconnect();
                this.analyser = null;
            }
            this.showToast('Media sync disabled', 'info');
        },

        startBeatDetection() {
            if (!navigator.mediaDevices || !navigator.mediaDevices.getUserMedia) {
                console.warn('[LIFXMediaTouchV2] Media devices not available');
                this.showToast('Beat detection not available on this device', 'warning');
                return;
            }
            
            navigator.mediaDevices.getUserMedia({ audio: true })
                .then(stream => {
                    try {
                        this.audioContext = new (window.AudioContext || window.webkitAudioContext)();
                        this.analyser = this.audioContext.createAnalyser();
                        const source = this.audioContext.createMediaStreamSource(stream);
                        source.connect(this.analyser);
                        this.analyser.fftSize = 256;
                        this.detectBeats();
                        this.showToast('Beat detection active', 'success');
                    } catch (error) {
                        console.error('[LIFXMediaTouchV2] Audio context error:', error);
                        this.showToast('Audio initialization failed', 'error');
                    }
                })
                .catch(err => {
                    console.error('[LIFXMediaTouchV2] Beat detection error:', err);
                    this.showToast(`Beat detection unavailable: ${err.message}`, 'warning');
                });
        },

        detectBeats() {
            if (!this.analyser || !this.state.mediaSyncActive) return;
            
            try {
                const dataArray = new Uint8Array(this.analyser.frequencyBinCount);
                this.analyser.getByteFrequencyData(dataArray);
                
                this.state.frequencyData = new Uint8Array(6);
                
                const bands = {
                    subBass: dataArray.slice(0, 4),
                    bass: dataArray.slice(4, 10),
                    lowMid: dataArray.slice(10, 20),
                    mid: dataArray.slice(20, 40),
                    highMid: dataArray.slice(40, 80),
                    treble: dataArray.slice(80, 128)
                };
                
                const bandAverages = {
                    subBass: bands.subBass.reduce((a, b) => a + b, 0) / bands.subBass.length,
                    bass: bands.bass.reduce((a, b) => a + b, 0) / bands.bass.length,
                    lowMid: bands.lowMid.reduce((a, b) => a + b, 0) / bands.lowMid.length,
                    mid: bands.mid.reduce((a, b) => a + b, 0) / bands.mid.length,
                    highMid: bands.highMid.reduce((a, b) => a + b, 0) / bands.highMid.length,
                    treble: bands.treble.reduce((a, b) => a + b, 0) / bands.treble.length
                };
                
                this.state.frequencyData[0] = bandAverages.subBass;
                this.state.frequencyData[1] = bandAverages.bass;
                this.state.frequencyData[2] = bandAverages.lowMid;
                this.state.frequencyData[3] = bandAverages.mid;
                this.state.frequencyData[4] = bandAverages.highMid;
                this.state.frequencyData[5] = bandAverages.treble;
                
                const bassEnergy = (bandAverages.subBass + bandAverages.bass) / 2;
                const lowMidEnergy = bandAverages.lowMid;
                const totalEnergy = Object.values(bandAverages).reduce((a, b) => a + b, 0) / 6;
                
                this.state.beatHistory.push(bassEnergy);
                if (this.state.beatHistory.length > 30) {
                    this.state.beatHistory.shift();
                }
                
                const avgEnergy = this.state.beatHistory.reduce((a, b) => a + b, 0) / this.state.beatHistory.length;
                const variance = this.state.beatHistory.reduce((sum, val) => sum + Math.pow(val - avgEnergy, 2), 0) / this.state.beatHistory.length;
                const stdDev = Math.sqrt(variance);
                
                if (this.config.enableAdaptiveSensitivity && !this.state.sensitivityCalibrated) {
                    this.state.baselineEnergy = avgEnergy;
                    this.state.sensitivityCalibrated = true;
                }
                
                const energyRatio = avgEnergy / 255;
                const stdDevRatio = stdDev / 50;
                const dynamicThreshold = energyRatio * 0.6 + stdDevRatio * 0.4 + 0.15;
                this.state.adaptiveThreshold = Math.max(0.5, Math.min(0.9, dynamicThreshold));
                
                const beatThreshold = Math.max(
                    this.state.adaptiveThreshold,
                    this.config.beatDetectionThreshold * 0.85 * (1 - this.state.beatSensitivity)
                );
                
                const bassRatio = bassEnergy / 255;
                const bassSpike = bassRatio > (this.state.lastBassEnergy / 255) * 1.15;
                const bassTransient = bassEnergy > (this.state.baselineEnergy * 1.3);
                const lowMidSupport = lowMidEnergy > (bassEnergy * 0.4);
                const energySpike = totalEnergy > (avgEnergy + stdDev * 0.8);
                const isBeat = (bassRatio > beatThreshold && bassEnergy > 160 && bassSpike) ||
                               (bassTransient && lowMidSupport && energySpike);
                
                if (this.state.beatCalibrationInProgress) {
                    this.state.beatCalibrationData.push({
                        bassEnergy,
                        totalEnergy,
                        stdDev,
                        timestamp: Date.now()
                    });
                    if (this.state.beatCalibrationData.length >= this.config.beatCalibrationSamples) {
                        this.completeBeatCalibration();
                    }
                }
                
                this.state.lastBassEnergy = bassEnergy;
                
                if (isBeat && Date.now() - this.state.lastBeatTime > 180) {
                    const prevBeatTime = this.state.lastBeatTime;
                    this.state.lastBeatTime = Date.now();
                    
                    const interval = prevBeatTime ? (this.state.lastBeatTime - prevBeatTime) : 500;
                    const instantBPM = Math.round(60000 / interval);
                    
                    if (instantBPM > 60 && instantBPM < 200) {
                        this.state.bpmHistory.push(instantBPM);
                        if (this.state.bpmHistory.length > 12) {
                            this.state.bpmHistory.shift();
                        }
                        
                        const sortedBPM = [...this.state.bpmHistory].sort((a, b) => a - b);
                        const median = sortedBPM[Math.floor(sortedBPM.length / 2)];
                        
                        const bpmVariance = this.state.bpmHistory.reduce((sum, bpm) => sum + Math.pow(bpm - median, 2), 0) / this.state.bpmHistory.length;
                        const bpmStdDev = Math.sqrt(bpmVariance);
                        
                        if (bpmStdDev < 15) {
                            const weights = this.state.bpmHistory.map((bpm, idx) => Math.pow(0.9, this.state.bpmHistory.length - idx - 1));
                            const weightedSum = this.state.bpmHistory.reduce((sum, bpm, idx) => sum + bpm * weights[idx], 0);
                            const weightTotal = weights.reduce((a, b) => a + b, 0);
                            this.state.bpmSmoothed = Math.round(weightedSum / weightTotal);
                        } else {
                            this.state.bpmSmoothed = median;
                        }
                    }
                    
                    this.state.bpmDetected = this.state.bpmSmoothed || instantBPM;
                    this.triggerBeatEffect(bandAverages);
                    this.updateFrequencyVisualization(bandAverages);
                    this.updateBPMVisualization();
                }
                
                this.updateRealtimeBPM();
                this.updateBeatCalibrationDisplay(bandAverages);
                requestAnimationFrame(this.detectBeats.bind(this));
            } catch (error) {
                console.error('[LIFXMediaTouchV2] Beat detection error:', error);
            }
        },

        startBeatCalibration() {
            this.state.beatCalibrationInProgress = true;
            this.state.beatCalibrationData = [];
            this.showToast('Beat detection calibration started - let it analyze for a few seconds', 'info');
            this.triggerHaptic('calibration');
        },
        
        completeBeatCalibration() {
            this.state.beatCalibrationInProgress = false;
            const data = this.state.beatCalibrationData;
            
            if (data.length < 10) {
                this.showToast('Calibration failed - not enough data', 'error');
                this.triggerHaptic('error');
                return;
            }
            
            const avgBassEnergy = data.reduce((sum, d) => sum + d.bassEnergy, 0) / data.length;
            const avgStdDev = data.reduce((sum, d) => sum + d.stdDev, 0) / data.length;
            const maxBassEnergy = Math.max(...data.map(d => d.bassEnergy));
            const minBassEnergy = Math.min(...data.map(d => d.bassEnergy));
            
            const optimalThreshold = (avgBassEnergy / 255) + (avgStdDev / 255) * 0.5;
            this.state.beatSensitivity = Math.max(0.3, Math.min(0.9, optimalThreshold));
            
            this.showToast(`Beat calibration complete! Sensitivity: ${Math.round(this.state.beatSensitivity * 100)}%`, 'success');
            this.triggerHaptic('success');
            
            this.updateBeatCalibrationUI();
        },
        
        setBeatSensitivity(level) {
            this.state.beatSensitivity = Math.max(0.1, Math.min(1.0, level));
            this.showToast(`Beat sensitivity: ${Math.round(level * 100)}%`, 'info');
            this.updateBeatCalibrationUI();
        },
        
        updateBeatCalibrationUI() {
            const statsDiv = document.querySelector('.beat-detection-calibration .calibration-stats');
            if (!statsDiv) return;
            
            const now = Date.now();
            const lastBeat = this.state.lastBeatTime || 0;
            const timeSinceBeat = now - lastBeat;
            
            statsDiv.innerHTML = `
                <div class="stat-item">
                    <span class="stat-label">Sensitivity</span>
                    <span class="stat-value">${Math.round(this.state.beatSensitivity * 100)}%</span>
                </div>
                <div class="stat-item">
                    <span class="stat-label">Last Beat</span>
                    <span class="stat-value">${timeSinceBeat < 2000 ? Math.round(timeSinceBeat) + 'ms' : '--'}</span>
                </div>
                <div class="stat-item">
                    <span class="stat-label">BPM</span>
                    <span class="stat-value">${this.state.bpmDetected || '--'}</span>
                </div>
                <div class="stat-item">
                    <span class="stat-label">Threshold</span>
                    <span class="stat-value">${Math.round(this.state.adaptiveThreshold * 100)}%</span>
                </div>
            `;
        },
        
        updateBeatCalibrationDisplay(bandAverages) {
            if (!this.state.beatCalibrationInProgress) return;
            
            const progressEl = document.getElementById('beat-calibration-progress');
            if (progressEl) {
                const progress = (this.state.beatCalibrationData.length / this.config.beatCalibrationSamples) * 100;
                progressEl.style.width = `${progress}%`;
            }
            
            const liveDisplay = document.getElementById('beat-calibration-live');
            if (liveDisplay) {
                liveDisplay.innerHTML = `
                    <div class="live-value">${Math.round(bandAverages.bass || 0)}</div>
                    <div class="live-label">Bass Energy</div>
                `;
            }
        },

        triggerBeatEffect(bandAverages = {}) {
            if (!this.state.mediaSyncActive) return;
            
            try {
                const intensity = Math.min(1, (bandAverages.bass || 200) / 255);
                const duration = 60 + (1 - intensity) * 100;
                
                const effectConfig = {
                    selector: 'all',
                    brightness: Math.min(1, 0.6 + intensity * 0.4),
                    duration: duration / 1000
                };
                
                if (this.state.mediaSyncMode === 'color' || this.state.mediaSyncMode === 'spectrum') {
                    const hue = this.bpmToHue(this.state.bpmDetected);
                    effectConfig.color = `hsb(${hue},100,100)`;
                }
                
                fetch('/api/services/lifx/set_state', {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify(effectConfig)
                }).catch(err => {
                    console.warn('[LIFXMediaTouchV2] Beat effect failed:', err);
                });
                
                setTimeout(() => {
                    fetch('/api/services/lifx/set_state', {
                        method: 'POST',
                        headers: { 'Content-Type': 'application/json' },
                        body: JSON.stringify({
                            selector: 'all',
                            brightness: this.state.brightnessLevel / 100,
                            duration: 0.15
                        })
                    }).catch(err => {
                        console.warn('[LIFXMediaTouchV2] Beat recovery failed:', err);
                    });
                }, duration);
                
                this.showBeatFlashOverlay(intensity);
                
                if (this.config.enableHapticFeedback && navigator.vibrate && intensity > 0.6) {
                    const hapticPattern = intensity > 0.85 
                        ? [20, 15, 20, 15, 20]
                        : intensity > 0.7
                        ? [30, 20, 30]
                        : [25];
                    try {
                        navigator.vibrate(hapticPattern);
                    } catch (e) {
                        console.warn('[LIFXMediaTouchV2] Beat haptic failed:', e);
                    }
                }
            } catch (error) {
                console.error('[LIFXMediaTouchV2] Trigger beat effect error:', error);
            }
        },

        bpmToHue(bpm) {
            if (!bpm || bpm < 60) bpm = 60;
            if (bpm > 200) bpm = 200;
            return Math.round(((bpm - 60) / 140) * 360) % 360;
        },

        showBeatFlashOverlay(intensity) {
            let overlay = document.querySelector('.lifx-beat-flash-overlay');
            if (!overlay) {
                overlay = document.createElement('div');
                overlay.className = 'lifx-beat-flash-overlay';
                overlay.style.cssText = `
                    position: fixed;
                    top: 0;
                    left: 0;
                    right: 0;
                    bottom: 0;
                    background: radial-gradient(circle, rgba(0, 212, 255, ${0.15 * intensity}) 0%, transparent 70%);
                    pointer-events: none;
                    z-index: 99998;
                    opacity: 0;
                    transition: opacity 0.08s ease;
                `;
                document.body.appendChild(overlay);
            }
            
            overlay.style.opacity = intensity * 0.3;
            setTimeout(() => {
                overlay.style.opacity = 0;
            }, 80);
        },

        updateFrequencyVisualization(bandAverages) {
            const vizContainer = document.querySelector('.frequency-viz');
            if (!vizContainer) return;
            
            const bands = ['subBass', 'bass', 'lowMid', 'mid', 'highMid', 'treble'];
            const bandColors = {
                subBass: '#ff4545',
                bass: '#ff6b6b',
                lowMid: '#ffa500',
                mid: '#ffc93c',
                highMid: '#7fdbca',
                treble: '#00d4ff'
            };
            
            let peakDetected = false;
            let maxEnergy = 0;
            
            bands.forEach((band, index) => {
                const bar = document.getElementById(`band-${band.toLowerCase()}`);
                if (bar) {
                    const height = Math.max(5, (bandAverages[band] / 255) * 100);
                    const energy = bandAverages[band] / 255;
                    
                    if (energy > maxEnergy) {
                        maxEnergy = energy;
                    }
                    
                    bar.style.height = `${height}%`;
                    bar.style.background = bandColors[band];
                    
                    if (bandAverages[band] > 220) {
                        bar.classList.add('peak');
                        peakDetected = true;
                        setTimeout(() => bar.classList.remove('peak'), 100);
                    }
                    
                    const glowIntensity = Math.min(1, energy * 1.5);
                    bar.style.boxShadow = `0 0 ${10 + glowIntensity * 20}px ${bandColors[band]}`;
                }
            });
            
            if (peakDetected && maxEnergy > 0.85) {
                this.triggerBeatFlash(maxEnergy);
            }
        },
        
        triggerBeatFlash(energy) {
            const flash = document.createElement('div');
            flash.className = 'beat-flash';
            flash.style.cssText = `
                position: fixed;
                top: 0;
                left: 0;
                right: 0;
                bottom: 0;
                background: radial-gradient(circle, rgba(255, 107, 107, ${energy * 0.3}) 0%, transparent 70%);
                pointer-events: none;
                z-index: 9999;
                animation: beat-flash-anim 0.3s ease-out forwards;
            `;
            
            if (!document.getElementById('beat-flash-style')) {
                const style = document.createElement('style');
                style.id = 'beat-flash-style';
                style.textContent = `
                    @keyframes beat-flash-anim {
                        0% { opacity: 1; transform: scale(1); }
                        100% { opacity: 0; transform: scale(1.5); }
                    }
                `;
                document.head.appendChild(style);
            }
            
            document.body.appendChild(flash);
            setTimeout(() => {
                if (flash.parentNode) flash.parentNode.removeChild(flash);
            }, 300);
        },

        updateRealtimeBPM() {
            const bpmDisplay = document.querySelector('.bpm-value');
            const bpmIndicator = document.querySelector('.bpm-realtime-indicator');
            
            if (!bpmDisplay) return;
            
            const now = Date.now();
            const lastBeat = this.state.lastBeatTime || 0;
            
            if (now - lastBeat < 2000 && this.state.mediaSyncActive) {
                if (bpmIndicator) bpmIndicator.classList.add('visible');
                bpmDisplay.textContent = this.state.bpmDetected || '--';
                bpmDisplay.style.color = '#ff6b6b';
            } else {
                if (bpmIndicator) bpmIndicator.classList.remove('visible');
                bpmDisplay.textContent = '--';
                bpmDisplay.style.color = '#adb5bd';
            }
        },

        updateBPMVisualization() {
            const bpmBars = document.querySelectorAll('.bpm-bar');
            if (!bpmBars || bpmBars.length === 0) return;
            
            const bpm = this.state.bpmDetected || 0;
            const normalizedBPM = bpm / 200;
            
            bpmBars.forEach((bar, index) => {
                const delay = index * 0.1;
                const targetHeight = 20 + (normalizedBPM * 60) * (0.5 + Math.sin(index + Date.now() / 200) * 0.5);
                bar.style.height = `${Math.min(100, targetHeight)}%`;
                
                if (this.state.lastBeatTime && Date.now() - this.state.lastBeatTime < 150) {
                    bar.classList.add('active');
                    setTimeout(() => bar.classList.remove('active'), 150);
                }
            });
            
            const bpmRing = document.querySelector('.bpm-ring');
            if (bpmRing) {
                const rotationSpeed = Math.max(0.5, 2 - normalizedBPM);
                bpmRing.style.animationDuration = `${rotationSpeed}s`;
                
                if (this.state.lastBeatTime && Date.now() - this.state.lastBeatTime < 150) {
                    bpmRing.classList.add('pulsing');
                    setTimeout(() => bpmRing.classList.remove('pulsing'), 150);
                }
            }
        },
    };

    window.LIFXMediaTouchV2 = LIFXMediaTouchV2;

    document.addEventListener('DOMContentLoaded', () => {
        LIFXMediaTouchV2.init();
    });
})();
