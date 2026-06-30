set -- gen-orb-mcp save
set -- "$@" --paths "${PATHS}"
[[ -n "${MESSAGE:-}" ]] && set -- "$@" --message "${MESSAGE}"
[[ -n "${PUSH:-}" ]] && set -- "$@" --push "${PUSH}"
[[ "${NO_PUSH:-false}" = "true" ]] && set -- "$@" --no-push
[[ "${DRY_RUN:-false}" = "true" ]] && set -- "$@" --dry-run
[[ "${SIGN:-false}" = "true" ]] && set -- "$@" --sign
[[ -n "${CONFIG:-}" ]] && set -- "$@" --config "${CONFIG}"
[[ -n "${GPG_KEY_ENV:-}" ]] && set -- "$@" --gpg-key-env "${GPG_KEY_ENV}"
[[ -n "${TRUST_ENV:-}" ]] && set -- "$@" --trust-env "${TRUST_ENV}"
[[ -n "${USER_NAME_ENV:-}" ]] && set -- "$@" --user-name-env "${USER_NAME_ENV}"
[[ -n "${USER_EMAIL_ENV:-}" ]] && set -- "$@" --user-email-env "${USER_EMAIL_ENV}"
[[ -n "${SIGN_KEY_ENV:-}" ]] && set -- "$@" --sign-key-env "${SIGN_KEY_ENV}"
"$@"
