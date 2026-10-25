set -- gen-orb-mcp build
set -- "$@" --input "${GCO_INPUT}"
[[ ! "${GCO_BUILD_NAME:-}" =~ ^[[:space:]]*$ ]] && set -- "$@" --name "${GCO_BUILD_NAME}"
[[ ! "${GCO_TARGET:-}" =~ ^[[:space:]]*$ ]] && set -- "$@" --target "${GCO_TARGET}"
[[ "${GCO_DRY_RUN:-false}" = "true" ]] && set -- "$@" --dry-run
"$@"
