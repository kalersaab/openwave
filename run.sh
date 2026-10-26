#!/bin/bash
# Wrapper script to run OpenWave with VLC libraries

export DYLD_LIBRARY_PATH="/Applications/VLC.app/Contents/MacOS/lib:$DYLD_LIBRARY_PATH"
export VLC_PLUGIN_PATH="/Applications/VLC.app/Contents/MacOS/plugins"

./target/debug/openwave "$@"
