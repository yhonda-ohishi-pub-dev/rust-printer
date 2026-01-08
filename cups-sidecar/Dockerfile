FROM debian:bookworm-slim

LABEL maintainer="rust-printer"
LABEL description="CUPS print server with Canon UFRII drivers"

ENV DEBIAN_FRONTEND=noninteractive

# Install CUPS and dependencies
RUN apt-get update && apt-get install -y \
    cups \
    cups-client \
    cups-filters \
    cups-pdf \
    ghostscript \
    wget \
    libcups2 \
    libcupsimage2 \
    avahi-daemon \
    libnss-mdns \
    && rm -rf /var/lib/apt/lists/*

# Download and install Canon UFRII driver
# Note: You may need to update this URL for newer driver versions
WORKDIR /tmp
RUN wget -q --no-check-certificate \
    "https://gdlp01.c-wss.com/gds/0/0300029840/14/linux-UFRII-drv-v580-uken-18.tar.gz" \
    -O cndrvcups-ufr2.tar.gz || \
    echo "Warning: Could not download Canon driver. You may need to install it manually."

# Extract and install driver (if downloaded successfully)
RUN if [ -f cndrvcups-ufr2.tar.gz ] && [ -s cndrvcups-ufr2.tar.gz ]; then \
        tar xzf cndrvcups-ufr2.tar.gz && \
        cd linux-UFRII-drv-* && \
        dpkg -i packages/cndrvcups-common_*.deb packages/cndrvcups-ufr2-uk_*.deb || \
        apt-get -f install -y; \
    fi && \
    rm -rf /tmp/*

# Copy CUPS configuration
COPY cupsd.conf /etc/cups/cupsd.conf

# Copy scripts
COPY scripts/entrypoint.sh /entrypoint.sh
COPY scripts/setup-printer.sh /setup-printer.sh
RUN chmod +x /entrypoint.sh /setup-printer.sh

# Create required directories
RUN mkdir -p /var/run/cups /var/spool/cups /var/cache/cups

# Expose CUPS port
EXPOSE 631

ENTRYPOINT ["/entrypoint.sh"]
CMD ["cupsd", "-f"]
