/**
 * SAM Media Center Enhanced
 * Advanced media controls with beat synchronization and visual feedback
 * Copyright 2021-2026 The Open Sam Foundation (OSF)
 */

export class MediaCenterEnhanced {
    constructor(options = {}) {
        this.config = {
            enableBeatSync: options.enableBeatSync ?? true,
            enableVisualizer: options.enableVisualizer ?? true,
            beatThreshold: options.beatThreshold ?? 0.75,
            visualizerMode: options.visualizerMode ?? 'bars',
            bpmSmoothing: options.bpmSmoothing ?? 5,
            bassFrequencyRange: options.bassFrequencyRange ?? [20, 250],
            midFrequencyRange: options.midFrequencyRange ?? [250, 4000],
            highFrequencyRange: options.highFrequencyRange ?? [4000, 20000]
        };
        
        this.state = {
            isPlaying: false,
            currentTrack: null,
            bpm: 0,
            bpmHistory: [],
            beatDetected: false,
            lastBeatTime: 0,
            audioContext: null,
            analyser: null,
            frequencyData: null,
            waveformData: null,
            visualizerActive: false,
            beatSyncEnabled: false,
            crossfadeActive: false,
            crossfadeProgress: 0,
            nowPlayingHistory: [],
            repeatMode: 'off',
            shuffleMode: false,
            volume: 0.7,
            isMuted: false
        };
        
        this.eventCallbacks = {
            play: [], pause: [], stop: [], trackChange: [],
            beatDetected: [], bpmUpdate: [], volumeChange: [],
            visualizerUpdate: [], shuffleChange: [], repeatChange: []
        };
        
        this.canvas = null;
        this.ctx = null;
        this.animationFrame = null;
    }

    async initializeAudioContext() {
        if (this.state.audioContext) return;
        
        try {
            this.state.audioContext = new (window.AudioContext || window.webkitAudioContext)();
            this.state.analyser = this.state.audioContext.createAnalyser();
            this.state.analyser.fftSize = 256;
            this.state.analyser.smoothingTimeConstant = 0.8;
            
            const bufferLength = this.state.analyser.frequencyBinCount;
            this.state.frequencyData = new Uint8Array(bufferLength);
            this.state.waveformData = new Uint8Array(bufferLength);
        } catch (error) {
            console.error('Failed to initialize audio context:', error);
        }
    }

    connectAudioSource(element) {
        if (!this.state.audioContext || !element) return;
        
        try {
            let source;
            if (element.tagName === 'AUDIO' || element.tagName === 'VIDEO') {
                source = this.state.audioContext.createMediaElementSource(element);
            }
            source.connect(this.state.analyser);
            this.state.analyser.connect(this.state.audioContext.destination);
        } catch (error) {
            console.error('Failed to connect audio source:', error);
        }
    }

    detectBeat() {
        if (!this.state.analyser) return false;
        
        this.state.analyser.getByteFrequencyData(this.state.frequencyData);
        
        const bassRange = this.config.bassFrequencyRange;
        const bassBins = Math.floor((bassRange[1] - bassRange[0]) / (this.state.audioContext.sampleRate / this.state.analyser.fftSize));
        
        let bassEnergy = 0;
        for (let i = 0; i < bassBins; i++) {
            bassEnergy += this.state.frequencyData[i];
        }
        bassEnergy /= bassBins;
        
        const now = Date.now();
        const timeSinceLastBeat = now - this.state.lastBeatTime;
        
        this.state.frequencyData[0] = bassEnergy;
        
        if (bassEnergy > this.config.beatThreshold * 255 && timeSinceLastBeat > 200) {
            this.state.lastBeatTime = now;
            this.state.beatDetected = true;
            
            this.state.bpmHistory.push(60000 / timeSinceLastBeat);
            if (this.state.bpmHistory.length > this.config.bpmSmoothing) {
                this.state.bpmHistory.shift();
            }
            
            if (this.state.bpmHistory.length >= 3) {
                this.state.bpm = Math.round(this.state.bpmHistory.reduce((a, b) => a + b, 0) / this.state.bpmHistory.length);
                this.triggerEvent('bpmUpdate', { bpm: this.state.bpm });
            }
            
            this.triggerEvent('beatDetected', { 
                energy: bassEnergy / 255, 
                bpm: this.state.bpm,
                timestamp: now 
            });
            
            setTimeout(() => { this.state.beatDetected = false; }, 100);
            return true;
        }
        
        return false;
    }

    startVisualizer(canvas) {
        if (!this.config.enableVisualizer) return;
        
        this.canvas = canvas;
        this.ctx = canvas.getContext('2d');
        this.state.visualizerActive = true;
        
        const draw = () => {
            if (!this.state.visualizerActive) return;
            
            const width = this.canvas.width;
            const height = this.canvas.height;
            
            this.ctx.clearRect(0, 0, width, height);
            
            if (this.state.analyser) {
                this.state.analyser.getByteFrequencyData(this.state.frequencyData);
            }
            
            const mode = this.config.visualizerMode;
            
            if (mode === 'bars') {
                this.drawBars(width, height);
            } else if (mode === 'waveform') {
                this.drawWaveform(width, height);
            } else if (mode === 'circular') {
                this.drawCircular(width, height);
            } else if (mode === 'particles') {
                this.drawParticles(width, height);
            }
            
            this.triggerEvent('visualizerUpdate', { 
                frequencyData: this.state.frequencyData,
                mode 
            });
            
            this.animationFrame = requestAnimationFrame(draw);
        };
        
        draw();
    }

    drawBars(width, height) {
        const bufferLength = this.state.analyser.frequencyBinCount;
        const barWidth = width / bufferLength * 2.5;
        let x = 0;
        
        for (let i = 0; i < bufferLength; i++) {
            const barHeight = (this.state.frequencyData[i] / 255) * height;
            
            const gradient = this.ctx.createLinearGradient(0, height, 0, height - barHeight);
            gradient.addColorStop(0, '#27a0b9');
            gradient.addColorStop(0.5, '#00d4ff');
            gradient.addColorStop(1, '#00ff88');
            
            this.ctx.fillStyle = gradient;
            this.ctx.fillRect(x, height - barHeight, barWidth - 1, barHeight);
            
            x += barWidth;
        }
    }

    drawWaveform(width, height) {
        this.state.analyser.getByteTimeDomainData(this.state.waveformData);
        
        this.ctx.lineWidth = 3;
        this.ctx.strokeStyle = '#27a0b9';
        this.ctx.beginPath();
        
        const sliceWidth = width / this.state.waveformData.length;
        let x = 0;
        
        for (let i = 0; i < this.state.waveformData.length; i++) {
            const v = this.state.waveformData[i] / 128.0;
            const y = v * height / 2;
            
            if (i === 0) {
                this.ctx.moveTo(x, y);
            } else {
                this.ctx.lineTo(x, y);
            }
            
            x += sliceWidth;
        }
        
        this.ctx.lineTo(width, height / 2);
        this.ctx.stroke();
    }

    drawCircular(width, height) {
        const centerX = width / 2;
        const centerY = height / 2;
        const baseRadius = Math.min(width, height) / 4;
        
        this.ctx.beginPath();
        this.ctx.arc(centerX, centerY, baseRadius, 0, Math.PI * 2);
        this.ctx.strokeStyle = 'rgba(39, 160, 185, 0.3)';
        this.ctx.stroke();
        
        const bufferLength = this.state.analyser.frequencyBinCount;
        const step = Math.PI * 2 / bufferLength;
        
        for (let i = 0; i < bufferLength; i++) {
            const value = this.state.frequencyData[i] / 255;
            const radius = baseRadius + value * baseRadius * 0.8;
            const angle = i * step;
            
            const x1 = centerX + Math.cos(angle) * baseRadius;
            const y1 = centerY + Math.sin(angle) * baseRadius;
            const x2 = centerX + Math.cos(angle) * radius;
            const y2 = centerY + Math.sin(angle) * radius;
            
            const hue = (i / bufferLength) * 360;
            this.ctx.strokeStyle = `hsla(${hue}, 80%, 60%, ${0.5 + value * 0.5})`;
            this.ctx.lineWidth = 2;
            this.ctx.beginPath();
            this.ctx.moveTo(x1, y1);
            this.ctx.lineTo(x2, y2);
            this.ctx.stroke();
        }
    }

    drawParticles(width, height) {
        const particleCount = 50;
        const bassEnergy = this.state.frequencyData[0] / 255;
        
        for (let i = 0; i < particleCount; i++) {
            const angle = (i / particleCount) * Math.PI * 2;
            const baseRadius = Math.min(width, height) / 6;
            const variation = Math.sin(Date.now() / 500 + i) * 20;
            const radius = baseRadius + variation + bassEnergy * 100;
            
            const x = width / 2 + Math.cos(angle) * radius;
            const y = height / 2 + Math.sin(angle) * radius;
            const size = 2 + bassEnergy * 8;
            
            const gradient = this.ctx.createRadialGradient(x, y, 0, x, y, size);
            gradient.addColorStop(0, `rgba(39, 160, 185, ${0.8 - bassEnergy * 0.3})`);
            gradient.addColorStop(1, 'rgba(39, 160, 185, 0)');
            
            this.ctx.fillStyle = gradient;
            this.ctx.beginPath();
            this.ctx.arc(x, y, size, 0, Math.PI * 2);
            this.ctx.fill();
        }
    }

    stopVisualizer() {
        this.state.visualizerActive = false;
        if (this.animationFrame) {
            cancelAnimationFrame(this.animationFrame);
        }
    }

    play() {
        this.state.isPlaying = true;
        this.triggerEvent('play');
    }

    pause() {
        this.state.isPlaying = false;
        this.triggerEvent('pause');
    }

    stop() {
        this.state.isPlaying = false;
        this.state.currentTrack = null;
        this.triggerEvent('stop');
    }

    togglePlay() {
        if (this.state.isPlaying) {
            this.pause();
        } else {
            this.play();
        }
        return this.state.isPlaying;
    }

    setVolume(volume) {
        this.state.volume = Math.max(0, Math.min(1, volume));
        this.triggerEvent('volumeChange', { volume: this.state.volume });
    }

    toggleMute() {
        this.state.isMuted = !this.state.isMuted;
        return this.state.isMuted;
    }

    setShuffle(enabled) {
        this.state.shuffleMode = enabled;
        this.triggerEvent('shuffleChange', { enabled });
    }

    toggleShuffle() {
        this.state.shuffleMode = !this.state.shuffleMode;
        this.triggerEvent('shuffleChange', { enabled: this.state.shuffleMode });
        return this.state.shuffleMode;
    }

    setRepeat(mode) {
        const modes = ['off', 'all', 'one'];
        if (!modes.includes(mode)) return;
        this.state.repeatMode = mode;
        this.triggerEvent('repeatChange', { mode });
    }

    toggleRepeat() {
        const modes = ['off', 'all', 'one'];
        const currentIndex = modes.indexOf(this.state.repeatMode);
        this.state.repeatMode = modes[(currentIndex + 1) % modes.length];
        this.triggerEvent('repeatChange', { mode: this.state.repeatMode });
        return this.state.repeatMode;
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

    getFrequencyData() {
        return this.state.frequencyData;
    }

    getBPM() {
        return this.state.bpm;
    }

    isBeatDetected() {
        return this.state.beatDetected;
    }

    destroy() {
        this.stopVisualizer();
        if (this.state.audioContext) {
            this.state.audioContext.close();
        }
        this.eventCallbacks = {};
    }
}

export default MediaCenterEnhanced;
