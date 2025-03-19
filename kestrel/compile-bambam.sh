#!/bin/bash
#
# run this script with working directory one level above the routee libraries, such as:
# dev/
#   compile-mep.sh           <- this script
#   bambam/
#   routee-compass-tomtom/
#   routee-compass

echo "compiling and deploying bambam for kestrel."

SCRIPT_DIR=$(dirname "$0")
BIN_DIR="/projects/mepcore/bin/"
LIB_DIR="/projects/mepcore/lib/bambam"

# kestrel modules
module load anaconda3
module load git-lfs

ENV_EXISTS=$(conda env list | grep routee-compile)
if [ ! "$ENV_EXISTS" ]; 
then
    CREATE_RESULT=$(conda env create -y -f "$SCRIPT_DIR/environment.yml")
    if [ "$CREATE_RESULT" -ne 0 ];
    then
        echo "did not find 'routee-compile' conda environment and could not build it."
        exit 1
    fi
else    
    echo "found anaconda environment 'routee-compile'"
fi

if conda activate routee-compile; 
then
    echo "routee-compile environment loaded"
else
    echo "did not find 'routee-compile' conda environment and could not build it."
    exit 1
fi
# module swap PrgEnv-cray PrgEnv-gnu  # PrgEnv-gnu is now the default


if cargo build -r --manifest-path "$SCRIPT_DIR/../rust/Cargo.toml"; 
then
    echo "bambam compiled successfully."
else
    echo "failure building the bambam applications."
    exit 1
fi

echo "copying bin assets to $BIN_DIR"
mkdir -p "$BIN_DIR"
cp "$SCRIPT_DIR/../rust/target/release/bambam" "$BIN_DIR"
cp "$SCRIPT_DIR/../rust/target/release/bambam-osm" "$BIN_DIR"
cp "$SCRIPT_DIR/bambam.sh" "$BIN_DIR"
cp "$SCRIPT_DIR/bambam_access.sh" "$BIN_DIR"
cp "$SCRIPT_DIR/bambam_isochrone.sh" "$BIN_DIR"

echo "copying lib assets to $LIB_DIR"
mkdir -p "$LIB_DIR"
cp "$SCRIPT_DIR/grid_access_natl_tod.toml" "$LIB_DIR"
cp "$SCRIPT_DIR/grid_isochrone_natl_tod.toml" "$LIB_DIR"