import test from "node:test";
import assert from "node:assert/strict";
import {
  buildLabelPlan,
  inferPriority,
  parseAutomaticArea,
  parseAutomaticPriority,
  renderPolicyComment,
  validateIssue,
} from "./issue-policy.mjs";

const acknowledgement = "- [x] I confirm that this report contains no sensitive data.";

function bugBody({ area = "Editor", impact = "High — Blocks a core workflow", acknowledge = true } = {}) {
  return `### Problem

Preview playback stops unexpectedly.

### Steps to reproduce

1. Import a clip.\n2. Start playback.

### Expected behavior

Playback continues.

### Actual behavior

Playback stops.

### Environment

macOS 15, commit abc1234, desktop mode.

### Primary area

${area}

### Impact

${impact}

### Sanitized diagnostics

N/A

### Sensitive data

${acknowledge ? acknowledgement : "- [ ] I confirm that this report contains no sensitive data."}
`;
}

function featureBody() {
  return `### Problem

Editors cannot reuse a saved preset.

### Desired outcome

Editors can save and reuse presets.

### Proposed workflow

Save a preset and select it later.

### Acceptance criteria

- [ ] Presets persist across sessions.

### Alternatives considered

N/A

### Primary area

Workflow

### Impact

Medium — Meaningful improvement to a common workflow

### Sensitive data

${acknowledgement}
`;
}

function questionBody() {
  return `### Question

Which module owns project normalization?

### Context

I am adding a saved project field.

### What I tried

Read DESIGN.md and AGENTS.md.

### Primary area

Repository

### Sensitive data

${acknowledgement}
`;
}

test("accepts valid Bug, Feature, and Question forms", () => {
  assert.equal(validateIssue({ title: "bug(editor): preview playback stops", body: bugBody() }).valid, true);
  assert.equal(validateIssue({ title: "feat(workflow): add reusable presets", body: featureBody() }).valid, true);
  assert.equal(validateIssue({ title: "question(repository): clarify normalization ownership", body: questionBody() }).valid, true);
});

test("rejects malformed titles and mismatched form types", () => {
  const malformed = validateIssue({ title: "Preview playback stops.", body: bugBody() });
  assert.ok(malformed.errors.some((error) => error.startsWith("Title must")));
  const mismatch = validateIssue({ title: "feat(editor): preview playback stops", body: bugBody() });
  assert.ok(mismatch.errors.some((error) => error.includes("does not match")));
});

test("rejects a title area that differs from the form", () => {
  const result = validateIssue({ title: "bug(export): preview playback stops", body: bugBody() });
  assert.ok(result.errors.some((error) => error.includes("must match")));
});

test("rejects missing required fields and acknowledgement", () => {
  const body = bugBody({ acknowledge: false }).replace("Playback continues.", "No response");
  const result = validateIssue({ title: "bug(editor): preview playback stops", body });
  assert.ok(result.errors.some((error) => error.includes("Expected behavior")));
  assert.ok(result.errors.some((error) => error.includes("Sensitive data")));
});

test("maps structured impact to priority", () => {
  assert.equal(inferPriority("bug", "Critical — Data loss"), "priority: critical");
  assert.equal(inferPriority("bug", "High — Blocks a workflow"), "priority: high");
  assert.equal(inferPriority("feat", "Medium — Meaningful improvement"), "priority: medium");
  assert.equal(inferPriority("question", null), "priority: low");
});

test("adds needs-info for invalid API-created Issues", () => {
  const result = validateIssue({ title: "help", body: "missing form" });
  const plan = buildLabelPlan({
    result,
    currentLabels: [],
    previousAutomaticPriority: null,
    previousAutomaticArea: null,
    action: "opened",
  });
  assert.ok(plan.add.includes("status: needs info"));
  assert.equal(plan.add.includes("status: needs triage"), false);
});

test("removes needs-info after correction and initializes triage", () => {
  const result = validateIssue({ title: "bug(editor): preview playback stops", body: bugBody() });
  const plan = buildLabelPlan({
    result,
    currentLabels: ["status: needs info"],
    previousAutomaticPriority: null,
    previousAutomaticArea: null,
    action: "edited",
  });
  assert.ok(plan.remove.includes("status: needs info"));
  assert.ok(plan.add.includes("status: needs triage"));
});

test("updates automation priority but preserves maintainer overrides", () => {
  const result = validateIssue({
    title: "bug(editor): preview playback stops",
    body: bugBody({ impact: "Medium — Meaningful degradation with a workaround" }),
  });
  const automated = buildLabelPlan({
    result,
    currentLabels: ["priority: high"],
    previousAutomaticPriority: "priority: high",
    previousAutomaticArea: null,
    action: "edited",
  });
  assert.deepEqual(automated.remove, ["priority: high"]);
  assert.ok(automated.add.includes("priority: medium"));

  const overridden = buildLabelPlan({
    result,
    currentLabels: ["priority: critical"],
    previousAutomaticPriority: "priority: high",
    previousAutomaticArea: null,
    action: "edited",
  });
  assert.equal(overridden.add.some((label) => label.startsWith("priority: ")), false);
  assert.equal(overridden.remove.some((label) => label.startsWith("priority: ")), false);
  assert.equal(overridden.automaticPriority, null);
});

test("normalizes type and primary area labels", () => {
  const result = validateIssue({ title: "bug(editor): preview playback stops", body: bugBody() });
  const plan = buildLabelPlan({
    result,
    currentLabels: ["enhancement", "area: export", "area: workflow"],
    previousAutomaticPriority: null,
    previousAutomaticArea: "area: export",
    action: "edited",
  });
  assert.ok(plan.remove.includes("enhancement"));
  assert.ok(plan.remove.includes("area: export"));
  assert.ok(plan.add.includes("bug"));
  assert.ok(plan.add.includes("area: editor"));
  assert.equal(plan.remove.includes("area: workflow"), false);
});

test("policy comment remains identifiable and records automatic priority", () => {
  const result = validateIssue({ title: "question(repository): clarify normalization ownership", body: questionBody() });
  const comment = renderPolicyComment(result, "priority: low", "area: repository");
  assert.match(comment, /<!-- issue-policy -->/);
  assert.equal(parseAutomaticPriority(comment), "priority: low");
  assert.equal(parseAutomaticArea(comment), "area: repository");
});
