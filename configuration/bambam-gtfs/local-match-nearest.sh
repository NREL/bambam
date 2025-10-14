#!/bin/sh

./rust/target/release/bambam_gtfs preprocess-bundle \
    --input "boulder_co/ucb-gtfs.zip" \
    --starting-edge-list-id 1 \
    --parallelism 1 \
    --date-mapping-policy nearest-date \
    --date-mapping-date-tolerance 7 \
    --date-mapping-match-weekday true \
    --output-directory "boulder_co/transit" \
    --vertices-compass-filename "boulder_co/vertices-complete.csv.gz" \
    --start-date 08-15-2025 \
    --end-date 08-15-2025