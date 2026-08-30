import { createSSRApp, h } from "vue";
import { renderToString } from "@vue/server-renderer";
import { describe, expect, it } from "vitest";
import {
  UiDashboardToolbar,
  UiSelect,
  UiTabs,
} from ".";

describe("Nuxt UI migration parity", () => {
  it("keeps dashboard toolbar named slots visible", async () => {
    const app = createSSRApp({
      render: () => h(
        UiDashboardToolbar,
        { ui: { left: "toolbar-left", right: "toolbar-right" } },
        {
          left: () => h("span", "剪辑时间轴"),
          default: () => h("span", "toolbar-center"),
          right: () => h("button", "放大时间线"),
        },
      ),
    });

    const html = await renderToString(app);

    expect(html).toContain("剪辑时间轴");
    expect(html).toContain("toolbar-center");
    expect(html).toContain("放大时间线");
    expect(html).toContain("toolbar-left");
    expect(html).toContain("toolbar-right");
  });

  it("keeps leading content for tabs and selects", async () => {
    const app = createSSRApp({
      render: () => h("div", [
        h(
          UiTabs,
          {
            modelValue: "film",
            items: [{ label: "胶片机", value: "film" }],
          },
          { leading: () => h("span", { "data-leading": "tab" }, "camera-icon") },
        ),
        h(
          UiSelect,
          {
            modelValue: "Inter",
            items: ["Inter"],
          },
          { leading: () => h("span", { "data-leading": "select" }, "font-icon") },
        ),
      ]),
    });

    const html = await renderToString(app);

    expect(html).toContain('data-leading="tab"');
    expect(html).toContain('data-leading="select"');
  });
});
