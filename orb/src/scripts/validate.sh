set -- gen-orb-mcp validate
[[ -n "${ORB_PATH:-}" ]] && set -- "$@" --orb-path "${ORB_PATH}"
"$@"
