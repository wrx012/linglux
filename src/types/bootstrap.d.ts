export {};

declare global {
  interface Window {
    __LINGLUX_BOOT__?: {
      ready: () => void;
      fail: (reason: unknown) => void;
    };
  }
}
