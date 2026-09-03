# Troubleshooting

Rust linker missing:

- Symptom: `error: linker link.exe not found`.
- Fix: install Visual Studio Build Tools with C++ workload or run the backend
  gate in Linux/CI.
- WSL Ubuntu is an acceptable local Linux gate when Rust is installed there.

Frontend cannot reach API:

- Check backend on `127.0.0.1:23801`.
- In Playwright/screenshot mode, API calls may be mocked.

Session list shows only one user:

- This is default behavior.
- Multi-user and all-user filters require admin/operator permissions.

Leader selector is empty:

- Verify the primary agent is an executor.
- Verify `leader_executors` contains the leader/executor pair.

Java Agent actions fail:

- Expected in phase 2; operations return typed `not_implemented`.

Unexpected files in agent folder:

- Check `.fleet-agent.json` marker.
- Do not reuse or overwrite folders outside `agents_root`.
