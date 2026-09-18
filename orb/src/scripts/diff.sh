set -- gen-orb-mcp diff
set -- "$@" --current "${GCO_CURRENT}"
set -- "$@" --previous "${GCO_PREVIOUS}"
set -- "$@" --since-version "${GCO_SINCE_VERSION}"
[[ -n "${GCO_OUTPUT:-}" ]] && set -- "$@" --output "${GCO_OUTPUT}"
"$@"
