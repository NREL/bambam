#!/bin/sh

# setup.sh
# This script sets up the developer environment for the bambam project. this is
# intended to be run once after cloning the repository.

set -e

# create a virtual environment
echo "downloading denver_co scenario"
uv run --with osmnx --with "nrel.routee.compass[all]" script/setup_test_bambam.py
