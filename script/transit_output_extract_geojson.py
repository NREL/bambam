import argparse
import pandas as pd
import geopandas as gpd
from pathlib import Path
from shapely.geometry import Point, LineString

def process_csv_into_geometry(compas_edges_path, compas_vertices_path, output_dir, suffix="", output_vertices=False):
    edges_df = pd.read_csv(compas_edges_path, compression='gzip')
    vertices_df = pd.read_csv(compas_vertices_path, compression='gzip')
    vertices_gdf = gpd.GeoDataFrame(vertices_df, geometry=vertices_df.apply(lambda r: Point(r.x, r.y), axis=1))
    src_points = vertices_gdf.geometry.loc[edges_df["src_vertex_id"]]
    dst_points = vertices_gdf.geometry.loc[edges_df["dst_vertex_id"]]
    lines = [LineString([s, d]) for s, d in zip(src_points, dst_points)]
    lines_gdf = gpd.GeoDataFrame(edges_df, geometry=lines, crs=vertices_gdf.crs)

    lines_gdf.to_file(f"{output_dir}/edges{suffix}.geojson", driver="GeoJSON")
    if output_vertices:
        vertices_gdf.to_file(f"{output_dir}/vertices{suffix}.geojson", driver="GeoJSON")

if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="Utility for processing csv.gz files into plottable geojson")
    parser.add_argument("compas_edges_path", type=Path, help="Path to compass-edges.csv.gz")
    parser.add_argument("compas_vertices_path", type=Path, help="Path to compass-vertices.csv.gz")
    parser.add_argument("output_dir", type=Path, help="Output Directory")
    parser.add_argument("--suffix", required=False, default="", type=str, help="File suffix")
    parser.add_argument("--output_vertices", action="store_true", help="If set, store vertices geometries")
    args = parser.parse_args()

    process_csv_into_geometry(
        args.compas_edges_path,
        args.compas_vertices_path,
        args.output_dir,
        suffix=args.suffix,
        output_vertices=args.output_vertices
    )
