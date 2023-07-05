build:
    wrangler build
fmt:
    cargo +nightly fmt -- --config group_imports=StdExternalCrate,imports_granularity=Module
deploy:
    wrangler deploy