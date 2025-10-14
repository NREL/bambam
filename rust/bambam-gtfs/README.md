# bambam-gtfs

support utilities for working with GTFS datasets.

assumed user has downloaded [the GTFS archive list](https://files.mobilitydatabase.org/feeds_v2.csv) from the [mobility-database-catalogs](https://github.com/MobilityData/mobility-database-catalogs) GitHub repository.

## Batch Processing with Error Handling

The `preprocess-bundle` command supports the `--ignore-failures` flag for batch processing of multiple GTFS archives. When enabled:

- Failed bundles are logged to stderr but do not halt processing
- Successful bundles are processed to completion
- Edge list IDs are reassigned sequentially based only on successful bundles
- This ensures a contiguous sequence of edge list outputs without gaps

Example usage:
```bash
bambam_gtfs preprocess-bundle \
  --input /path/to/gtfs/bundles \
  --starting-edge-list-id 0 \
  --output-directory /path/to/output \
  --vertices-compass-filename vertices.csv.gz \
  --start-date 01-01-2024 \
  --end-date 12-31-2024 \
  --date-mapping-policy exact-date \
  --ignore-failures
```

This is particularly useful when processing large batches of GTFS feeds where occasional failures are expected but should not block the entire pipeline.