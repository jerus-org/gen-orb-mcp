set -- gen-orb-mcp publish
[[ -n "${GCO_PUBLISH_NAME:-}" ]] && set -- "$@" --name "${GCO_PUBLISH_NAME}"
[[ -n "${GCO_INPUT:-}" ]] && set -- "$@" --input "${GCO_INPUT}"
[[ -n "${GCO_BINARY:-}" ]] && set -- "$@" --binary "${GCO_BINARY}"
[[ -n "${GCO_ASSET_NAME:-}" ]] && set -- "$@" --asset-name "${GCO_ASSET_NAME}"
[[ -n "${GCO_TAG:-}" ]] && set -- "$@" --tag "${GCO_TAG}"
[[ -n "${GCO_TAG_ENV:-}" ]] && set -- "$@" --tag-env "${GCO_TAG_ENV}"
[[ -n "${GCO_CONFIG:-}" ]] && set -- "$@" --config "${GCO_CONFIG}"
[[ "${GCO_DRY_RUN:-false}" = "true" ]] && set -- "$@" --dry-run
"$@"
