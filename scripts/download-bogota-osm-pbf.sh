#!/usr/bin/env bash
set -euo pipefail

# Compact Bogotá OSM PBF extract used to build the routing graph.
# Fuente: https://download.bbbike.org/osm/bbbike/Bogota/Bogota.osm.pbf
SOURCE_URL="${1:-${OSM_PBF_SOURCE_URL:-https://download.bbbike.org/osm/bbbike/Bogota/Bogota.osm.pbf}}"
OUTPUT_PATH="${2:-data/bogota.osm.pbf}"

mkdir -p "$(dirname "$OUTPUT_PATH")"
temporary_output="$(mktemp "${TMPDIR:-/tmp}/bogota.XXXXXX.osm.pbf")"
cleanup() { rm -f "$temporary_output"; }
trap cleanup EXIT

echo "Downloading Bogotá OSM PBF from: $SOURCE_URL"
curl --fail --location --retry 3 --retry-delay 2 --progress-bar "$SOURCE_URL" --output "$temporary_output"
mv "$temporary_output" "$OUTPUT_PATH"
trap - EXIT
echo "OSM PBF saved to: $OUTPUT_PATH"
