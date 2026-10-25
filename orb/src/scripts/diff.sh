set -- gen-orb-mcp diff
set -- "$@" --current "${GCO_CURRENT}"
set -- "$@" --previous "${GCO_PREVIOUS}"
set -- "$@" --since-version "${GCO_SINCE_VERSION}"
[[ ! "${GCO_OUTPUT:-}" =~ ^[[:space:]]*$ ]] && set -- "$@" --output "${GCO_OUTPUT}"
"$@"
