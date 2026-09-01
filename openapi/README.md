# Fleet Control OpenAPI

`openapi.json` is generated from the Rust API with:

```bash
cd backend
cargo run -p api --bin gen-openapi > ../openapi/openapi.json
```

The checked-in file is intentionally kept small in the first scaffold so the
frontend build can generate types before the backend toolchain is available on a
fresh workstation.
