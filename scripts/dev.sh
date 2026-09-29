#!/usr/bin/env bash
# Run through `just dev`.
#
# Serves the front end on a free port rather than a fixed one, so memo runs beside other
# projects' dev servers: Vite takes the port from MEMO_PORT, and Tauri is pointed at it.
set -euo pipefail

port=$(deno eval 'const l = Deno.listen({ hostname: "127.0.0.1", port: 0 }); console.log(l.addr.port); l.close();')
export MEMO_PORT=$port
url="http://127.0.0.1:$port"

exec deno task tauri dev --config "{\"build\":{\"devUrl\":\"$url\"}}"
