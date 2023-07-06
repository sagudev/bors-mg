# Install wrangler
bootstrap:
    cargo install -q worker-build
# Build wasm
build:
    wrangler build
# Strict formating
fmt:
    cargo +nightly fmt -- --config group_imports=StdExternalCrate,imports_granularity=Module
# Build&Deploy
deploy:
    wrangler deploy
# Log on worker
log:
    wrangler tail
work:
    just fmt
    clear
    wrangler deploy
    wrangler tail