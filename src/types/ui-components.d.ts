declare module "vue" {
  export interface GlobalComponents {
    UiApp: typeof import("../components/ui").UiApp;
    UiBadge: typeof import("../components/ui").UiBadge;
    UiButton: typeof import("../components/ui").UiButton;
    UiCard: typeof import("../components/ui").UiCard;
    UiChip: typeof import("../components/ui").UiChip;
    UiDashboardGroup: typeof import("../components/ui").UiDashboardGroup;
    UiDashboardNavbar: typeof import("../components/ui").UiDashboardNavbar;
    UiDashboardPanel: typeof import("../components/ui").UiDashboardPanel;
    UiDashboardToolbar: typeof import("../components/ui").UiDashboardToolbar;
    UiInput: typeof import("../components/ui").UiInput;
    UiInputNumber: typeof import("../components/ui").UiInputNumber;
    UiModal: typeof import("../components/ui").UiModal;
    UiPopover: typeof import("../components/ui").UiPopover;
    UiProgress: typeof import("../components/ui").UiProgress;
    UiSelect: typeof import("../components/ui").UiSelect;
    UiSeparator: typeof import("../components/ui").UiSeparator;
    UiSlideover: typeof import("../components/ui").UiSlideover;
    UiSlider: typeof import("../components/ui").UiSlider;
    UiSwitch: typeof import("../components/ui").UiSwitch;
    UiTabs: typeof import("../components/ui").UiTabs;
    UiTextarea: typeof import("../components/ui").UiTextarea;
    UiTooltip: typeof import("../components/ui").UiTooltip;
  }
}

export {};
