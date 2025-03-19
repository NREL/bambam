#!/bin/bash
RUST_LOG=info rust/target/release/bambam-osm pbf --pbf-file ~/data/mep/mep3/input/osm/arvada_geos_primrose.pbf --output-directory rust/bambam-osm/out
