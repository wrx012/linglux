import { appendFileSync, readFileSync } from "node:fs";
import { pathToFileURL } from "node:url";

export const AREAS = [
  "agent",
  "desktop",
  "editor",
  "export",
  "media-core",
  "storyboard",
  "tts",
  "workflow",
  "repository",
];

export const TYPE_LABELS = {
  bug: "bug",
  feat: "enhancement",
  question: "question",
};

export const REQUIRED_SECTIONS = {
  bug: ["Problem", "Steps to reproduce", "Expected behavior", "Actual behavior", "Environment", "Primary area", "Impact", "Sanitized diagnostics", "Sensitive data"],
  feat: ["Problem", "Desired outcome", "Proposed workflow", "Acceptance criteria", "Alternatives considered", "Primary area", "Impact", "Sensitive data"],
  question: ["Question", "Context", "What I tried", "Primary area", "Sensitive data"],
};

export const MANAGED_TYPE_LABELS = ["bug", "enhancement", "question"];
export const PRIORITY_LABELS = ["priority: critical", "priority: high", "priority: medium", "priority: low"];
export const ACTIVE_STATUS_LABELS = ["status: needs triage", "status: in progress", "status: blocked", "status: next release"];

const TITLE_PATTERN = new RegExp(`^(bug|feat|question)\\((${AREAS.join("|")})\\): ([A-Za-z0-9][^\\r\\n]*[^.\\s])$`);
const FORM_SENTINELS = [
  ["bug", "steps to reproduce"],
  ["feat", "desired outcome"],
  ["question", "what i tried"],
];

export function parseSections(body) {
  const sections = new Map();
  const matches = [...String(body ?? "").matchAll(/^###\s+(.+?)\s*$/gm)];
  for (let index = 0; index < matches.length; index += 1) {
    const match = matches[index];
    const start = match.index + match[0].length;
    const end = matches[index + 1]?.index ?? body.length;
    sections.set(match[1].trim().toLowerCase(), body.slice(start, end).trim());
  }
  return sections;
}

export function detectIssueType(sections) {
  return FORM_SENTINELS.find(([, sentinel]) => sections.has(sentinel))?.[0] ?? null;
}

function meaningful(value) {
  const normalized = String(value ?? "")
    .replace(/<!--[\s\S]*?-->/g, "")
    .replace(/^_?No response_?$/i, "")
    .trim();
  return normalized.length > 0;
}

function normalizeArea(value) {
  return String(value ?? "").trim().toLowerCase().replace(/\s+/g, "-");
}

export function inferPriority(issueType, impact) {
  if (issueType === "question") return "priority: low";
  const normalized = String(impact ?? "").trim().toLowerCase();
  if (normalized.startsWith("critical")) return "priority: critical";
  if (normalized.startsWith("high")) return "priority: high";
  if (normalized.startsWith("medium")) return "priority: medium";
  if (normalized.startsWith("low")) return "priority: low";
  return null;
}

export function validateIssue(issue) {
  const title = String(issue.title ?? "");
  const sections = parseSections(issue.body ?? "");
  const formType = detectIssueType(sections);
  const titleMatch = title.match(TITLE_PATTERN);
  const titleType = titleMatch?.[1] ?? null;
  const titleArea = titleMatch?.[2] ?? null;
  const errors = [];

  if (title.length > 100 || !titleMatch) {
    errors.push("Title must match `type(area): summary`, use an allowed lowercase type and area, be at most 100 characters, and have no trailing period.");
  }
  if (!formType) {
    errors.push("Use one of the repository Issue Forms: Bug report, Feature request, or Question.");
  } else if (titleType && titleType !== formType) {
    errors.push(`Title type \`${titleType}\` does not match the ${formType} Issue Form.`);
  }

  if (formType) {
    for (const heading of REQUIRED_SECTIONS[formType]) {
      if (!meaningful(sections.get(heading.toLowerCase()))) {
        errors.push(`Complete the \`${heading}\` field.`);
      }
    }
  }

  const bodyArea = normalizeArea(sections.get("primary area"));
  if (bodyArea && !AREAS.includes(bodyArea)) {
    errors.push("Select a supported Primary area from the Issue Form.");
  } else if (titleArea && bodyArea && titleArea !== bodyArea) {
    errors.push(`Title area \`${titleArea}\` must match the selected Primary area \`${bodyArea}\`.`);
  }

  const sensitiveData = sections.get("sensitive data") ?? "";
  if (!/^- \[[xX]\]\s+/m.test(sensitiveData)) {
    errors.push("Confirm the Sensitive data acknowledgement.");
  }

  const issueType = formType ?? titleType;
  return {
    valid: errors.length === 0,
    errors,
    issueType,
    typeLabel: issueType ? TYPE_LABELS[issueType] : null,
    areaLabel: bodyArea && AREAS.includes(bodyArea) ? `area: ${bodyArea}` : titleArea ? `area: ${titleArea}` : null,
    priorityLabel: inferPriority(issueType, sections.get("impact")),
  };
}

export function parseAutomaticPriority(commentBody) {
  return String(commentBody ?? "").match(/<!-- issue-policy-priority:(priority: (?:critical|high|medium|low)) -->/)?.[1] ?? null;
}

export function parseAutomaticArea(commentBody) {
  return String(commentBody ?? "").match(/<!-- issue-policy-area:(area: (?:agent|desktop|editor|export|media-core|storyboard|tts|workflow|repository)) -->/)?.[1] ?? null;
}

export function buildLabelPlan({ result, currentLabels, previousAutomaticPriority, previousAutomaticArea, action }) {
  const current = new Set(currentLabels);
  const add = new Set();
  const remove = new Set();

  if (result.typeLabel) {
    for (const label of MANAGED_TYPE_LABELS) {
      if (label !== result.typeLabel && current.has(label)) remove.add(label);
    }
    if (!current.has(result.typeLabel)) add.add(result.typeLabel);
  }

  let automaticArea = null;
  if (result.areaLabel) {
    automaticArea = result.areaLabel;
    if (previousAutomaticArea && previousAutomaticArea !== result.areaLabel && current.has(previousAutomaticArea)) {
      remove.add(previousAutomaticArea);
    }
    if (!current.has(result.areaLabel)) add.add(result.areaLabel);
  }

  const currentPriorities = PRIORITY_LABELS.filter((label) => current.has(label));
  let automaticPriority = null;
  if (result.priorityLabel && currentPriorities.length === 0) {
    add.add(result.priorityLabel);
    automaticPriority = result.priorityLabel;
  } else if (
    result.priorityLabel
    && previousAutomaticPriority
    && currentPriorities.length === 1
    && currentPriorities[0] === previousAutomaticPriority
  ) {
    automaticPriority = result.priorityLabel;
    if (currentPriorities[0] !== result.priorityLabel) {
      remove.add(currentPriorities[0]);
      add.add(result.priorityLabel);
    }
  }

  if (result.valid) {
    const wasWaitingForInfo = current.has("status: needs info");
    if (wasWaitingForInfo) remove.add("status: needs info");
    const hasActiveStatus = ACTIVE_STATUS_LABELS.some((label) => current.has(label));
    if ((action === "opened" || action === "reopened" || wasWaitingForInfo) && !hasActiveStatus) add.add("status: needs triage");
  } else if (!current.has("status: needs info")) {
    add.add("status: needs info");
  }

  for (const label of remove) add.delete(label);
  return { add: [...add], remove: [...remove], automaticPriority, automaticArea };
}

export function renderPolicyComment(result, automaticPriority, automaticArea) {
  const marker = "<!-- issue-policy -->";
  const priorityMarker = automaticPriority ? `\n<!-- issue-policy-priority:${automaticPriority} -->` : "";
  const areaMarker = automaticArea ? `\n<!-- issue-policy-area:${automaticArea} -->` : "";
  if (result.valid) {
    return `${marker}\n✅ Issue policy validation passed. The initial type, area, and priority labels have been applied. Maintainers may adjust triage labels.${priorityMarker}${areaMarker}`;
  }
  const findings = result.errors.map((error) => `- ${error}`).join("\n");
  return `${marker}\n⚠️ This Issue needs more information before triage:\n\n${findings}\n\nEdit the title or description to resolve these items. The Issue will remain open and validation will run again automatically.${priorityMarker}${areaMarker}`;
}

function writeResult(result) {
  if (process.env.GITHUB_OUTPUT) appendFileSync(process.env.GITHUB_OUTPUT, `result=${JSON.stringify(result)}\n`);
  console.log(JSON.stringify(result, null, 2));
}

export function main(argv = process.argv.slice(2)) {
  const eventIndex = argv.indexOf("--event");
  const eventPath = eventIndex >= 0 ? argv[eventIndex + 1] : null;
  if (!eventPath) throw new Error("Usage: issue-policy.mjs --event <path>");
  const event = JSON.parse(readFileSync(eventPath, "utf8"));
  writeResult(validateIssue(event.issue ?? {}));
}

if (import.meta.url === pathToFileURL(process.argv[1] ?? "").href) main();
