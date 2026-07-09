# Codapi sandbox for the Prela user guide. Bakes the `prela` crate in as a
# path dependency so guide snippets compile against the real engine — no JOB
# / TPC-H cache needed, since guide examples build small in-memory relations
# (Dense/Sparse/InVec literals) rather than loading `cache/`.
FROM rust:slim-bookworm

RUN adduser --home /sandbox --disabled-password sandbox

# Only the library is needed (engine.rs et al.) — not the cache/ directory.
COPY rust/Cargo.toml rust/Cargo.lock /opt/prela/
COPY rust/src /opt/prela/src

# Throwaway crate depending on `prela`, built once at image-build time so the
# registry + target caches are warm: a submitted snippet then only compiles
# its own main.rs, not prela's dependency tree from scratch.
RUN cargo new --bin /opt/playground \
    && printf '\n[dependencies]\nprela = { path = "/opt/prela" }\n' >> /opt/playground/Cargo.toml \
    && cd /opt/playground && cargo build

RUN chown -R sandbox:sandbox /opt/prela /opt/playground "$CARGO_HOME"

USER sandbox
WORKDIR /sandbox
