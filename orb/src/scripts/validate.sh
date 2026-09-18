set -- gen-orb-mcp validate
[[ -n "${GCO_ORB_PATH:-}" ]] && set -- "$@" --orb-path "${GCO_ORB_PATH}"
"$@"
