set -- gen-orb-mcp prime
[[ -n "${ORB_PATH:-}" ]] && set -- "$@" --orb-path "${ORB_PATH}"
[[ -n "${GIT_REPO:-}" ]] && set -- "$@" --git-repo "${GIT_REPO}"
[[ -n "${TAG_PREFIX:-}" ]] && set -- "$@" --tag-prefix "${TAG_PREFIX}"
[[ -n "${EARLIEST_VERSION:-}" ]] && set -- "$@" --earliest-version "${EARLIEST_VERSION}"
[[ -n "${SINCE:-}" ]] && set -- "$@" --since "${SINCE}"
[[ -n "${PRIOR_VERSIONS_DIR:-}" ]] && set -- "$@" --prior-versions-dir "${PRIOR_VERSIONS_DIR}"
[[ -n "${MIGRATIONS_DIR:-}" ]] && set -- "$@" --migrations-dir "${MIGRATIONS_DIR}"
[[ "${EPHEMERAL:-false}" = "true" ]] && set -- "$@" --ephemeral
[[ -n "${RENAME_MAP:-}" ]] && set -- "$@" --rename-map "${RENAME_MAP}"
[[ "${DRY_RUN:-false}" = "true" ]] && set -- "$@" --dry-run
"$@"
