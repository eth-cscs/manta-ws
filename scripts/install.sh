#!/bin/bash

set -e

# build cama binary
cargo build --target x86_64-unknown-linux-musl --release

# copy config file
scp ./scripts/config.toml root@hashicorp-vault:/root/.config/manta/config.toml

# copy cert files
scp ./scripts/alps_root_cert.pem root@hashicorp-vault:/root/.config/manta/alps_root_cert.pem

# copy systemd unit file
scp ./scripts/cama.service root@hashicorp-vault:/etc/systemd/system/cama.service

# stop cama
ssh root@hashicorp-vault.cscs.ch systemctl stop cama

# copy cama binary
scp ./target/x86_64-unknown-linux-musl/release/cama root@hashicorp-vault:/usr/local/bin/cama

# stop cama
ssh root@hashicorp-vault.cscs.ch systemctl stop cama

ssh root@hashicorp-vault.cscs.ch systemctl daemon-reload

ssh root@hashicorp-vault.cscs.ch systemctl enable cama.service

# start cama
ssh root@hashicorp-vault.cscs.ch systemctl restart cama.service
