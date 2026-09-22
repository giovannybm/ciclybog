#!/usr/bin/env bash
set -euo pipefail

# Usage:
#   PMTILES_SOURCE_URL=https://build.protomaps.com/YYYYMMDD.pmtiles \
#     ./scripts/download-bogota-pmtiles.sh
#
# Or pass the source archive directly:
#   ./scripts/download-bogota-pmtiles.sh https://build.protomaps.com/YYYYMMDD.pmtiles

SOURCE_URL="${1:-${PMTILES_SOURCE_URL:-}}"
OUTPUT_PATH="${2:-public/data/bogota.pmtiles}"
MAX_ZOOM="${MAX_ZOOM:-15}"
BOGOTA_BBOX="${BOGOTA_BBOX:--74.25,4.45,-73.95,4.90}"

if [[ -z "$SOURCE_URL" ]]; then
  echo "The source PMTiles URL is required."
  echo "Uso: PMTILES_SOURCE_URL=https://.../archivo.pmtiles $0"
  exit 2
fi

if ! command -v pmtiles >/dev/null 2>&1; then
  echo "The 'pmtiles' CLI was not found. Install it from: https://github.com/protomaps/go-pmtiles/releases"
  exit 1
fi

mkdir -p "$(dirname "$OUTPUT_PATH")"
temporary_output="$(mktemp "${TMPDIR:-/tmp}/bogota.XXXXXX.pmtiles")"
cleanup() { rm -f "$temporary_output"; }
trap cleanup EXIT

echo "Extracting Bogotá from: $SOURCE_URL"
echo "Bounding box: $BOGOTA_BBOX | maximum zoom: $MAX_ZOOM"

pmtiles extract "$SOURCE_URL" "$temporary_output" \
  --bbox="$BOGOTA_BBOX" \
  --maxzoom="$MAX_ZOOM" \
  --overfetch=0

mv "$temporary_output" "$OUTPUT_PATH"
trap - EXIT
echo "PMTiles saved to: $OUTPUT_PATH"
