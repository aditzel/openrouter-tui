# Contributing

This repository is a placeholder for a future OpenRouter terminal UI. It has no
application code, build or tests yet. It currently contains `JUNIE.md` and the
manually dispatched Junie workflow in `.github/workflows/junie.yml`.

## Commit messages and pull request titles

New commits and pull request titles should follow
[Conventional Commits](https://www.conventionalcommits.org/en/v1.0.0/):

```text
<type>(<optional scope>): <short summary>
```

Use a type such as `feat`, `fix`, `docs`, `chore` or `ci`. For example,
`docs: describe planned features` or `chore: add project scaffold`.

This applies to new work only. Existing history is not rewritten, and no
automated title or commit-message check is enforced.

## Agent contributions

When a coding agent writes or co-writes a commit, add a `Co-authored-by` trailer
for that agent at the end of the commit message. For Factory Droid, use:

```text
Co-authored-by: factory-droid[bot] <138933559+factory-droid[bot]@users.noreply.github.com>
```

Commits made without agent involvement do not need the trailer.
