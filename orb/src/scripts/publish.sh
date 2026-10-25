set -- gen-orb-mcp publish
[[ ! "${GCO_PUBLISH_NAME:-}" =~ ^[[:space:]]*$ ]] && set -- "$@" --name "${GCO_PUBLISH_NAME}"
[[ ! "${GCO_INPUT:-}" =~ ^[[:space:]]*$ ]] && set -- "$@" --input "${GCO_INPUT}"
[[ ! "${GCO_BINARY:-}" =~ ^[[:space:]]*$ ]] && set -- "$@" --binary "${GCO_BINARY}"
[[ ! "${GCO_ASSET_NAME:-}" =~ ^[[:space:]]*$ ]] && set -- "$@" --asset-name "${GCO_ASSET_NAME}"
[[ ! "${GCO_TAG:-}" =~ ^[[:space:]]*$ ]] && set -- "$@" --tag "${GCO_TAG}"
[[ ! "${GCO_TAG_ENV:-}" =~ ^[[:space:]]*$ ]] && set -- "$@" --tag-env "${GCO_TAG_ENV}"
[[ ! "${GCO_CONFIG:-}" =~ ^[[:space:]]*$ ]] && set -- "$@" --config "${GCO_CONFIG}"
[[ "${GCO_DRY_RUN:-false}" = "true" ]] && set -- "$@" --dry-run
"$@"
