set -- gen-orb-mcp publish
[[ -n "${PUBLISH_NAME:-}" ]] && set -- "$@" --name "${PUBLISH_NAME}"
[[ -n "${INPUT:-}" ]] && set -- "$@" --input "${INPUT}"
[[ -n "${BINARY:-}" ]] && set -- "$@" --binary "${BINARY}"
[[ -n "${ASSET_NAME:-}" ]] && set -- "$@" --asset-name "${ASSET_NAME}"
[[ -n "${TAG:-}" ]] && set -- "$@" --tag "${TAG}"
[[ "${DRY_RUN:-false}" = "true" ]] && set -- "$@" --dry-run
"$@"
