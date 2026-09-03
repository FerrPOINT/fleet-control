# Error Handling

Error response shape:

- `400`: invalid request or unsupported state transition.
- `401`: missing or invalid authentication.
- `403`: authenticated user lacks permission.
- `404`: resource not found or not visible to the current user.
- `409`: idempotency payload conflict, ordinal/path ownership conflict or
  invalid leader assignment conflict.
- `422`: semantically valid JSON that fails domain validation.
- `500`: unexpected internal failure with a correlation id in logs.

Guidelines:

- Do not leak filesystem roots beyond the safe path returned by the API.
- Do not return unredacted env values, tokens or process command secrets.
- Runtime failures should include typed reason codes where possible:
  `not_implemented`, `runtime_unavailable`, `health_failed`,
  `adapter_contract_error`.
- UI pages must show loading, empty, error, access denied and disabled mutation
  states.
