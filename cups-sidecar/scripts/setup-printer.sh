#!/bin/bash
set -e

PRINTER_NAME="${PRINTER_NAME:-Canon_LBP221}"
PRINTER_URI="${PRINTER_URI:-socket://192.168.1.100:9100}"
PRINTER_DRIVER="${PRINTER_DRIVER:-CNRCUPSLBP221ZK.ppd}"

echo "Setting up printer: $PRINTER_NAME"
echo "Printer URI: $PRINTER_URI"
echo "Printer Driver: $PRINTER_DRIVER"

# Wait for CUPS to be ready
MAX_WAIT=30
WAIT_COUNT=0
while ! lpstat -r 2>/dev/null | grep -q "scheduler is running"; do
    if [ $WAIT_COUNT -ge $MAX_WAIT ]; then
        echo "Error: CUPS daemon did not start within ${MAX_WAIT} seconds"
        exit 1
    fi
    echo "Waiting for CUPS daemon to start... ($WAIT_COUNT/$MAX_WAIT)"
    sleep 1
    WAIT_COUNT=$((WAIT_COUNT + 1))
done

echo "CUPS daemon is running."

# Check if printer already exists
if lpstat -p "$PRINTER_NAME" 2>/dev/null; then
    echo "Printer $PRINTER_NAME already exists."
else
    echo "Adding printer $PRINTER_NAME..."

    # Try to find Canon driver PPD
    PPD_FILE=""
    if [ -f "/usr/share/cups/model/$PRINTER_DRIVER" ]; then
        PPD_FILE="/usr/share/cups/model/$PRINTER_DRIVER"
    elif [ -f "/usr/share/ppd/cnrdrvcups-ufr2/$PRINTER_DRIVER" ]; then
        PPD_FILE="/usr/share/ppd/cnrdrvcups-ufr2/$PRINTER_DRIVER"
    fi

    if [ -n "$PPD_FILE" ]; then
        echo "Using PPD file: $PPD_FILE"
        lpadmin -p "$PRINTER_NAME" \
            -E \
            -v "$PRINTER_URI" \
            -P "$PPD_FILE" \
            -o printer-is-shared=true
    else
        # Fallback: Use generic driver
        echo "Canon driver not found, using generic driver..."
        lpadmin -p "$PRINTER_NAME" \
            -E \
            -v "$PRINTER_URI" \
            -m drv:///sample.drv/generic.ppd \
            -o printer-is-shared=true
    fi
fi

# Set as default printer
echo "Setting $PRINTER_NAME as default printer..."
lpadmin -d "$PRINTER_NAME"

# Enable the printer
echo "Enabling printer..."
cupsenable "$PRINTER_NAME"
cupsaccept "$PRINTER_NAME"

# Verify printer setup
echo ""
echo "Printer setup complete. Current printers:"
lpstat -p -d

echo ""
echo "Printer $PRINTER_NAME is ready to accept jobs."
