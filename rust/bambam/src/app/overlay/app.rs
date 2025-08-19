use super::OverlayOperation;
use crate::{
    app::overlay::{Grouping, MepRow, OutRow, OverlaySource},
    util::polygonal_rtree::PolygonalRTree as PrtBambam,
};
use bamcensus_core::model::identifier::Geoid;
use csv::StringRecord;
use flate2::read::GzDecoder;
use geo::Geometry;
use itertools::Itertools;
use kdam::{tqdm, Bar, BarBuilder, BarExt};
use rayon::prelude::*;
use routee_compass_core::util::geo::PolygonalRTree;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs::File,
    io::{BufRead, BufReader, Read},
    path::Path,
    sync::{Arc, Mutex},
};
use wkt::{ToWkt, TryFromWkt};

/// function to aggregate mep output rows to some overlay geometry dataset.
/// the number of output rows is not dependent on the size of the source geometry dataset,
/// instead based on the number of geometry rows with matches in the mep dataset.
/// only mep score and population data are aggregated at this time, via summation.
pub fn run(
    mep_filepath: &str,
    output_filepath: &str,
    overlay_source: &OverlaySource,
    how: &OverlayOperation,
) -> Result<(), String> {
    // fail early if IO error from read/write destinations
    let mep_result_file =
        File::open(mep_filepath).map_err(|e| format!("error reading '{mep_filepath}': {e}"))?;
    let mut output_writer = csv::Writer::from_path(output_filepath)
        .map_err(|e| format!("failure opening output file '{output_filepath}': {e}"))?;

    // read overlay dataset
    let overlay_data = overlay_source.build()?;
    log::info!("found {} rows in overlay dataset", overlay_data.len());
    let overlay_lookup = overlay_data
        .iter()
        .map(|(geom, geoid)| (geoid.clone(), geom.clone()))
        .collect::<HashMap<_, _>>();
    let overlay: Arc<PolygonalRTree<f64, Geoid>> = Arc::new(PolygonalRTree::new(overlay_data)?);

    let mut csv_reader = csv::Reader::from_reader(mep_result_file);
    let rows_iter = tqdm!(csv_reader.deserialize(), desc = "reading MEP rows");
    let rows = rows_iter
        .collect::<Result<Vec<MepRow>, _>>()
        .map_err(|e| format!("failed reading {mep_filepath}: {e}"))?;
    eprintln!();
    log::info!("processed {} rows", rows.len());

    let grouped_rows: Vec<(Grouping, MepRow)> = spatial_lookup(rows, overlay.clone())?;

    let mut grouped_lookup: HashMap<Grouping, (Geometry, Vec<MepRow>)> = HashMap::new();
    for (grouping, row) in grouped_rows.into_iter() {
        match grouped_lookup.get_mut(&grouping) {
            Some((_, v)) => v.push(row),
            None => {
                let geometry = overlay_lookup.get(&grouping.geoid).ok_or_else(|| {
                    format!(
                        "internal error, lookup missing geometry entry for geoid '{}'",
                        grouping.geoid
                    )
                })?;
                let _ = grouped_lookup.insert(grouping.clone(), (geometry.clone(), vec![row]));
            }
        }
    }

    // aggregate results into the overlay dataset
    let agg_iter = tqdm!(
        grouped_lookup.iter(),
        desc = "aggregating results",
        total = grouped_lookup.len()
    );
    let result = agg_iter
        .map(|(grouping, (geometry, mep_rows))| OutRow::new(grouping, geometry, mep_rows))
        .collect_vec();

    for row in result.into_iter() {
        output_writer
            .serialize(row)
            .map_err(|e| format!("failure writing row to output: {e}"))?;
    }

    println!("written to {output_filepath}");
    Ok(())
}

/// performs batch geospatial intersection operations to assign each [`MepRow`] its
/// grouping identifier (GEOID). run in parallel over the rows argument, a chunk of
/// the source MEP dataset.
fn spatial_lookup(
    rows: Vec<MepRow>,
    overlay: Arc<PolygonalRTree<f64, Geoid>>,
) -> Result<Vec<(Grouping, MepRow)>, String> {
    let bar = Arc::new(Mutex::new(
        BarBuilder::default()
            .desc("spatial lookup")
            .total(rows.len())
            .build()?,
    ));

    let result = rows
        .into_par_iter()
        .flat_map(|row| {
            if let Ok(mut b) = bar.clone().lock() {
                let _ = b.update(1);
            }
            let point = geo::Geometry::Point(geo::Point::new(row.lon, row.lat));
            let intersection_result = overlay.intersection(&point);
            let found = match intersection_result {
                Err(e) => return vec![Err(e)],
                Ok(found) => found.collect_vec(),
            };
            match found[..] {
                [] => vec![],
                [single] => vec![Ok((
                    Grouping::new(single.data.clone(), row.mode.clone()),
                    row,
                ))],
                _ => {
                    let found_geoids = found.iter().map(|r| r.data.to_string()).join(", ");
                    vec![Err(format!(
                        "point {} unexpectedly found multiple geoids: [{}]",
                        point.to_wkt(),
                        found_geoids
                    ))]
                }
            }
        })
        .collect::<Result<Vec<_>, _>>()?;
    eprintln!();
    Ok(result)
}
