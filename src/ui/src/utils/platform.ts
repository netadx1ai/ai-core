/**
 * Platform detection utilities for adaptive UI components
 */

export interface PlatformInfo {
  isTauri: boolean;
  isWeb: boolean;
  isMobile: boolean;
  isDesktop: boolean;
  platform: "web" | "desktop" | "mobile";
  os?: "windows" | "macos" | "linux" | "ios" | "android";
}

export function detectPlatform(): PlatformInfo {
  // Check if running in Tauri
  const isTauri = typeof window !== "undefined" && "__TAURI__" in window;

  // Basic mobile detection
  const isMobile =
    typeof window !== "undefined" &&
    /Android|webOS|iPhone|iPad|iPod|BlackBerry|IEMobile|Opera Mini/i.test(
      navigator.userAgent,
    );

  const isDesktop = isTauri && !isMobile;
  const isWeb = !isTauri;

  let platform: "web" | "desktop" | "mobile";
  if (isMobile) {
    platform = "mobile";
  } else if (isDesktop) {
    platform = "desktop";
  } else {
    platform = "web";
  }

  // OS detection
  let os: PlatformInfo["os"];
  if (typeof window !== "undefined") {
    const userAgent = navigator.userAgent.toLowerCase();
    if (userAgent.includes("win")) os = "windows";
    else if (userAgent.includes("android")) os = "android";
    else if (
      userAgent.includes("ios") ||
      userAgent.includes("iphone") ||
      userAgent.includes("ipad")
    )
      os = "ios";
    else if (userAgent.includes("mac")) os = "macos";
    else if (userAgent.includes("linux")) os = "linux";
  }

  return {
    isTauri,
    isWeb,
    isMobile,
    isDesktop,
    platform,
    os,
  };
}

export function getAdaptiveStyles(platform: PlatformInfo) {
  const baseStyles = "transition-all duration-200";

  switch (platform.platform) {
    case "mobile":
      return `${baseStyles} touch-manipulation select-none`;
    case "desktop":
      return `${baseStyles} cursor-pointer`;
    default:
      return baseStyles;
  }
}
