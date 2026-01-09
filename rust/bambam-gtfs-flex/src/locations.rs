use geo_types::{Coord, Geometry, MultiPolygon, Polygon};
use serde_json::Value;
use std::{fs::File, io, path::Path};
use zip::ZipArchive;

/// a single location feature from `locations.geojson`
#[derive(Debug)]
pub struct Location {
    pub id: String,
    pub geometry: Geometry<f64>, // Polygon or MultiPolygon
}

/// read locations.geojson from a single GTFS-Flex ZIP file
///
/// streams data directly from the ZIP
/// returns None if locations.geojson is missing or duplicated
/// returns typed Location structs on success
pub fn read_locations_from_flex(zip_path: &Path) -> io::Result<Option<Vec<Location>>> {
    // open the ZIP file
    let file = File::open(zip_path)?;
    let mut archive = ZipArchive::new(file)?;

    // locate locations.geojson inside the ZIP
    let mut locations_name: Option<String> = None;

    for i in 0..archive.len() {
        let file_in_zip = archive.by_index(i)?;
        if file_in_zip.name().ends_with("locations.geojson") {
            if locations_name.is_some() {
                eprintln!(
                    "WARNING: Multiple locations.geojson found in {:?}. Skipping ZIP.",
                    zip_path
                );
                return Ok(None);
            }
            locations_name = Some(file_in_zip.name().to_string());
        }
    }

    // handle missing locations.geojson
    let locations_name = match locations_name {
        Some(name) => name,
        None => {
            println!("No locations.geojson found in {:?}", zip_path);
            return Ok(None);
        }
    };

    // open the locations.geojson file
    let mut file_in_zip = archive.by_name(&locations_name)?;
    let geojson: Value = serde_json::from_reader(&mut file_in_zip)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

    // parse the features array
    let features = geojson
        .get("features")
        .and_then(|f| f.as_array())
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "Missing features array"))?;

    let mut locations = Vec::new();

    for feature in features {
        let id = feature
            .get("id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "Missing feature id"))?
            .to_string();

        let geometry = feature
            .get("geometry")
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "Missing geometry"))?;

        let geom_type = geometry
            .get("type")
            .and_then(|v| v.as_str())
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "Missing geometry type"))?;

        let coordinates = geometry
            .get("coordinates")
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "Missing coordinates"))?;

        let geo = match geom_type {
            "Polygon" => {
                let polygon = parse_polygon(coordinates)?;
                Geometry::Polygon(polygon)
            }
            "MultiPolygon" => {
                let multipolygon = parse_multipolygon(coordinates)?;
                Geometry::MultiPolygon(multipolygon)
            }
            _ => {
                eprintln!(
                    "Warning: unsupported geometry type '{}' for id '{}'",
                    geom_type, id
                );
                continue;
            }
        };

        locations.push(Location { id, geometry: geo });
    }

    Ok(Some(locations))
}

/// helper to parse a Polygon from JSON coordinates
fn parse_polygon(value: &Value) -> io::Result<Polygon<f64>> {
    let rings = value.as_array().ok_or_else(|| {
        io::Error::new(io::ErrorKind::InvalidData, "Polygon coordinates not array")
    })?;

    let exterior_ring = &rings[0];
    let exterior = parse_ring(exterior_ring)?;

    let mut interiors = vec![];
    for ring in &rings[1..] {
        interiors.push(parse_ring(ring)?);
    }

    Ok(Polygon::new(exterior, interiors))
}

/// helper to parse a MultiPolygon from JSON coordinates
fn parse_multipolygon(value: &Value) -> io::Result<MultiPolygon<f64>> {
    let polygons = value.as_array().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            "MultiPolygon coordinates not array",
        )
    })?;
    let mut result = vec![];
    for poly_coords in polygons {
        let polygon = parse_polygon(poly_coords)?;
        result.push(polygon);
    }
    Ok(MultiPolygon(result))
}

/// helper to parse a ring of coordinates into geo_types LineString
fn parse_ring(value: &Value) -> io::Result<geo_types::LineString<f64>> {
    let points = value
        .as_array()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "Ring not array"))?;
    let coords = points
        .iter()
        .map(|pt| {
            let arr = pt.as_array().unwrap();
            Coord {
                x: arr[0].as_f64().unwrap(),
                y: arr[1].as_f64().unwrap(),
            }
        })
        .collect();
    Ok(geo_types::LineString(coords))
}
