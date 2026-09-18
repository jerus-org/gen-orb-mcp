set -- gen-orb-mcp migrate
[[ -n "${GCO_CI_DIR:-}" ]] && set -- "$@" --ci-dir "${GCO_CI_DIR}"
set -- "$@" --orb "${GCO_ORB}"
set -- "$@" --rules "${GCO_RULES}"
[[ "${GCO_DRY_RUN:-false}" = "true" ]] && set -- "$@" --dry-run
"$@"
