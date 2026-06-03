set -- gen-orb-mcp diff
set -- "$@" --current "${CURRENT}"
set -- "$@" --previous "${PREVIOUS}"
set -- "$@" --since-version "${SINCE_VERSION}"
[[ -n "${OUTPUT:-}" ]] && set -- "$@" --output "${OUTPUT}"
"$@"
