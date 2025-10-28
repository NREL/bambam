#!/bin/sh

# This script sets up the developer environment for testing multimodal routing functionlity of bambam. this is
# intended to be run once after cloning the repository and building with -r flag.

echo "Download the Denver RTD GTFS archive to denver_rtd/rtd-gtfs.zip"
mkdir denver_rtd
cd denver_rtd
curl -L -o rtd_gtfs.zip https://www.rtd-denver.com/files/gtfs/google_transit.zip

echo "Unzip the archive"
unzip rtd_gtfs.zip -d "gtfs/"

echo "Prepare the compass files"
uv run --with "geopandas,numpy,osmnx,nrel.routee.compass[all]" ../script/setup_test_bambam_gtfs.py ./ --output_geometries

echo "Process gtfs archive for date matching"
echo "(You need to build the rust binaries first)"
../rust/target/release/bambam_gtfs preprocess-bundle \
    --input "rtd_gtfs.zip" \
    --starting-edge-list-id 1 \
    --parallelism 1 \
    --vertex-match-tolerance 2500 \
    --date-mapping-policy nearest-date-time-range \
    --date-mapping-date-tolerance 365 \
    --date-mapping-match-weekday true \
    --output-directory "./transit" \
    --vertices-compass-filename "./compass/vertices-complete.csv.gz" \
    --start-date 09-01-2025 \
    --end-date 09-01-2025 \
    --start-time 08:00:00 \
    --end-time 09:00:00

echo "Produce transit geometries"
uv run --with "geopandas,numpy" ../script/transit_output_extract_geojson.py --suffix="-transit-1" ./transit/edges-compass-1.csv.gz ./compass/vertices-compass.csv.gz ./geometries

cd ..
echo "running gtfs-config to modify an existing BAMBAM TOML configuration with this transit dataset"
./rust/target/release/bambam_util gtfs-config --directory ./denver_rtd/transit --base-config ./configuration/test_gtfs_config_denver_rtd.toml

# echo "running BAMBAM with a walk-transit trip"
# ./rust/target/release/bambam -c configuration/test_gtfs_config_denver_rtd_gtfs.toml -q denver_rtd/geometries/query.json  