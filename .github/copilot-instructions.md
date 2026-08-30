# Copilot code review instructions

Before reviewing a pull request, read and follow the repository-root `AGENTS.md`.
Treat `DESIGN.md` and `MEDIA_CORE.md` as the architecture and media-core sources of
truth for the areas they cover.

Write review feedback in Chinese while keeping code identifiers, paths, commands,
and established technical terms in English.

Prioritize actionable findings about:

- correctness and regressions;
- security, secrets, permissions, and unsafe path handling;
- local media durability or possible data loss;
- Vue/TypeScript and Tauri/Rust request-response compatibility;
- task progress, cancellation, cleanup, and recovery;
- compatibility with older saved projects and serde/default normalization;
- missing or insufficient tests for changed behavior.

Use inline comments only when the finding points to a tight, actionable code range.
Prefix each finding title with one severity marker:

- `[P0]` — immediate security or destructive data-loss risk;
- `[P1]` — correctness issue that should block merging;
- `[P2]` — meaningful reliability or maintainability issue;
- `[P3]` — low-risk improvement worth addressing.

Avoid compliments, summaries that repeat the pull request description, speculative
concerns without a concrete failure mode, and style-only comments already handled by
the repository conventions. If there are no actionable findings, leave no inline
comments.
