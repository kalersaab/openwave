#!/bin/bash
# Wrapper script to run OpenWave

echo "Starting OpenWave..."
echo ""

./target/debug/openwave "$@"
