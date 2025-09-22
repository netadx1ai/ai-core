import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { detectPlatform, getAdaptiveStyles } from "../platform";

// Mock window and navigator
const mockNavigator = {
  userAgent: "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
};

Object.defineProperty(global, "navigator", {
  value: mockNavigator,
  writable: true,
});

Object.defineProperty(global, "window", {
  value: {},
  writable: true,
});

describe("platform utilities", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    // Reset window object
    global.window = {} as Window & typeof globalThis;
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  describe("detectPlatform", () => {
    it("should detect web platform by default", () => {
      const platform = detectPlatform();

      expect(platform.isWeb).toBe(true);
      expect(platform.isTauri).toBe(false);
      expect(platform.isDesktop).toBe(false);
      expect(platform.platform).toBe("web");
    });

    it("should detect Tauri desktop platform", () => {
      global.window = { __TAURI__: {} } as Window &
        typeof globalThis & { __TAURI__: unknown };

      const platform = detectPlatform();

      expect(platform.isTauri).toBe(true);
      expect(platform.isWeb).toBe(false);
      expect(platform.isDesktop).toBe(true);
      expect(platform.platform).toBe("desktop");
    });

    it("should detect mobile platform", () => {
      mockNavigator.userAgent =
        "Mozilla/5.0 (iPhone; CPU iPhone OS 14_0 like Mac OS X)";

      const platform = detectPlatform();

      expect(platform.isMobile).toBe(true);
      expect(platform.platform).toBe("mobile");
    });

    it("should detect Windows OS", () => {
      mockNavigator.userAgent = "Mozilla/5.0 (Windows NT 10.0; Win64; x64)";

      const platform = detectPlatform();

      expect(platform.os).toBe("windows");
    });

    it("should detect macOS", () => {
      mockNavigator.userAgent =
        "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7)";

      const platform = detectPlatform();

      expect(platform.os).toBe("macos");
    });

    it("should detect Linux", () => {
      mockNavigator.userAgent = "Mozilla/5.0 (X11; Linux x86_64)";

      const platform = detectPlatform();

      expect(platform.os).toBe("linux");
    });

    it("should detect Android", () => {
      mockNavigator.userAgent = "Mozilla/5.0 (Linux; Android 10; SM-G975F)";

      const platform = detectPlatform();

      expect(platform.os).toBe("android");
      expect(platform.isMobile).toBe(true);
    });

    it("should detect iOS", () => {
      mockNavigator.userAgent =
        "Mozilla/5.0 (iPhone; CPU iPhone OS 14_0 like Mac OS X) AppleWebKit/605.1.15";

      const platform = detectPlatform();

      expect(platform.os).toBe("ios");
      expect(platform.isMobile).toBe(true);
    });
  });

  describe("getAdaptiveStyles", () => {
    it("should return mobile styles for mobile platform", () => {
      const platform = {
        isTauri: false,
        isWeb: true,
        isMobile: true,
        isDesktop: false,
        platform: "mobile" as const,
      };

      const styles = getAdaptiveStyles(platform);

      expect(styles).toContain("touch-manipulation");
      expect(styles).toContain("select-none");
    });

    it("should return desktop styles for desktop platform", () => {
      const platform = {
        isTauri: true,
        isWeb: false,
        isMobile: false,
        isDesktop: true,
        platform: "desktop" as const,
      };

      const styles = getAdaptiveStyles(platform);

      expect(styles).toContain("cursor-pointer");
    });

    it("should return base styles for web platform", () => {
      const platform = {
        isTauri: false,
        isWeb: true,
        isMobile: false,
        isDesktop: false,
        platform: "web" as const,
      };

      const styles = getAdaptiveStyles(platform);

      expect(styles).toBe("transition-all duration-200");
    });

    it("should always include base styles", () => {
      const platforms = [
        { platform: "mobile" },
        { platform: "desktop" },
        { platform: "web" },
      ] as const;

      platforms.forEach(({ platform }) => {
        const platformInfo = {
          isTauri: false,
          isWeb: true,
          isMobile: false,
          isDesktop: false,
          platform,
        };

        const styles = getAdaptiveStyles(platformInfo);
        expect(styles).toContain("transition-all duration-200");
      });
    });
  });
});
