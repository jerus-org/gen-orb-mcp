set -- gen-orb-mcp build
set -- "$@" --input "${GCO_INPUT}"
[[ -n "${GCO_BUILD_NAME:-}" ]] && set -- "$@" --name "${GCO_BUILD_NAME}"
[[ -n "${GCO_TARGET:-}" ]] && set -- "$@" --target "${GCO_TARGET}"
[[ "${GCO_DRY_RUN:-false}" = "true" ]] && set -- "$@" --dry-run
"$@"
