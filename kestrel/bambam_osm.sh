#!/bin/bash --login

#SBATCH --job-name=bambam-osm
#SBATCH --nodes=1
#SBATCH --ntasks=100
#SBATCH --mem=240GB
#SBATCH --tmp=500GB
#SBATCH --mail-type=ALL

PROJECT_DIR="/projects/mepcore"
BIN_DIR="$PROJECT_DIR/bin"
BAMBAM_OSM_PATH="$BIN_DIR/bambam-osm"
PBF_FILE="$1"
OUTPUT_DIR="$2"
CONFIG_FILE="$3"

if [[ "${PWD}" =~ ^"${PROJECT_DIR}"/* ]]; then
    echo "cannot run this script from within ${PROJECT_DIR}, please change directory to your home or scratch space and retry."
    exit 1
fi

if [ ! -f "$PBF_FILE" ]; then
    echo "file not found, please confirm the path exists: '$PBF_FILE'"
    exit 1
fi

if [ ! -d "$OUTPUT_DIR" ]; then
    echo "directory not found, please confirm the path exists: '$OUTPUT_DIR'"
    exit 1
fi

if [ ! -f "$CONFIG_FILE" ]; then
    echo "file not found, please confirm the path exists: '$CONFIG_FILE'"
    exit 1
fi

# Execute the command
if [ -x "$BAMBAM_OSM_PATH" ]; then
  echo "found command: $BAMBAM_OSM_PATH"
else
  echo "command not found or not executable: $BAMBAM_OSM_PATH"
  exit 1
fi

# POLYGON_COUNT=$(grep -c 'POLYGON' "$PBF_FILE")
# MULTIPOLYGON_COUNT=$(grep -c 'MULTIPOLYGON' "$PBF_FILE")
# if [ ! "$POLYGON_COUNT" -eq 1 ] && [ ! "$MULTIPOLYGON_COUNT" -eq 1 ]; then
#   echo "query file should contain exactly one WKT POLYGON or MULTIPOLYGON"
#   exit 1
# fi

module load bzip2

"$BAMBAM_OSM_PATH" pbf --pbf-file "$PBF_FILE" --output-directory "$OUTPUT_DIR" --configuration-file "$CONFIG_FILE"