#!/bin/bash

set -e

# build manta-ws binary
cargo build --target x86_64-unknown-linux-musl --release

# copy config file
scp ./scripts/config.toml root@hashicorp-vault:/root/.config/manta/config.toml

# copy cert files
scp ./scripts/alps_root_cert.pem root@hashicorp-vault:/root/.config/manta/alps_root_cert.pem

# copy systemd unit file
scp ./scripts/manta-ws.service root@hashicorp-vault:/etc/systemd/system/manta-ws.service

# stop manta-ws
ssh root@hashicorp-vault.cscs.ch systemctl stop manta-ws

# copy manta-ws binary
scp ./target/x86_64-unknown-linux-musl/release/manta-ws root@hashicorp-vault:/usr/local/bin/manta-ws

# stop manta-ws
ssh root@hashicorp-vault.cscs.ch systemctl stop manta-ws

ssh root@hashicorp-vault.cscs.ch systemctl daemon-reload

ssh root@hashicorp-vault.cscs.ch systemctl enable manta-ws.service

# start manta-ws
ssh root@hashicorp-vault.cscs.ch systemctl restart manta-ws.service
