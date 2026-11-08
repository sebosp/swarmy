#!/bin/bash
# Installs swarmy --serve as a system service of another computer, for a binary that is not from a
# package: a build in ~/swarmy-preview with its start.sh in ~/swarmy-server, as corvid-zt has them.
# The packages have packaging/linux/swarmy-server-system.service instead.
#
#   scripts/install-server-service-over-ssh.sh corvid-zt
set -euo pipefail
host=${1:?The ssh host of the server}
user=${2:-$(ssh "$host" id -un)}
ssh "$host" 'bash -s' <<REMOTE
sudo -n tee /etc/systemd/system/swarmy-server.service >/dev/null <<'UNIT'
[Unit]
Description=swarmy remote backend on :7878
After=network-online.target
Wants=network-online.target

[Service]
ExecStart=/home/$user/swarmy-server/start.sh
User=$user
Group=$user
Restart=always
RestartSec=10

[Install]
WantedBy=multi-user.target
UNIT
pkill -f "[d]ata-dir /home/$user/swarmy-server" || true
sleep 2
sudo -n systemctl daemon-reload
sudo -n systemctl enable --now swarmy-server
sleep 5
systemctl is-active swarmy-server
curl -s -m5 -o /dev/null -w "%{http_code}\n" localhost:7878/api/version
REMOTE
