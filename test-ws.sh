#!/bin/bash

echo "Testing WebSocket connection to SOLUSync-X server..."

# Generate a UUID v4
NODE_ID=$(uuidgen | tr '[:upper:]' '[:lower:]')
MSG_ID=$(uuidgen | tr '[:upper:]' '[:lower:]')

# Create hello message with proper UUID format
HELLO_MSG=$(cat <<EOF
{
  "type": "hello",
  "header": {
    "id": "$MSG_ID",
    "timestamp": $(date +%s.%3N),
    "node_id": "$NODE_ID",
    "sequence": 0
  },
  "protocol_version": "0.1.0",
  "capabilities": ["audio", "clock_sync"],
  "node_type": "Client"
}
EOF
)

echo "Sending hello message:"
echo "$HELLO_MSG" | jq .

# Use websocat if available, otherwise use curl
if command -v websocat &> /dev/null; then
    echo "$HELLO_MSG" | websocat ws://localhost:8080/ws
elif command -v wscat &> /dev/null; then
    echo "$HELLO_MSG" | wscat -c ws://localhost:8080/ws
else
    echo "Installing websocat for WebSocket testing..."
    brew install websocat 2>/dev/null || echo "Please install websocat or wscat for WebSocket testing"
fi