#!/bin/bash
# Example systemd user timer to refresh trotd cache periodically
# This ensures the cache is always fresh when you open a terminal

# Create the service file at: ~/.config/systemd/user/trotd-refresh.service
cat > ~/.config/systemd/user/trotd-refresh.service <<'EOF'
[Unit]
Description=Refresh trotd cache
After=network.target

[Service]
Type=oneshot
ExecStart=/usr/bin/env bash -c 'trotd --no-cache > /dev/null 2>&1'

[Install]
WantedBy=default.target
EOF

# Create the timer file at: ~/.config/systemd/user/trotd-refresh.timer
cat > ~/.config/systemd/user/trotd-refresh.timer <<'EOF'
[Unit]
Description=Refresh trotd cache every hour

[Timer]
OnBootSec=5min
OnUnitActiveSec=1h
Persistent=true

[Install]
WantedBy=timers.target
EOF

# Reload systemd and enable the timer
systemctl --user daemon-reload
systemctl --user enable trotd-refresh.timer
systemctl --user start trotd-refresh.timer

echo "✅ Systemd timer created and enabled!"
echo "The cache will be refreshed every hour."
echo ""
echo "Useful commands:"
echo "  systemctl --user status trotd-refresh.timer  - Check timer status"
echo "  systemctl --user stop trotd-refresh.timer    - Stop the timer"
echo "  systemctl --user disable trotd-refresh.timer - Disable the timer"
echo "  journalctl --user -u trotd-refresh.service   - View logs"
