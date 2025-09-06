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

Before using S.A.M., please review the [SECURITY.md](docs/security/SECURITY.md) file for important security considerations. Recent security fixes include:

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

## 🚢 CapRover Deployment

CapRover is a Platform-as-a-Service (PaaS) solution that makes deploying applications easy. S.A.M. is optimized for CapRover deployment with built-in support for containerization and scalability.

### Prerequisites for CapRover Deployment

1. **CapRover Server** - Set up your CapRover instance following the [official documentation](https://caprover.com/docs/get-started.html)
2. **CapRover CLI** - Install the CapRover command line tool:
   ```bash
   npm install -g caprover
   ```
3. **Git Repository** - Your S.A.M. code should be in a Git repository

### Deployment Methods

#### Method 1: Direct Git Deployment (Recommended)

1. **Login to your CapRover instance**
   ```bash
   caprover login
   ```

2. **Create a new app in CapRover**
   - Log into your CapRover web interface
   - Go to "Apps" → "Create New App"
   - Enter app name: `sam` (or your preferred name)
   - Click "Create New App"

3. **Configure environment variables**
   In your CapRover app settings, add these environment variables:
   ```bash
   # Database Configuration
   DATABASE_ENGINE=sqlite
   SQLITE_DATABASE_PATH=/var/lib/sam/sam.db
   
   # Optional: Use PostgreSQL instead
   # DATABASE_ENGINE=postgresql
   # DATABASE_URL=postgresql://user:pass@hostname:5432/sam_db
   
   # Redis Configuration (optional)
   # REDIS_URL=redis://your-redis-instance:6379
   # REDIS_DISABLED=false
   
   # Application Configuration
   PORT=8000
   SAM_HOME=/app
   SAM_DATA=/var/lib/sam
   SAM_LOGS=/var/log/sam
   RUST_LOG=info
   RUST_BACKTRACE=1
   
   # Migration Settings
   RUN_MIGRATIONS=true
   ```

4. **Deploy using Git**
   ```bash
   # In your S.A.M. project directory
   caprover deploy --appName sam
   ```

5. **Enable HTTPS and configure domain**
   - In CapRover web interface, go to your app
   - Under "HTTP Settings": 
     - Enable HTTPS
     - Configure your domain (e.g., `sam.yourdomain.com`)
     - Force HTTPS redirect

#### Method 2: Dockerfile Deployment

1. **Create your app in CapRover**
2. **Use the "Deploy via Image Name" option**
   - Build image locally: `docker build -t sam:latest .`
   - Push to registry or use CapRover's built-in deployment

#### Method 3: Upload tar file

1. **Create deployment package**
   ```bash
   # Create a tar file excluding unnecessary files
   tar --exclude='target' --exclude='.git' --exclude='node_modules' \
       -czf sam-deployment.tar.gz .
   ```

2. **Upload via CapRover web interface**
   - Go to your app in CapRover
   - Use "Deploy via Tarball" option
   - Upload `sam-deployment.tar.gz`

### Post-Deployment Configuration

#### Persistent Volumes

Configure persistent volumes for data storage:

1. In CapRover app settings, go to "App Configs"
2. Add persistent directories:
   ```
   /var/lib/sam → sam-data
   /var/log/sam → sam-logs
   ```

#### Database Setup

**For SQLite (Default):**
- No additional setup needed
- Database will be created automatically on first run

**For PostgreSQL:**
1. Create a PostgreSQL app in CapRover or use external service
2. Update the `DATABASE_URL` environment variable
3. Enable `RUN_MIGRATIONS=true`

#### Health Checks

CapRover will automatically use the Docker HEALTHCHECK. S.A.M. provides these endpoints:
- `GET /health/live` - Liveness probe
- `GET /health/ready` - Readiness probe  
- `GET /health` - Basic health check

### Production Recommendations

#### Security
```bash
# Environment variables for production
RUST_LOG=warn
RUST_BACKTRACE=0
DATABASE_ENGINE=postgresql  # Recommended for production
```

#### Performance
- **CPU**: Minimum 1 CPU, recommended 2+ CPUs
- **RAM**: Minimum 1GB, recommended 2GB+ for AI features
- **Storage**: At least 10GB for logs, database, and assets

#### Monitoring
Enable CapRover's built-in monitoring or integrate with external services:
- **Metrics**: Prometheus endpoints available at `/metrics`
- **Logs**: Structured logging with JSON format
- **Health**: Built-in health check endpoints

### Scaling

#### Horizontal Scaling
```bash
# Scale to multiple instances
caprover api --path "/api/v2/user/apps/data/sam" --method "POST" \
  --data '{"instanceCount": 3}'
```

#### Load Balancing
CapRover automatically handles load balancing between instances.

### Troubleshooting

#### Common Issues

1. **Build Failures**
   ```bash
   # Check build logs
   caprover logs --app sam --lines 100
   
   # Common solutions:
   # - Increase build timeout in CapRover settings
   # - Check Dockerfile syntax
   # - Verify all required files are included
   ```

2. **Database Connection Issues**
   ```bash
   # Check environment variables
   caprover api --path "/api/v2/user/apps/data/sam" --method "GET"
   
   # Test database connectivity
   caprover exec --app sam --command "pg_isready -d $DATABASE_URL"
   ```

3. **Memory Issues**
   ```bash
   # Increase app memory limit in CapRover settings
   # Monitor memory usage
   caprover stats --app sam
   ```

#### Logs and Debugging
```bash
# View real-time logs
caprover logs --app sam --follow

# View recent logs
caprover logs --app sam --lines 1000

# Execute commands in container
caprover exec --app sam --command "/bin/bash"
```

### Example Production Deployment Script

```bash
#!/bin/bash
set -e

APP_NAME="sam"
DOMAIN="sam.yourdomain.com"

echo "Deploying S.A.M. to CapRover..."

# Login to CapRover
caprover login

# Deploy application
caprover deploy --appName "$APP_NAME"

# Wait for deployment
echo "Waiting for deployment to complete..."
sleep 30

# Check health
curl -f "https://$DOMAIN/health" || {
    echo "Health check failed!"
    caprover logs --app "$APP_NAME" --lines 50
    exit 1
}

echo "✅ S.A.M. deployed successfully!"
echo "🌐 Access your application at: https://$DOMAIN"
```

### Environment-Specific Configurations

#### Development
```bash
RUST_LOG=debug
RUST_BACKTRACE=full
DATABASE_ENGINE=sqlite
RUN_MIGRATIONS=true
```

#### Staging
```bash
RUST_LOG=info
RUST_BACKTRACE=1
DATABASE_ENGINE=postgresql
RUN_MIGRATIONS=true
```

#### Production
```bash
RUST_LOG=warn
RUST_BACKTRACE=0
DATABASE_ENGINE=postgresql
RUN_MIGRATIONS=false  # Run migrations manually
```

### Local CapRover Testing

To test your CapRover deployment locally before deploying:

```bash
# Test the CapRover-optimized configuration locally
docker-compose -f deploy/docker-compose.caprover.yml up

# Include management tools (Redis Commander, pgAdmin)
docker-compose -f deploy/docker-compose.caprover.yml --profile tools up

# Test just the S.A.M. application
docker-compose -f deploy/docker-compose.caprover.yml up sam
```

This will spin up S.A.M. with the same configuration and environment as CapRover deployment.

For detailed CapRover documentation, visit: https://caprover.com/docs/

## 📁 Project Structure

This project follows an organized directory structure for better maintainability:

```
├── docs/                    # 📚 All documentation (organized by category)
│   ├── api/                # API documentation  
│   ├── deployment/         # Deployment guides
│   ├── development/        # Technical implementation docs
│   ├── features/           # Feature descriptions
│   └── security/           # Security guides and fixes
├── deploy/                 # 🚢 Deployment configurations
│   ├── docker-compose*.yml # Docker Compose files
│   ├── Dockerfile*         # Docker build files
│   └── docker-entrypoint.sh
├── config/                 # ⚙️ Development configuration
├── scripts/                # 🔧 Application scripts
├── tools/                  # 🛠️ Development tools
├── tests/                  # 🧪 All test files and utilities
├── src/                    # 💻 Rust source code
└── www/                    # 🌐 Web frontend
```

For detailed information about the directory structure, see [docs/DIRECTORY_STRUCTURE.md](docs/DIRECTORY_STRUCTURE.md).

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