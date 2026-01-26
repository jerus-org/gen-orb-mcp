#!/bin/bash
set -exo pipefail

# Usage: ./scripts/release-prlog.sh <version>
# Example: ./scripts/release-prlog.sh 0.1.0
#
# PRLOG uses standard v prefix tags (v0.1.0) for workspace releases.

VERSION="${1}"
DATE=$(date +%Y-%m-%d)
TAG="v${VERSION}"

if [[ -z "${VERSION}" ]]; then
    echo "Usage: $0 <version>" >&2
    exit 1
fi

# Update PRLOG.md - replace Unreleased with version and date
sed -i "s/## \[Unreleased\]/## [${VERSION}] - ${DATE}/" PRLOG.md

# Add new Unreleased section after the header
sed -i "/## \[${VERSION}\]/i ## [Unreleased]\n" PRLOG.md

# Commit and tag
git add PRLOG.md
git commit -S -s -m "chore: Release PRLOG v${VERSION}"
git tag -s -m "${TAG}" "${TAG}"

echo "PRLOG released as ${TAG}"
