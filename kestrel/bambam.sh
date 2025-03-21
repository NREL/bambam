#!/bin/bash --login

#SBATCH --job-name=bambam-a
#SBATCH --nodes=1
#SBATCH --ntasks=100
#SBATCH --mem=240GB
#SBATCH --tmp=500GB
#SBATCH --mail-type=ALL

PROJECT_DIR="/projects/mepcore"
BIN_DIR="$PROJECT_DIR/bin"
BAMBAM_PATH="$BIN_DIR/bambam"
QUERY_FILE="$1"
CONFIG_FILE="$2"

if [[ "${PWD}" =~ ^"${PROJECT_DIR}"/* ]]; then
    echo "cannot run this script from within ${PROJECT_DIR}, please change directory to your home or scratch space and retry."
    exit 1
fi

if [ ! -f "$QUERY_FILE" ]; then
    echo "file not found, please confirm the path exists: '$QUERY_FILE'"
    exit 1
fi

if [ ! -f "$CONFIG_FILE" ]; then
    echo "file not found, please confirm the path exists: '$CONFIG_FILE'"
    exit 1
fi

# Execute the command
if [ -x "$BAMBAM_PATH" ]; then
  echo "found command: $BAMBAM_PATH"
else
  echo "command not found or not executable: $BAMBAM_PATH"
  exit 1
fi

POLYGON_COUNT=$(grep -c 'POLYGON' "$QUERY_FILE")
MULTIPOLYGON_COUNT=$(grep -c 'MULTIPOLYGON' "$QUERY_FILE")
if [ ! "$POLYGON_COUNT" -eq 1 ] && [ ! "$MULTIPOLYGON_COUNT" -eq 1 ]; then
  echo "query file should contain exactly one WKT POLYGON or MULTIPOLYGON"
  exit 1
fi

module load bzip2

"$BAMBAM_PATH" -q "$QUERY_FILE" -c "$CONFIG_FILE"