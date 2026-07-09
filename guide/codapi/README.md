# Codapi sandbox setup

This registers a `prela` sandbox with a [codapi](https://github.com/nalgeon/codapi)
server so the snippets in `guide/guide.md` are runnable in the browser.
See `guide/guide.md` for the embedding side (the `<codapi-snippet>` tags and
the CSS/JS include); this directory is only the server-side sandbox.

Follow codapi's own [install guide](https://github.com/nalgeon/codapi/blob/main/docs/install.md)
first to get a bare codapi server running on a dedicated machine (codapi runs
arbitrary submitted code — never colocate it with anything sensitive). Then:

1. Build the sandbox image. The Dockerfile `COPY`s `rust/`, so the build
   context must be the **repo root**, not this directory:

   ```sh
   cd /path/to/prela   # repo root
   docker build --file guide/codapi/sandboxes/prela/Dockerfile \
                --tag codapi/prela guide/codapi/../..
   # equivalently, from the repo root:
   docker build -f guide/codapi/sandboxes/prela/Dockerfile -t codapi/prela .
   ```

2. Copy (or symlink) `guide/codapi/sandboxes/prela/` into the codapi
   install's `sandboxes/` directory, alongside its own `box.json`/`commands.json`
   layout:

   ```sh
   cp -r guide/codapi/sandboxes/prela /opt/codapi/sandboxes/prela
   ```

3. Restart the codapi server. Its startup log should list `prela` in both
   `boxes` and `commands`.

4. Smoke-test directly against the API:

   ```sh
   curl -H "content-type: application/json" \
        -d '{"sandbox": "prela", "command": "run", "files": {"": "fn main() { println!(\"hello\"); }"}}' \
        http://localhost:1313/v1/exec
   ```

   Expect `"ok": true` and `stdout` containing `hello`.

5. Point `guide/guide.md`'s `<codapi-settings url="...">` at the running
   server (see `guide/codapi-include.html`).

## Rebuilding after engine changes

The image bakes in `rust/src` and `rust/Cargo.{toml,lock}` at build time — it
does **not** pick up engine changes automatically. Re-run step 1 whenever
`rust/src/engine.rs` (or its deps) changes and guide snippets need the update.

## Why no cache data

Guide snippets build small in-memory relations (`Dense::new`, `Sparse::from_pairs`,
literal `Vec`s, ...) rather than the JOB/TPC-H datasets, so the sandbox never
needs `cache/` (1.7 GB — far too large to bake into a snippet image). If a
future guide chapter wants to demonstrate queries over real data, that needs
a separate, much smaller sample cache and its own sandbox variant.
