# Example: create isochrones

### 1. query file

An isochrone query file is a JSON object should at least contain the key "extent", which should be a WGS:84 [WKT](https://en.wikipedia.org/wiki/Well-known_text_representation_of_geometry) POLYGON or MULTIPOLYGON in (x,y) format:

```python
# query_file.json
{
  "extent": "POLYGON((..))"
}
```

### 2. working directory

The script will choose your current working directory for output. As an output can be quite large, this should always be run in your scratch directory:

```bash
$ cd /scratch/$USER
```

### 3. run 

The script for executing an isochrone search is `/projects/mepcore/bin/bambam_isochrone.sh`. You submit it to [slurm](https://slurm.schedmd.com/documentation.html) with your query file as an argument. You must provide an `--account` to sbatch to charge for your HPC usage and may want to include your email with the `--mail-user` argument for notifications. The `--time` is a slurm argument for a time limit.

```bash
$ sbatch --account mepcore --time=2:00:00 --mail-user=my.email@nrel.gov /projects/mepcore/bin/bambam_isochrone.sh /home/$USER/data/mep/query_file.json
```

### 4. outputs

#### reviewing output files

A successful run produces the following files in your output location:

filename | description
--- | ---
isochrones.csv | all isochrones as a flat file
complete.json | the complete set of fields on each response row as newline-delimited JSON
errors.csv | any error descriptions for failed rows
slurm-$JOB_ID.out | the stdout for the slurm process

You may want to delete complete.json as it is fairly large, and, compress isochrones.csv before downloading.

#### file contents

The isochrones.csv file has the source h3 hex and WKB isochrones for each time bin and travel mode. This can be processed with GeoPandas, Pandas and Shapely like this:

```python
import pandas as pd
import geopandas as gpd
from shapely import wkb

df = pd.read_csv("isochrones.csv")
for t in [10,20,30,40]:
  col = f'isochrones_{t}'
  df[col] = df[col].apply(wkb.loads)

# look at some drive-mode 20-minute isochrones
gdf = gpd.GeoDataFrame(df[df['mode']=='drive'], geometry='isochrone_20', crs="EPSG:4326")
gdf.sample(n=100).plot()
```
