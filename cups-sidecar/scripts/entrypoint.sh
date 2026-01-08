#!/bin/bash
set -e

echo "Starting CUPS sidecar container..."

# Create required directories
mkdir -p /var/run/cups
mkdir -p /var/spool/cups
mkdir -p /var/cache/cups

# Set permissions
chown -R root:lp /var/run/cups
chmod 755 /var/run/cups

# Start Avahi daemon for printer discovery (optional)
if command -v avahi-daemon &> /dev/null; then
    echo "Starting Avahi daemon..."
    avahi-daemon --daemonize --no-drop-root 2>/dev/null || true
fi

# Wait a moment for services to start
sleep 2

# Setup printer if environment variables are set
if [ -n "$PRINTER_NAME" ] && [ -n "$PRINTER_URI" ]; then
    echo "Setting up printer: $PRINTER_NAME at $PRINTER_URI"
    /setup-printer.sh &
fi

# Execute the main command
echo "Starting CUPS daemon..."
exec "$@"
