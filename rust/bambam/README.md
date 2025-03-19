BAMBAM!
=======

NREL **B**ehavior and **A**dvanced **M**obility group's **B**ig **A**ccess **M**odel 

Access modeling toolkit for Rust built on the RouteE Compass energy-aware route planner.

## Run Contexts

### Local

For this example, it is assumed you have also installed the routee-compass-tomtom repo on your computer, and have placed it in the same parent directory as this repo:
```
/your/file/system/
  mep-routee-compass/
  routee-compass-tomtom/
```

To run locally, first compile for release via cargo (comes with a [rust](https://www.rust-lang.org/tools/install) installation):

```
$ cargo build -r
```

Review the app's command line usage:

```
$ target/release/mep-routee-compass --help
The Mobility, Energy and Productivity Metric Built on the RouteE Compass Energy-Aware Route Planner

Usage: mep-routee-compass <CONFIG_FILE> <QUERY_FILE>

Arguments:
  <CONFIG_FILE>  
  <QUERY_FILE>   

Options:
  -h, --help     Print help
  -V, --version  Print version
```

The app takes two arguments, first a configuration TOML file and second, a file with the mep grid. See "configuration/" and "queries/" for examples. This configuration is a good starting point, it loads only the denver metro region and has a single grid cell. 

```
$ target/release/mep-routee-compass configuration/mep_all_denver.toml queries/denver_h3_8_acs2022_single.json
edge list: 100%|██████████████████████████████████████████████████████████| 928123/928123 [00:01<00:00, 528157.56it/s]
vertex list: 100%|████████████████████████████████████████████████████████| 425045/425045 [00:00<00:00, 760595.62it/s]
geometry file: 100%|██████████████████████████████████████████████████████| 928123/928123 [00:01<00:00, 521279.25it/s]
input plugins: 100%|██████████████████████████████████████████████████████████████████| 1/1 [00:00<00:00, 1149.72it/s]
search: 100%|████████████████████████████████████████████████████████████████████████████| 3/3 [00:07<00:00, 0.40it/s]
```

The results are in an output JSON file, `mep_denver_output.json`, in newline-delimited format. This can be loaded in Python via Pandas:

```python
import pandas as pd
df = pd.read_json("mep_denver_output.json", lines=True)
```

### Kestrel

#### Download

First, pull down this repository to a location on Kestrel. This depends on the git-lfs module:

```
$ module load git-lfs
$ git clone https://github.nrel.gov/rfitzger/mep-routee-compass.git
$ cd mep-routee-compass
```

#### Compilation

We need a rust compiler, which can be installed by creating a conda environment:

```
$ module load anaconda3
$ conda create -n rust python=3.10 rust
$ conda activate rust
```

Next, compile the mep app. Kestrel's default 'Cray' supercomputing environment can be swapped for a 'GNU' programming stack for compilation:

```
$ module swap PrgEnv-cray PrgEnv-gnu
```

Then build the repo:

```
$ cargo build -r
```

#### Run (Kestrel)

First, we need to activate the bzip2 module, which is a dependency of the gtfs-structures library:

```
$ module load bzip2
```