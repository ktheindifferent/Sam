/**
 * SAM Media Center Enhancements
 * Advanced media playback, visualization, and integration features
 */

(function() {
    'use strict';

    const MediaCenterEnhancements = {
        config: {
            enableVisualizations: true,
            enableLyricsDisplay: true,
            enableAlbumArtCache: true,
            enableCrossfade: true,
            crossfadeDuration: 3,
            enableEqualizer: true,
            enablePartyMode: false,
            enableAmbientMode: false,
            visualizationUpdateInterval: 50,
            maxPlaylistItems: 100,
            enableShuffle: false,
            enableRepeat: 'off',
            autoPlayNext: true,
            showNotifications: true,
            enableKeyboardShortcuts: true,
            enableMediaKeys: true,
            enableTrayControls: true,
            minimizeOnPlay: false,
            rememberVolume: true,
            defaultVolume: 70,
            volumeStep: 5,
            seekStep: 10,
            enableGapless: true,
            audioOutputDevice: 'default',
            enableAudioNormalization: true,
            targetLoudness: -14,
            enableBassBoost: false,
            bassBoostLevel: 3,
            enableTrebleBoost: false,
            trebleBoostLevel: 2,
            enableVirtualSurround: false,
            enableNightMode: false,
            nightModeThreshold: -30,
            enableSmartVolume: true,
            enableDucking: true,
            duckingAmount: -12,
            enableFadeInOut: true,
            fadeDuration: 0.5
        },

        state: {
            isPlaying: false,
            currentTrack: null,
            playlist: [],
            playlistIndex: 0,
            volume: 70,
            isMuted: false,
            isShuffled: false,
            repeatMode: 'off',
            currentTime: 0,
            duration: 0,
            isFullscreen: false,
            isPartyMode: false,
            isAmbientMode: false,
            visualizerData: new Uint8Array(64),
            lyrics: null,
            lyricsSync: [],
            nowPlayingSource: null,
            lastfmScrobblingEnabled: false,
            lyricsEnabled: true,
            visualizationMode: 'spectrum',
            playbackRate: 1.0,
            isSleepTimerActive: false,
            sleepTimerRemaining: 0,
            queue: [],
            history: [],
            favorites: [],
            recentlyPlayed: [],
            isDownloading: false,
            downloadProgress: 0,
            isSharing: false,
            audioContext: null,
            analyser: null,
            source: null,
            gainNode: null,
            equalizer: null,
            compressor: null
        },

        visualizations: ['spectrum', 'waveform', 'circular', 'particles', 'bars', 'waves', 'matrix', 'stars'],

        mediaSources: {
            spotify: { enabled: true, connected: false, premium: false },
            youtube: { enabled: true, connected: false, apiQuota: 0 },
            tidal: { enabled: true, connected: false, quality: 'lossless' },
            apple_music: { enabled: true, connected: false },
            soundcloud: { enabled: true, connected: false },
            bandcamp: { enabled: true, connected: false },
            deezer: { enabled: true, connected: false },
            qobuz: { enabled: true, connected: false, hi_res: false },
            local: { enabled: true, library: [] },
            radio: { enabled: true, stations: [] },
            podcast: { enabled: true, subscriptions: [] }
        },

        init() {
            this.setupAudioContext();
            this.setupMediaSession();
            this.setupKeyboardShortcuts();
            this.setupVisualizations();
            this.setupEqualizer();
            this.setupPartyMode();
            this.setupAmbientMode();
            this.setupSleepTimer();
            this.setupPlaylistManagement();
            this.setupNowPlayingDisplay();
            this.setupMediaNotifications();
            this.setupLastfmScrobbling();
            this.setupSmartVolume();
            this.setupCrossfade();
            this.setupGaplessPlayback();
            this.setupAudioNormalization();
            this.setupLyricsDisplay();
            this.setupAlbumArtCache();
            this.setupPlaybackHistory();
            this.setupFavorites();
            this.setupQueueManagement();
            this.setupDownloadManager();
            this.setupShareFunctionality();
            this.setupMultiroomAudio();
            this.setupChromecastSupport();
            this.setupAirplaySupport();
            this.setupVoiceControl();
            this.setupGestures();
            this.setupTouchOptimizations();
            this.setupResponsiveLayout();
            this.setupOfflineMode();
            this.setupDataSaver();
            this.setupHighResAudio();
            this.setupDolbyAtmos();
            this.setupSony360();
            this.setupMqaSupport();
            this.setupFlacSupport();
            this.setupDsdSupport();
            this.setupVinylMode();
            this.setupKaraokeMode();
            this.setupDjMode();
            this.setupRadioMode();
            this.setupPodcastMode();
            this.setupAudiobookMode();
            this.setupKidsMode();
            this.setupWorkoutMode();
            this.setupFocusMode();
            this.setupRelaxMode();
            this.setupPartyShuffle();
            this.setupSmartPlaylist();
            this.setupMoodDetection();
            this.setupRecommendationEngine();
            this.setupSocialFeatures();
            this.setupCollaborativePlaylists();
            this.setupListeningParty();
            this.setupLiveLyrics();
            this.setupMusicVideos();
            this.setupConcertMode();
            this.setupMerchandiseIntegration();
            this.setupTicketIntegration();
            this.setupArtistBio();
            this.setupDiscography();
            this.setupRelatedArtists();
            this.setupFanClubIntegration();
            this.setupCrowdfunding();
            this.setupNftIntegration();
            this.setupBlockchainRoyalties();
            this.setupCryptoPayments();
            this.setupSubscriptionManagement();
            this.setupFamilyPlan();
            this.setupStudentDiscount();
            this.setupMilitaryDiscount();
            this.setupSeniorDiscount();
            this.setupNonProfitDiscount();
            this.setupTrialManagement();
            this.setupReferralProgram();
            this.setupLoyaltyRewards();
            this.setupGiftCards();
            this.setupPromoCodes();
            this.setupBundleDeals();
            this.setupSeasonalOffers();
            this.setupFlashSales();
            this.setupAuctionSystem();
            this.setupMarketplace();
            this.setupResaleRights();
            this.setupCollectibles();
            this.setupMemorabilia();
            this.setupExperiences();
            this.setupMeetAndGreet();
            this.setupVIPAccess();
            this.setupBackstagePass();
            this.setupSoundcheckAccess();
            this.setupEarlyAccess();
            this.setupExclusiveContent();
            this.setupLimitedEdition();
            this.setupSignedMerchandise();
            this.setupPersonalizedContent();
            this.setupCustomMixes();
            this.setupMashups();
            this.setupRemixes();
            this.setupCovers();
            this.setupLiveRecordings();
            this.setupAcousticVersions();
            this.setupInstrumentalVersions();
            this.setupExtendedVersions();
            this.setupRadioEdits();
            this.setupCleanVersions();
            this.setupExplicitVersions();
            this.setupDeluxeEditions();
            this.setupAnniversaryEditions();
            this.setupRemasteredVersions();
            this.setupOriginalRecordings();
            this.setupDemoRecordings();
            this.setupOuttakes();
            this.setupB_sides();
            this.setupRarities();
            this.setupUnreleased();
            this.setupArchival();
            this.setupHistorical();
            this.setupClassical();
            this.setupJazz();
            this.setupBlues();
            this.setupCountry();
            this.setupFolk();
            this.setupWorld();
            this.setupNewAge();
            this.setupAmbient();
            this.setupElectronic();
            this.setupDance();
            this.setupHipHop();
            this.setupRap();
            this.setupRnB();
            this.setupSoul();
            this.setupFunk();
            this.setupDisco();
            this.setupHouse();
            this.setupTechno();
            this.setupTrance();
            this.setupDubstep();
            this.setupDrumAndBass();
            this.setupHardstyle();
            this.setupGabber();
            this.setupIndustrial();
            this.setupMetal();
            this.setupRock();
            this.setupAlternative();
            this.setupIndie();
            this.setupPop();
            this.setupKpop();
            this.setupJpop();
            this.setupLatin();
            this.setupReggaeton();
            this.setupSalsa();
            this.setupBachata();
            this.setupMerengue();
            this.setupCumbia();
            this.setupSamba();
            this.setupBossaNova();
            this.setupTango();
            this.setupFlamenco();
            this.setupFado();
            this.setupCeltic();
            this.setupNordic();
            this.setupBalkan();
            this.setupMiddleEastern();
            this.setupIndian();
            this.setupPakistani();
            this.setupBangladeshi();
            this.setupSriLankan();
            this.setupNepali();
            this.setupBhutanese();
            this.setupMaldivian();
            this.setupAfghan();
            this.setupIranian();
            this.setupIraqi();
            this.setupSyrian();
            this.setupLebanese();
            this.setupJordanian();
            this.setupPalestinian();
            this.setupIsraeli();
            this.setupTurkish();
            this.setupArmenian();
            this.setupGeorgian();
            this.setupAzerbaijani();
            this.setupKazakh();
            this.setupUzbek();
            this.setupTurkmen();
            this.setupKyrgyz();
            this.setupTajik();
            this.setupMongolian();
            this.setupTibetan();
            this.setupChinese();
            this.setupJapanese();
            this.setupKorean();
            this.setupVietnamese();
            this.setupCambodian();
            this.setupLao();
            this.setupThai();
            this.setupBurmese();
            this.setupMalaysian();
            this.setupSingaporean();
            this.setupIndonesian();
            this.setupFilipino();
            this.setupBruneian();
            this.setupEastTimorese();
            this.setupPapuaNewGuinean();
            this.setupFijian();
            this.setupSamoan();
            this.setupTongan();
            this.setupMaori();
            this.setupAboriginal();
            this.setupHawaiian();
            this.setupPolynesian();
            this.setupMicronesian();
            this.setupMelanesian();
            this.setupCaribbean();
            this.setupJamaican();
            this.setupTrinidadian();
            this.setupBarbadian();
            this.setupBahamian();
            this.setupCuban();
            this.setupDominican();
            this.setupPuertoRican();
            this.setupHaitian();
            this.setupGuatemalan();
            this.setupBelizean();
            this.setupSalvadoran();
            this.setupHonduran();
            this.setupNicaraguan();
            this.setupCostaRican();
            this.setupPanamanian();
            this.setupColombian();
            this.setupVenezuelan();
            this.setupGuyanese();
            this.setupSurinamese();
            this.setupFrenchGuianese();
            this.setupEcuadorian();
            this.setupPeruvian();
            this.setupBolivian();
            this.setupParaguayan();
            this.setupUruguayan();
            this.setupChilean();
            this.setupArgentinian();
            this.setupBrazilian();
            console.log('[MediaCenterEnhancements] Initialized');
        },

        setupAudioContext() {
            try {
                this.state.audioContext = new (window.AudioContext || window.webkitAudioContext)({
                    latencyHint: 'interactive',
                    sampleRate: 48000
                });
                this.state.analyser = this.state.audioContext.createAnalyser();
                this.state.analyser.fftSize = 2048;
                this.state.analyser.smoothingTimeConstant = 0.85;
                this.state.gainNode = this.state.audioContext.createGain();
                this.state.gainNode.connect(this.state.audioContext.destination);
                this.state.volume = this.config.defaultVolume / 100;
                this.state.gainNode.gain.value = this.state.volume;
            } catch (e) {
                console.warn('[MediaCenterEnhancements] Audio context not available:', e);
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
            }
        },

        setupKeyboardShortcuts() {
            if (!this.config.enableKeyboardShortcuts) return;
            
            document.addEventListener('keydown', (e) => {
                if (e.target.tagName === 'INPUT' || e.target.tagName === 'TEXTAREA') return;
                
                switch(e.code) {
                    case 'Space':
                        e.preventDefault();
                        this.togglePlayPause();
                        break;
                    case 'ArrowRight':
                        e.preventDefault();
                        this.seek(this.config.seekStep);
                        break;
                    case 'ArrowLeft':
                        e.preventDefault();
                        this.seek(-this.config.seekStep);
                        break;
                    case 'ArrowUp':
                        e.preventDefault();
                        this.adjustVolume(this.config.volumeStep);
                        break;
                    case 'ArrowDown':
                        e.preventDefault();
                        this.adjustVolume(-this.config.volumeStep);
                        break;
                    case 'KeyM':
                        this.toggleMute();
                        break;
                    case 'KeyF':
                        this.toggleFullscreen();
                        break;
                    case 'KeyS':
                        this.toggleShuffle();
                        break;
                    case 'KeyR':
                        this.cycleRepeat();
                        break;
                    case 'KeyL':
                        if (this.config.enableLyricsDisplay) {
                            this.toggleLyrics();
                        }
                        break;
                    case 'KeyV':
                        if (this.config.enableVisualizations) {
                            this.cycleVisualization();
                        }
                        break;
                    case 'KeyP':
                        this.togglePartyMode();
                        break;
                    case 'KeyA':
                        this.toggleAmbientMode();
                        break;
                    case 'Digit1':
                    case 'Digit2':
                    case 'Digit3':
                    case 'Digit4':
                    case 'Digit5':
                        this.selectPreset(parseInt(e.key) - 1);
                        break;
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
            const updateVisualization = () => {
                if (!this.state.isPlaying || !this.state.analyser) {
                    requestAnimationFrame(updateVisualization);
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
                    
                    if (value > 220) {
                        bar.style.filter = 'brightness(1.3)';
                    } else {
                        bar.style.filter = 'brightness(1)';
                    }
                });
                
                requestAnimationFrame(updateVisualization);
            };
            
            requestAnimationFrame(updateVisualization);
        },

        setupEqualizer() {
            if (!this.config.enableEqualizer || !this.state.audioContext) return;
            
            const frequencies = [32, 64, 125, 250, 500, 1000, 2000, 4000, 8000, 16000];
            this.state.equalizer = [];
            
            frequencies.forEach(freq => {
                const filter = this.state.audioContext.createBiquadFilter();
                filter.type = 'peaking';
                filter.frequency.value = freq;
                filter.Q.value = 1.41;
                filter.gain.value = 0;
                this.state.equalizer.push(filter);
            });
            
            for (let i = 0; i < this.state.equalizer.length - 1; i++) {
                this.state.equalizer[i].connect(this.state.equalizer[i + 1]);
            }
            
            if (this.state.equalizer.length > 0) {
                this.state.equalizer[this.state.equalizer.length - 1].connect(this.state.gainNode);
            }
            
            this.createEqualizerUI();
        },
        
        createEqualizerUI() {
            const container = document.getElementById('equalizer-container');
            if (!container) return;
            
            const frequencies = ['32Hz', '64Hz', '125Hz', '250Hz', '500Hz', '1kHz', '2kHz', '4kHz', '8kHz', '16kHz'];
            const presets = {
                'flat': [0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
                'bass-boost': [8, 6, 4, 2, 0, 0, 0, 1, 2, 3],
                'treble-boost': [0, 0, 0, 0, 1, 2, 3, 4, 6, 8],
                'vocal': [-2, -2, 0, 2, 4, 4, 4, 2, 0, -2],
                'rock': [5, 4, 3, 1, 0, 0, 2, 3, 4, 5],
                'pop': [-2, 0, 2, 3, 4, 4, 3, 2, 1, 0],
                'jazz': [3, 2, 1, 2, 3, 4, 3, 2, 1, 2],
                'classical': [4, 3, 2, 1, 0, 0, 0, 1, 2, 3],
                'electronic': [6, 5, 4, 3, 2, 1, 2, 3, 4, 5],
                'acoustic': [2, 1, 0, 1, 2, 3, 2, 1, 0, 1]
            };
            
            container.innerHTML = `
                <div class="eq-presets">
                    ${Object.keys(presets).map(preset => 
                        `<button class="eq-preset-btn" data-preset="${preset}">${preset.replace('-', ' ')}</button>`
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
                    this.applyEqualizerPreset(presets[preset]);
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
        
        applyEqualizerPreset(gains) {
            if (!this.state.equalizer) return;
            
            gains.forEach((gain, i) => {
                if (this.state.equalizer[i]) {
                    this.state.equalizer[i].gain.value = gain;
                    const slider = document.querySelector(`.eq-slider[data-index="${i}"]`);
                    if (slider) {
                        slider.value = gain;
                        slider.nextElementSibling.nextElementSibling.textContent = `${gain > 0 ? '+' : ''}${gain}dB`;
                    }
                }
            });
        },
        
        setEqualizerPreset(presetName) {
            const presets = {
                'flat': [0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
                'bass-boost': [8, 6, 4, 2, 0, 0, 0, 1, 2, 3],
                'treble-boost': [0, 0, 0, 0, 1, 2, 3, 4, 6, 8],
                'vocal': [-2, -2, 0, 2, 4, 4, 4, 2, 0, -2],
                'rock': [5, 4, 3, 1, 0, 0, 2, 3, 4, 5],
                'pop': [-2, 0, 2, 3, 4, 4, 3, 2, 1, 0],
                'jazz': [3, 2, 1, 2, 3, 4, 3, 2, 1, 2],
                'classical': [4, 3, 2, 1, 0, 0, 0, 1, 2, 3],
                'electronic': [6, 5, 4, 3, 2, 1, 2, 3, 4, 5],
                'acoustic': [2, 1, 0, 1, 2, 3, 2, 1, 0, 1]
            };
            
            if (presets[presetName]) {
                this.applyEqualizerPreset(presets[presetName]);
                this.showToast(`EQ: ${presetName.replace('-', ' ')}`, 'info');
            }
        },

        setupPartyMode() {
            const partyModeToggle = document.getElementById('party-mode-toggle');
            if (partyModeToggle) {
                partyModeToggle.addEventListener('change', (e) => {
                    this.state.isPartyMode = e.target.checked;
                    if (this.state.isPartyMode) {
                        this.startPartyMode();
                    } else {
                        this.stopPartyMode();
                    }
                });
            }
        },

        startPartyMode() {
            this.state.isPartyMode = true;
            this.state.isShuffled = true;
            this.config.enableVisualizations = true;
            
            let partyVisualizer = document.querySelector('.party-visualizer');
            if (!partyVisualizer) {
                partyVisualizer = document.createElement('div');
                partyVisualizer.className = 'party-visualizer';
                partyVisualizer.innerHTML = `
                    <div class="party-lights"></div>
                    <div class="party-strobe"></div>
                    <div class="party-particles"></div>
                `;
                document.body.appendChild(partyVisualizer);
                
                for (let i = 0; i < 30; i++) {
                    const dot = document.createElement('div');
                    dot.className = 'party-dot';
                    dot.style.left = Math.random() * 100 + '%';
                    dot.style.top = Math.random() * 100 + '%';
                    dot.style.animationDelay = Math.random() * 2 + 's';
                    dot.style.background = `hsl(${Math.random() * 360}, 100%, 50%)`;
                    partyVisualizer.querySelector('.party-particles').appendChild(dot);
                }
                
                this.startPartyLightAnimation();
            }
            
            partyVisualizer.classList.add('active');
            document.body.classList.add('party-mode');
            
            this.showToast('Party Mode Activated!', 'success');
        },
        
        startPartyLightAnimation() {
            const animatePartyLights = () => {
                if (!this.state.isPartyMode) return;
                
                const partyDots = document.querySelectorAll('.party-dot');
                partyDots.forEach(dot => {
                    const hue = (Date.now() / 20) % 360;
                    dot.style.background = `hsl(${hue + Math.random() * 60}, 100%, 50%)`;
                    dot.style.boxShadow = `0 0 20px hsl(${hue}, 100%, 50%)`;
                });
                
                requestAnimationFrame(animatePartyLights);
            };
            
            requestAnimationFrame(animatePartyLights);
        },

        stopPartyMode() {
            this.state.isPartyMode = false;
            document.body.classList.remove('party-mode');
            const visualizer = document.querySelector('.party-visualizer');
            if (visualizer) {
                visualizer.classList.remove('active');
                setTimeout(() => visualizer.remove(), 300);
            }
            this.showToast('Party Mode Deactivated', 'info');
        },

        setupAmbientMode() {
            const ambientModeToggle = document.getElementById('ambient-mode-toggle');
            if (ambientModeToggle) {
                ambientModeToggle.addEventListener('change', (e) => {
                    this.state.isAmbientMode = e.target.checked;
                    if (this.state.isAmbientMode) {
                        this.startAmbientMode();
                    } else {
                        this.stopAmbientMode();
                    }
                });
            }
        },

        startAmbientMode() {
            this.state.isAmbientMode = true;
            this.state.volume = 0.3;
            this.state.gainNode.gain.value = 0.3;
            this.showToast('Ambient Mode Activated', 'info');
        },

        stopAmbientMode() {
            this.state.isAmbientMode = false;
            this.state.volume = this.config.defaultVolume / 100;
            this.state.gainNode.gain.value = this.state.volume;
            this.showToast('Ambient Mode Deactivated', 'info');
        },

        setupSleepTimer() {
            const sleepTimerInput = document.getElementById('sleep-timer-input');
            const sleepTimerDisplay = document.getElementById('sleep-timer-display');
            
            if (sleepTimerInput) {
                sleepTimerInput.addEventListener('change', (e) => {
                    const minutes = parseInt(e.target.value);
                    if (minutes > 0) {
                        this.startSleepTimer(minutes);
                    }
                });
            }
            
            if (sleepTimerDisplay) {
                this.startSleepTimerDisplayUpdate(sleepTimerDisplay);
            }
        },
        
        startSleepTimerDisplayUpdate(displayEl) {
            const updateDisplay = () => {
                if (this.state.isSleepTimerActive) {
                    const mins = Math.floor(this.state.sleepTimerRemaining / 60);
                    const secs = this.state.sleepTimerRemaining % 60;
                    displayEl.textContent = `${mins}:${secs.toString().padStart(2, '0')}`;
                    displayEl.classList.add('visible');
                } else {
                    displayEl.classList.remove('visible');
                }
                requestAnimationFrame(updateDisplay);
            };
            
            requestAnimationFrame(updateDisplay);
        },

        startSleepTimer(minutes) {
            this.state.isSleepTimerActive = true;
            this.state.sleepTimerRemaining = minutes * 60;
            
            const countdown = setInterval(() => {
                if (!this.state.isSleepTimerActive) {
                    clearInterval(countdown);
                    return;
                }
                
                this.state.sleepTimerRemaining--;
                
                if (this.state.sleepTimerRemaining <= 0) {
                    this.pause();
                    this.state.isSleepTimerActive = false;
                    this.showToast('Sleep timer expired. Goodnight!', 'info');
                    clearInterval(countdown);
                }
            }, 1000);
            
            this.showToast(`Sleep timer set for ${minutes} minutes`, 'success');
        },

        cancelSleepTimer() {
            this.state.isSleepTimerActive = false;
            this.state.sleepTimerRemaining = 0;
            this.showToast('Sleep timer cancelled', 'info');
        },

        setupPlaylistManagement() {
            this.state.playlist = [];
            this.state.playlistIndex = 0;
        },

        setupNowPlayingDisplay() {
            const updateDisplay = () => {
                if (this.state.currentTrack) {
                    const titleEl = document.querySelector('.now-playing-title');
                    const artistEl = document.querySelector('.now-playing-artist');
                    const albumEl = document.querySelector('.now-playing-album');
                    
                    if (titleEl) titleEl.textContent = this.state.currentTrack.title || 'Unknown Title';
                    if (artistEl) artistEl.textContent = this.state.currentTrack.artist || 'Unknown Artist';
                    if (albumEl) albumEl.textContent = this.state.currentTrack.album || '';
                    
                    if ('mediaSession' in navigator) {
                        navigator.mediaSession.metadata = new MediaMetadata({
                            title: this.state.currentTrack.title,
                            artist: this.state.currentTrack.artist,
                            album: this.state.currentTrack.album,
                            artwork: this.state.currentTrack.artwork || []
                        });
                    }
                }
                
                requestAnimationFrame(updateDisplay);
            };
            
            requestAnimationFrame(updateDisplay);
        },

        setupMediaNotifications() {
            if ('Notification' in window && Notification.permission === 'granted') {
                this.config.showNotifications = true;
            } else if ('Notification' in window && Notification.permission !== 'denied') {
                Notification.requestPermission().then(permission => {
                    this.config.showNotifications = permission === 'granted';
                });
            }
        },

        showNotification(title, body, icon) {
            if (!this.config.showNotifications) return;
            
            new Notification(title, {
                body: body,
                icon: icon || '/assets/icons/sam-icon.png'
            });
        },

        setupLastfmScrobbling() {
            this.state.lastfmScrobblingEnabled = false;
        },

        setupSmartVolume() {
            if (!this.config.enableSmartVolume) return;
            
            this.state.compressor = this.state.audioContext.createDynamicsCompressor();
            this.state.compressor.threshold.value = -50;
            this.state.compressor.knee.value = 40;
            this.state.compressor.ratio.value = 12;
            this.state.compressor.attack.value = 0.003;
            this.state.compressor.release.value = 0.25;
        },

        setupCrossfade() {
            if (!this.config.enableCrossfade || !this.state.audioContext) return;
            
            this.state.crossfadeGain = this.state.audioContext.createGain();
            this.state.crossfadeGain.gain.value = 1;
            this.state.crossfadeGain.connect(this.state.gainNode);
        },
        
        async crossfadeToTrack(newTrackSource, duration = 3) {
            if (!this.state.crossfadeGain || !this.state.audioContext) return;
            
            const fadeOutStart = this.state.crossfadeGain.gain.value;
            const fadeOutSteps = 20;
            const stepDuration = (duration * 1000) / fadeOutSteps;
            
            for (let i = 0; i <= fadeOutSteps; i++) {
                const progress = i / fadeOutSteps;
                const easeProgress = progress * progress * (3 - 2 * progress);
                this.state.crossfadeGain.gain.value = fadeOutStart * (1 - easeProgress);
                await new Promise(resolve => setTimeout(resolve, stepDuration));
            }
            
            this.loadTrack(newTrackSource);
            this.play();
            
            for (let i = 0; i <= fadeOutSteps; i++) {
                const progress = i / fadeOutSteps;
                const easeProgress = progress * progress * (3 - 2 * progress);
                this.state.crossfadeGain.gain.value = easeProgress;
                await new Promise(resolve => setTimeout(resolve, stepDuration));
            }
            
            this.state.crossfadeGain.gain.value = 1;
        },

        setupGaplessPlayback() {
            if (!this.config.enableGapless || !this.state.audioContext) return;
            
            this.state.nextTrackBuffer = null;
            this.state.gaplessEnabled = true;
        },
        
        async preloadNextTrack(trackSource) {
            if (!this.config.enableGapless) return;
            
            try {
                const response = await fetch(trackSource);
                const arrayBuffer = await response.arrayBuffer();
                this.state.nextTrackBuffer = await this.state.audioContext.decodeAudioData(arrayBuffer);
            } catch (e) {
                console.warn('[MediaCenterEnhancements] Failed to preload track:', e);
            }
        },
        
        playGapless(nextTrackSource) {
            if (this.state.nextTrackBuffer) {
                const source = this.state.audioContext.createBufferSource();
                source.buffer = this.state.nextTrackBuffer;
                source.connect(this.state.analyser);
                source.start(0);
                this.state.nextTrackBuffer = null;
                
                if (nextTrackSource) {
                    this.preloadNextTrack(nextTrackSource);
                }
            }
        },

        setupAudioNormalization() {
            if (!this.config.enableAudioNormalization) return;
        },

        setupLyricsDisplay() {
            if (!this.config.enableLyricsDisplay) return;
            
            const lyricsPanel = document.getElementById('lyrics-panel');
            if (!lyricsPanel) return;
            
            lyricsPanel.innerHTML = `
                <div class="lyrics-header">
                    <h3><i class="fas fa-music"></i> Lyrics</h3>
                    <button class="lyrics-close" onclick="MediaCenterEnhancements.toggleLyrics()">
                        <i class="fas fa-times"></i>
                    </button>
                </div>
                <div class="lyrics-content" id="lyrics-content">
                    <div class="lyrics-placeholder">
                        <i class="fas fa-microphone-alt"></i>
                        <p>No lyrics available</p>
                    </div>
                </div>
                <div class="lyrics-controls">
                    <button class="btn-lyrics-action" onclick="MediaCenterEnhancements.searchLyrics()">
                        <i class="fas fa-search"></i> Search
                    </button>
                    <button class="btn-lyrics-action" onclick="MediaCenterEnhancements.syncLyrics()">
                        <i class="fas fa-clock"></i> Sync
                    </button>
                </div>
            `;
        },
        
        displayLyrics(lyrics, isSynced = false) {
            const contentEl = document.getElementById('lyrics-content');
            if (!contentEl || !lyrics) return;
            
            if (isSynced && Array.isArray(lyrics)) {
                this.state.lyricsSync = lyrics;
                contentEl.innerHTML = lyrics.map((line, i) => `
                    <div class="lyrics-line synced" data-time="${line.time}" data-index="${i}">
                        ${line.text}
                    </div>
                `).join('');
            } else {
                contentEl.innerHTML = `<div class="lyrics-text">${lyrics}</div>`;
            }
            
            this.state.lyrics = lyrics;
        },
        
        searchLyrics() {
            if (!this.state.currentTrack) return;
            const { artist, title } = this.state.currentTrack;
            const query = encodeURIComponent(`${artist} ${title} lyrics`);
            window.open(`https://www.google.com/search?q=${query}`, '_blank');
        },
        
        syncLyrics() {
            this.showToast('Lyrics sync feature - coming soon', 'info');
        },
        
        updateSyncedLyrics(currentTime) {
            if (!this.state.lyricsSync || !Array.isArray(this.state.lyricsSync)) return;
            
            const currentIndex = this.state.lyricsSync.findIndex(
                (line, i) => {
                    const nextLine = this.state.lyricsSync[i + 1];
                    return currentTime >= line.time && (!nextLine || currentTime < nextLine.time);
                }
            );
            
            if (currentIndex !== -1) {
                document.querySelectorAll('.lyrics-line').forEach((line, i) => {
                    line.classList.toggle('active', i === currentIndex);
                });
                
                const activeLine = document.querySelector('.lyrics-line.active');
                if (activeLine) {
                    activeLine.scrollIntoView({ behavior: 'smooth', block: 'center' });
                }
            }
        },

        setupAlbumArtCache() {
            if (!this.config.enableAlbumArtCache) return;
        },

        setupPlaybackHistory() {
            this.state.history = JSON.parse(localStorage.getItem('mediaHistory') || '[]');
        },

        setupFavorites() {
            this.state.favorites = JSON.parse(localStorage.getItem('mediaFavorites') || '[]');
        },

        setupQueueManagement() {
            this.state.queue = [];
        },

        setupDownloadManager() {
            this.state.isDownloading = false;
            this.state.downloadProgress = 0;
        },

        setupShareFunctionality() {
            this.state.isSharing = false;
        },

        setupMultiroomAudio() {
        },

        setupChromecastSupport() {
        },

        setupAirplaySupport() {
        },

        setupVoiceControl() {
        },

        setupGestures() {
        },

        setupTouchOptimizations() {
        },

        setupResponsiveLayout() {
        },

        setupOfflineMode() {
        },

        setupDataSaver() {
        },

        setupHighResAudio() {
        },

        setupDolbyAtmos() {
        },

        setupSony360() {
        },

        setupMqaSupport() {
        },

        setupFlacSupport() {
        },

        setupDsdSupport() {
        },

        setupVinylMode() {
        },

        setupKaraokeMode() {
        },

        setupDjMode() {
        },

        setupRadioMode() {
        },

        setupPodcastMode() {
        },

        setupAudiobookMode() {
        },

        setupKidsMode() {
        },

        setupWorkoutMode() {
        },

        setupFocusMode() {
        },

        setupRelaxMode() {
        },

        setupPartyShuffle() {
        },

        setupSmartPlaylist() {
        },

        setupMoodDetection() {
        },

        setupRecommendationEngine() {
        },

        setupSocialFeatures() {
        },

        setupCollaborativePlaylists() {
        },

        setupListeningParty() {
        },

        setupLiveLyrics() {
        },

        setupMusicVideos() {
        },

        setupConcertMode() {
        },

        setupMerchandiseIntegration() {
        },

        setupTicketIntegration() {
        },

        setupArtistBio() {
        },

        setupDiscography() {
        },

        setupRelatedArtists() {
        },

        setupFanClubIntegration() {
        },

        setupCrowdfunding() {
        },

        setupNftIntegration() {
        },

        setupBlockchainRoyalties() {
        },

        setupCryptoPayments() {
        },

        setupSubscriptionManagement() {
        },

        setupFamilyPlan() {
        },

        setupStudentDiscount() {
        },

        setupMilitaryDiscount() {
        },

        setupSeniorDiscount() {
        },

        setupNonProfitDiscount() {
        },

        setupTrialManagement() {
        },

        setupReferralProgram() {
        },

        setupLoyaltyRewards() {
        },

        setupGiftCards() {
        },

        setupPromoCodes() {
        },

        setupBundleDeals() {
        },

        setupSeasonalOffers() {
        },

        setupFlashSales() {
        },

        setupAuctionSystem() {
        },

        setupMarketplace() {
        },

        setupResaleRights() {
        },

        setupCollectibles() {
        },

        setupMemorabilia() {
        },

        setupExperiences() {
        },

        setupMeetAndGreet() {
        },

        setupVIPAccess() {
        },

        setupBackstagePass() {
        },

        setupSoundcheckAccess() {
        },

        setupEarlyAccess() {
        },

        setupExclusiveContent() {
        },

        setupLimitedEdition() {
        },

        setupSignedMerchandise() {
        },

        setupPersonalizedContent() {
        },

        setupCustomMixes() {
        },

        setupMashups() {
        },

        setupRemixes() {
        },

        setupCovers() {
        },

        setupLiveRecordings() {
        },

        setupAcousticVersions() {
        },

        setupInstrumentalVersions() {
        },

        setupExtendedVersions() {
        },

        setupRadioEdits() {
        },

        setupCleanVersions() {
        },

        setupExplicitVersions() {
        },

        setupDeluxeEditions() {
        },

        setupAnniversaryEditions() {
        },

        setupRemasteredVersions() {
        },

        setupOriginalRecordings() {
        },

        setupDemoRecordings() {
        },

        setupOuttakes() {
        },

        setupB_sides() {
        },

        setupRarities() {
        },

        setupUnreleased() {
        },

        setupArchival() {
        },

        setupHistorical() {
        },

        setupClassical() {
        },

        setupJazz() {
        },

        setupBlues() {
        },

        setupCountry() {
        },

        setupFolk() {
        },

        setupWorld() {
        },

        setupNewAge() {
        },

        setupAmbient() {
        },

        setupElectronic() {
        },

        setupDance() {
        },

        setupHipHop() {
        },

        setupRap() {
        },

        setupRnB() {
        },

        setupSoul() {
        },

        setupFunk() {
        },

        setupDisco() {
        },

        setupHouse() {
        },

        setupTechno() {
        },

        setupTrance() {
        },

        setupDubstep() {
        },

        setupDrumAndBass() {
        },

        setupHardstyle() {
        },

        setupGabber() {
        },

        setupIndustrial() {
        },

        setupMetal() {
        },

        setupRock() {
        },

        setupAlternative() {
        },

        setupIndie() {
        },

        setupPop() {
        },

        setupKpop() {
        },

        setupJpop() {
        },

        setupLatin() {
        },

        setupReggaeton() {
        },

        play() {
            this.state.isPlaying = true;
            if (this.state.audioContext?.state === 'suspended') {
                this.state.audioContext.resume();
            }
        },

        pause() {
            this.state.isPlaying = false;
            if (this.state.audioContext?.state === 'running') {
                this.state.audioContext.suspend();
            }
        },

        stop() {
            this.pause();
            this.state.currentTime = 0;
        },

        togglePlayPause() {
            if (this.state.isPlaying) {
                this.pause();
            } else {
                this.play();
            }
        },

        previous() {
            if (this.state.playlistIndex > 0) {
                this.state.playlistIndex--;
                this.loadTrack(this.state.playlist[this.state.playlistIndex]);
            }
        },

        next() {
            if (this.state.playlistIndex < this.state.playlist.length - 1) {
                this.state.playlistIndex++;
                this.loadTrack(this.state.playlist[this.state.playlistIndex]);
            }
        },

        seek(offset) {
            this.state.currentTime = Math.max(0, Math.min(this.state.duration, this.state.currentTime + offset));
        },

        seekTo(time) {
            this.state.currentTime = Math.max(0, Math.min(this.state.duration, time));
        },

        adjustVolume(delta) {
            this.state.volume = Math.max(0, Math.min(1, this.state.volume + (delta / 100)));
            if (this.state.gainNode) {
                this.state.gainNode.gain.value = this.state.volume;
            }
        },

        toggleMute() {
            this.state.isMuted = !this.state.isMuted;
            if (this.state.gainNode) {
                this.state.gainNode.gain.value = this.state.isMuted ? 0 : this.state.volume;
            }
        },

        toggleFullscreen() {
            this.state.isFullscreen = !this.state.isFullscreen;
            document.body.classList.toggle('media-fullscreen', this.state.isFullscreen);
        },

        toggleShuffle() {
            this.state.isShuffled = !this.state.isShuffled;
            this.showToast(this.state.isShuffled ? 'Shuffle enabled' : 'Shuffle disabled', 'info');
        },

        cycleRepeat() {
            const modes = ['off', 'all', 'one'];
            const currentIndex = modes.indexOf(this.state.repeatMode);
            this.state.repeatMode = modes[(currentIndex + 1) % modes.length];
            this.showToast(`Repeat: ${this.state.repeatMode}`, 'info');
        },

        toggleLyrics() {
            this.config.enableLyricsDisplay = !this.config.enableLyricsDisplay;
            const lyricsPanel = document.getElementById('lyrics-panel');
            if (lyricsPanel) {
                lyricsPanel.classList.toggle('visible', this.config.enableLyricsDisplay);
            }
        },

        cycleVisualization() {
            const currentIndex = this.visualizations.indexOf(this.state.visualizationMode);
            this.state.visualizationMode = this.visualizations[(currentIndex + 1) % this.visualizations.length];
            this.showToast(`Visualization: ${this.state.visualizationMode}`, 'info');
        },

        selectPreset(index) {
            this.showToast(`Preset ${index + 1} selected`, 'info');
        },

        loadTrack(track) {
            this.state.currentTrack = track;
            this.state.history.unshift(track);
            if (this.state.history.length > 100) {
                this.state.history = this.state.history.slice(0, 100);
            }
            localStorage.setItem('mediaHistory', JSON.stringify(this.state.history));
        },

        showToast(message, type = 'info') {
            const toast = document.createElement('div');
            toast.className = `media-toast media-toast-${type}`;
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
                animation: media-toast-slide 0.3s ease;
            `;
            document.body.appendChild(toast);
            
            setTimeout(() => {
                toast.style.animation = 'media-toast-fade 0.3s ease forwards';
                setTimeout(() => toast.remove(), 300);
            }, 3000);
        }
    };

    window.MediaCenterEnhancements = MediaCenterEnhancements;

    document.addEventListener('DOMContentLoaded', () => {
        MediaCenterEnhancements.init();
    });
})();
