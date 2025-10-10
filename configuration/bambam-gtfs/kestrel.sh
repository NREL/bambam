#!/bin/sh

#SBATCH --job-name=bambam-gtfs
#SBATCH --nodes=1
#SBATCH --ntasks=100
#SBATCH --mem=240GB
#SBATCH --tmp=500GB
#SBATCH --mail-type=ALL

module load bzip2

/projects/mepcore/bin/bambam_gtfs \
    preprocess-bundle \
    --input /home/$USER/data/bam/gtfs/2025-10-08 \
    --starting-edge-list-id 1 \
    --parallelism 50 \
    --output-directory "/projects/mepcore/data/out/rfitzger/2025-10-08-gtfs" \
    --vertices-compass-filename "/projects/mepcore/lib/routee-compass-tomtom/data/tomtom_national/vertices-complete.csv.gz" \
    --start-date 09-01-2025 \
    --end-date 09-01-2025