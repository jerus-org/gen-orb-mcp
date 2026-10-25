set -- gen-orb-mcp generate
[[ ! "${GCO_ORB_PATH:-}" =~ ^[[:space:]]*$ ]] && set -- "$@" --orb-path "${GCO_ORB_PATH}"
[[ ! "${GCO_OUTPUT:-}" =~ ^[[:space:]]*$ ]] && set -- "$@" --output "${GCO_OUTPUT}"
[[ ! "${GCO_FORMAT:-}" =~ ^[[:space:]]*$ ]] && set -- "$@" --format "${GCO_FORMAT}"
[[ ! "${GCO_GENERATE_NAME:-}" =~ ^[[:space:]]*$ ]] && set -- "$@" --name "${GCO_GENERATE_NAME}"
[[ ! "${GCO_CRATE_VERSION:-}" =~ ^[[:space:]]*$ ]] && set -- "$@" --crate-version "${GCO_CRATE_VERSION}"
[[ "${GCO_FORCE:-false}" = "true" ]] && set -- "$@" --force
[[ ! "${GCO_MIGRATIONS:-}" =~ ^[[:space:]]*$ ]] && set -- "$@" --migrations "${GCO_MIGRATIONS}"
[[ ! "${GCO_PRIOR_VERSIONS:-}" =~ ^[[:space:]]*$ ]] && set -- "$@" --prior-versions "${GCO_PRIOR_VERSIONS}"
[[ ! "${GCO_TAG_PREFIX:-}" =~ ^[[:space:]]*$ ]] && set -- "$@" --tag-prefix "${GCO_TAG_PREFIX}"
"$@"
