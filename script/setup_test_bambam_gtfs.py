import argparse
import osmnx as ox
import pandas as pd
import geopandas as gpd
from pathlib import Path
from shapely.geometry import Point
from nrel.routee.compass.io import generate_compass_dataset

if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="Setup GTFS bambam test.")
    parser.add_argument("dir", type=Path, help="Path to working directory (parent of gtfs folder)")
    parser.add_argument("--hull_buffer", default=10, type=float, help="Buffer (in meters) for the convex hull of stop points")
    args = parser.parse_args()

    print("Reading stops.txt into a geodataframe")
    raw_df = pd.read_csv(f"{args.dir}/gtfs/stops.txt", sep=",")
    gdf = gpd.GeoSeries(raw_df.apply(lambda r: Point(r.stop_lon, r.stop_lat), axis=1), crs="EPSG:4326")

    print("Estimate UTM CRS and compute buffered convex hull")
    utm_crs = gdf.estimate_utm_crs()
    hull_geometry = gdf.to_crs(utm_crs).geometry.union_all().convex_hull.buffer(args.hull_buffer)
    hull_gdf = gpd.GeoDataFrame(geometry=[hull_geometry], crs=utm_crs).to_crs("EPSG:4326")

    print("Download osmnx grpah")
    g = ox.graph_from_polygon(hull_gdf.geometry.iloc[0], network_type="drive")
    generate_compass_dataset(g, output_directory=f"{args.dir}/compass")