import { describe, expect, it, vi } from "vitest";
import { createFrontendBootController } from "./frontendBoot";

describe("frontend boot controller", () => {
  it("reports readiness when startup completes", () => {
    const bridge = { ready: vi.fn(), fail: vi.fn() };
    const controller = createFrontendBootController(() => bridge);

    controller.ready();

    expect(bridge.ready).toHaveBeenCalledOnce();
    expect(bridge.fail).not.toHaveBeenCalled();
  });

  it("does not overwrite a startup failure with readiness", () => {
    const bridge = { ready: vi.fn(), fail: vi.fn() };
    const controller = createFrontendBootController(() => bridge);
    const error = new Error("mount failed");

    controller.fail(error);
    controller.ready();

    expect(bridge.fail).toHaveBeenCalledWith(error);
    expect(bridge.ready).not.toHaveBeenCalled();
  });
});
