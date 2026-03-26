#!/bin/bash -l
#SBATCH --partition=small
#SBATCH --job-name={{ project_name }}
#SBATCH --mail-type=BEGIN,END,FAIL
#SBATCH --mail-user={{ slurm_email }}
#SBATCH --account={{ slurm_account }}
#SBATCH --nodes=1
#SBATCH --tasks-per-node=20
#SBATCH --mem=128gb
#SBATCH --time=00-14:00:00

echo "Running {{ title }}"
