set -- gen-orb-mcp migrate
[ -n "${CI_DIR:-}" ] && set -- "$@" --ci-dir "${CI_DIR}"
set -- "$@" --orb "${ORB}"
set -- "$@" --rules "${RULES}"
[ "${DRY_RUN:-false}" = "true" ] && set -- "$@" --dry-run
"$@"