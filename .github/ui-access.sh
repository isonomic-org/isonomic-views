#!/usr/bin/env bash
# Lets git, and so Cargo, fetch the private isonomic-org/isonomic-ui repository over SSH with
# the read-only deploy key passed in $KEY (the UI_DEPLOY_KEY secret).
set -euo pipefail
if [ -z "${KEY:-}" ]; then
  echo "::error::Secret UI_DEPLOY_KEY is not set, so the private isonomic-ui repository cannot be fetched."
  exit 1
fi
mkdir -p ~/.ssh
chmod 700 ~/.ssh
printf '%s\n' "$KEY" > ~/.ssh/isonomic_ui
chmod 600 ~/.ssh/isonomic_ui
# GitHub's published SSH host key.
echo "github.com ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIOMqqnkVzrm0SdG6UOoqKLsabgH5C9okWi0dh2l9GKJl" >> ~/.ssh/known_hosts
git config --global url."ssh://git@github.com/isonomic-org/isonomic-ui".insteadOf "https://github.com/isonomic-org/isonomic-ui"
git config --global core.sshCommand "ssh -i ~/.ssh/isonomic_ui -o IdentitiesOnly=yes"
