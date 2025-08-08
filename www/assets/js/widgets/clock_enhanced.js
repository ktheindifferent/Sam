// ███████     █████     ███    ███    
// ██         ██   ██    ████  ████    
// ███████    ███████    ██ ████ ██    
//      ██    ██   ██    ██  ██  ██    
// ███████ ██ ██   ██ ██ ██      ██ ██ 
// Copyright 2021-2026 The Open Sam Foundation (OSF)
// Developed by Caleb Mitchell Smith (ktheindifferent, PixelCoda, p0indexter)
// Licensed under GPLv3....see LICENSE file.

class EnhancedClock {
    constructor(elementId, options = {}) {
        this.element = document.getElementById(elementId);
        this.options = {
            format: options.format || '12h', // '12h', '24h', 'digital', 'analog', 'binary', 'unix'
            showSeconds: options.showSeconds !== undefined ? options.showSeconds : true,
            showDate: options.showDate !== undefined ? options.showDate : false,
            showWeekday: options.showWeekday !== undefined ? options.showWeekday : false,
            showTimezone: options.showTimezone !== undefined ? options.showTimezone : false,
            dateFormat: options.dateFormat || 'long', // 'short', 'long', 'iso'
            theme: options.theme || 'default', // 'default', 'minimal', 'neon', 'retro', 'matrix'
            updateInterval: options.updateInterval || 1000,
            animations: options.animations !== undefined ? options.animations : true,
            locale: options.locale || 'en-US',
            timezone: options.timezone || Intl.DateTimeFormat().resolvedOptions().timeZone,
            customFormat: options.customFormat || null // Function for custom formatting
        };
        
        this.intervalId = null;
        this.init();
    }

    init() {
        if (!this.element) {
            console.error('Clock element not found');
            return;
        }
        
        this.setupStyles();
        this.start();
        
        // Add settings button if not in minimal mode
        if (this.options.theme !== 'minimal') {
            this.addSettingsButton();
        }
    }

    setupStyles() {
        const themes = {
            default: {
                fontFamily: 'system-ui, -apple-system, sans-serif',
                fontSize: '2rem',
                color: '#ffffff',
                textShadow: 'none',
                background: 'transparent'
            },
            minimal: {
                fontFamily: 'Helvetica, Arial, sans-serif',
                fontSize: '1.5rem',
                color: '#e0e0e0',
                textShadow: 'none',
                background: 'transparent'
            },
            neon: {
                fontFamily: 'monospace',
                fontSize: '2.5rem',
                color: '#00ff00',
                textShadow: '0 0 10px #00ff00, 0 0 20px #00ff00',
                background: 'linear-gradient(135deg, #1a1a2e 0%, #16213e 100%)'
            },
            retro: {
                fontFamily: '"Courier New", monospace',
                fontSize: '2rem',
                color: '#ff6b6b',
                textShadow: '2px 2px 0px #000000',
                background: 'linear-gradient(135deg, #667eea 0%, #764ba2 100%)'
            },
            matrix: {
                fontFamily: 'monospace',
                fontSize: '2rem',
                color: '#00ff41',
                textShadow: '0 0 5px #00ff41',
                background: '#000000'
            }
        };

        const theme = themes[this.options.theme] || themes.default;
        Object.assign(this.element.style, theme);
        
        if (this.options.animations) {
            this.element.style.transition = 'all 0.3s ease';
        }
    }

    formatTime(date) {
        if (this.options.customFormat) {
            return this.options.customFormat(date);
        }

        switch (this.options.format) {
            case '24h':
                return this.format24Hour(date);
            case 'digital':
                return this.formatDigital(date);
            case 'analog':
                return this.formatAnalog(date);
            case 'binary':
                return this.formatBinary(date);
            case 'unix':
                return this.formatUnix(date);
            case 'relative':
                return this.formatRelative(date);
            case 'fuzzy':
                return this.formatFuzzy(date);
            default:
                return this.format12Hour(date);
        }
    }

    format12Hour(date) {
        let hours = date.getHours();
        let minutes = date.getMinutes();
        let seconds = date.getSeconds();
        let period = hours >= 12 ? 'PM' : 'AM';
        
        hours = hours % 12 || 12;
        
        const timeString = this.padZero(hours) + ':' + this.padZero(minutes) + 
                          (this.options.showSeconds ? ':' + this.padZero(seconds) : '') +
                          ' ' + period;
        
        return this.addDateInfo(timeString, date);
    }

    format24Hour(date) {
        let hours = date.getHours();
        let minutes = date.getMinutes();
        let seconds = date.getSeconds();
        
        const timeString = this.padZero(hours) + ':' + this.padZero(minutes) + 
                          (this.options.showSeconds ? ':' + this.padZero(seconds) : '');
        
        return this.addDateInfo(timeString, date);
    }

    formatDigital(date) {
        const segments = {
            0: '⬚⬚⬚\n⬚  ⬚\n⬚  ⬚\n⬚  ⬚\n⬚⬚⬚',
            1: '  ⬚\n  ⬚\n  ⬚\n  ⬚\n  ⬚',
            2: '⬚⬚⬚\n   ⬚\n⬚⬚⬚\n⬚\n⬚⬚⬚',
            3: '⬚⬚⬚\n   ⬚\n⬚⬚⬚\n   ⬚\n⬚⬚⬚',
            4: '⬚  ⬚\n⬚  ⬚\n⬚⬚⬚\n   ⬚\n   ⬚',
            5: '⬚⬚⬚\n⬚\n⬚⬚⬚\n   ⬚\n⬚⬚⬚',
            6: '⬚⬚⬚\n⬚\n⬚⬚⬚\n⬚  ⬚\n⬚⬚⬚',
            7: '⬚⬚⬚\n   ⬚\n   ⬚\n   ⬚\n   ⬚',
            8: '⬚⬚⬚\n⬚  ⬚\n⬚⬚⬚\n⬚  ⬚\n⬚⬚⬚',
            9: '⬚⬚⬚\n⬚  ⬚\n⬚⬚⬚\n   ⬚\n⬚⬚⬚',
            ':': '\n●\n\n●\n'
        };
        
        const time = this.format24Hour(date).split(' ')[0];
        const lines = ['', '', '', '', ''];
        
        for (let char of time) {
            const segment = segments[char] || segments[':'];
            const segmentLines = segment.split('\n');
            for (let i = 0; i < 5; i++) {
                lines[i] += (segmentLines[i] || '') + ' ';
            }
        }
        
        return '<pre style="font-size: 0.8em; line-height: 1;">' + lines.join('\n') + '</pre>';
    }

    formatAnalog(date) {
        const hours = date.getHours() % 12;
        const minutes = date.getMinutes();
        const seconds = date.getSeconds();
        
        const hourAngle = (hours * 30) + (minutes * 0.5);
        const minuteAngle = minutes * 6;
        const secondAngle = seconds * 6;
        
        return `
            <svg width="150" height="150" viewBox="0 0 150 150">
                <circle cx="75" cy="75" r="70" fill="none" stroke="currentColor" stroke-width="2"/>
                ${this.drawClockNumbers()}
                <line x1="75" y1="75" x2="75" y2="30" 
                      stroke="currentColor" stroke-width="4" stroke-linecap="round"
                      transform="rotate(${hourAngle} 75 75)"/>
                <line x1="75" y1="75" x2="75" y2="20" 
                      stroke="currentColor" stroke-width="3" stroke-linecap="round"
                      transform="rotate(${minuteAngle} 75 75)"/>
                ${this.options.showSeconds ? `
                <line x1="75" y1="75" x2="75" y2="15" 
                      stroke="red" stroke-width="1" stroke-linecap="round"
                      transform="rotate(${secondAngle} 75 75)"/>` : ''}
                <circle cx="75" cy="75" r="4" fill="currentColor"/>
            </svg>
        `;
    }

    drawClockNumbers() {
        let numbers = '';
        for (let i = 1; i <= 12; i++) {
            const angle = (i * 30 - 90) * Math.PI / 180;
            const x = 75 + 55 * Math.cos(angle);
            const y = 75 + 55 * Math.sin(angle) + 5;
            numbers += `<text x="${x}" y="${y}" text-anchor="middle" font-size="14" fill="currentColor">${i}</text>`;
        }
        return numbers;
    }

    formatBinary(date) {
        const hours = date.getHours();
        const minutes = date.getMinutes();
        const seconds = date.getSeconds();
        
        const toBinary = (num) => num.toString(2).padStart(6, '0');
        
        const binaryTime = `
            <div style="font-family: monospace; font-size: 1.2em;">
                <div>H: ${toBinary(hours)} (${hours})</div>
                <div>M: ${toBinary(minutes)} (${minutes})</div>
                ${this.options.showSeconds ? `<div>S: ${toBinary(seconds)} (${seconds})</div>` : ''}
            </div>
        `;
        
        return binaryTime;
    }

    formatUnix(date) {
        return Math.floor(date.getTime() / 1000).toString();
    }

    formatRelative(date) {
        const now = new Date();
        const midnight = new Date(now);
        midnight.setHours(0, 0, 0, 0);
        
        const diff = date - midnight;
        const hours = Math.floor(diff / 3600000);
        const minutes = Math.floor((diff % 3600000) / 60000);
        
        if (hours === 0) {
            return `${minutes} minute${minutes !== 1 ? 's' : ''} past midnight`;
        } else if (hours === 12) {
            return `${minutes} minute${minutes !== 1 ? 's' : ''} past noon`;
        } else {
            return `${hours} hour${hours !== 1 ? 's' : ''} ${minutes} minute${minutes !== 1 ? 's' : ''} since midnight`;
        }
    }

    formatFuzzy(date) {
        const hours = date.getHours();
        const minutes = date.getMinutes();
        
        const hourNames = ['twelve', 'one', 'two', 'three', 'four', 'five', 
                          'six', 'seven', 'eight', 'nine', 'ten', 'eleven'];
        
        let fuzzyTime = '';
        
        if (minutes === 0) {
            fuzzyTime = `${hourNames[hours % 12]} o'clock`;
        } else if (minutes === 15) {
            fuzzyTime = `quarter past ${hourNames[hours % 12]}`;
        } else if (minutes === 30) {
            fuzzyTime = `half past ${hourNames[hours % 12]}`;
        } else if (minutes === 45) {
            fuzzyTime = `quarter to ${hourNames[(hours + 1) % 12]}`;
        } else if (minutes < 30) {
            fuzzyTime = `${minutes} past ${hourNames[hours % 12]}`;
        } else {
            fuzzyTime = `${60 - minutes} to ${hourNames[(hours + 1) % 12]}`;
        }
        
        return fuzzyTime.charAt(0).toUpperCase() + fuzzyTime.slice(1);
    }

    addDateInfo(timeString, date) {
        let result = timeString;
        
        if (this.options.showWeekday) {
            const weekdays = ['Sunday', 'Monday', 'Tuesday', 'Wednesday', 'Thursday', 'Friday', 'Saturday'];
            result += ' • ' + weekdays[date.getDay()];
        }
        
        if (this.options.showDate) {
            if (this.options.dateFormat === 'short') {
                result += ' • ' + date.toLocaleDateString(this.options.locale);
            } else if (this.options.dateFormat === 'iso') {
                result += ' • ' + date.toISOString().split('T')[0];
            } else {
                result += ' • ' + date.toLocaleDateString(this.options.locale, { 
                    year: 'numeric', 
                    month: 'long', 
                    day: 'numeric' 
                });
            }
        }
        
        if (this.options.showTimezone) {
            result += ' • ' + this.options.timezone;
        }
        
        return result;
    }

    padZero(num) {
        return num < 10 ? '0' + num : num.toString();
    }

    update() {
        const now = new Date();
        const formattedTime = this.formatTime(now);
        
        if (this.options.format === 'analog' || this.options.format === 'digital') {
            this.element.innerHTML = formattedTime;
        } else {
            this.element.textContent = formattedTime;
        }
        
        // Add subtle animation on update
        if (this.options.animations && this.options.showSeconds) {
            this.element.style.opacity = '0.8';
            setTimeout(() => {
                this.element.style.opacity = '1';
            }, 100);
        }
    }

    start() {
        this.update();
        this.intervalId = setInterval(() => this.update(), this.options.updateInterval);
    }

    stop() {
        if (this.intervalId) {
            clearInterval(this.intervalId);
            this.intervalId = null;
        }
    }

    setOptions(newOptions) {
        this.options = { ...this.options, ...newOptions };
        this.setupStyles();
        this.update();
    }

    addSettingsButton() {
        const settingsBtn = document.createElement('button');
        settingsBtn.innerHTML = '⚙️';
        settingsBtn.style.cssText = `
            position: absolute;
            top: 5px;
            right: 5px;
            background: transparent;
            border: none;
            cursor: pointer;
            font-size: 1rem;
            opacity: 0.5;
            transition: opacity 0.3s;
        `;
        settingsBtn.onmouseover = () => settingsBtn.style.opacity = '1';
        settingsBtn.onmouseout = () => settingsBtn.style.opacity = '0.5';
        settingsBtn.onclick = () => this.showSettingsDialog();
        
        if (this.element.parentElement) {
            this.element.parentElement.style.position = 'relative';
            this.element.parentElement.appendChild(settingsBtn);
        }
    }

    showSettingsDialog() {
        const dialog = document.createElement('div');
        dialog.style.cssText = `
            position: fixed;
            top: 50%;
            left: 50%;
            transform: translate(-50%, -50%);
            background: #2a2a3a;
            padding: 20px;
            border-radius: 10px;
            z-index: 10000;
            box-shadow: 0 10px 30px rgba(0,0,0,0.5);
            color: white;
            min-width: 300px;
        `;
        
        dialog.innerHTML = `
            <h3 style="margin-top: 0;">Clock Settings</h3>
            <div style="margin-bottom: 10px;">
                <label>Format: </label>
                <select id="clockFormat" style="background: #1a1a2e; color: white; border: 1px solid #444; padding: 5px;">
                    <option value="12h" ${this.options.format === '12h' ? 'selected' : ''}>12 Hour</option>
                    <option value="24h" ${this.options.format === '24h' ? 'selected' : ''}>24 Hour</option>
                    <option value="digital" ${this.options.format === 'digital' ? 'selected' : ''}>Digital</option>
                    <option value="analog" ${this.options.format === 'analog' ? 'selected' : ''}>Analog</option>
                    <option value="binary" ${this.options.format === 'binary' ? 'selected' : ''}>Binary</option>
                    <option value="unix" ${this.options.format === 'unix' ? 'selected' : ''}>Unix Timestamp</option>
                    <option value="relative" ${this.options.format === 'relative' ? 'selected' : ''}>Relative</option>
                    <option value="fuzzy" ${this.options.format === 'fuzzy' ? 'selected' : ''}>Fuzzy</option>
                </select>
            </div>
            <div style="margin-bottom: 10px;">
                <label>Theme: </label>
                <select id="clockTheme" style="background: #1a1a2e; color: white; border: 1px solid #444; padding: 5px;">
                    <option value="default" ${this.options.theme === 'default' ? 'selected' : ''}>Default</option>
                    <option value="minimal" ${this.options.theme === 'minimal' ? 'selected' : ''}>Minimal</option>
                    <option value="neon" ${this.options.theme === 'neon' ? 'selected' : ''}>Neon</option>
                    <option value="retro" ${this.options.theme === 'retro' ? 'selected' : ''}>Retro</option>
                    <option value="matrix" ${this.options.theme === 'matrix' ? 'selected' : ''}>Matrix</option>
                </select>
            </div>
            <div style="margin-bottom: 10px;">
                <label><input type="checkbox" id="showSeconds" ${this.options.showSeconds ? 'checked' : ''}> Show Seconds</label>
            </div>
            <div style="margin-bottom: 10px;">
                <label><input type="checkbox" id="showDate" ${this.options.showDate ? 'checked' : ''}> Show Date</label>
            </div>
            <div style="margin-bottom: 10px;">
                <label><input type="checkbox" id="showWeekday" ${this.options.showWeekday ? 'checked' : ''}> Show Weekday</label>
            </div>
            <div style="margin-bottom: 10px;">
                <label><input type="checkbox" id="showTimezone" ${this.options.showTimezone ? 'checked' : ''}> Show Timezone</label>
            </div>
            <div style="text-align: right;">
                <button id="cancelSettings" style="margin-right: 10px; padding: 5px 15px; background: #666; color: white; border: none; border-radius: 5px; cursor: pointer;">Cancel</button>
                <button id="saveSettings" style="padding: 5px 15px; background: #27a0b9; color: white; border: none; border-radius: 5px; cursor: pointer;">Save</button>
            </div>
        `;
        
        document.body.appendChild(dialog);
        
        document.getElementById('cancelSettings').onclick = () => {
            document.body.removeChild(dialog);
        };
        
        document.getElementById('saveSettings').onclick = () => {
            this.setOptions({
                format: document.getElementById('clockFormat').value,
                theme: document.getElementById('clockTheme').value,
                showSeconds: document.getElementById('showSeconds').checked,
                showDate: document.getElementById('showDate').checked,
                showWeekday: document.getElementById('showWeekday').checked,
                showTimezone: document.getElementById('showTimezone').checked
            });
            
            // Save to localStorage
            localStorage.setItem('clockSettings', JSON.stringify(this.options));
            
            document.body.removeChild(dialog);
        };
    }

    static loadFromStorage(elementId) {
        const savedSettings = localStorage.getItem('clockSettings');
        const options = savedSettings ? JSON.parse(savedSettings) : {};
        return new EnhancedClock(elementId, options);
    }
}

// Initialize default clock if element exists
document.addEventListener('DOMContentLoaded', () => {
    if (document.getElementById('Clock')) {
        // Load saved settings or use defaults
        const clock = EnhancedClock.loadFromStorage('Clock');
        
        // Make clock instance globally available
        window.samClock = clock;
    }
});

// Export for module usage
if (typeof module !== 'undefined' && module.exports) {
    module.exports = EnhancedClock;
}