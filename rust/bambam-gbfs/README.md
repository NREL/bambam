# bambam-gbfs

tooling for building and running BAMBAM models from GBFS sources.

## import script

example CLI calls:

### top-level GBFS CLI

```
% ./target/release/bambam-gbfs
GBFS Extensions for The Behavior and Advanced Mobility Big Access Model

Usage: bambam-gbfs <COMMAND>

Commands:
  download  runs a GBFS download, writing data from some source URL to an output directory
  help      Print this message or the help of the given subcommand(s)

Options:
  -h, --help     Print help
  -V, --version  Print version
```

### GBFS download command

```
% cd rust
% cargo build -r 
% ./target/release/bambam-gbfs download --help
runs a GBFS download, writing data from some source URL to an output directory

Usage: bambam-gbfs download [OPTIONS] --gbfs-url <GBFS_URL>

Options:
  -g, --gbfs-url <GBFS_URL>
          a GBFS API URL
  -o, --output-directory <OUTPUT_DIRECTORY>
          output directory path [default: .]
  -c, --collect-duration <COLLECT_DURATION>
          duration to collect data rows. provide in human-readable time values 2m, 30s, 2h, 2days... [default: 10m]
  -h, --help
          Print help
  -V, --version
          Print version
```

### running GBFS download with arguments

this isn't yet implemented so with debug logging enabled, the command will run the download function, log the arguments, then panic when it hits the todo!() line.

```
% RUST_LOG=debug ./target/release/bambam-gbfs download -g https://example.com
[2025-12-22T17:15:15Z DEBUG bambam_gbfs::app::download::run] run_gbfs_download with url=https://example.com, out_dir=".", duration (seconds)=600

thread 'main' (2745448) panicked at bambam-gbfs/src/app/download/run.rs:20:5:
not yet implemented: download + post-processing logic
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
```