#!/bin/sh

# setup.sh
# This script sets up the developer environment for the bambam project. this is
# intended to be run once after cloning the repository.

set -e

# 1. bambam-osm test dataset
cd rust/bambam-osm/src/test/
./get_test.sh
cd ../../../../

