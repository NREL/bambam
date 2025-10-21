#!/bin/sh

# This script sets up the developer environment for testing multimodal routing functionlity of bambam. this is
# intended to be run once after cloning the repository.

echo "Download the Denver RTD GTFS archive to denver_rtd/rtd-gtfs.zip"
mkdir denver_rtd
cd denver_rtd
# curl -L -o rtd_gtfs.zip https://www.rtd-denver.com/files/gtfs/google_transit.zip

echo "Unzip the archive"
unzip rtd_gtfs.zip -d "gtfs/"
