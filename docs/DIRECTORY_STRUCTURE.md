# S.A.M. Directory Structure

This document describes the organized directory structure of the S.A.M. (Smart Artificial Mind) project.

## Root Directory

```
/
├── .cargo/                    # Cargo configuration
├── .git/                     # Git repository data
├── .gitattributes            # Git attributes configuration
├── .gitignore               # Git ignore patterns
├── .gitmodules              # Git submodules configuration
├── .vscode/                 # VSCode workspace settings
├── .dockerignore            # Docker ignore patterns
├── AGENTS.md                # Codex/agent project instructions
├── benches/                 # Rust benchmarks
├── build.rs                 # Rust build script
├── captain-definition       # CapRover deployment configuration
├── Cargo.lock              # Dependency lock file
├── Cargo.toml              # Rust project configuration
├── CLAUDE.md               # Claude project instructions
├── cfg/                    # Application configuration files
├── config/                 # Build and development configuration
├── data/                   # Data files, models, and assets
├── deploy/                 # Deployment configurations and scripts
├── docs/                   # All project documentation
├── examples/               # Example code and usage
├── fonts/                  # Font files
├── LICENSE.md             # Project license
├── packages/              # Package definitions and scripts
├── README.md              # Main project README
├── scripts/               # Utility and deployment scripts
├── src/                   # Rust source code
├── target/                # Rust build artifacts (generated)
├── tests/                 # All test files and utilities
├── tools/                 # Development tools and utilities
└── www/                   # Web frontend assets
```

Generated directories such as `target/`, `logs/`, `www/node_modules/`, and
`www/assets/dist/` are intentionally ignored and should be rebuilt locally.

## Detailed Directory Descriptions

### `/config/` - Configuration Files
Configuration files for development, testing, and CI/CD:
```
config/
├── .env.example           # Environment variables template
├── TaskTemplate.yml       # Task template configuration
└── tarpaulin.toml        # Code coverage configuration
```

### `/deploy/` - Deployment Configurations
All deployment-related configurations and Docker files:
```
deploy/
├── docker-compose.yml          # Full PostgreSQL + Redis setup
├── docker-compose.sqlite.yml   # SQLite setup (lightweight)
├── docker-compose.caprover.yml # CapRover testing configuration
├── Dockerfile                  # Production Docker image
├── Dockerfile-GPU             # GPU-enabled Docker image
└── docker-entrypoint.sh       # Docker container entrypoint script
```

### `/docs/` - Documentation
Organized documentation by category:
```
docs/
├── api/
│   └── API_DOCUMENTATION.md   # API endpoint documentation
├── deployment/
│   ├── CAPROVER_DEPLOYMENT.md # CapRover deployment guide
│   └── DATABASE_SETUP.md      # Database setup instructions
├── development/
│   ├── coding-agent/          # Coding agent design/refactor notes
│   ├── refactoring/           # Historical refactor plans and summaries
│   ├── reports/               # Generated analysis reports retained for reference
│   ├── test-notes/            # Test notes that are not executable tests
│   ├── CACHE_IMPLEMENTATION.md
│   ├── CRAWLER_ENHANCEMENTS.md
│   ├── MIGRATION_SYSTEM.md
│   ├── REDIS_REFACTORING_SUMMARY.md
│   ├── RESOURCE_MANAGEMENT_IMPLEMENTATION.md
│   ├── RESTART_IMPLEMENTATION.md
│   ├── THREAD_*.md
│   └── WEBSOCKET_SECURITY.md
├── features/
│   ├── COMPREHENSIVE_ENHANCEMENT_REPORT.md
│   ├── FEATURE_*.md
│   └── Related feature documentation
├── security/
│   ├── SECURITY.md            # Main security documentation
│   ├── SECURITY_*.md          # Security implementation guides
│   ├── SQL_INJECTION_FIX_SUMMARY.md
│   └── WEBSOCKET_SECURITY.md
├── CHANGELOG.md               # Version history and changes
├── CLAUDE.md                  # Claude AI assistant documentation
├── overview.md                # Project overview
├── project_description.md     # Detailed project description
└── todo.md                   # Project TODO list
```

### `/scripts/` - Utility Scripts
Deployment and development scripts:
```
scripts/
├── deploy-caprover.sh         # Automated CapRover deployment
├── darknet/                   # AI/ML model scripts
├── rivescript/               # Conversation engine scripts
└── Other utility scripts
```

### `/tests/` - Test Files and Utilities
All testing-related files:
```
tests/
├── integration/              # Integration tests
├── fixtures/                 # Test data and fixtures
├── helpers/                  # Test helper functions
├── *.rs                     # Individual test files
├── run_tests.sh             # Test runner script
├── test_*.sh                # Specific test scripts
├── test_*.rs                # Rust test files
├── test_runner              # Compiled test runner
└── verify_sql_fixes.sh      # SQL injection test verification
```

Markdown-only test notes live under `docs/development/test-notes/`.

### `/tools/` - Development Tools
Development utilities and helper tools:
```
tools/
├── *.sh                     # Shell utility scripts
├── *.deb                    # Debian packages
├── *.json                   # Configuration and data files
├── *.jpg                    # Assets and test images
├── setup_env.sh            # Environment setup
├── env-gpu.sh              # GPU environment setup
└── release-armv7.sh        # ARM release script
```

## File Organization Principles

### 1. **Separation of Concerns**
- **Source code** (`src/`) - Only Rust source files
- **Documentation** (`docs/`) - All markdown documentation with subcategories
- **Configuration** (`config/`) - Development and build configurations
- **Deployment** (`deploy/`) - Docker and deployment configurations
- **Tests** (`tests/`) - All test-related files and utilities

### 2. **Categorized Documentation**
Documentation is organized by purpose:
- **API** - Endpoint and interface documentation
- **Deployment** - Setup and deployment guides
- **Development** - Implementation details and technical guides
- **Features** - Feature descriptions and enhancement reports
- **Security** - Security guides, fixes, and best practices

### 3. **Tool Separation**
- **Scripts** (`scripts/`) - Application-specific scripts and utilities
- **Tools** (`tools/`) - Development tools and system utilities
- **Tests** (`tests/`) - Testing infrastructure and test files

### 4. **Clean Root Directory**
The root directory contains only essential files:
- Core Rust files (`Cargo.toml`, `Cargo.lock`, `build.rs`)
- Main documentation (`README.md`, `LICENSE.md`)
- Docker configuration (`.dockerignore`, `captain-definition`)
- Git configuration (`.git*`)

## Migration Notes

When files were reorganized:
1. All `*.md` files moved to appropriate `docs/` subcategories
2. Docker and deployment files moved to `deploy/`
3. Configuration files moved to `config/`
4. Test files consolidated in `tests/`
5. Utility scripts organized in `scripts/` and `tools/`
6. References updated in documentation and configuration files

## Benefits of This Structure

1. **Easy Navigation** - Related files are grouped together
2. **Clear Purpose** - Each directory has a single, clear purpose
3. **Scalable** - Easy to add new files in appropriate locations
4. **Standard** - Follows Rust and modern project conventions
5. **Maintainable** - Reduces confusion and improves maintainability

## Finding Files

Use these patterns to locate files:

- **Documentation**: Always in `docs/` with appropriate subcategory
- **Configuration**: Check `config/` first, then `cfg/` for app configs
- **Deployment**: All in `deploy/` directory
- **Tests**: All in `tests/` directory
- **Scripts**: Application scripts in `scripts/`, dev tools in `tools/`

---

**Last Updated:** September 2025  
**S.A.M. Version:** 0.0.5
