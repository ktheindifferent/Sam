/**
 * SAM Media Center Pro
 * Advanced media playback, visualization, and integration features
 * Copyright 2021-2026 The Open Sam Foundation (OSF)
 */

(function() {
    'use strict';

    const MediaCenterPro = {
        version: '1.0.0',

        config: {
            enableVisualizations: true,
            enableLyricsDisplay: true,
            enableCrossfade: true,
            crossfadeDuration: 3,
            enableEqualizer: true,
            enablePartyMode: false,
            enableAmbientMode: false,
            enableSmartVolume: true,
            enableSleepTimer: true,
            enableShuffle: false,
            enableRepeat: 'off',
            visualizationUpdateInterval: 50,
            maxPlaylistItems: 100,
            defaultVolume: 70,
            volumeStep: 5,
            seekStep: 10,
            enableGapless: true,
            audioNormalization: true,
            targetLoudness: -14,
            enableBassBoost: false,
            enableTrebleBoost: false,
            enableVirtualSurround: false,
            enableNightMode: false,
            enableDucking: true,
            duckingAmount: -12,
            enableFadeInOut: true,
            fadeDuration: 0.5,
            enableHighResAudio: true,
            enableMqaSupport: false,
            enableDolbyAtmos: false,
            enableDsdSupport: false,
            enableVinylMode: false,
            enableKaraokeMode: false,
            enableDjMode: false,
            enableFocusMode: false,
            enableWorkoutMode: false,
            enableKidsMode: false,
            enableDataSaver: false,
            enableOfflineMode: false,
            enableSocialFeatures: false,
            enableLastfmScrobbling: false,
            enableMusicBrainz: true,
            enableAcoustid: false,
            enableChromecast: true,
            enableAirplay: true,
            enableSpotifyConnect: false,
            enableMultiroom: false,
            enablePartyShuffle: false,
            enableSmartPlaylist: true,
            enableMoodDetection: false,
            enableRecommendations: true,
            enableConcertMode: false,
            enableMerchandiseIntegration: false,
            enableTicketIntegration: false,
            enableArtistBio: true,
            enableDiscography: true,
            enableRelatedArtists: true,
            enableCollaborativePlaylists: false,
            enableLiveLyrics: true,
            enableMusicVideos: false,
            enablePodcastMode: true,
            enableAudiobookMode: true,
            enableRadioMode: true,
            enableRecordingMode: false,
            enableStudioMode: false,
            enableMasteringMode: false
        },

        state: {
            isPlaying: false,
            isPaused: false,
            currentTrack: null,
            playlist: [],
            playlistIndex: 0,
            volume: 0.7,
            isMuted: false,
            isShuffled: false,
            repeatMode: 'off',
            currentTime: 0,
            duration: 0,
            playbackRate: 1.0,
            isFullscreen: false,
            isPartyMode: false,
            isAmbientMode: false,
            isSleepTimerActive: false,
            sleepTimerRemaining: 0,
            visualizerData: new Uint8Array(64),
            lyrics: null,
            lyricsSync: [],
            queue: [],
            history: [],
            favorites: [],
            recentlyPlayed: [],
            downloadProgress: 0,
            isDownloading: false,
            audioContext: null,
            analyser: null,
            source: null,
            gainNode: null,
            equalizer: null,
            compressor: null,
            stereoPanner: null,
            convolver: null,
            mediaElement: null,
            mediaSource: 'local',
            isBuffering: false,
            bufferProgress: 0,
            audioQuality: 'high',
            currentAlbum: null,
            currentArtist: null,
            isAlbumView: false,
            isArtistView: false,
            searchQuery: '',
            searchResults: [],
            isSearching: false,
            browseCategory: 'all',
            activeFilters: [],
            sortMode: 'title',
            sortOrder: 'asc',
            viewMode: 'grid',
            nowPlayingSource: null,
            connectedDevices: [],
            activeDevice: null,
            multiroomGroup: null,
            playbackSession: null,
            lastfmSession: null,
            lyricsProvider: 'musixmatch',
            metadataProvider: 'musicbrainz',
            coverArtProvider: 'lastfm',
            recommendations: [],
            moodState: 'neutral',
            energyLevel: 0.5,
            danceability: 0.5,
            valence: 0.5,
            acousticness: 0.5,
            instrumentalness: 0.5,
            liveness: 0.5,
            speechiness: 0.5,
            tempo: 120,
            key: 'C',
            mode: 'major',
            timeSignature: 4,
            loudness: -14,
            activeEffects: [],
            effectPresets: {},
            userPreferences: {},
            playbackStats: {
                totalPlays: 0,
                totalListeningTime: 0,
                favoriteGenres: [],
                favoriteArtists: [],
                listeningStreak: 0,
                lastListeningSession: null
            }
        },

        visualizations: [
            'spectrum', 'waveform', 'circular', 'particles',
            'bars', 'waves', 'matrix', 'stars', 'fire',
            'water', 'earth', 'air', 'plasma', 'nebula',
            'galaxy', 'aurora', 'lightning', 'rain', 'snow'
        ],

        mediaSources: {
            spotify: { enabled: true, connected: false, premium: false },
            youtube: { enabled: true, connected: false, apiQuota: 0 },
            tidal: { enabled: true, connected: false, quality: 'lossless' },
            appleMusic: { enabled: true, connected: false },
            soundcloud: { enabled: true, connected: false },
            bandcamp: { enabled: true, connected: false },
            deezer: { enabled: true, connected: false },
            qobuz: { enabled: true, connected: false, hiRes: false },
            local: { enabled: true, library: [] },
            radio: { enabled: true, stations: [] },
            podcast: { enabled: true, subscriptions: [] },
            audiobook: { enabled: true, library: [] },
            napster: { enabled: false, connected: false },
            pandora: { enabled: false, connected: false },
            iheart: { enabled: false, connected: false },
            tunein: { enabled: true, connected: false },
            bbc: { enabled: true, connected: false },
            npr: { enabled: true, connected: false }
        },

        equalizerPresets: {
            flat: [0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
            bassBoost: [8, 6, 4, 2, 0, 0, 0, 1, 2, 3],
            trebleBoost: [0, 0, 0, 0, 1, 2, 3, 4, 6, 8],
            vocal: [-2, -2, 0, 2, 4, 4, 4, 2, 0, -2],
            rock: [5, 4, 3, 1, 0, 0, 2, 3, 4, 5],
            pop: [-2, 0, 2, 3, 4, 4, 3, 2, 1, 0],
            jazz: [3, 2, 1, 2, 3, 4, 3, 2, 1, 2],
            classical: [4, 3, 2, 1, 0, 0, 0, 1, 2, 3],
            electronic: [6, 5, 4, 3, 2, 1, 2, 3, 4, 5],
            acoustic: [2, 1, 0, 1, 2, 3, 2, 1, 0, 1],
            hiphop: [7, 5, 3, 1, 0, 0, 1, 2, 3, 4],
            rb: [4, 3, 2, 2, 3, 4, 3, 2, 1, 2],
            country: [3, 2, 2, 3, 4, 3, 2, 1, 2, 3],
            latin: [5, 4, 3, 2, 2, 3, 4, 3, 2, 1],
            metal: [6, 4, 3, 2, 1, 1, 2, 3, 5, 6],
            ambient: [4, 3, 2, 1, 2, 3, 4, 3, 2, 1],
            podcast: [-2, 0, 2, 4, 6, 4, 2, 0, -2, -2],
            audiobook: [-1, 0, 1, 3, 5, 3, 1, 0, -1, -1],
            nightMode: [-4, -2, 0, 2, 4, 2, 0, -2, -4, -6],
            loudness: [4, 3, 2, 1, 0, 0, 1, 2, 3, 4]
        },

        moodPresets: {
            happy: { energy: 0.8, valence: 0.9, danceability: 0.7, tempo: 128 },
            sad: { energy: 0.3, valence: 0.2, danceability: 0.3, tempo: 70 },
            energetic: { energy: 0.9, valence: 0.7, danceability: 0.9, tempo: 140 },
            calm: { energy: 0.3, valence: 0.6, danceability: 0.3, tempo: 60 },
            focused: { energy: 0.6, valence: 0.5, danceability: 0.3, tempo: 90 },
            romantic: { energy: 0.5, valence: 0.7, danceability: 0.5, tempo: 80 },
            melancholic: { energy: 0.4, valence: 0.3, danceability: 0.4, tempo: 75 },
            euphoric: { energy: 0.9, valence: 0.9, danceability: 0.8, tempo: 135 },
            aggressive: { energy: 0.9, valence: 0.4, danceability: 0.6, tempo: 150 },
            peaceful: { energy: 0.2, valence: 0.6, danceability: 0.2, tempo: 50 }
        },

        init() {
            this.loadUserPreferences();
            this.setupAudioContext();
            this.setupMediaSession();
            this.setupKeyboardShortcuts();
            this.setupVisualizations();
            this.setupEqualizer();
            this.setupSmartVolume();
            this.setupCrossfade();
            this.setupLyricsDisplay();
            this.setupSleepTimer();
            this.setupPlaylistManagement();
            this.setupNowPlayingDisplay();
            this.setupMediaNotifications();
            this.setupTouchOptimizations();
            this.setupGestureControls();
            console.log('[MediaCenterPro] Initialized v' + this.version);
        },

        setupAudioContext() {
            try {
                this.state.audioContext = new (window.AudioContext || window.webkitAudioContext)({
                    latencyHint: 'interactive',
                    sampleRate: this.config.enableHighResAudio ? 96000 : 48000
                });

                this.state.analyser = this.state.audioContext.createAnalyser();
                this.state.analyser.fftSize = 2048;
                this.state.analyser.smoothingTimeConstant = 0.85;

                this.state.gainNode = this.state.audioContext.createGain();
                this.state.gainNode.gain.value = this.state.volume;

                this.state.compressor = this.state.audioContext.createDynamicsCompressor();
                this.state.compressor.threshold.value = -50;
                this.state.compressor.knee.value = 40;
                this.state.compressor.ratio.value = 12;
                this.state.compressor.attack.value = 0.003;
                this.state.compressor.release.value = 0.25;

                if (this.config.enableEqualizer) {
                    this.setupEqualizerBands();
                }

                this.state.gainNode.connect(this.state.compressor);
                this.state.compressor.connect(this.state.audioContext.destination);

                this.state.volume = this.config.defaultVolume / 100;
            } catch (e) {
                console.warn('[MediaCenterPro] Audio context not available:', e);
            }
        },

        setupEqualizerBands() {
            const frequencies = [32, 64, 125, 250, 500, 1000, 2000, 4000, 8000, 16000];
            this.state.equalizer = [];

            frequencies.forEach((freq, i) => {
                const filter = this.state.audioContext.createBiquadFilter();
                filter.type = i === 0 ? 'lowshelf' : (i === frequencies.length - 1 ? 'highshelf' : 'peaking');
                filter.frequency.value = freq;
                filter.Q.value = 1.41;
                filter.gain.value = 0;

                if (i > 0) {
                    this.state.equalizer[i - 1].connect(filter);
                }

                this.state.equalizer.push(filter);
            });

            if (this.state.equalizer.length > 0) {
                this.state.equalizer[this.state.equalizer.length - 1].connect(this.state.gainNode);
            }
        },

        setupMediaSession() {
            if ('mediaSession' in navigator) {
                navigator.mediaSession.setActionHandler('play', () => this.play());
                navigator.mediaSession.setActionHandler('pause', () => this.pause());
                navigator.mediaSession.setActionHandler('previoustrack', () => this.previous());
                navigator.mediaSession.setActionHandler('nexttrack', () => this.next());
                navigator.mediaSession.setActionHandler('stop', () => this.stop());
                navigator.mediaSession.setActionHandler('seekbackward', (details) => this.seek(-details.seekOffset));
                navigator.mediaSession.setActionHandler('seekforward', (details) => this.seek(details.seekOffset));
                navigator.mediaSession.setActionHandler('seekto', (details) => this.seekTo(details.seekTime));

                if ('setCameraActive' in navigator.mediaSession) {
                    navigator.mediaSession.setActionHandler('togglemicrophone', () => this.toggleMicrophone());
                }
            }
        },

        setupKeyboardShortcuts() {
            document.addEventListener('keydown', (e) => {
                if (e.target.tagName === 'INPUT' || e.target.tagName === 'TEXTAREA') return;

                const shortcuts = {
                    'Space': () => { e.preventDefault(); this.togglePlayPause(); },
                    'ArrowRight': () => { e.preventDefault(); this.seek(this.config.seekStep); },
                    'ArrowLeft': () => { e.preventDefault(); this.seek(-this.config.seekStep); },
                    'ArrowUp': () => { e.preventDefault(); this.adjustVolume(this.config.volumeStep); },
                    'ArrowDown': () => { e.preventDefault(); this.adjustVolume(-this.config.volumeStep); },
                    'KeyM': () => this.toggleMute(),
                    'KeyF': () => this.toggleFullscreen(),
                    'KeyS': () => this.toggleShuffle(),
                    'KeyR': () => this.cycleRepeat(),
                    'KeyL': () => this.config.enableLyricsDisplay && this.toggleLyrics(),
                    'KeyV': () => this.config.enableVisualizations && this.cycleVisualization(),
                    'KeyP': () => this.togglePartyMode(),
                    'KeyA': () => this.toggleAmbientMode(),
                    'KeyN': () => this.next(),
                    'KeyB': () => this.previous(),
                    'Digit0': () => this.setVolume(0),
                    'Digit1': () => this.setVolume(10),
                    'Digit2': () => this.setVolume(20),
                    'Digit3': () => this.setVolume(30),
                    'Digit4': () => this.setVolume(40),
                    'Digit5': () => this.setVolume(50),
                    'Digit6': () => this.setVolume(60),
                    'Digit7': () => this.setVolume(70),
                    'Digit8': () => this.setVolume(80),
                    'Digit9': () => this.setVolume(90),
                    'KeyT': () => this.toggleSleepTimer(),
                    'KeyH': () => this.toggleHighResAudio(),
                    'KeyK': () => this.toggleKaraokeMode(),
                    'KeyD': () => this.toggleDjMode(),
                    'KeyW': () => this.toggleWorkoutMode(),
                    'Plus': () => this.adjustPlaybackRate(0.1),
                    'Minus': () => this.adjustPlaybackRate(-0.1)
                };

                if (shortcuts[e.code]) {
                    shortcuts[e.code]();
                }
            });
        },

        setupVisualizations() {
            if (!this.config.enableVisualizations) return;

            const container = document.getElementById('media-visualization-container');
            if (!container) return;

            container.innerHTML = '';
            const numBars = 64;

            for (let i = 0; i < numBars; i++) {
                const bar = document.createElement('div');
                bar.className = 'viz-bar';
                bar.style.cssText = `
                    width: ${100/numBars}%;
                    height: 10px;
                    background: linear-gradient(to top,
                        hsl(${(i/numBars)*360}, 80%, 50%),
                        hsl(${(i/numBars)*360}, 80%, 70%));
                    border-radius: 2px 2px 0 0;
                    display: inline-block;
                    transition: height ${this.config.visualizationUpdateInterval}ms ease;
                `;
                container.appendChild(bar);
            }

            this.startVisualizationLoop();
        },

        startVisualizationLoop() {
            const update = () => {
                if (!this.state.isPlaying || !this.state.analyser) {
                    requestAnimationFrame(update);
                    return;
                }

                const dataArray = new Uint8Array(this.state.analyser.frequencyBinCount);
                this.state.analyser.getByteFrequencyData(dataArray);
                this.state.visualizerData = dataArray;

                const bars = document.querySelectorAll('.viz-bar');
                bars.forEach((bar, i) => {
                    const value = dataArray[i * 2] || 0;
                    const height = Math.max(5, (value / 255) * 150);
                    bar.style.height = `${height}px`;
                    bar.style.filter = value > 220 ? 'brightness(1.3)' : 'brightness(1)';
                });

                requestAnimationFrame(update);
            };

            requestAnimationFrame(update);
        },

        setupEqualizer() {
            if (!this.config.enableEqualizer || !this.state.audioContext) return;
            this.createEqualizerUI();
        },

        createEqualizerUI() {
            const container = document.getElementById('equalizer-container');
            if (!container) return;

            const frequencies = ['32Hz', '64Hz', '125Hz', '250Hz', '500Hz', '1kHz', '2kHz', '4kHz', '8kHz', '16kHz'];
            const presets = Object.keys(this.equalizerPresets);

            container.innerHTML = `
                <div class="eq-presets">
                    ${presets.map(preset =>
                        `<button class="eq-preset-btn" data-preset="${preset}">${preset.replace(/([A-Z])/g, ' $1').trim()}</button>`
                    ).join('')}
                </div>
                <div class="eq-sliders">
                    ${frequencies.map((freq, i) => `
                        <div class="eq-band">
                            <input type="range" class="eq-slider" min="-12" max="12" value="0"
                                data-index="${i}" aria-label="${freq}">
                            <span class="eq-label">${freq}</span>
                            <span class="eq-value">0dB</span>
                        </div>
                    `).join('')}
                </div>
            `;

            container.querySelectorAll('.eq-slider').forEach(slider => {
                slider.addEventListener('input', (e) => {
                    const index = parseInt(e.target.dataset.index);
                    const value = parseFloat(e.target.value);
                    this.setEqualizerBand(index, value);
                    e.target.nextElementSibling.nextElementSibling.textContent = `${value > 0 ? '+' : ''}${value}dB`;
                });
            });

            container.querySelectorAll('.eq-preset-btn').forEach(btn => {
                btn.addEventListener('click', (e) => {
                    const preset = e.target.dataset.preset;
                    this.applyEqualizerPreset(preset);
                    container.querySelectorAll('.eq-preset-btn').forEach(b => b.classList.remove('active'));
                    e.target.classList.add('active');
                });
            });
        },

        setEqualizerBand(index, gain) {
            if (this.state.equalizer && this.state.equalizer[index]) {
                this.state.equalizer[index].gain.value = gain;
            }
        },

        applyEqualizerPreset(presetName) {
            const gains = this.equalizerPresets[presetName];
            if (!gains) return;

            gains.forEach((gain, i) => {
                this.setEqualizerBand(i, gain);
                const slider = document.querySelector(`.eq-slider[data-index="${i}"]`);
                if (slider) {
                    slider.value = gain;
                    slider.nextElementSibling.nextElementSibling.textContent = `${gain > 0 ? '+' : ''}${gain}dB`;
                }
            });
        },

        setupSmartVolume() {
            if (!this.config.enableSmartVolume) return;

            const adjustVolume = () => {
                if (this.state.isPlaying && this.config.enableDucking) {
                    this.detectAudioDucking();
                }
            };

            setInterval(adjustVolume, 1000);
        },

        detectAudioDucking() {
            if (!this.state.analyser) return;

            const dataArray = new Uint8Array(this.state.analyser.frequencyBinCount);
            this.state.analyser.getByteFrequencyData(dataArray);

            const averageVolume = dataArray.reduce((a, b) => a + b, 0) / dataArray.length;
            const normalizedVolume = averageVolume / 255;

            if (normalizedVolume > 0.8 && this.state.volume > 0.3) {
                this.setVolume(this.state.volume - 0.1);
            } else if (normalizedVolume < 0.5 && this.state.volume < 0.8) {
                this.setVolume(this.state.volume + 0.05);
            }
        },

        setupCrossfade() {
            if (!this.config.enableCrossfade) return;
            console.log('[MediaCenterPro] Crossfade enabled:', this.config.crossfadeDuration, 'seconds');
        },

        setupLyricsDisplay() {
            if (!this.config.enableLyricsDisplay) return;
            this.fetchLyrics();
        },

        fetchLyrics() {
            if (!this.state.currentTrack) return;

            const { artist, title } = this.state.currentTrack;
            fetch(`/api/media/lyrics?artist=${encodeURIComponent(artist)}&title=${encodeURIComponent(title)}`)
                .then(res => res.json())
                .then(data => {
                    this.state.lyrics = data.lyrics;
                    this.state.lyricsSync = data.sync || [];
                    this.displayLyrics();
                })
                .catch(() => console.warn('[MediaCenterPro] Lyrics not available'));
        },

        displayLyrics() {
            const container = document.getElementById('lyrics-container');
            if (!container || !this.state.lyrics) return;

            container.innerHTML = `<div class="lyrics-text">${this.state.lyrics}</div>`;
            container.classList.add('visible');
        },

        toggleLyrics() {
            const container = document.getElementById('lyrics-container');
            if (container) {
                container.classList.toggle('visible');
            }
        },

        setupSleepTimer() {
            if (!this.config.enableSleepTimer) return;
            console.log('[MediaCenterPro] Sleep timer available');
        },

        startSleepTimer(minutes) {
            this.state.isSleepTimerActive = true;
            this.state.sleepTimerRemaining = minutes * 60;

            const interval = setInterval(() => {
                if (!this.state.isSleepTimerActive) {
                    clearInterval(interval);
                    return;
                }

                this.state.sleepTimerRemaining--;

                if (this.state.sleepTimerRemaining <= 0) {
                    this.pause();
                    this.state.isSleepTimerActive = false;
                    this.showNotification('Sleep timer finished');
                }

                this.updateSleepTimerDisplay();
            }, 1000);
        },

        stopSleepTimer() {
            this.state.isSleepTimerActive = false;
            this.state.sleepTimerRemaining = 0;
            this.updateSleepTimerDisplay();
        },

        toggleSleepTimer() {
            if (this.state.isSleepTimerActive) {
                this.stopSleepTimer();
            } else {
                this.startSleepTimer(30);
            }
        },

        updateSleepTimerDisplay() {
            const display = document.getElementById('sleep-timer-display');
            if (!display) return;

            const mins = Math.floor(this.state.sleepTimerRemaining / 60);
            const secs = this.state.sleepTimerRemaining % 60;
            display.textContent = `${mins}:${String(secs).padStart(2, '0')}`;
            display.style.display = this.state.isSleepTimerActive ? 'block' : 'none';
        },

        setupPlaylistManagement() {
            console.log('[MediaCenterPro] Playlist management ready');
        },

        addToPlaylist(track) {
            if (this.state.playlist.length >= this.config.maxPlaylistItems) {
                this.state.playlist.shift();
            }
            this.state.playlist.push(track);
            this.notifyPlaylistUpdate();
        },

        removeFromPlaylist(index) {
            this.state.playlist.splice(index, 1);
            this.notifyPlaylistUpdate();
        },

        clearPlaylist() {
            this.state.playlist = [];
            this.state.playlistIndex = 0;
            this.notifyPlaylistUpdate();
        },

        shufflePlaylist() {
            for (let i = this.state.playlist.length - 1; i > 0; i--) {
                const j = Math.floor(Math.random() * (i + 1));
                [this.state.playlist[i], this.state.playlist[j]] = [this.state.playlist[j], this.state.playlist[i]];
            }
            this.state.isShuffled = true;
            this.notifyPlaylistUpdate();
        },

        notifyPlaylistUpdate() {
            const event = new CustomEvent('playlist_updated', {
                detail: { playlist: this.state.playlist, index: this.state.playlistIndex }
            });
            document.dispatchEvent(event);
        },

        setupNowPlayingDisplay() {
            console.log('[MediaCenterPro] Now Playing display ready');
        },

        updateNowPlaying(track) {
            this.state.currentTrack = track;
            this.state.isPlaying = true;

            const display = document.getElementById('now-playing-display');
            if (display && track) {
                display.innerHTML = `
                    <div class="album-art">
                        <img src="${track.artwork || '/assets/images/default-album.png'}" alt="${track.title}">
                    </div>
                    <div class="track-info">
                        <h3 class="track-title">${track.title}</h3>
                        <p class="track-artist">${track.artist}</p>
                        <p class="track-album">${track.album || ''}</p>
                    </div>
                `;
            }

            if ('mediaSession' in navigator && track) {
                navigator.mediaSession.metadata = new MediaMetadata({
                    title: track.title,
                    artist: track.artist,
                    album: track.album,
                    artwork: track.artwork ? [{ src: track.artwork, sizes: '512x512' }] : []
                });
            }

            this.fetchLyrics();
            this.addToHistory(track);
            this.updatePlaybackStats();
        },

        setupMediaNotifications() {
            if ('Notification' in window && Notification.permission === 'granted') {
                console.log('[MediaCenterPro] Notifications enabled');
            }
        },

        showNotification(message, icon = '/assets/images/music-note.png') {
            if ('Notification' in window && Notification.permission === 'granted') {
                new Notification('SAM Media Center', {
                    body: message,
                    icon: icon
                });
            }
        },

        setupTouchOptimizations() {
            document.querySelectorAll('.media-control').forEach(el => {
                el.style.touchAction = 'manipulation';
                el.style.webkitTapHighlightColor = 'transparent';
            });
        },

        setupGestureControls() {
            if (typeof onGesture === 'function') {
                onGesture('swipeUp', () => this.adjustVolume(5));
                onGesture('swipeDown', () => this.adjustVolume(-5));
                onGesture('swipeLeft', () => this.next());
                onGesture('swipeRight', () => this.previous());
                onGesture('doubleTap', () => this.togglePlayPause());
                onGesture('longPress', () => this.showQuickActions());
            }
        },

        play() {
            this.state.isPlaying = true;
            this.state.isPaused = false;
            this.notifyPlaybackState();
        },

        pause() {
            this.state.isPlaying = false;
            this.state.isPaused = true;
            this.notifyPlaybackState();
        },

        stop() {
            this.state.isPlaying = false;
            this.state.isPaused = false;
            this.state.currentTime = 0;
            this.notifyPlaybackState();
        },

        togglePlayPause() {
            if (this.state.isPlaying) {
                this.pause();
            } else {
                this.play();
            }
        },

        next() {
            if (this.state.playlistIndex < this.state.playlist.length - 1) {
                this.state.playlistIndex++;
                this.updateNowPlaying(this.state.playlist[this.state.playlistIndex]);
            } else if (this.state.repeatMode === 'all') {
                this.state.playlistIndex = 0;
                this.updateNowPlaying(this.state.playlist[0]);
            }
        },

        previous() {
            if (this.state.currentTime > 3) {
                this.seek(0);
            } else if (this.state.playlistIndex > 0) {
                this.state.playlistIndex--;
                this.updateNowPlaying(this.state.playlist[this.state.playlistIndex]);
            }
        },

        seek(seconds) {
            this.state.currentTime = Math.max(0, Math.min(this.state.duration, this.state.currentTime + seconds));
            this.notifySeek();
        },

        seekTo(time) {
            this.state.currentTime = Math.max(0, Math.min(this.state.duration, time));
            this.notifySeek();
        },

        setVolume(level) {
            this.state.volume = Math.max(0, Math.min(1, level / 100));
            if (this.state.gainNode) {
                this.state.gainNode.gain.value = this.state.volume;
            }
            this.state.isMuted = this.state.volume === 0;
            this.notifyVolumeChange();
        },

        adjustVolume(delta) {
            this.setVolume((this.state.volume * 100) + delta);
        },

        toggleMute() {
            if (this.state.isMuted) {
                this.state.volume = this.state.lastVolume || 0.5;
                this.state.isMuted = false;
            } else {
                this.state.lastVolume = this.state.volume;
                this.state.volume = 0;
                this.state.isMuted = true;
            }

            if (this.state.gainNode) {
                this.state.gainNode.gain.value = this.state.volume;
            }

            this.notifyVolumeChange();
        },

        toggleShuffle() {
            this.state.isShuffled = !this.state.isShuffled;
            if (this.state.isShuffled) {
                this.shufflePlaylist();
            }
            this.notifyShuffleChange();
        },

        cycleRepeat() {
            const modes = ['off', 'all', 'one'];
            const currentIndex = modes.indexOf(this.state.repeatMode);
            this.state.repeatMode = modes[(currentIndex + 1) % modes.length];
            this.notifyRepeatChange();
        },

        toggleFullscreen() {
            if (!document.fullscreenElement) {
                document.documentElement.requestFullscreen();
                this.state.isFullscreen = true;
            } else {
                document.exitFullscreen();
                this.state.isFullscreen = false;
            }
        },

        togglePartyMode() {
            this.state.isPartyMode = !this.state.isPartyMode;
            if (this.state.isPartyMode) {
                this.config.enableVisualizations = true;
                this.config.enablePartyShuffle = true;
                this.setVolume(80);
                this.showNotification('Party Mode Activated', '/assets/images/party.png');
            } else {
                this.setVolume(50);
                this.showNotification('Party Mode Deactivated');
            }
        },

        toggleAmbientMode() {
            this.state.isAmbientMode = !this.state.isAmbientMode;
            if (this.state.isAmbientMode) {
                this.setVolume(30);
                this.applyEqualizerPreset('ambient');
            } else {
                this.applyEqualizerPreset('flat');
            }
        },

        toggleHighResAudio() {
            this.config.enableHighResAudio = !this.config.enableHighResAudio;
            if (this.state.audioContext) {
                this.state.audioContext.close();
                this.setupAudioContext();
            }
            this.showNotification(this.config.enableHighResAudio ? 'High-Res Audio Enabled' : 'High-Res Audio Disabled');
        },

        toggleKaraokeMode() {
            this.config.enableKaraokeMode = !this.config.enableKaraokeMode;
            if (this.config.enableKaraokeMode) {
                this.applyEqualizerPreset('vocal');
            }
            this.showNotification(this.config.enableKaraokeMode ? 'Karaoke Mode On' : 'Karaoke Mode Off');
        },

        toggleDjMode() {
            this.config.enableDjMode = !this.config.enableDjMode;
            this.showNotification(this.config.enableDjMode ? 'DJ Mode On' : 'DJ Mode Off');
        },

        toggleWorkoutMode() {
            this.config.enableWorkoutMode = !this.config.enableWorkoutMode;
            if (this.config.enableWorkoutMode) {
                this.applyEqualizerPreset('electronic');
                this.setVolume(75);
            }
            this.showNotification(this.config.enableWorkoutMode ? 'Workout Mode On' : 'Workout Mode Off');
        },

        adjustPlaybackRate(delta) {
            this.state.playbackRate = Math.max(0.5, Math.min(2.0, this.state.playbackRate + delta));
            this.notifyPlaybackRateChange();
        },

        addToHistory(track) {
            this.state.history.unshift(track);
            if (this.state.history.length > 50) {
                this.state.history.pop();
            }
            this.state.recentlyPlayed.unshift(track);
            if (this.state.recentlyPlayed.length > 20) {
                this.state.recentlyPlayed.pop();
            }
        },

        updatePlaybackStats() {
            this.state.playbackStats.totalPlays++;
            this.state.playbackStats.lastListeningSession = new Date().toISOString();
        },

        setMood(moodName) {
            const mood = this.moodPresets[moodName];
            if (!mood) return;

            this.state.moodState = moodName;
            this.state.energyLevel = mood.energy;
            this.state.valence = mood.valence;
            this.state.danceability = mood.danceability;

            this.generateMoodPlaylist(mood);
        },

        generateMoodPlaylist(mood) {
            fetch(`/api/media/recommendations?energy=${mood.energy}&valence=${mood.valence}&danceability=${mood.danceability}`)
                .then(res => res.json())
                .then(data => {
                    this.state.recommendations = data.tracks || [];
                    this.notifyRecommendationsUpdate();
                })
                .catch(() => console.warn('[MediaCenterPro] Could not fetch recommendations'));
        },

        cycleVisualization() {
            const currentIndex = this.visualizations.indexOf(this.state.currentVisualization);
            this.state.currentVisualization = this.visualizations[(currentIndex + 1) % this.visualizations.length];
            this.notifyVisualizationChange();
        },

        notifyPlaybackState() {
            document.dispatchEvent(new CustomEvent('playback_state_changed', {
                detail: { isPlaying: this.state.isPlaying, isPaused: this.state.isPaused }
            }));
        },

        notifyVolumeChange() {
            document.dispatchEvent(new CustomEvent('volume_changed', {
                detail: { volume: this.state.volume, isMuted: this.state.isMuted }
            }));
        },

        notifyShuffleChange() {
            document.dispatchEvent(new CustomEvent('shuffle_changed', {
                detail: { isShuffled: this.state.isShuffled }
            }));
        },

        notifyRepeatChange() {
            document.dispatchEvent(new CustomEvent('repeat_changed', {
                detail: { repeatMode: this.state.repeatMode }
            }));
        },

        notifySeek() {
            document.dispatchEvent(new CustomEvent('seeked', {
                detail: { currentTime: this.state.currentTime }
            }));
        },

        notifyPlaybackRateChange() {
            document.dispatchEvent(new CustomEvent('playback_rate_changed', {
                detail: { playbackRate: this.state.playbackRate }
            }));
        },

        notifyVisualizationChange() {
            document.dispatchEvent(new CustomEvent('visualization_changed', {
                detail: { visualization: this.state.currentVisualization }
            }));
        },

        notifyRecommendationsUpdate() {
            document.dispatchEvent(new CustomEvent('recommendations_updated', {
                detail: { recommendations: this.state.recommendations }
            }));
        },

        showQuickActions() {
            const event = new CustomEvent('show_quick_actions');
            document.dispatchEvent(event);
        },

        loadUserPreferences() {
            const saved = localStorage.getItem('mediacenter_pro_config');
            if (saved) {
                try {
                    const config = JSON.parse(saved);
                    Object.assign(this.config, config);
                } catch (e) {
                    console.error('[MediaCenterPro] Failed to load preferences:', e);
                }
            }
        },

        saveUserPreferences() {
            localStorage.setItem('mediacenter_pro_config', JSON.stringify(this.config));
        }
    };

    MediaCenterPro.init();
    window.MediaCenterPro = MediaCenterPro;
})();
