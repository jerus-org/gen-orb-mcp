set -- gen-orb-mcp save
set -- "$@" --paths "${PATHS}"
[[ -n "${MESSAGE:-}" ]] && set -- "$@" --message "${MESSAGE}"
[[ -n "${PUSH:-}" ]] && set -- "$@" --push "${PUSH}"
[[ "${NO_PUSH:-false}" = "true" ]] && set -- "$@" --no-push
[[ "${DRY_RUN:-false}" = "true" ]] && set -- "$@" --dry-run
[[ "${SIGN:-false}" = "true" ]] && set -- "$@" --sign
"$@"
