# Orb Author Guide: Job Renames and Migration Rule Quality

This guide explains how to structure breaking changes in your orb so that
`gen-orb-mcp prime` generates correct `JobRenamed` migration rules
automatically, without manual correction.

## How rename detection works

`gen-orb-mcp prime` generates `migrations/<version>.json` by comparing the
orb snapshot at the previous tag with the snapshot at the current tag.  It
uses **two strategies** to detect job renames, applied in priority order:

1. **Git rename hints** — `git log <old_tag>..<new_tag> --diff-filter=R
   --name-status` is run against the orb repository.  Any file pair that git
   reports as a rename (e.g. `R085 src/jobs/old.yml src/jobs/new.yml`) is
   treated as an authoritative `old_job → new_job` hint and used directly.

2. **Jaccard parameter fallback** — For removed jobs not covered by a git
   hint, the tool compares parameter-name sets (Jaccard similarity ≥ 0.7)
   against jobs that are *truly new* (absent from the previous version).

Both strategies have limits.  Understanding those limits lets you structure
your commits so the automation works correctly.

---

## The fundamental constraint

Git rename detection only tracks **deleted → added** file pairs.  A file that
exists under the same name in both the old and new revision is treated as a
**modification**, never as a rename source or target.

This creates a blind spot for **job-family swaps** — the most common pattern
in a major version bump where a rolling variant replaces the standard name:

| Intent | Old file | New file |
|--------|----------|----------|
| Old standard becomes `_pinned` | `common_tests.yml` (deleted) | `common_tests_pinned.yml` (added) |
| Old rolling becomes new standard | `common_tests_rolling.yml` (deleted) | `common_tests.yml` (modified) |

When both renames are in **one commit**, git can only pair one deleted file
with the one added file (`common_tests_rolling.yml` →
`common_tests_pinned.yml` by content similarity).  The modification of
`common_tests.yml` goes undetected as a rename.

Result: `prime` generates `JobRenamed { from: "common_tests_rolling", to:
"common_tests_pinned" }` — the wrong target.

---

## Recommended practice: two-commit rename

Split a job-family swap across **two commits** so git can track each rename
independently.

### Step 1 — rename the existing standard job to `_pinned`

```bash
git mv src/jobs/common_tests.yml src/jobs/common_tests_pinned.yml
# update content: description, executor stanza, parameter names
git commit -s -m "refactor: rename common_tests -> common_tests_pinned (pinned executor)"
```

At this point `common_tests.yml` is gone and `common_tests_pinned.yml` is
new.  Git correctly records a rename with high similarity.

### Step 2 — rename the rolling job to the standard name

```bash
git mv src/jobs/common_tests_rolling.yml src/jobs/common_tests.yml
# update content: description cross-reference to _pinned variant
git commit -s -m "refactor: rename common_tests_rolling -> common_tests (rolling executor)"
```

Now `common_tests_rolling.yml` is gone and `common_tests.yml` is new.  Git
records a second independent rename.

### What `git log --diff-filter=R` reports after both commits

```
R092  src/jobs/common_tests.yml          src/jobs/common_tests_pinned.yml
R088  src/jobs/common_tests_rolling.yml  src/jobs/common_tests.yml
```

`prime` receives both hints and generates the correct rules:

```json
{ "type": "JobRenamed", "data": { "from": "common_tests_rolling", "to": "common_tests", "removed_parameters": [] } }
```

---

## When to apply the two-commit rule

Apply it any time a breaking release:

- removes a job that previously existed under a simpler name *and simultaneously* adds a new job under that same name
- renames a job to a name that was already occupied

In practice this arises in:

- **executor-tier swaps** — rolling variants become the default, pinned variants get a `_pinned` suffix
- **job consolidation** — two jobs merged into one that takes over the name of the more commonly used one
- **naming-scheme changes** — bulk renames where new names collide with old names

For straightforward renames (old name gone, new name never existed), a single
commit is fine: git detects the pair without ambiguity.

---

## Escape hatch: `--rename-map` (see issue #89)

When the commits cannot be restructured (e.g. history already pushed and
tagged), `prime` will support a `--rename-map OLD=NEW` flag that injects
authoritative hints directly, bypassing git detection:

```bash
gen-orb-mcp prime \
  --git-repo . \
  --tag-prefix "v" \
  --orb-path src/@orb.yml \
  --prior-versions-dir prior-versions \
  --migrations-dir migrations \
  --rename-map "common_tests_rolling=common_tests" \
  --rename-map "required_builds_rolling=required_builds"
```

Until that feature ships, incorrect migration rules must be corrected manually
in the generated JSON files before committing them to the repository.

---

## Checking generated rules

After running `prime`, inspect `migrations/<version>.json` before committing:

```bash
# Show all JobRenamed rules
jq '.[] | select(.type == "JobRenamed") | .data | "\(.from) -> \(.to) removed=\(.removed_parameters)"' \
  migrations/6.0.0.json
```

A rename is suspicious if the `to` name ends in `_pinned` or `_rolling` when
you intended to point consumers toward the plain standard name.  Cross-check
against the commit messages for that version range:

```bash
git log v5.3.10..v6.0.0 --oneline --diff-filter=R --name-status -- 'src/jobs/*.yml'
```

If git reports `R075 src/jobs/foo_rolling.yml src/jobs/foo_pinned.yml` but you
intended `foo_rolling → foo`, the two-commit rule was not followed and the
generated rule needs manual correction (or a `--rename-map` override).

---

## Summary

| Scenario | Commit structure | Automation outcome |
|----------|------------------|--------------------|
| Simple rename: `foo → bar` | Single commit OK | Correct `JobRenamed` generated |
| Family swap: `foo → foo_pinned`, `foo_rolling → foo` | **Two commits required** | Correct rules generated |
| Family swap in one commit | Single commit | Wrong target (`→ foo_pinned`); needs manual fix or `--rename-map` |
| Already released, history fixed | N/A | Use `--rename-map` override (issue #89) |
