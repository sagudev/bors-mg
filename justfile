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
