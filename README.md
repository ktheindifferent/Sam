# S.A.M.
Smart Artificial Mind
WIP. Dont use this software yet.
Licensed under GPL version 3.

TTS API:
https://tts.opensam.foundation/api/tts?text=hello%20world&speaker_id=&style_wav=
https://tts.alpha.opensam.foundation/api/tts?text=hello%20world&speaker_id=&style_wav=


Features:
- Touch Friendly User Interface
- Built with rust-lang
- Open Source (GPLv3)

Lifx:
  - Works online and offline (with offline lifx server package)
  - Ability to set the light color/kelvin/power from touch panels

Media Center Features:
  - Games
  - App Support (Netflix, Youtube, Spotify, etc.)
  - Game Controller Support

## 🚨 Security Notice

**THIS SOFTWARE IS IN ACTIVE DEVELOPMENT AND NOT PRODUCTION READY**

Before using S.A.M., please review the [SECURITY.md](SECURITY.md) file for important security considerations. Recent security fixes include:

- ✅ Fixed critical command injection vulnerabilities
- ✅ Fixed SQL injection prevention
- ✅ Fixed application crash from network errors
- 🔄 Ongoing work to replace unsafe `.unwrap()` calls

## 🛠️ Installation & Setup

### Prerequisites

- **Rust 1.70+** - Install from [rustup.rs](https://rustup.rs/)
- **PostgreSQL 13+** - Required for data storage
- **Redis** (optional) - For caching and session management
- **Docker** (optional) - For containerized services
- **Python 3.8+** - For AI/ML components
- **FFmpeg** - For media processing

### Quick Setup

1. **Clone the repository**
   ```bash
   git clone https://github.com/your-org/sam.git
   cd sam
   ```

2. **Install system dependencies**
   ```bash
   # Ubuntu/Debian
   sudo apt update
   sudo apt install postgresql-client libpq-dev redis-server ffmpeg python3-pip

   # macOS
   brew install postgresql redis ffmpeg python3
   
   # Arch Linux
   sudo pacman -S postgresql-libs redis ffmpeg python
   ```

3. **Setup database**
   ```bash
   # Create PostgreSQL database
   createdb sam_db
   
   # Set environment variables
   export DATABASE_URL="postgresql://username:password@localhost/sam_db"
   export REDIS_URL="redis://localhost:6379"
   ```

4. **Build and run**
   ```bash
   cargo build --release
   ./target/release/sam --help
   ```

### Configuration

Create `/opt/sam/config.json` with your settings:
```json
{
  "database_url": "postgresql://localhost/sam_db",
  "redis_url": "redis://localhost:6379",
  "http_port": 8000,
  "enable_tts": true,
  "enable_stt": true,
  "log_level": "info"
}
```

### Development Setup

1. **Run installer (development mode)**
   ```bash
   cargo run --bin installer -- --dev-setup
   ```

2. **Start development server**
   ```bash
   cargo run -- serve --dev
   ```

3. **Run tests**
   ```bash
   cargo test
   ```

## 📋 TODO/Roadmap

### Security & Stability (High Priority)
- 🔄 Replace remaining `.unwrap()` calls with proper error handling
- 🔄 Audit and fix remaining command injection vectors
- ⏳ Add comprehensive input validation
- ⏳ Implement proper session management
- ⏳ Add rate limiting and DOS protection

### Core Features
- Add clock_widget_display_format setting for the clock widget
- Intergrate Whisper as a primary STT/TTS engine https://github.com/ggerganov/whisper.cpp
- Use whisper for realtime STT/TTS using wasm2js
- Keep exsting TTS/STT methods to be used as a backup
- Add metadata to file storage api
- SSH command pipeline support for cli
- Password manager
- Vulnerability scanning and classification of internal network
- Ext Web crawler for links, summaries, ports, etc.
- P2P communications between sam instances for Job tasking, hive communications

### Platform Support
- Stablize windows build
- Mobile App
- Docker containerization

### UI/UX Improvements  
- Overhaul web interface for no jquery, gulp asset pipelines, etc
- Overhaul help command

### Gaming & Emulation
- PS1 emulation and native file support (.ps1) https://github.com/js-emulators/WASMpsx
- NES emulation and native file support (.nes) https://github.com/takahirox/nes-rust
- Gameboy emulation https://github.com/andrewimm/wasm-gb
- Chip-8 Emulation (.ch8)

0.0.4(WIP):
- database restructured
- crawler for deep web research
- docker, redis, postgres installer workflow for automated setup


0.0.3:
- Fix file browser (dropbox, deleting files, moving files, etc.)
- Add support for visual RSTP observations
- amd64 support for ffmpeg and whisper packages
- Copy fonts file to sam directory during setup (DONE)
- Finish package installer (search, install, uninstall)
- Fix settings to actually do something
- Redesign humans page with avatar support
- Fix tracker for heard_count
- Add ability to correct observations in the observation deck
- Review build sprec code
- Redesign notifications to be instant when initiadted from the client side
- Link web session microphone to new sound pipeline s1,s2,s3
- Associate observations with things and/or web sessions
- Redesign locations UIX
- Build calendar/clock widget


AI Features to Expolore:
- Image Super Resoloution
- Minst handwriting
- Speech recognition (DONE)
- Speaker recognition (DONE)
- Deep Vision
- GAN art generation
- Ability to generate reports on any topic
- Ability to sumerize news stories and prioritize feed based on user preferences


Socket API:
- Notifications
- Ability to launch web apps/files on individual devices (web->socket sesssions)

Weather API:
- Uses in house server package "Jupiter" to generate weather reports for your zip code

Console:
- whoami: returns current human users name
- whereami: returns current human users location

News API:
- Copy RSS feed comsuption code from BOT
- Use rust-bert to generate summaries for articles

Stonks API:
- Ability to track blockchain prices
- Ability to track stock prices
- Ability to track multiple wallet realtime values using public keys
- Ability to set high/low notifications

Notifications API:
- Store in SQL
- User Specific
- Can be location specific, but not necessarily
- Ability to send notifications via email or SMS after X minutes of being unread

Storage API:
- Ability to store files in SQL, Cloud Storage, etc. (WIP)

Media Center:
- Controller Support (DONE)
- Music (Depends on new storage pipeline)
- Movies (Depends on new storage pipeline)
- Make games packages that can be installed into the storage pipeline

Settings:
- Ability to set audio recognition noise threshold (WIP)
- Ability to set custom STT server (WIP)

Speech pipeline redesign:
- Recode web recorder to store wav files in the stream pipeline instead of running stt in the browser
- Stitch continuous speech wav file streams and trim whitenoise (DONEish)
- Perform STT using API from settings
- Use sprec to identify the speaker
- Store speech as an observation 
- Split pipeline stream by thing_oid:xxx AND web_session:xxx

Remote Device Management API:
- Ability to execute remote commands over an ssh tunnel connection
- Installs remote utility which gathers system information
- Very useful for keeping multiple touch screen panels, servers updated

Network Security API:
- Scans local network for security vulnerabilities and exposed ports
- Allows sam to make recomendations for incresing network security

#### To Override DNS:
{
  "TaskTemplate": {
      "DNSConfig": {
        "Nameservers": [
          "172.16.0.15"
        ]
      }
    }
}