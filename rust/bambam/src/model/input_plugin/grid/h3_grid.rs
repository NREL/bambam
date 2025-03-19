use super::grid_ops;
use geo::Centroid;
use h3o::geom::{PolyfillConfig, ToCells, ToGeo};
use itertools::Itertools;
use wkt::ToWkt;

pub fn from_polygon_extent(
    extent: &geo::Polygon,
    template: &serde_json::Value,
    resolution: &h3o::Resolution,
) -> Result<Vec<serde_json::Value>, String> {
    let h3o_polygon = h3o::geom::Polygon::from_degrees(extent.clone())
        .map_err(|e| format!("failure reading polygon into h3o lib: {}", e))?;
    let hex_ids = h3o_polygon
        .to_cells(PolyfillConfig::new(*resolution))
        .collect_vec();

    hex_ids
        .into_iter()
        .map(|cell| {
            let polygon = cell
                .to_geom(true)
                .map_err(|e| format!("internal error on CellIndex.to_geom: {}", e))?;
            let centroid = polygon.centroid().ok_or_else(|| {
                format!(
                    "unable to retrieve centroid of polygon: {}",
                    polygon.to_wkt()
                )
            })?;
            let row = grid_ops::create_grid_row(
                cell.to_string(),
                centroid.x(),
                centroid.y(),
                &geo::Geometry::Polygon(polygon),
                template,
            )?;
            Ok(row)
        })
        .collect::<Result<Vec<_>, _>>()
}
