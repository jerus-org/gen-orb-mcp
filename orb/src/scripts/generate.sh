set -- gen-orb-mcp generate
[[ -n "${GCO_ORB_PATH:-}" ]] && set -- "$@" --orb-path "${GCO_ORB_PATH}"
[[ -n "${GCO_OUTPUT:-}" ]] && set -- "$@" --output "${GCO_OUTPUT}"
[[ -n "${GCO_FORMAT:-}" ]] && set -- "$@" --format "${GCO_FORMAT}"
[[ -n "${GCO_GENERATE_NAME:-}" ]] && set -- "$@" --name "${GCO_GENERATE_NAME}"
[[ -n "${GCO_CRATE_VERSION:-}" ]] && set -- "$@" --crate-version "${GCO_CRATE_VERSION}"
[[ "${GCO_FORCE:-false}" = "true" ]] && set -- "$@" --force
[[ -n "${GCO_MIGRATIONS:-}" ]] && set -- "$@" --migrations "${GCO_MIGRATIONS}"
[[ -n "${GCO_PRIOR_VERSIONS:-}" ]] && set -- "$@" --prior-versions "${GCO_PRIOR_VERSIONS}"
[[ -n "${GCO_TAG_PREFIX:-}" ]] && set -- "$@" --tag-prefix "${GCO_TAG_PREFIX}"
"$@"
