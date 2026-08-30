type FrontendBootBridge = {
  ready: () => void;
  fail: (reason: unknown) => void;
};

export function createFrontendBootController(getBridge: () => FrontendBootBridge | undefined) {
  let startupFailed = false;

  return {
    fail(reason: unknown) {
      startupFailed = true;
      getBridge()?.fail(reason);
    },
    ready() {
      if (!startupFailed) {
        getBridge()?.ready();
      }
    },
  };
}
