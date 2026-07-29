# Prela: A Compositional & Controllable Query Language

## Reproducibility

To fetch the data and set it up for the benchmark, run

```bash
./rust/bench/get_imdb.sh
./rust/bench/setup_tpch.sh
```

These scripts

- download the JOB dataset,
- clone the JOB schema and queries from the JOB [repo](https://github.com/gregrahn/join-order-benchmark),
- load the JOB CSVs into DuckDB and generate the parquet files
- and generate the Prela binary caches for JOB and TPC-H.

These scripts only need to be run once.
Downloading the JOB dataset and building the TPCH may each take a few minutes.

To collect the data and regenerate the charts, run

```bash
cd rust/bench/tpch
./run_tpch_all.sh

cd ../job
./run_job_all.sh
```

The charts are available at `rust/bench/{tpch,job}_scatter.{pdf,png}`.

You can expect the absolute numbers to vary from those presented in the paper, though the relative performance should be the same.

### Requirements

Tools:

- Rust 1.85+.
- `duckdb` version `1.5.x` must be on `PATH`.
- `python3` with `matplotlib` or `uv` (for plotting only, can skip with `PLOT=0`).
- `perl`, for a compatibility rename.

Disk:

- About 11GB free space.
