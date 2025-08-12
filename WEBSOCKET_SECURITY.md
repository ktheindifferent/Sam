# WebSocket Security Implementation

## Overview

This document outlines the comprehensive security features implemented for the WebSocket server in `src/sam/websocket/`. The implementation addresses critical vulnerabilities identified in the original code.

## Security Features Implemented

### 1. Message Validation and Size Limits ✅
- **Max Message Size**: 64KB default limit prevents DoS via large messages
- **JSON Structure Validation**: All messages must be valid JSON
- **Injection Detection**: Pattern matching to detect XSS/script injection attempts
- **Implementation**: `MessageValidator` in `src/sam/websocket/security.rs`

### 2. Rate Limiting ✅
- **Per-Client Limits**: 100 messages per minute (configurable)
- **Exponential Backoff**: Increasing penalties for repeat violations
- **Window-Based Tracking**: Sliding window algorithm for accurate rate limiting
- **Implementation**: `WsRateLimiter` with automatic bucket cleanup

### 3. Connection Limits per IP ✅
- **Max Connections**: 5 concurrent connections per IP address (configurable)
- **IP Tracking**: Automatic tracking and cleanup of stale connections
- **Activity Monitoring**: Updates last activity timestamp on each message
- **Implementation**: `ConnectionTracker` with IP-based connection management

### 4. Session Re-authentication ✅
- **Session Timeout**: 1 hour default (configurable)
- **Re-authentication Support**: `Authenticate` message type for token refresh
- **Permission System**: Role-based permissions for commands
- **Implementation**: `SessionManager` with token validation hooks

### 5. Connection Health Monitoring ✅
- **Heartbeat/Ping-Pong**: Built-in WebSocket ping/pong mechanism
- **Custom Heartbeat**: Application-level heartbeat messages
- **Idle Detection**: 5-minute idle timeout (configurable)
- **Automatic Cleanup**: Background task removes inactive clients

### 6. Message Queue with Backpressure ✅
- **Queue Size Limit**: 1000 messages per client default
- **Priority System**: Critical > High > Normal > Low priority ordering
- **Backpressure**: Returns error when queue is full
- **Implementation**: `MessageQueue` with priority-based dequeuing

### 7. Audit Logging ✅
- **Comprehensive Events**: All security-relevant events logged
- **Event Types**:
  - Connection established/rejected/closed
  - Authentication success/failure
  - Rate limit violations
  - Invalid messages
  - Command execution
- **Severity Levels**: Info, Warning, Error, Critical
- **Output**: Console logging + dedicated audit log file

## Configuration

```rust
WebSocketSecurityConfig {
    max_message_size: 64 * 1024,           // 64KB
    max_messages_per_minute: 100,          // Rate limit
    max_connections_per_ip: 5,             // Connection limit
    session_timeout_seconds: 3600,         // 1 hour
    idle_timeout_seconds: 300,             // 5 minutes
    enable_message_validation: true,
    enable_rate_limiting: true,
    enable_connection_limits: true,
    enable_session_validation: true,
    message_queue_size: 1000,
}
```

## Usage Example

```rust
// Create server with custom security config
let config = WebSocketSecurityConfig {
    max_messages_per_minute: 50,
    max_connections_per_ip: 3,
    ..Default::default()
};

let server = WsServer::with_config(config);
server.start("0.0.0.0:8080").await?;
```

## Message Flow

1. **Connection Request** → IP limit check → Session creation → Connection accepted
2. **Incoming Message** → Rate limit check → Size validation → Content validation → Session check → Process
3. **Command Execution** → Permission check → Audit log → Execute → Response
4. **Disconnection** → Cleanup connections → Remove session → Clear queue → Audit log

## Security Considerations

### Prevented Attacks
- **DoS via Large Messages**: Size limits prevent memory exhaustion
- **Spam/Flood Attacks**: Rate limiting with exponential backoff
- **Connection Exhaustion**: Per-IP connection limits
- **Session Hijacking**: Session expiry and re-authentication
- **XSS/Injection**: Pattern-based content validation
- **Unauthorized Actions**: Permission-based command validation

### Monitoring
- Real-time audit logging for security events
- Connection tracking with activity timestamps
- Rate limit violation tracking with escalating penalties
- Idle connection detection and automatic cleanup

## Testing

Run the security tests:
```bash
cargo test --test websocket_security_test
```

Test coverage includes:
- Rate limiting behavior
- Message validation (size, format, injection)
- Connection tracking and limits
- Session management lifecycle
- Message queue with priorities
- Integration testing of all components

## Future Enhancements

Potential improvements for consideration:
- [ ] Distributed rate limiting via Redis
- [ ] JWT token validation for authentication
- [ ] WebSocket compression support
- [ ] Metrics collection (Prometheus)
- [ ] DDoS protection at network level
- [ ] Client certificate authentication
- [ ] Message encryption for sensitive data

## Files Modified

- `src/sam/websocket/mod.rs` - Main WebSocket server integration
- `src/sam/websocket/security.rs` - Security implementation module (new)
- `tests/websocket_security_test.rs` - Security test suite (new)

## Compliance

This implementation helps meet common security requirements:
- OWASP WebSocket Security Guidelines
- Rate limiting for API protection
- Session management best practices
- Audit logging for compliance
- Input validation and sanitization