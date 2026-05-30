set -- gen-orb-mcp publish
set -- "$@" --binary "${BINARY}"
set -- "$@" --asset-name "${ASSET_NAME}"
[[ -n "${TAG:-}" ]] && set -- "$@" --tag "${TAG}"
[[ "${DRY_RUN:-false}" = "true" ]] && set -- "$@" --dry-run
"$@"
