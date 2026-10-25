set -- gen-orb-mcp validate
[[ ! "${GCO_ORB_PATH:-}" =~ ^[[:space:]]*$ ]] && set -- "$@" --orb-path "${GCO_ORB_PATH}"
"$@"
