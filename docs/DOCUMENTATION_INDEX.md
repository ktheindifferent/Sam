# S.A.M. Documentation Index

Complete guide to all S.A.M. documentation files and their contents.

---

## Core Documentation

### 📘 [API.md](./API.md) - API Reference
**1,169 lines | 20 KB**

Complete REST API documentation including:
- Authentication & session management
- Voice services (STT, TTS, assistant)
- P2P networking endpoints
- Media crawling and streaming
- Job queue management
- Error handling and rate limiting
- 50+ endpoint examples with request/response bodies

**Use when:** Integrating with S.A.M. API, understanding endpoint behavior

---

### 🏗️ [ARCHITECTURE.md](./ARCHITECTURE.md) - System Design
**442 lines | 16 KB**

High-level system architecture including:
- Core architecture diagram (HTTP → Services → Data)
- Module organization (`lib/voice`, `lib/p2p`, `lib/security`, etc.)
- Voice services architecture
- P2P networking design
- Security layer architecture
- Media services design
- Job queue architecture
- Database schema overview
- Concurrency model (Tokio runtime)
- Data flow diagrams
- Security measures

**Use when:** Understanding system design, working with core modules, planning features

---

### ⚙️ [CONFIGURATION.md](./CONFIGURATION.md) - Configuration Guide
**257 lines | 8 KB**

Configuration and environment setup:
- Database configuration (PostgreSQL & SQLite)
- Redis caching setup
- Application settings
- External services (TTS, STT)
- Security settings
- Configuration precedence
- Development vs production examples
- Quick start guides
- Troubleshooting

**Use when:** Configuring S.A.M., setting up development environment

---

### 🚀 [DEPLOYMENT.md](./DEPLOYMENT.md) - Deployment Guide
**639 lines | 16 KB**

Complete deployment instructions:
- Quick start (Docker)
- Docker deployment (Dockerfile, docker-compose)
- Traditional server deployment (8-step guide)
  - System requirements
  - Dependency installation
  - Source build
  - Systemd service
  - PostgreSQL setup
  - Nginx reverse proxy
  - SSL/TLS certificates
- CapRover deployment
- Database setup
- Monitoring & maintenance
- Troubleshooting (service startup, DB connection, memory, performance, WebSocket)

**Use when:** Deploying to production, setting up new environment, troubleshooting deployment issues

---

### 🔐 [ENVIRONMENT_VARIABLES.md](./ENVIRONMENT_VARIABLES.md) - Variable Reference
**543 lines | 16 KB**

Complete environment variable reference:
- Quick reference table
- Database configuration (PostgreSQL, SQLite)
- Redis cache configuration
- Logging configuration
- Authentication settings
- Voice services (STT/TTS)
- External integrations
- Monitoring and Sentry
- Development setup examples
- Production setup examples
- Configuration loading order
- Validation procedures

**Use when:** Configuring environment, understanding variables, troubleshooting config issues

---

## Deployment Documentation

### 📋 [deployment/DEPLOYMENT_CHECKLIST.md](./deployment/DEPLOYMENT_CHECKLIST.md)
**551 lines | 12 KB**

Pre-deployment and deployment checklist:
- Pre-deployment checks (system, dependencies, credentials)
- Environment setup (database, Redis, secrets)
- Application configuration
- Security configuration
- Service startup procedures
- Verification steps
- Post-deployment validation

**Use when:** Preparing for deployment, verifying setup complete

---

### 🐳 [deployment/CAPROVER_DEPLOYMENT.md](./deployment/CAPROVER_DEPLOYMENT.md)
**4,678 bytes | 8 KB**

CapRover-specific deployment guide:
- CapRover setup prerequisites
- Application deployment
- Environment configuration
- Database setup for CapRover
- SSL certificate configuration
- Monitoring setup

**Use when:** Deploying to CapRover platform

---

### 🔑 [deployment/CAPROVER_ENV_SETUP.md](./deployment/CAPROVER_ENV_SETUP.md)
**4,868 bytes | 8 KB**

CapRover environment configuration:
- Required environment variables
- Database credentials
- Redis configuration
- Service URLs
- Security settings

**Use when:** Configuring CapRover environment

---

### 🗄️ [deployment/DATABASE_SETUP.md](./deployment/DATABASE_SETUP.md)
**4,699 bytes | 8 KB**

Database initialization and management:
- PostgreSQL setup
- User and role creation
- Schema initialization
- Backup procedures
- Recovery procedures
- Performance tuning

**Use when:** Setting up database, backing up, or recovering database

---

## Security Documentation

### 🛡️ [security/SECURITY_GUIDE.md](./security/SECURITY_GUIDE.md)

Security best practices and hardening:
- Authentication security
- Input validation
- Session management
- API security
- WebSocket security
- SQL injection prevention

**Use when:** Implementing security features, hardening deployment

---

### 🔒 [SECURITY_AUDIT.md](./SECURITY_AUDIT.md)

Security audit findings and fixes:
- Vulnerability assessment
- Fixed issues
- Security improvements
- Compliance status

**Use when:** Understanding security posture, reviewing audit findings

---

## Development Documentation

### 📝 [development/MIGRATION_SYSTEM.md](./development/MIGRATION_SYSTEM.md)

Database migration system:
- Migration tools
- Creating migrations
- Running migrations
- Rollback procedures

**Use when:** Working with database schema changes

---

## Feature Documentation

### 🎙️ [SNAPCAST_API.md](./SNAPCAST_API.md)

Snapcast integration:
- Audio streaming
- Device management
- Group control

---

### 💡 [LIFX_TOUCH_CONTROLS.md](./LIFX_TOUCH_CONTROLS.md)

LIFX light integration:
- Light control
- Touch interface integration

---

### 🎵 [LIBRESPOT_INTEGRATION.md](./LIBRESPOT_INTEGRATION.md)

Spotify/LibreSpot integration:
- Music streaming
- Playlist management

---

## Additional References

### 📖 [overview.md](./overview.md)
High-level project overview and features

### 🎯 [project_description.md](./project_description.md)
Detailed project description and capabilities

### 📐 [design.md](./design.md)
System design details and architecture decisions

### 📋 [CHANGELOG.md](./CHANGELOG.md)
Version history and changes

### 🐛 [bugs.md](./bugs.md)
Known bugs and issues

### ✅ [todo.md](./todo.md)
Future work and planned features

---

## Documentation by Use Case

### 🚀 Getting Started
1. Start with [overview.md](./overview.md)
2. Read [ARCHITECTURE.md](./ARCHITECTURE.md)
3. Follow [DEPLOYMENT.md](./DEPLOYMENT.md)
4. Configure with [CONFIGURATION.md](./CONFIGURATION.md)

### 🔧 Development
1. Review [ARCHITECTURE.md](./ARCHITECTURE.md)
2. Reference [API.md](./API.md)
3. Check [development/MIGRATION_SYSTEM.md](./development/MIGRATION_SYSTEM.md)
4. Follow [SECURITY_GUIDE.md](./security/SECURITY_GUIDE.md)

### 📦 Deployment
1. Read [DEPLOYMENT.md](./DEPLOYMENT.md)
2. Use [DEPLOYMENT_CHECKLIST.md](./deployment/DEPLOYMENT_CHECKLIST.md)
3. Configure [ENVIRONMENT_VARIABLES.md](./ENVIRONMENT_VARIABLES.md)
4. Setup database [deployment/DATABASE_SETUP.md](./deployment/DATABASE_SETUP.md)

### 🔐 Security
1. Review [SECURITY_GUIDE.md](./security/SECURITY_GUIDE.md)
2. Check [SECURITY_AUDIT.md](./SECURITY_AUDIT.md)
3. Implement [security/WEBSOCKET_SECURITY.md](./security/WEBSOCKET_SECURITY.md)

### 🎛️ Operations
1. Use [DEPLOYMENT_CHECKLIST.md](./deployment/DEPLOYMENT_CHECKLIST.md)
2. Monitor with [DEPLOYMENT.md](./DEPLOYMENT.md#monitoring--maintenance)
3. Troubleshoot [DEPLOYMENT.md](./DEPLOYMENT.md#troubleshooting)
4. Backup [deployment/DATABASE_SETUP.md](./deployment/DATABASE_SETUP.md)

### 🔌 API Integration
1. Read [API.md](./API.md)
2. Review authentication section
3. Study endpoint examples
4. Check error handling

---

## Documentation Statistics

| Category | Count | Lines |
|----------|-------|-------|
| Core Docs | 5 | 3,050 |
| Deployment | 4 | 1,500+ |
| Security | 5 | 2,000+ |
| Feature | 5 | 1,500+ |
| Development | 1+ | 500+ |
| **Total** | **20+** | **8,500+** |

---

## Key Documentation Files (by Importance)

### Critical (Must Have)
1. [DEPLOYMENT.md](./DEPLOYMENT.md) - Production deployment
2. [ENVIRONMENT_VARIABLES.md](./ENVIRONMENT_VARIABLES.md) - Configuration
3. [API.md](./API.md) - API reference
4. [ARCHITECTURE.md](./ARCHITECTURE.md) - System design

### Important (Should Have)
1. [CONFIGURATION.md](./CONFIGURATION.md) - Setup guide
2. [DEPLOYMENT_CHECKLIST.md](./deployment/DEPLOYMENT_CHECKLIST.md) - Pre-deployment
3. [SECURITY_GUIDE.md](./security/SECURITY_GUIDE.md) - Security practices

### Reference (Nice to Have)
1. [overview.md](./overview.md) - Project overview
2. [design.md](./design.md) - Design decisions
3. [CHANGELOG.md](./CHANGELOG.md) - Version history

---

## Quick Links

- **GitHub:** https://github.com/opensam/sam
- **Issues:** https://github.com/opensam/sam/issues
- **Discussions:** https://github.com/opensam/sam/discussions
- **License:** See LICENSE.md

---

## Feedback

- Found documentation unclear? Open an issue on GitHub
- Have improvements? Submit a PR with documentation updates
- Missing something? Request documentation in Discussions

---

**Last Updated:** April 2, 2026  
**Documentation Version:** 2.0 (Complete Production Release)
