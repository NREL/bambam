import pandas as pd
from shapely.geometry import Polygon
from shapely import wkt
import h3
import argparse
import json
from tqdm import tqdm

parser = argparse.ArgumentParser()
parser.add_argument("wkt-boundary-file")
parser.add_argument("mep-matrix-file")
parser.add_argument("output_file")
parser.add_argument("chunksize", default=500_000)

wkt_boundary_file = ""
mep_matrix_file = ""
output_file = ""
chunksize = 50_000


def run():
    args = parser.parse_args()
    print("running mep analysis script with arguments:")
    print(json.dumps(vars(args), indent=4))
    with open(args.wkt_boundary_file) as f:
        bounds = wkt.loads(f.read())

    def hex_to_poly(hex_id: str) -> Polygon:
        coords = h3.cell_to_boundary(hex_id)
        polygon = Polygon([(t[1], t[0]) for t in coords])
        return polygon

    def in_bounds(row):
        return bounds.contains(row["geometry"])

    in_bounds_acc = []
    for df in tqdm(pd.read_csv(args.mep_matrix_file, chunksize=args.chunksize)):
        df["geometry"] = df.grid_id.apply(hex_to_poly)
        in_bounds_acc.append(df[df.geometry.apply(in_bounds)].copy())
    result = pd.concat(in_bounds_acc)
    print(f"finished. resulting dataset has {len(result)} rows.")
    result.to_csv(args.output_file, index=False)


if __name__ == "__main__":
    run()
