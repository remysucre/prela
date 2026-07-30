Prela's JOB cache differs from the canonical schema in that Prela's `Company` entity is `movie_companies ⋈ company_name`.
This raises a concern: perhaps Prela's performance on JOB is an artifact of the schema.

To rule out this hypothesis, `run_job_prela_schema.sh` builds two DuckDB databases according to the two schemas (the canonical one, and Prela's schema).
It then times both versions.

Results: DuckDB under the canonical schema takes 15.44 s while under the Prela schema takes 15.97 s.
When comparing Prela to DuckDB on JOB, DuckDB uses the canonical schema.
This suggests that Prela's schema does not confer it an unfair advantage over DuckDB on the JOB benchmark.
