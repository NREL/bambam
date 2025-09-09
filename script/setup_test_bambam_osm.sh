#!/bin/sh

# setup.sh
# This script sets up the developer environment for the bambam-osm project. this is
# intended to be run once after cloning the repository.

set -e

echo "downloading test osm.pbf dataset"
cd rust/bambam-osm/src/test/
curl -O http://download.geofabrik.de/europe/liechtenstein-250908.osm.pbf
cd ../../../../
