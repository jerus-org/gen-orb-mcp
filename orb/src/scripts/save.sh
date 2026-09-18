set -- gen-orb-mcp save
set -- "$@" --paths "${GCO_PATHS}"
[[ -n "${GCO_MESSAGE:-}" ]] && set -- "$@" --message "${GCO_MESSAGE}"
[[ -n "${GCO_PUSH:-}" ]] && set -- "$@" --push "${GCO_PUSH}"
[[ "${GCO_NO_PUSH:-false}" = "true" ]] && set -- "$@" --no-push
[[ "${GCO_DRY_RUN:-false}" = "true" ]] && set -- "$@" --dry-run
[[ "${GCO_SIGN:-false}" = "true" ]] && set -- "$@" --sign
[[ -n "${GCO_CONFIG:-}" ]] && set -- "$@" --config "${GCO_CONFIG}"
[[ -n "${GCO_GPG_KEY_ENV:-}" ]] && set -- "$@" --gpg-key-env "${GCO_GPG_KEY_ENV}"
[[ -n "${GCO_TRUST_ENV:-}" ]] && set -- "$@" --trust-env "${GCO_TRUST_ENV}"
[[ -n "${GCO_USER_NAME_ENV:-}" ]] && set -- "$@" --user-name-env "${GCO_USER_NAME_ENV}"
[[ -n "${GCO_USER_EMAIL_ENV:-}" ]] && set -- "$@" --user-email-env "${GCO_USER_EMAIL_ENV}"
[[ -n "${GCO_SIGN_KEY_ENV:-}" ]] && set -- "$@" --sign-key-env "${GCO_SIGN_KEY_ENV}"
"$@"
