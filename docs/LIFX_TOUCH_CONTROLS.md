# LIFX Touch Gesture Controls

## Overview

Enhanced touch gesture support for LIFX lighting control, providing intuitive swipe and pinch gestures to control brightness, color temperature, and scenes.

## Features

### Gesture Controls

- **Swipe Up/Down**: Adjust brightness (+/- 10%)
- **Swipe Left/Right**: Adjust color temperature (cooler/warmer)
- **Pinch In/Out**: Cycle through preset scenes
- **Tap**: Select bulb for control

### Preset Scenes

1. **Relax**: 40% brightness, 2700K (warm white)
2. **Focus**: 80% brightness, 5000K (neutral white)
3. **Energize**: 100% brightness, 6500K (cool white)
4. **Night**: 20% brightness, 2000K (very warm)

## Files Added

### JavaScript
- `www/assets/js/lifx-touch-controls.js` - Main touch control logic

### CSS
- `www/assets/css/lifx-touch-controls.css` - Visual styles and animations

## Integration

### HTML Integration

Add the following to your dashboard HTML after loading the core JS files:

```html
<!-- LIFX Touch Controls -->
<link rel="stylesheet" href="/assets/css/lifx-touch-controls.css">
<script src="/assets/js/lifx-touch-controls.js"></script>
```

### Usage

The touch controls automatically enable on touch-enabled devices. To manually control:

```javascript
// Enable touch controls
LifXTouchControls.enable();

// Disable touch controls
LifXTouchControls.disable();

// Programmatically select a bulb
LifXTouchControls.selectBulb('bulb-id-here');

// Adjust brightness
LifXTouchControls.adjustBrightness(10); // +10%
LifXTouchControls.adjustBrightness(-10); // -10%

// Adjust color temperature
LifXTouchControls.adjustColorTemp(200); // warmer
LifXTouchControls.adjustColorTemp(-200); // cooler

// Scene control
LifXTouchControls.nextScene();
LifXTouchControls.previousScene();
LifXTouchControls.applyScene('relax');
```

## Visual Feedback

### Selected Bulb
- Blue border glow (#00d4ff)
- Checkmark indicator in top-right corner
- Enhanced shadow effect

### Brightness/Color Adjustment
- Pulse animation on change
- Temporary overlay showing current level
- Smooth transitions (0.3s duration)

### Gesture Hints
- Centered overlay with gradient background
- Fade in/out animation
- Disappears after 2 seconds

## Requirements

- Requires core gesture recognition system (`core.js`)
- Requires jQuery for API calls
- Requires existing LIFX service endpoints:
  - `/api/services/lifx/set_state`
  - `/api/services/lifx/set_color`

## Browser Support

- iOS Safari (iOS 10+)
- Android Chrome (Android 6+)
- Desktop browsers with touch screens
- Falls back gracefully on non-touch devices

## Performance Considerations

- Uses passive event listeners for better scroll performance
- Debounced API calls prevent flooding
- CSS animations use GPU acceleration
- Minimal memory footprint

## Customization

### Adjust Sensitivity

Edit `TouchGestures.minSwipeDistance` and `TouchGestures.maxSwipeTime` in `core.js`:

```javascript
var TouchGestures = {
    minSwipeDistance: 50, // pixels
    maxSwipeTime: 300,    // milliseconds
    // ...
};
```

### Add Custom Scenes

Modify the `sceneSettings` object in `lifx-touch-controls.js`:

```javascript
const sceneSettings = {
    relax: { brightness: 40, kelvin: 2700 },
    focus: { brightness: 80, kelvin: 5000 },
    energize: { brightness: 100, kelvin: 6500 },
    night: { brightness: 20, kelvin: 2000 },
    // Add your custom scenes here
    'party': { brightness: 100, kelvin: 4000 }
};
```

### Customize Colors

Edit CSS variables in `lifx-touch-controls.css`:

```css
.lifx-bulb-control.selected {
    border: 2px solid #00d4ff; /* Change accent color */
    box-shadow: 0 0 15px rgba(0, 212, 255, 0.5);
}
```

## Troubleshooting

### Gestures Not Working

1. Ensure `core.js` is loaded before `lifx-touch-controls.js`
2. Check browser console for errors
3. Verify touch device detection: `is_touch_enabled()`
4. Confirm gesture callbacks are registered

### Bulbs Not Responding

1. Check LIFX service is running
2. Verify bulb ID is correct
3. Check network connectivity
4. Review API endpoint responses in browser dev tools

### Visual Issues

1. Clear browser cache
2. Check CSS file is loaded
3. Verify no conflicting styles
4. Test in different browsers

## Future Enhancements

Potential improvements for future versions:

- [ ] Multi-bulb selection (control multiple bulbs simultaneously)
- [ ] Custom gesture mappings
- [ ] Voice command integration
- [ ] Scheduling/timer gestures
- [ ] Energy usage feedback
- [ ] Sync with music/media
- [ ] Haptic feedback patterns
- [ ] Gesture macros (complex sequences)

## License

GPLv3 - See LICENSE file for details

## Author

Developed by Caleb Mitchell Smith for the Open Sam project
© 2021-2026 The Open Sam Foundation (OSF)
