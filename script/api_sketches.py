from typing import Optional, Union, List
from shapely.geometry import Polygon, MultiPolygon
from shapely.ops import transform
import geopandas
import h3


def polygon_to_h3_geojson(polygon: Polygon):
    """
    creates a GeoJSON that is a valid h3 polygon in lat/lon
    coordinate ordering.

    :param polygon: the source polygon, assumed to be lon/lat
    :type polygon: Union[]
    :return: _description_
    :rtype: _type_
    """
    transformed_polygon = transform(lambda x, y: (y, x), polygon)
    coordinates = list(transformed_polygon.exterior.coords)
    return {"type": "Polygon", "coordinates": [coordinates]}


def polygon_for_hex_id(hex_id):
    """
    creates a polygon from an h3 hex_id that is in lon/lat (x,y)
    ordering.

    :param hex_id: hex to convert
    :type hex_id: str
    :return: a hexagonal polygon representing this h3 hex id
    :rtype: Polygon
    """
    boundary = h3.h3_to_geo_boundary(hex_id)
    polygon = transform(lambda x, y: (y, x), Polygon(boundary))
    return polygon


def get_hexes_from_geometry(
    geometry: Union[Polygon, MultiPolygon],
) -> List[str]:
    """collects MEP rows either by
        1. using a GeoDataFrame's spatial index
        2. running a bounds check on bounding box columns added to the dataset

    :param geometry: a bounded geometry to check for intersecting rows
    :type geometry: Union[shapely.geometry.Polygon, shapely.geometry.MultiPolygon]
    :return: a list of h3 geo ids
    :rtype: List[str]
    """

    # Handle MultiPolygon
    if geometry.geometry_type == "MultiPolygon":
        h3_cells = set()
        for polygon in geometry.geoms:
            geojson_polygon = polygon_to_h3_geojson(polygon)
            # Use h3.polyfill on each polygon
            h3_cells.update(h3.polyfill(geojson_polygon, res=8))
    else:
        # Handle single Polygon
        geojson_polygon = polygon_to_h3_geojson(geometry)
        h3_cells = h3.polyfill(geojson_polygon, res=8)
    return h3_cells


def get_gdf_from_geometry(
    geometry: Union[Polygon, MultiPolygon], crs: str = "EPSG:4326"
) -> geopandas.GeoDataFrame:
    """collects MEP rows either by
        1. using a GeoDataFrame's spatial index
        2. running a bounds check on bounding box columns added to the dataset

    :param geometry: a bounded geometry to check for intersecting rows
    :type geometry: Union[shapely.geometry.Polygon, shapely.geometry.MultiPolygon]
    :param crs: coordinate system, defaults to "EPSG:4326"
    :type crs: str, optional
    :param modes: travel modes, defaults to ["walk", "bike", "drive", "transit"]
    :rtype: geopandas.GeoDataFrame
    """
    hex_ids = get_hexes_from_geometry(geometry)
    # TODO:
    # 1. get all parquet rows with matching hex_id indices
    # 2. get hex polygon for each hex id, set to a column (see polygon_for_hex_id, above)
    # 3. convert to GeoDataFrame with crs="EPSG:4326"
    # 4. if the user-provided crs above is different, call gdf.to_crs(crs)
    # 5. return
    pass


def calculate_mep(
    gdf: geopandas.GeoDataFrame,
    modes: List[str] = ["walk", "bike", "drive", "transit"],
    activities: List[str] = [
        "food",
        "services",
        "entertainment",
        "retail",
        "healthcare",
        "jobs",
    ],
    weighting_column: Optional[str] = "population",
) -> float:
    """
    calculates a MEP value over the provided dataset, filtered to the provided modes and activities, and
    weighted by the provided weighting column.

    :param gdf: GeoDataFrame of MEP rows
    :type gdf: geopandas.GeoDataFrame
    :param modes: travel modes included in score, defaults to ["walk", "bike", "drive", "transit"]
    :type modes: List[str], optional
    :param activities: activities included in score, defaults to ["food", "services", "entertainment", "retail", "healthcare", "jobs"]
    :type activities: List[str], optional
    :param weighting_column: if provided, a column used to weight score values by row, defaults to "population"
    :type weighting_column: str, optional
    :return: MEP score
    :rtype: float
    """
    pass
