# Snapcast Media Control API

## Overview

RESTful API for controlling Snapcast multi-room audio playback in the Sam AI assistant. This API provides complete control over media playback, volume, and client management.

## Base URL

```
/api/services/media/snapcast
```

## Endpoints

### 1. Get Server Status

**GET** `/api/services/media/snapcast/status`

Returns the current status of the Snapcast server.

**Response:**
```json
{
  "running": true,
  "message": "Snapcast server is active",
  "info": "{...JSON-RPC response...}"
}
```

or if not running:
```json
{
  "running": false,
  "message": "Snapcast server is not running"
}
```

---

### 2. Get Connected Clients

**GET** `/api/services/media/snapcast/clients`

Returns a list of all connected Snapcast clients with their status and volume levels.

**Response:**
```json
[
  {
    "name": "Kitchen Speaker",
    "connected": true,
    "volume": {
      "percent": 75,
      "muted": false
    }
  },
  {
    "name": "Living Room",
    "connected": true,
    "volume": {
      "percent": 50,
      "muted": false
    }
  }
]
```

---

### 3. Play/Resume Playback

**POST** `/api/services/media/snapcast/play`

Starts or resumes media playback.

**Request:** No body required

**Response:**
```json
{
  "success": true,
  "message": "Playback started"
}
```

---

### 4. Pause Playback

**POST** `/api/services/media/snapcast/pause`

Pauses media playback.

**Request:** No body required

**Response:**
```json
{
  "success": true,
  "message": "Playback paused"
}
```

---

### 5. Set Volume

**POST** `/api/services/media/snapcast/volume`

Sets the volume level for the stream or a specific client.

**Request Body:**
```json
{
  "level": 75,
  "client_id": "optional-client-id"
}
```

**Parameters:**
- `level` (required): Volume level from 0-100
- `client_id` (optional): Specific client ID to control. If omitted, controls global stream volume.

**Response:**
```json
{
  "success": true,
  "message": "Volume set to 75%",
  "level": 75
}
```

---

### 6. Toggle Mute

**POST** `/api/services/media/snapcast/mute`

Toggles the mute state of the media stream.

**Request:** No body required

**Response:**
```json
{
  "success": true,
  "message": "Muted",
  "muted": true
}
```

or when unmuting:
```json
{
  "success": true,
  "message": "Unmuted",
  "muted": false
}
```

---

### 7. Next Track

**POST** `/api/services/media/snapcast/next`

Skips to the next track (source-dependent).

**Request:** No body required

**Response:**
```json
{
  "success": true,
  "message": "Next track command sent (source-dependent)",
  "note": "Track navigation depends on the active media source (Spotify, pipe, etc.)"
}
```

**Note:** Track navigation functionality depends on the active media source. For Spotify (librespot), this requires additional integration with the Spotify API.

---

### 8. Previous Track

**POST** `/api/services/media/snapcast/previous`

Goes to the previous track (source-dependent).

**Request:** No body required

**Response:**
```json
{
  "success": true,
  "message": "Previous track command sent (source-dependent)",
  "note": "Track navigation depends on the active media source (Spotify, pipe, etc.)"
}
```

---

## Usage Examples

### JavaScript (Frontend)

```javascript
// Play music
fetch('/api/services/media/snapcast/play', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' }
})
.then(response => response.json())
.then(data => console.log(data));

// Set volume to 50%
fetch('/api/services/media/snapcast/volume', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ level: 50 })
});

// Get connected clients
fetch('/api/services/media/snapcast/clients')
    .then(response => response.json())
    .then(clients => {
        clients.forEach(client => {
            console.log(`${client.name}: ${client.volume.percent}%`);
        });
    });

// Toggle mute
fetch('/api/services/media/snapcast/mute', {
    method: 'POST'
}).then(r => r.json()).then(data => {
    console.log(`Mute state: ${data.muted}`);
});
```

### cURL (Command Line)

```bash
# Check server status
curl http://localhost:1780/api/services/media/snapcast/status

# Play music
curl -X POST http://localhost:1780/api/services/media/snapcast/play

# Set volume to 75%
curl -X POST http://localhost:1780/api/services/media/snapcast/volume \
  -H "Content-Type: application/json" \
  -d '{"level": 75}'

# Get connected clients
curl http://localhost:1780/api/services/media/snapcast/clients

# Toggle mute
curl -X POST http://localhost:1780/api/services/media/snapcast/mute
```

---

## Integration with Frontend

The API is automatically integrated with the media center frontend (`www/assets/js/media.js`). The following UI elements are supported:

- **Play/Pause Button**: Toggles playback state
- **Volume Slider**: Controls volume level (0-100%)
- **Mute Button**: Toggles mute state
- **Track Navigation**: Next/Previous buttons (source-dependent)
- **Keyboard Shortcuts**:
  - `Space`: Play/Pause
  - `Arrow Up`: Increase volume
  - `Arrow Down`: Decrease volume
  - `Arrow Right`: Next track
  - `Arrow Left`: Previous track
  - `M`: Toggle mute

- **Touch Gestures** (on touch-enabled devices):
  - Swipe up/down: Volume control
  - Swipe left/right: Track navigation
  - Double-tap: Play/Pause

---

## Error Handling

All endpoints return consistent error responses:

```json
{
  "success": false,
  "message": "Error description",
  "error": "Detailed error message (if available)"
}
```

Common errors:
- **Server not running**: Snapcast server process is not active
- **Connection timeout**: Cannot reach Snapcast JSON-RPC interface
- **Invalid parameters**: Missing or invalid request parameters

The frontend gracefully degrades to local state management if the API is unavailable.

---

## Security Considerations

- API endpoints do not require authentication for local network access
- For production deployments, consider adding authentication middleware
- Snapcast JSON-RPC interface should be bound to localhost only (default: 127.0.0.1:1780)
- Use environment variables `SNAPCAST_BIND_ADDRESS` to configure binding

---

## Dependencies

- **Snapcast Server**: Must be installed and running
- **curl**: Used internally to communicate with Snapcast JSON-RPC
- **Serde JSON**: Rust JSON serialization/deserialization

---

## Testing

Test the API endpoints using the built-in test suite:

```bash
cd ~/Projects/Sam
cargo test --package sam --lib services::media::snapcast_api
```

Manual testing:
1. Ensure Snapcast server is running: `pgrep snapserver`
2. Test status endpoint: `curl http://localhost:1780/api/services/media/snapcast/status`
3. Verify clients: `curl http://localhost:1780/api/services/media/snapcast/clients`

---

## Troubleshooting

### API returns 404
- Ensure the media service is properly registered in the HTTP router
- Check that the request path includes `/api/services/media/snapcast`

### Server shows as not running
- Verify Snapcast is installed: `which snapserver`
- Start the server: `snapserver` or `service snapserver start`
- Check logs: `/var/log/snapserver.log`

### Clients not appearing
- Ensure clients are connected to the same network
- Check Snapcast configuration: `/etc/snapserver.conf`
- Verify client applications are running (e.g., snapclient)

### Volume control not working
- Check that the JSON-RPC interface is enabled in snapserver.conf
- Verify port 1780 is accessible: `netstat -tlnp | grep 1780`

---

## Future Enhancements

Potential improvements for future versions:

- [ ] Authentication/authorization for remote access
- [ ] Group/zone control for multi-room synchronization
- [ ] Playlist management
- [ ] Queue manipulation
- [ ] Source switching (Spotify, pipe, etc.)
- [ ] Real-time metadata display (track info, album art)
- [ ] WebSocket support for real-time updates
- [ ] Client-specific volume groups

---

## License

GPLv3 - See LICENSE file for details

## Author

Developed by Caleb Mitchell Smith for the Open Sam project
© 2021-2026 The Open Sam Foundation (OSF)
