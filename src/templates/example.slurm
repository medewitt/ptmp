#!/bin/bash -l
#SBATCH --partition=small
#SBATCH --job-name=EstimateR
#SBATCH --mail-type=BEGIN,END,FAIL
#SBATCH --mail-user=dewime23@wfu.edu
#SBATCH --account=kortessisGrp
#SBATCH --nodes=1
#SBATCH --tasks-per-node=20
#SBATCH --mem=128gb
#SBATCH --time=00-14:00:00

module load apps/r/4.3.3
bash /deac/bio/kortessisGrp/dewime23/measles-risk/run.sh