import test from "node:test";
import assert from "node:assert/strict";
import {
  REQUIRED_CHECKLIST_ITEMS,
  classifyPaths,
  validatePullRequest,
} from "./pr-policy.mjs";

function body({ classification = "internal", screenshots = "N/A — no visual changes" } = {}) {
  const userVisible = classification === "user" ? "x" : " ";
  const internalOnly = classification === "internal" ? "x" : " ";
  return `## Summary

Standardize pull request governance.

## Changes

- Add policy validation.

## Related issue

N/A

## Validation

- \`node --test .github/scripts/pr-policy.node.mjs\` — passed

## Screenshots

${screenshots}

## Change classification

- [${userVisible}] User-visible change
- [${internalOnly}] Internal-only change

## Checklist

${REQUIRED_CHECKLIST_ITEMS.map((item) => `- [x] ${item}`).join("\n")}
`;
}

function event(overrides = {}) {
  return {
    number: 12,
    repository: { full_name: "wrx012/linglux" },
    pull_request: {
      number: 12,
      title: "ci(github): enforce pull request policy",
      body: body(),
      draft: false,
      ...overrides,
    },
  };
}

function validate({ eventOverrides, changedFiles = ["CONTRIBUTING.md"], trackedOutputFiles = [], changelog = "" } = {}) {
  return validatePullRequest({
    event: event(eventOverrides),
    changedFiles,
    trackedOutputFiles,
    changelog,
  });
}

test("accepts a compliant internal-only documentation PR", () => {
  assert.deepEqual(validate().errors, []);
});

test("rejects invalid titles", () => {
  const result = validate({ eventOverrides: { title: "Update things." } });
  assert.match(result.errors[0], /PR title/);
});

test("rejects missing sections and unchecked checklist items", () => {
  const result = validate({ eventOverrides: { body: "## Summary\n\nIncomplete" } });
  assert.ok(result.errors.some((error) => error.includes("Changes")));
  assert.ok(result.errors.some((error) => error.includes("checklist item")));
});

test("requires exactly one change classification", () => {
  const invalidBody = body().replace("- [x] Internal-only change", "- [ ] Internal-only change");
  const result = validate({ eventOverrides: { body: invalidBody } });
  assert.ok(result.errors.some((error) => error.includes("exactly one")));
});

test("allows an optional issue to be N/A", () => {
  assert.deepEqual(validate().errors, []);
});

test("requires evidence or a specific reason for UI changes", () => {
  const result = validate({
    eventOverrides: { body: body({ screenshots: "N/A" }) },
    changedFiles: ["src/App.vue"],
  });
  assert.ok(result.errors.some((error) => error.includes("UI changes require")));
});

test("accepts a specific screenshot exception for UI changes", () => {
  const result = validate({
    eventOverrides: { body: body({ screenshots: "N/A — copy-only change with no layout impact" }) },
    changedFiles: ["src/App.vue"],
  });
  assert.deepEqual(result.errors, []);
});

test("requires a verified Unreleased changelog reference for user-visible changes", () => {
  const userBody = body({ classification: "user" });
  const missing = validate({ eventOverrides: { body: userBody }, changedFiles: ["src/App.vue"] });
  assert.ok(missing.errors.some((error) => error.includes("CHANGELOG.md")));

  const changelog = `# Changelog

## [Unreleased]

### Changed

- Added visible behavior. ([PR #12])

[PR #12]: https://github.com/wrx012/linglux/pull/12
`;
  const valid = validate({
    eventOverrides: { body: userBody },
    changedFiles: ["src/App.vue", "CHANGELOG.md"],
    changelog,
  });
  assert.deepEqual(valid.errors, []);
});

test("rejects tracked output artifacts", () => {
  const result = validate({ trackedOutputFiles: ["output/local-export.png"] });
  assert.ok(result.errors.some((error) => error.includes("must not be tracked")));
});

test("classifies validation paths", () => {
  assert.deepEqual(classifyPaths(["README.md"]), {
    frontend: false,
    rust: false,
    mediaCore: false,
    packaging: false,
    ui: false,
  });
  assert.equal(classifyPaths(["src/App.vue"]).frontend, true);
  assert.equal(classifyPaths(["src-tauri/src/lib.rs"]).rust, true);
  assert.equal(classifyPaths(["src-tauri/crates/linglux-media-core/src/task.rs"]).mediaCore, true);
  assert.equal(classifyPaths(["src-tauri/tauri.conf.json"]).packaging, true);
});
