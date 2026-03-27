#!/usr/bin/env bash
# Build the azalea-bridge coremod JAR
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
OUT="$SCRIPT_DIR/azalea-bridge-2.5.0.jar"

cd "$SCRIPT_DIR"
jar cfm "$OUT" META-INF/MANIFEST.MF \
    META-INF/coremods.json \
    META-INF/neoforge.mods.toml \
    azalea_bridge.js

echo "Built: $OUT"
echo "Size: $(du -h "$OUT" | cut -f1)"
