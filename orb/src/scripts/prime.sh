set -- gen-orb-mcp prime
[[ -n "${GCO_ORB_PATH:-}" ]] && set -- "$@" --orb-path "${GCO_ORB_PATH}"
[[ -n "${GCO_GIT_REPO:-}" ]] && set -- "$@" --git-repo "${GCO_GIT_REPO}"
[[ -n "${GCO_TAG_PREFIX:-}" ]] && set -- "$@" --tag-prefix "${GCO_TAG_PREFIX}"
[[ -n "${GCO_EARLIEST_VERSION:-}" ]] && set -- "$@" --earliest-version "${GCO_EARLIEST_VERSION}"
[[ -n "${GCO_SINCE:-}" ]] && set -- "$@" --since "${GCO_SINCE}"
[[ -n "${GCO_PRIOR_VERSIONS_DIR:-}" ]] && set -- "$@" --prior-versions-dir "${GCO_PRIOR_VERSIONS_DIR}"
[[ -n "${GCO_MIGRATIONS_DIR:-}" ]] && set -- "$@" --migrations-dir "${GCO_MIGRATIONS_DIR}"
[[ "${GCO_EPHEMERAL:-false}" = "true" ]] && set -- "$@" --ephemeral
[[ -n "${GCO_RENAME_MAP:-}" ]] && set -- "$@" --rename-map "${GCO_RENAME_MAP}"
[[ "${GCO_DRY_RUN:-false}" = "true" ]] && set -- "$@" --dry-run
"$@"
