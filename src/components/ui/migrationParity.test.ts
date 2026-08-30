// @vitest-environment happy-dom

import { createSSRApp, createApp, defineComponent, h, nextTick, ref } from "vue";
import type { Component } from "vue";
import { renderToString } from "@vue/server-renderer";
import { afterEach, describe, expect, it } from "vitest";
import {
  UiCard,
  UiDashboardToolbar,
  UiInput,
  UiModal,
  UiPopover,
  UiSelect,
  UiTabs,
} from ".";

const mountedApps: Array<ReturnType<typeof createApp>> = [];

function mount(component: Component) {
  const host = document.createElement("div");
  document.body.append(host);
  const app = createApp(component);
  mountedApps.push(app);
  app.mount(host);
  return host;
}

async function settle() {
  await nextTick();
  await new Promise((resolve) => window.setTimeout(resolve, 0));
}

afterEach(() => {
  while (mountedApps.length > 0) mountedApps.pop()?.unmount();
  document.body.replaceChildren();
});

describe("local UI migration parity", () => {
  it("keeps named slots and caller ui overrides visible", async () => {
    const app = createSSRApp({
      render: () => h("div", [
        h(
          UiDashboardToolbar,
          { class: "toolbar-root", ui: { left: "toolbar-left", right: "toolbar-right" } },
          {
            left: () => h("span", "剪辑时间轴"),
            default: () => h("span", "toolbar-center"),
            right: () => h("button", "放大时间线"),
          },
        ),
        h(UiCard, { variant: "outline", class: "card-root", ui: { body: "card-body" } }, () => "card-content"),
      ]),
    });

    const html = await renderToString(app);

    expect(html).toContain("剪辑时间轴");
    expect(html).toContain("toolbar-center");
    expect(html).toContain("放大时间线");
    expect(html).toContain("toolbar-root");
    expect(html).toContain("toolbar-left");
    expect(html).toContain("toolbar-right");
    expect(html).toContain("card-root");
    expect(html).toContain("card-body");
    expect(html).toContain("bg-transparent");
  });

  it("keeps leading content for tabs and selects", async () => {
    const app = createSSRApp({
      render: () => h("div", [
        h(
          UiTabs,
          { modelValue: "film", items: [{ label: "胶片机", value: "film" }] },
          { leading: () => h("span", { "data-leading": "tab" }, "camera-icon") },
        ),
        h(
          UiSelect,
          { modelValue: "Inter", items: ["Inter"] },
          { leading: () => h("span", { "data-leading": "select" }, "font-icon") },
        ),
      ]),
    });

    const html = await renderToString(app);

    expect(html).toContain('data-leading="tab"');
    expect(html).toContain('data-leading="select"');
  });

  it("emits model updates for form controls and tabs", async () => {
    const inputValue = ref("before");
    const selectValue = ref("film");
    const tabValue = ref("still");
    const host = mount(defineComponent({
      setup() {
        return () => h("div", [
          h(UiInput, { modelValue: inputValue.value, "onUpdate:modelValue": (value: string | number) => { inputValue.value = String(value); } }),
          h(UiSelect, { modelValue: selectValue.value, items: ["film", "digital"], "onUpdate:modelValue": (value: string | number) => { selectValue.value = String(value); } }),
          h(UiTabs, { modelValue: tabValue.value, items: ["still", "motion"], "onUpdate:modelValue": (value: string | number) => { tabValue.value = String(value); } }),
        ]);
      },
    }));

    const input = host.querySelector("input") as HTMLInputElement;
    input.value = "after";
    input.dispatchEvent(new Event("input", { bubbles: true }));
    const select = host.querySelector("select") as HTMLSelectElement;
    select.value = "digital";
    select.dispatchEvent(new Event("change", { bubbles: true }));
    const motionTab = [...host.querySelectorAll<HTMLElement>("[role=tab]")].find((tab) => tab.textContent === "motion");
    expect(motionTab).toBeDefined();
    motionTab?.dispatchEvent(new MouseEvent("mousedown", { button: 0, ctrlKey: false, bubbles: true }));
    await settle();

    expect(inputValue.value).toBe("after");
    expect(selectValue.value).toBe("digital");
    expect(tabValue.value).toBe("motion");
  });

  it("renders modal content through a portal and restores focus after Escape", async () => {
    const open = ref(false);
    let opener: HTMLButtonElement | null = null;
    mount(defineComponent({
      setup() {
        return () => h("div", [
          h("button", { ref: (element) => { opener = element as HTMLButtonElement; }, onClick: () => { open.value = true; } }, "open modal"),
          h(UiModal, {
            open: open.value,
            title: "Parity modal",
            class: "caller-content",
            ui: { overlay: "caller-overlay", content: "ui-content" },
            "onUpdate:open": (value: boolean) => { open.value = value; },
          }, { content: () => h("button", "inside modal") }),
        ]);
      },
    }));

    const mountedOpener = opener as HTMLButtonElement | null;
    expect(mountedOpener).not.toBeNull();
    mountedOpener?.focus();
    mountedOpener?.click();
    await settle();

    expect(document.body.querySelector(".caller-overlay")).not.toBeNull();
    expect(document.body.querySelector(".caller-content.ui-content")).not.toBeNull();
    expect(document.body.textContent).toContain("inside modal");

    document.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", bubbles: true }));
    await settle();

    expect(open.value).toBe(false);
    expect(document.activeElement).toBe(opener);
  });

  it("prevents modal dismissal when dismissible is false", async () => {
    const modalOpen = ref(true);
    mount(defineComponent({
      setup() {
        return () => h(UiModal, { open: modalOpen.value, dismissible: false, title: "Locked", "onUpdate:open": (value: boolean) => { modalOpen.value = value; } }, { content: () => "locked modal" });
      },
    }));
    await settle();

    document.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", bubbles: true }));
    await settle();

    expect(modalOpen.value).toBe(true);
    expect(document.body.textContent).toContain("locked modal");
  });

  it("prevents popover dismissal when dismissible is false", async () => {
    const open = ref(true);
    mount(defineComponent({
      setup() {
        return () => h(UiPopover, { open: open.value, dismissible: false, "onUpdate:open": (value: boolean) => { open.value = value; } }, { default: () => h("button", "trigger"), content: () => "locked popover" });
      },
    }));
    await settle();

    document.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", bubbles: true }));
    await settle();

    expect(open.value).toBe(true);
    expect(document.body.textContent).toContain("locked popover");
  });

  it("dismisses a controlled popover with Escape", async () => {
    const open = ref(true);
    mount(defineComponent({
      setup() {
        return () => h(UiPopover, { open: open.value, "onUpdate:open": (value: boolean) => { open.value = value; } }, { default: () => h("button", "trigger"), content: () => "popover content" });
      },
    }));
    await settle();

    document.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", bubbles: true }));
    await settle();

    expect(open.value).toBe(false);
  });
});
