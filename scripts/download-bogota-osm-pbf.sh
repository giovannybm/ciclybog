#!/usr/bin/env bash
set -euo pipefail

# Extracto OSM PBF compacto de Bogotá para construir el grafo de ruteo.
# Fuente: https://download.bbbike.org/osm/bbbike/Bogota/Bogota.osm.pbf
SOURCE_URL="${1:-${OSM_PBF_SOURCE_URL:-https://download.bbbike.org/osm/bbbike/Bogota/Bogota.osm.pbf}}"
OUTPUT_PATH="${2:-data/bogota.osm.pbf}"

mkdir -p "$(dirname "$OUTPUT_PATH")"
temporary_output="$(mktemp "${TMPDIR:-/tmp}/bogota.XXXXXX.osm.pbf")"
cleanup() { rm -f "$temporary_output"; }
trap cleanup EXIT

echo "Descargando OSM PBF de Bogotá desde: $SOURCE_URL"
curl --fail --location --retry 3 --retry-delay 2 --progress-bar "$SOURCE_URL" --output "$temporary_output"
mv "$temporary_output" "$OUTPUT_PATH"
trap - EXIT
echo "OSM PBF guardado en: $OUTPUT_PATH"
