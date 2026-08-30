# Contributing to Linglux

Linglux uses enforced Issue and pull request policies. Use the repository forms for new Issues, and ensure pull requests targeting `main` pass every required status check before merging.

## Issues

Use the Bug report, Feature request, or Question form. Blank Issues are disabled. Do not use a public Issue for vulnerabilities, exposed credentials, private media, personal data, or other sensitive material; follow `SECURITY.md` and use GitHub Private Vulnerability Reporting.

Issue titles must use this format:

```text
type(area): concise summary
```

- Use `bug`, `feat`, or `question` as the type selected by the form.
- Use one primary area: `agent`, `desktop`, `editor`, `export`, `media-core`, `storyboard`, `tts`, `workflow`, or `repository`.
- Make the title area match the Primary area field, keep the complete title at 100 characters or fewer, and do not end it with a period.
- Complete every required form field and sanitize diagnostics before submission.

Issue automation assigns the type and primary area labels, validates required information, and initializes a priority from the structured Impact response. Questions default to `priority: low`. Maintainers may override priority, add secondary labels, and move the Issue through `status: needs triage`, `status: in progress`, `status: blocked`, or `status: next release`.

Incomplete Issues remain open with `status: needs info`. Edit the title or description to address the single policy comment; validation reruns automatically and removes that label after correction.

## Pull Request Titles

Use this format:

```text
type(scope): imperative summary
```

Allowed types are `feat`, `fix`, `refactor`, `perf`, `docs`, `test`, `build`, `ci`, `chore`, and `revert`.

- Use a lowercase scope containing letters, numbers, or hyphens.
- Write a concise imperative summary, such as `fix(editor): preserve audio sync while scrubbing`.
- Keep the complete title at 100 characters or fewer.
- Do not end the title with a period.

## Pull Request Description

Complete every section of the pull request template. An Issue link is optional; write `N/A` when no Issue applies.

Choose exactly one change classification:

- **User-visible change:** behavior, UI, workflows, supported platforms, storage, export, or another capability experienced by users changes.
- **Internal-only change:** documentation, tests, CI, maintenance, or an implementation refactor with no user-visible effect.

UI changes must include screenshots or recordings. If visual evidence is not applicable, write `N/A — <reason>` in the Screenshots section.

## Validation

Run the narrowest reliable checks required by `AGENTS.md` and report every command and result in the Validation section. CI selects checks from the changed paths:

- Frontend changes run `pnpm test` and `pnpm build` after a frozen-lockfile install.
- Rust and Tauri source changes run `cargo check --manifest-path src-tauri/Cargo.toml`.
- Media-core changes additionally run `cargo test --manifest-path src-tauri/Cargo.toml -p linglux-media-core`.
- Packaging, permissions, window, bundle, or Tauri configuration changes run `pnpm tauri:build` on macOS.
- Documentation-only changes skip application builds but still run PR policy checks.

Do not claim a command passed unless it was executed against the pull request head after a clean dependency install when dependency or lockfile behavior is relevant.

## Changelog Policy

User-visible changes must add an entry under `Unreleased` in `CHANGELOG.md`. The entry must reference the current pull request, and the reference definition must point to its verified GitHub URL:

```markdown
- Added the user-visible capability. ([PR #123])

[PR #123]: https://github.com/wrx012/linglux/pull/123
```

Do not invent Issue, pull request, release, or contributor references. Internal-only changes do not require a changelog entry.

## Review and Labels

Ready-for-review pull requests enter the automated review workflow. Review labels communicate automation state but do not replace human approval:

- `review: pending` — automated review is still running.
- `review: complete` — automated review reported no inline findings; human review is still required.
- `review: needs attention` — review findings require attention.
- `status: blocked` — a known dependency or merge-blocking problem remains.

Pull request area and priority labels may be applied by maintainers. Authors should update the pull request after every material revision and resolve review findings before requesting another review.

## Repository Hygiene

Never commit credentials, local media, generated build output, dependency directories, Tauri build artifacts, or development exports under `output/`. Keep frontend/Tauri request-response contracts, saved project compatibility, cancellation behavior, and local media durability intact when changing their respective areas.
