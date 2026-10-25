set -- gen-orb-mcp prime
[[ ! "${GCO_ORB_PATH:-}" =~ ^[[:space:]]*$ ]] && set -- "$@" --orb-path "${GCO_ORB_PATH}"
[[ ! "${GCO_GIT_REPO:-}" =~ ^[[:space:]]*$ ]] && set -- "$@" --git-repo "${GCO_GIT_REPO}"
[[ ! "${GCO_TAG_PREFIX:-}" =~ ^[[:space:]]*$ ]] && set -- "$@" --tag-prefix "${GCO_TAG_PREFIX}"
[[ ! "${GCO_EARLIEST_VERSION:-}" =~ ^[[:space:]]*$ ]] && set -- "$@" --earliest-version "${GCO_EARLIEST_VERSION}"
[[ ! "${GCO_SINCE:-}" =~ ^[[:space:]]*$ ]] && set -- "$@" --since "${GCO_SINCE}"
[[ ! "${GCO_PRIOR_VERSIONS_DIR:-}" =~ ^[[:space:]]*$ ]] && set -- "$@" --prior-versions-dir "${GCO_PRIOR_VERSIONS_DIR}"
[[ ! "${GCO_MIGRATIONS_DIR:-}" =~ ^[[:space:]]*$ ]] && set -- "$@" --migrations-dir "${GCO_MIGRATIONS_DIR}"
[[ "${GCO_EPHEMERAL:-false}" = "true" ]] && set -- "$@" --ephemeral
[[ ! "${GCO_RENAME_MAP:-}" =~ ^[[:space:]]*$ ]] && set -- "$@" --rename-map "${GCO_RENAME_MAP}"
[[ "${GCO_DRY_RUN:-false}" = "true" ]] && set -- "$@" --dry-run
"$@"
