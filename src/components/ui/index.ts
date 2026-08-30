import type { App, Component, PropType } from "vue";
import { computed, defineComponent, h, nextTick, ref, watch } from "vue";
import { cva } from "class-variance-authority";
import { clsx, type ClassValue } from "clsx";
import {
  DialogContent,
  DialogOverlay,
  DialogPortal,
  DialogRoot,
  DialogTitle,
  PopoverContent,
  PopoverPortal,
  PopoverRoot,
  PopoverTrigger,
  SwitchRoot,
  SwitchThumb,
  TabsList,
  TabsRoot,
  TabsTrigger,
  TooltipContent,
  TooltipPortal,
  TooltipProvider,
  TooltipRoot,
  TooltipTrigger,
} from "reka-ui";
import { twMerge } from "tailwind-merge";

type UiConfig = Record<string, ClassValue>;
type Item = string | number | {
  label?: string;
  value?: unknown;
  id?: unknown;
  disabled?: boolean;
};

type FloatingContentConfig = {
  side?: "top" | "right" | "bottom" | "left";
  align?: "start" | "center" | "end";
  sideOffset?: number;
  alignOffset?: number;
  avoidCollisions?: boolean;
  collisionPadding?: number | Partial<Record<"top" | "right" | "bottom" | "left", number>>;
  sticky?: "partial" | "always";
  hideWhenDetached?: boolean;
};

function cn(...values: ClassValue[]) {
  return twMerge(clsx(values));
}

function itemValue(item: Item, valueKey = "value"): string | number {
  if (typeof item !== "object") return item;
  const value = (item as Record<string, unknown>)[valueKey] ?? item.value ?? item.id ?? item.label ?? "";
  return typeof value === "string" || typeof value === "number" ? value : String(value);
}

function itemLabel(item: Item, labelKey = "label") {
  if (typeof item !== "object") return String(item);
  return String((item as Record<string, unknown>)[labelKey] ?? item.label ?? item.value ?? item.id ?? "");
}

const buttonStyle = cva(
  "inline-flex items-center justify-center gap-1.5 rounded-md font-bold transition-colors outline-none focus-visible:ring-2 focus-visible:ring-primary/45 disabled:pointer-events-none disabled:opacity-45",
  {
    variants: {
      size: {
        xs: "min-h-7 px-2 text-[10px]",
        sm: "min-h-8 px-2.5 text-[11px]",
        md: "min-h-9 px-3 text-[12px]",
        lg: "min-h-10 px-4 text-[13px]",
      },
      square: { true: "aspect-square px-0" },
      block: { true: "w-full" },
    },
    defaultVariants: { size: "sm" },
  },
);

const buttonToneClasses: Record<string, Record<string, string>> = {
  primary: {
    solid: "bg-primary text-inverted hover:brightness-110",
    soft: "bg-primary/12 text-primary hover:bg-primary/20",
    subtle: "bg-primary/8 text-primary hover:bg-primary/15",
    outline: "border border-primary/40 bg-transparent text-primary hover:bg-primary/10",
    ghost: "bg-transparent text-primary hover:bg-primary/10",
    link: "bg-transparent text-primary underline-offset-4 hover:underline",
  },
  secondary: {
    solid: "bg-secondary text-white hover:brightness-110",
    soft: "bg-secondary/15 text-secondary hover:bg-secondary/25",
    subtle: "bg-secondary/10 text-secondary hover:bg-secondary/20",
    outline: "border border-secondary/45 bg-transparent text-secondary hover:bg-secondary/10",
    ghost: "bg-transparent text-secondary hover:bg-secondary/10",
    link: "bg-transparent text-secondary underline-offset-4 hover:underline",
  },
  success: {
    solid: "bg-success text-inverted hover:brightness-110",
    soft: "bg-success/15 text-success hover:bg-success/25",
    subtle: "bg-success/10 text-success hover:bg-success/20",
    outline: "border border-success/45 bg-transparent text-success hover:bg-success/10",
    ghost: "bg-transparent text-success hover:bg-success/10",
    link: "bg-transparent text-success underline-offset-4 hover:underline",
  },
  error: {
    solid: "bg-error text-white hover:brightness-110",
    soft: "bg-error/15 text-error hover:bg-error/25",
    subtle: "bg-error/10 text-error hover:bg-error/20",
    outline: "border border-error/45 bg-transparent text-error hover:bg-error/10",
    ghost: "bg-transparent text-error hover:bg-error/10",
    link: "bg-transparent text-error underline-offset-4 hover:underline",
  },
  warning: {
    solid: "bg-warning text-inverted hover:brightness-110",
    soft: "bg-warning/15 text-warning hover:bg-warning/25",
    subtle: "bg-warning/10 text-warning hover:bg-warning/20",
    outline: "border border-warning/45 bg-transparent text-warning hover:bg-warning/10",
    ghost: "bg-transparent text-warning hover:bg-warning/10",
    link: "bg-transparent text-warning underline-offset-4 hover:underline",
  },
  neutral: {
    solid: "bg-accented text-highlighted hover:brightness-110",
    soft: "bg-accented text-default hover:brightness-110",
    subtle: "bg-elevated text-default hover:bg-accented",
    outline: "border border-default bg-transparent text-default hover:bg-accented",
    ghost: "bg-transparent text-default hover:bg-accented",
    link: "bg-transparent text-default underline-offset-4 hover:underline",
  },
};

function buttonToneClass(color: string, variant: string) {
  const tones = buttonToneClasses[color] ?? buttonToneClasses.neutral;
  return tones[variant] ?? tones.solid;
}

const UiButton = defineComponent({
  name: "UiButton",
  inheritAttrs: false,
  props: {
    color: { type: String, default: "neutral" },
    variant: { type: String, default: "solid" },
    size: { type: String, default: "sm" },
    square: Boolean,
    block: Boolean,
    loading: Boolean,
  },
  setup(props, { attrs, slots }) {
    return () => h("button", {
      ...attrs,
      type: attrs.type ?? "button",
      disabled: props.loading || (attrs.disabled !== undefined && attrs.disabled !== false),
      "aria-busy": props.loading || undefined,
      class: cn(
        buttonStyle({ size: props.size as "xs" | "sm" | "md" | "lg", square: props.square, block: props.block }),
        buttonToneClass(props.color, props.variant),
        attrs.class as ClassValue,
      ),
    }, [
      props.loading ? h("span", { class: "size-3 animate-spin rounded-full border-2 border-current border-r-transparent", "aria-hidden": "true" }) : null,
      slots.leading?.(),
      slots.default?.(),
      slots.trailing?.(),
    ]);
  },
});

function simpleComponent(name: string, tag: string, base: string) {
  return defineComponent({
    name,
    inheritAttrs: false,
    props: { as: { type: String, default: tag }, ui: Object as PropType<UiConfig> },
    setup(props, { attrs, slots }) {
      return () => h(props.as, { ...attrs, class: cn(base, props.ui?.root, attrs.class as ClassValue) }, slots.default?.());
    },
  });
}

const UiApp = simpleComponent("UiApp", "div", "contents");
const UiDashboardGroup = simpleComponent("UiDashboardGroup", "section", "flex w-full min-w-0");
const UiDashboardPanel = simpleComponent("UiDashboardPanel", "section", "min-w-0 flex-1 basis-0");

const UiDashboardToolbar = defineComponent({
  name: "UiDashboardToolbar",
  inheritAttrs: false,
  props: { as: { type: String, default: "div" }, ui: Object as PropType<UiConfig> },
  setup(props, { attrs, slots }) {
    return () => h(props.as, {
      ...attrs,
      class: cn(
        "flex min-w-0 items-center justify-between border-b border-default",
        props.ui?.root,
        attrs.class as ClassValue,
      ),
    }, [
      slots.left ? h("div", { class: cn("flex min-w-0 items-center gap-2", props.ui?.left) }, slots.left()) : null,
      slots.default ? h("div", { class: cn("min-w-0 flex-1", props.ui?.center) }, slots.default()) : null,
      slots.right ? h("div", { class: cn("flex min-w-0 items-center gap-2", props.ui?.right) }, slots.right()) : null,
    ]);
  },
});

const UiDashboardNavbar = defineComponent({
  name: "UiDashboardNavbar",
  inheritAttrs: false,
  props: {
    as: { type: String, default: "header" },
    ui: Object as PropType<UiConfig>,
    toggle: { type: [Boolean, Object], default: true },
  },
  setup(props, { attrs, slots }) {
    return () => h(props.as, {
      ...attrs,
      class: cn(
        "flex min-w-0 items-center justify-between border-b border-default",
        props.ui?.root,
        attrs.class as ClassValue,
      ),
    }, [
      slots.left ? h("div", { class: cn("flex min-w-0 items-center gap-2", props.ui?.left) }, slots.left()) : null,
      slots.center || slots.default
        ? h("div", { class: cn("flex min-w-0 flex-1 items-center gap-2", props.ui?.center) }, (slots.center ?? slots.default)?.())
        : null,
      slots.right ? h("div", { class: cn("flex min-w-0 items-center gap-2", props.ui?.right) }, slots.right()) : null,
    ]);
  },
});

const UiCard = defineComponent({
  name: "UiCard",
  inheritAttrs: false,
  props: {
    as: { type: String, default: "div" },
    ui: Object as PropType<UiConfig>,
    variant: { type: String, default: "subtle" },
  },
  setup(props, { attrs, slots }) {
    return () => h(props.as, {
      ...attrs,
      class: cn("rounded-lg border border-default bg-elevated", props.ui?.root, attrs.class as ClassValue),
    }, [
      slots.header ? h("div", { class: cn("border-b border-default p-4", props.ui?.header) }, slots.header()) : null,
      h("div", { class: cn("p-4", props.ui?.body) }, slots.default?.()),
      slots.footer ? h("div", { class: cn("border-t border-default p-4", props.ui?.footer) }, slots.footer()) : null,
    ]);
  },
});

const UiBadge = defineComponent({
  name: "UiBadge",
  inheritAttrs: false,
  props: { color: { type: String, default: "neutral" }, variant: String, size: { type: String, default: "sm" } },
  setup(props, { attrs, slots }) {
    const colors: Record<string, string> = { primary: "bg-primary/15 text-primary", secondary: "bg-secondary/15 text-secondary", success: "bg-success/15 text-success", error: "bg-error/15 text-error", warning: "bg-warning/15 text-warning", neutral: "bg-accented text-toned" };
    return () => h("span", { ...attrs, class: cn("inline-flex min-w-0 items-center rounded px-1.5 py-0.5 text-[10px] font-bold", colors[props.color], props.variant === "outline" && "border border-current bg-transparent", attrs.class as string) }, slots.default?.());
  },
});

const UiChip = defineComponent({
  name: "UiChip",
  inheritAttrs: false,
  props: { color: { type: String, default: "neutral" } },
  setup(props, { attrs, slots }) {
    return () => h("span", { ...attrs, class: cn("inline-flex size-2 rounded-full", props.color === "success" ? "bg-success" : "bg-toned", attrs.class as string) }, slots.default?.());
  },
});

const UiSeparator = defineComponent({
  name: "UiSeparator",
  inheritAttrs: false,
  props: { orientation: { type: String, default: "horizontal" } },
  setup(props, { attrs }) {
    return () => h("div", { ...attrs, role: "separator", "aria-orientation": props.orientation, class: cn("shrink-0 bg-border", props.orientation === "vertical" ? "h-full w-px" : "h-px w-full", attrs.class as string) });
  },
});

const inputBase = "w-full rounded-md border border-default bg-elevated px-3 py-2 text-[12px] font-semibold text-highlighted outline-none placeholder:text-[var(--ui-text-muted)] placeholder:opacity-100 focus:border-primary focus:ring-2 focus:ring-primary/20 disabled:opacity-50";

const UiInput = defineComponent({
  name: "UiInput",
  inheritAttrs: false,
  props: {
    modelValue: [String, Number],
    ui: Object as PropType<UiConfig>,
    color: String,
    variant: String,
    size: String,
  },
  emits: ["update:modelValue"],
  setup(props, { attrs, emit, slots }) {
    return () => h("div", { class: cn("relative", props.ui?.root, attrs.class as ClassValue) }, [
      slots.leading ? h("span", { class: cn("pointer-events-none absolute inset-y-0 left-0 flex items-center pl-3", props.ui?.leading) }, slots.leading()) : null,
      h("input", { ...attrs, class: cn(inputBase, slots.leading && "pl-9", slots.trailing && "pr-10", props.ui?.base), value: props.modelValue ?? "", onInput: (event: Event) => emit("update:modelValue", (event.target as HTMLInputElement).type === "number" ? (event.target as HTMLInputElement).valueAsNumber : (event.target as HTMLInputElement).value) }),
      slots.trailing ? h("span", { class: cn("absolute inset-y-0 right-0 flex items-center pr-2", props.ui?.trailing) }, slots.trailing()) : null,
    ]);
  },
});

const UiInputNumber = defineComponent({
  name: "UiInputNumber",
  inheritAttrs: false,
  props: {
    modelValue: Number,
    ui: Object as PropType<UiConfig>,
    increment: { type: Boolean, default: true },
    decrement: { type: Boolean, default: true },
  },
  emits: ["update:modelValue"],
  setup(props, { attrs, emit }) {
    return () => h("input", {
      ...attrs,
      type: "number",
      class: cn(
        inputBase,
        props.increment === false && props.decrement === false
          && "[appearance:textfield] [&::-webkit-inner-spin-button]:appearance-none [&::-webkit-outer-spin-button]:appearance-none",
        props.ui?.base,
        attrs.class as ClassValue,
      ),
      value: props.modelValue,
      onInput: (event: Event) => emit("update:modelValue", (event.target as HTMLInputElement).valueAsNumber),
    });
  },
});

const UiTextarea = defineComponent({
  name: "UiTextarea",
  inheritAttrs: false,
  props: { modelValue: String, ui: Object as PropType<UiConfig> },
  emits: ["update:modelValue"],
  setup(props, { attrs, emit }) {
    return () => h("textarea", { ...attrs, class: cn(inputBase, props.ui?.base, attrs.class as string), value: props.modelValue, onInput: (event: Event) => emit("update:modelValue", (event.target as HTMLTextAreaElement).value) });
  },
});

const UiSelect = defineComponent({
  name: "UiSelect",
  inheritAttrs: false,
  props: {
    modelValue: [String, Number],
    items: { type: Array as PropType<Item[]>, default: () => [] },
    valueKey: { type: String, default: "value" },
    labelKey: { type: String, default: "label" },
    ui: Object as PropType<UiConfig>,
    color: String,
    variant: String,
    size: String,
  },
  emits: ["update:modelValue"],
  setup(props, { attrs, emit, slots }) {
    return () => {
      const { class: attrClass, ...selectAttrs } = attrs;
      return h("div", { class: cn("relative min-w-0", props.ui?.root, attrClass as ClassValue) }, [
        slots.leading
          ? h("span", { class: cn("pointer-events-none absolute inset-y-0 left-0 z-10 flex items-center pl-3", props.ui?.leading) }, slots.leading())
          : null,
        h("select", {
          ...selectAttrs,
          class: cn(inputBase, "appearance-none pr-8", slots.leading && "pl-9", props.ui?.base),
          value: props.modelValue,
          onChange: (event: Event) => {
            const selectedValue = (event.target as HTMLSelectElement).value;
            const selectedItem = props.items.find((item) => String(itemValue(item, props.valueKey)) === selectedValue);
            emit("update:modelValue", selectedItem === undefined ? selectedValue : itemValue(selectedItem, props.valueKey));
          },
        }, props.items.map((item) => h("option", {
          value: itemValue(item, props.valueKey),
          disabled: typeof item === "object" && item.disabled,
        }, itemLabel(item, props.labelKey)))),
      ]);
    };
  },
});

const UiSwitch = defineComponent({
  name: "UiSwitch",
  inheritAttrs: false,
  props: { modelValue: Boolean },
  emits: ["update:modelValue"],
  setup(props, { attrs, emit }) {
    return () => h(SwitchRoot, {
      ...attrs,
      modelValue: props.modelValue,
      class: cn("relative inline-flex h-5 w-9 shrink-0 rounded-full border border-transparent transition-colors", props.modelValue ? "bg-primary" : "bg-accented", attrs.class as string),
      "onUpdate:modelValue": (value: unknown) => emit("update:modelValue", Boolean(value)),
    }, { default: () => h(SwitchThumb, { class: cn("pointer-events-none block size-4 translate-y-px rounded-full bg-white shadow transition-transform", props.modelValue ? "translate-x-[17px]" : "translate-x-px") }) });
  },
});

const UiSlider = defineComponent({
  name: "UiSlider",
  inheritAttrs: false,
  props: { modelValue: Number },
  emits: ["update:modelValue"],
  setup(props, { attrs, emit }) {
    return () => h("input", { ...attrs, type: "range", class: cn("h-1.5 w-full cursor-pointer accent-primary", attrs.class as string), value: props.modelValue, onInput: (event: Event) => emit("update:modelValue", (event.target as HTMLInputElement).valueAsNumber) });
  },
});

const UiProgress = defineComponent({
  name: "UiProgress",
  inheritAttrs: false,
  props: { modelValue: { type: Number, default: 0 }, max: { type: Number, default: 100 }, color: { type: String, default: "primary" } },
  setup(props, { attrs }) {
    const percent = computed(() => Math.max(0, Math.min(100, props.max ? (props.modelValue / props.max) * 100 : 0)));
    return () => h("div", { ...attrs, role: "progressbar", "aria-valuenow": props.modelValue, "aria-valuemax": props.max, class: cn("h-1.5 overflow-hidden rounded-full bg-accented", attrs.class as string) }, h("div", { class: cn("h-full transition-[width]", props.color === "error" ? "bg-error" : props.color === "success" ? "bg-success" : props.color === "secondary" ? "bg-secondary" : "bg-primary"), style: { width: `${percent.value}%` } }));
  },
});

const UiTabs = defineComponent({
  name: "UiTabs",
  inheritAttrs: false,
  props: {
    modelValue: [String, Number],
    defaultValue: [String, Number],
    items: { type: Array as PropType<Item[]>, default: () => [] },
    valueKey: { type: String, default: "value" },
    labelKey: { type: String, default: "label" },
    ui: Object as PropType<UiConfig>,
    color: String,
    variant: String,
    size: String,
    content: { type: [Boolean, Object], default: true },
    orientation: { type: String as PropType<"horizontal" | "vertical">, default: "horizontal" },
    activationMode: { type: String as PropType<"automatic" | "manual">, default: "automatic" },
  },
  emits: ["update:modelValue"],
  setup(props, { attrs, emit, slots }) {
    return () => {
      const { class: attrClass, ...listAttrs } = attrs;
      const rootProps: Record<string, unknown> = {
        orientation: props.orientation,
        activationMode: props.activationMode,
        class: cn("min-w-0", props.ui?.root, attrClass as ClassValue),
        "onUpdate:modelValue": (value: string | number) => emit("update:modelValue", value),
      };

      if (props.modelValue !== undefined) rootProps.modelValue = props.modelValue;
      rootProps.defaultValue = props.defaultValue ?? (props.items[0] ? itemValue(props.items[0], props.valueKey) : undefined);

      return h(TabsRoot, rootProps, {
        default: () => h(TabsList, {
          ...listAttrs,
          class: cn(
            "inline-flex gap-1 rounded-lg bg-elevated p-1",
            props.orientation === "vertical" && "flex-col",
            props.ui?.list,
          ),
        }, {
          default: () => props.items.map((item, index) => {
            const value = itemValue(item, props.valueKey);
            const active = props.modelValue === value
              || (props.modelValue === undefined && (props.defaultValue ?? itemValue(props.items[0], props.valueKey)) === value);
            const disabled = typeof item === "object" && item.disabled === true;

            return h(TabsTrigger, {
              key: `${String(value)}-${index}`,
              value,
              disabled,
              class: cn(
                "inline-flex items-center justify-center gap-1.5 rounded-md px-2.5 py-1.5 text-[11px] font-bold text-muted outline-none transition focus-visible:ring-2 focus-visible:ring-primary/45 disabled:pointer-events-none disabled:opacity-45 data-[state=active]:bg-primary/15 data-[state=active]:text-primary",
                props.ui?.trigger,
                active && props.ui?.indicator,
              ),
            }, {
              default: () => [
                slots.leading?.({ item, index }),
                slots.label?.({ item, index }) ?? itemLabel(item, props.labelKey),
              ],
            });
          }),
        }),
      });
    };
  },
});

function dialogComponent(name: string, side = false) {
  return defineComponent({
    name,
    inheritAttrs: false,
    props: {
      open: { type: Boolean, default: undefined },
      defaultOpen: Boolean,
      dismissible: { type: Boolean, default: true },
      close: { type: [Boolean, Object], default: true },
      title: String,
      content: Object,
      ui: Object as PropType<UiConfig>,
    },
    emits: ["update:open"],
    setup(props, { attrs, emit, slots }) {
      const previouslyFocused = ref<HTMLElement | null>(null);

      watch(() => props.open, (open, wasOpen) => {
        if (open && !wasOpen && typeof document !== "undefined") {
          previouslyFocused.value = document.activeElement instanceof HTMLElement ? document.activeElement : null;
        }
      }, { flush: "sync", immediate: true });

      const preventDismiss = (event: Event) => {
        if (!props.dismissible) event.preventDefault();
      };

      const restoreFocus = (event: Event) => {
        if (!previouslyFocused.value) return;
        event.preventDefault();
        const target = previouslyFocused.value;
        void nextTick(() => target.isConnected && target.focus());
      };

      return () => h(DialogRoot, {
        open: props.open,
        defaultOpen: props.defaultOpen,
        modal: true,
        "onUpdate:open": (value: boolean) => emit("update:open", value),
      }, {
        default: () => h(DialogPortal, { to: "body" }, {
          default: () => [
            h(DialogOverlay, {
              class: cn("fixed inset-0 z-[70] bg-black/50", props.ui?.overlay),
            }),
            h(DialogContent, {
              ...attrs,
              class: cn(
                "fixed z-[71] outline-none",
                side
                  ? "inset-y-0 right-0 h-full max-w-[min(480px,92vw)] bg-elevated shadow-2xl"
                  : "left-1/2 top-1/2 max-h-[calc(100dvh-2rem)] max-w-[calc(100vw-2rem)] -translate-x-1/2 -translate-y-1/2",
                props.ui?.content,
                attrs.class as ClassValue,
              ),
              onEscapeKeyDown: preventDismiss,
              onPointerDownOutside: preventDismiss,
              onInteractOutside: preventDismiss,
              onCloseAutoFocus: restoreFocus,
            }, {
              default: () => [
                props.title ? h(DialogTitle, { class: "sr-only" }, { default: () => props.title }) : null,
                slots.content?.() ?? slots.default?.(),
              ],
            }),
          ],
        }),
      });
    },
  });
}

const UiModal = dialogComponent("UiModal");
const UiSlideover = dialogComponent("UiSlideover", true);

const UiPopover = defineComponent({
  name: "UiPopover",
  inheritAttrs: false,
  props: {
    open: { type: Boolean, default: undefined },
    defaultOpen: Boolean,
    dismissible: { type: Boolean, default: true },
    content: Object as PropType<FloatingContentConfig>,
    ui: Object as PropType<UiConfig>,
  },
  emits: ["update:open"],
  setup(props, { attrs, emit, slots }) {
    const preventDismiss = (event: Event) => {
      if (!props.dismissible) event.preventDefault();
    };

    return () => h(PopoverRoot, {
      open: props.open,
      defaultOpen: props.defaultOpen,
      modal: false,
      "onUpdate:open": (value: boolean) => emit("update:open", value),
    }, {
      default: () => [
        h("span", { ...attrs, class: cn("inline-flex min-w-0", props.ui?.root, attrs.class as ClassValue) }, [
          h(PopoverTrigger, { asChild: true }, { default: () => slots.default?.() }),
        ]),
        h(PopoverPortal, { to: "body" }, {
          default: () => h(PopoverContent, {
            ...(props.content ?? {}),
            side: props.content?.side ?? "bottom",
            align: props.content?.align ?? "center",
            sideOffset: props.content?.sideOffset ?? 8,
            collisionPadding: props.content?.collisionPadding ?? 8,
            "data-ui-popover-content": "",
            class: cn(
              "z-50 min-w-[var(--reka-popover-trigger-width)] rounded-lg border border-default bg-elevated p-3 shadow-2xl outline-none",
              props.ui?.content,
            ),
            onEscapeKeyDown: preventDismiss,
            onInteractOutside: preventDismiss,
            onPointerdown: (event: PointerEvent) => event.stopPropagation(),
          }, { default: () => slots.content?.() }),
        }),
      ],
    });
  },
});

const UiTooltip = defineComponent({
  name: "UiTooltip",
  inheritAttrs: false,
  props: {
    text: String,
    kbds: { type: Array as PropType<Array<string | number>>, default: () => [] },
    content: Object as PropType<FloatingContentConfig>,
    ui: Object as PropType<UiConfig>,
    delayDuration: { type: Number, default: 300 },
    disabled: Boolean,
  },
  setup(props, { attrs, slots }) {
    const kbdLabel = (key: string | number) => {
      const normalized = String(key).toLowerCase();
      return ({ meta: "⌘", cmd: "⌘", control: "Ctrl", ctrl: "Ctrl", alt: "⌥", shift: "⇧", escape: "Esc", enter: "↵" } as Record<string, string>)[normalized] ?? String(key);
    };

    return () => h("span", {
      ...attrs,
      class: cn("inline-flex min-w-0", props.ui?.root, attrs.class as ClassValue),
    }, [
      h(TooltipProvider, { delayDuration: props.delayDuration }, {
        default: () => h(TooltipRoot, { disabled: props.disabled, delayDuration: props.delayDuration }, {
          default: () => [
            h(TooltipTrigger, { asChild: true }, { default: () => slots.default?.() }),
            props.text || slots.content || props.kbds.length > 0
              ? h(TooltipPortal, { to: "body" }, {
                default: () => h(TooltipContent, {
                  ...(props.content ?? {}),
                  side: props.content?.side ?? "top",
                  align: props.content?.align ?? "center",
                  sideOffset: props.content?.sideOffset ?? 7,
                  collisionPadding: props.content?.collisionPadding ?? 8,
                  class: cn(
                    "z-[90] flex items-center gap-2 whitespace-nowrap rounded bg-[#27272a] px-2 py-1 text-[10px] text-white shadow-lg",
                    props.ui?.content,
                  ),
                }, {
                  default: () => slots.content?.() ?? [
                    props.text ? h("span", props.text) : null,
                    props.kbds.length > 0
                      ? h("span", { class: "flex items-center gap-1" }, props.kbds.map((key, index) => h("kbd", {
                        key: `${String(key)}-${index}`,
                        class: "rounded border border-white/20 bg-black/25 px-1 py-0.5 font-mono text-[9px] leading-none text-white/90",
                      }, kbdLabel(key))))
                      : null,
                  ],
                }),
              })
              : null,
          ],
        }),
      }),
    ]);
  },
});

const components: Record<string, Component> = {
  UiApp, UiBadge, UiButton, UiCard, UiChip, UiDashboardGroup, UiDashboardNavbar,
  UiDashboardPanel, UiDashboardToolbar, UiInput, UiInputNumber, UiModal, UiPopover,
  UiProgress, UiSelect, UiSeparator, UiSlideover, UiSlider, UiSwitch, UiTabs,
  UiTextarea, UiTooltip,
};

export const shadcnUi = {
  install(app: App) {
    Object.entries(components).forEach(([name, component]) => app.component(name, component));
  },
};

export {
  cn, UiApp, UiBadge, UiButton, UiCard, UiChip, UiDashboardGroup, UiDashboardNavbar,
  UiDashboardPanel, UiDashboardToolbar, UiInput, UiInputNumber, UiModal, UiPopover,
  UiProgress, UiSelect, UiSeparator, UiSlideover, UiSlider, UiSwitch, UiTabs,
  UiTextarea, UiTooltip,
};
