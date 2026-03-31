# Librespot Integration for Spotify Support

## Overview

This document describes the integration of [librespot](https://github.com/librespot-org/librespot) with Snapcast for Spotify streaming support in the Sam AI assistant.

## What is Librespot?

Librespot is an open-source client library for Spotify that provides access to Spotify's streaming service. It acts as a Spotify Connect device, allowing Sam to stream Spotify music through Snapcast's multi-room audio system.

## Architecture

```
┌─────────────┐     ┌──────────────┐     ┌─────────────┐     ┌──────────────┐
│   Spotify   │────▶│  Librespot   │────▶│  Snapcast   │────▶│  Clients     │
│   Service   │     │   (Source)   │     │   Server    │     │ (Speakers)   │
└─────────────┘     └──────────────┘     └─────────────┘     └──────────────┘
```

## Installation

### Step 1: Install Librespot

The recommended method is via Cargo (Rust package manager):

```bash
cargo install librespot
```

This will compile and install librespot to `~/.cargo/bin/librespot`.

### Step 2: Copy to System Path

For Snapcast to find librespot, copy it to a system-wide location:

```bash
sudo cp ~/.cargo/bin/librespot /usr/local/bin/
```

Alternative locations:
- `/usr/bin/librespot`
- `/bin/librespot`

### Step 3: Verify Installation

```bash
librespot --version
which librespot
```

### Step 4: Configure Credentials

You have two options for configuring Spotify credentials:

#### Option A: Environment Variables (Recommended)

Add these to your shell profile (`~/.bashrc`, `~/.zshenv`, etc.) or systemd service:

```bash
export SNAPCAST_SPOTIFY_USERNAME="your_spotify_username"
export SNAPCAST_SPOTIFY_PASSWORD="your_spotify_password"
export SNAPCAST_SPOTIFY_DEVICE_NAME="Sam"
export LIBRESPOT_PATH="/usr/local/bin/librespot"
```

#### Option B: Configuration File

Edit `/etc/snapserver.conf` and update the stream section:

```ini
[stream]
source = librespot:///usr/local/bin/librespot?name=Sam&username=YOUR_USERNAME&password=YOUR_PASSWORD&devicename=Sam&bitrate=320&normalize=true
```

**⚠️ Security Warning:** Storing passwords in plain text configuration files is not recommended for production environments. Use environment variables instead.

### Step 5: Restart Snapcast

```bash
sudo service snapserver restart
```

Or if running manually:

```bash
sudo systemctl restart snapserver
```

## Configuration Options

### Librespot Backend Options

Librespot supports multiple audio backends. The default is usually appropriate for Snapcast:

```bash
librespot --backend alsa --device hw:0,0
```

Available backends:
- `alsa` - Advanced Linux Sound Architecture (default on Linux)
- `pulseaudio` - PulseAudio sound server
- `jackaudio` - JACK Audio Connection Kit
- `rodio` - Pure Rust audio backend

### Bitrate Settings

Control the quality of Spotify streams:

```bash
librespot --bitrate 320  # High quality (default)
librespot --bitrate 160  # Medium quality
librespot --bitrate 96   # Low quality (saves bandwidth)
```

### Audio Normalization

Enable volume normalization for consistent playback:

```bash
librespot --enable-volume-normalization
```

## Snapcast Configuration

### Full Example `/etc/snapserver.conf`

```ini
[server]
threads = -1
pidfile = /var/run/snapserver/pid
user = snapserver
group = audio

[http]
enabled = true
bind_to_address = 127.0.0.1
port = 1780
doc_root = /usr/share/snapserver/snapweb

[tcp]
enabled = true
bind_to_address = 127.0.0.1
port = 1705

[stream]
bind_to_address = 127.0.0.1
port = 1704
source = librespot:///usr/local/bin/librespot?name=Sam&username=${SNAPCAST_SPOTIFY_USERNAME}&password=${SNAPCAST_SPOTIFY_PASSWORD}&devicename=Sam&bitrate=320&normalize=true
source = pipe:///tmp/snapfifo?name=samfifo&mode=0666

[logging]
loglevel = info
logfile = /var/log/snapserver.log
```

## Usage

### Via Sam API

Once configured, control Spotify playback through the Snapcast API endpoints:

```javascript
// Play music
fetch('/api/services/media/snapcast/play', { method: 'POST' });

// Pause music
fetch('/api/services/media/snapcast/pause', { method: 'POST' });

// Set volume
fetch('/api/services/media/snapcast/volume', {
    method: 'POST',
    body: JSON.stringify({ level: 75 })
});

// Toggle mute
fetch('/api/services/media/snapcast/mute', { method: 'POST' });
```

### Via Snapcast Web Interface

Access the Snapcast web UI at `http://localhost:1780` to:
- View connected clients
- Control individual zone volumes
- Monitor playback status

### Via Command Line

```bash
# Check status
curl http://localhost:1780/jsonrpc -d '{"id":1,"jsonrpc":"2.0","method":"Server.GetStatus"}'

# Get clients
curl http://localhost:1780/jsonrpc -d '{"id":1,"jsonrpc":"2.0","method":"Server.GetClients"}'

# Play
curl http://localhost:1780/jsonrpc -d '{"id":1,"jsonrpc":"2.0","method":"Stream.SetMute","params":{"mute":false}}'

# Pause
curl http://localhost:1780/jsonrpc -d '{"id":1,"jsonrpc":"2.0","method":"Stream.SetMute","params":{"mute":true}}'
```

## Troubleshooting

### Librespot Not Found

**Error:** `source = librespot:///...` shows as disabled

**Solution:**
1. Verify installation: `which librespot`
2. Check path in config matches actual location
3. Set `LIBRESPOT_PATH` environment variable
4. Restart Snapcast after changes

### Authentication Failures

**Error:** Librespot fails to connect to Spotify

**Possible causes:**
- Incorrect username/password
- Spotify Premium account required (free accounts may have limitations)
- Network connectivity issues

**Solution:**
1. Double-check credentials
2. Test librespot standalone: `librespot --username YOUR_USER --password YOUR_PASS`
3. Ensure firewall allows outbound connections to Spotify servers

### Audio Not Playing

**Error:** Snapcast shows connected but no audio

**Solution:**
1. Check librespot process is running: `pgrep librespot`
2. Verify audio backend: `aplay -l` to list available devices
3. Check Snapcast logs: `tail -f /var/log/snapserver.log`
4. Test pipe source to isolate the issue

### High CPU Usage

**Solution:**
1. Reduce bitrate: `--bitrate 160` or `--bitrate 96`
2. Use a more efficient audio backend
3. Check for other resource-intensive processes

## Security Considerations

### Credential Storage

**Never commit Spotify credentials to version control!**

Best practices:
- Use environment variables
- Restrict file permissions on config files: `chmod 640 /etc/snapserver.conf`
- Consider using a secrets manager in production

### Network Security

By default, Snapcast binds to localhost only. For remote access:

1. Update `bind_to_address` in config
2. Add authentication middleware
3. Use HTTPS/TLS for encrypted connections
4. Consider VPN or SSH tunneling for remote access

## API Reference

### Sam Media Service Functions

The following functions are available in `src/lib/services/media/snapcast.rs`:

```rust
// Check if librespot is installed
pub fn check_librespot() -> Result<String, String>

// Get installation instructions
pub fn get_installation_instructions() -> &'static str

// Configure Snapcast with security settings
pub fn configure()

// Initialize Snapcast server
pub fn init()
```

### Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `SNAPCAST_SPOTIFY_USERNAME` | Spotify account username | - |
| `SNAPCAST_SPOTIFY_PASSWORD` | Spotify account password | - |
| `SNAPCAST_SPOTIFY_DEVICE_NAME` | Device name shown in Spotify | "Sam" |
| `SNAPCAST_BIND_ADDRESS` | Network interface to bind | "127.0.0.1" |
| `LIBRESPOT_PATH` | Path to librespot binary | "/usr/local/bin/librespot" |

## Testing

### Manual Test Procedure

1. **Verify librespot installation:**
   ```bash
   librespot --version
   ```

2. **Test standalone librespot:**
   ```bash
   librespot --username YOUR_USER --password YOUR_PASS --backend alsa
   ```

3. **Start Snapcast server:**
   ```bash
   sudo service snapserver start
   ```

4. **Check server status:**
   ```bash
   curl http://localhost:1780/api/services/media/snapcast/status
   ```

5. **Verify clients:**
   ```bash
   curl http://localhost:1780/api/services/media/snapcast/clients
   ```

6. **Test playback control:**
   ```bash
   curl -X POST http://localhost:1780/api/services/media/snapcast/play
   ```

## Future Enhancements

Potential improvements for future versions:

- [ ] OAuth2 integration for secure Spotify authentication
- [ ] Playlist management via Spotify API
- [ ] Track metadata display in Sam UI
- [ ] Album art retrieval and display
- [ ] Spotify Connect device switching
- [ ] Multi-account support
- [ ] Voice command integration ("Play Spotify playlist X")

## Resources

- [Librespot GitHub](https://github.com/librespot-org/librespot)
- [Snapcast Documentation](https://github.com/badaix/snapcast)
- [Spotify Web API](https://developer.spotify.com/documentation/web-api/)
- [Rust Programming Language](https://www.rust-lang.org/)

## License

This integration is part of the Sam AI assistant project, licensed under GPLv3.

---

*Last Updated: 2026-03-31*
*Author: SAM-C (with assistance from Caleb Smith)*
