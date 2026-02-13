// Platform detection store using Svelte 5 runes
// Detects macOS/Linux/Windows via Tauri command

import { invoke } from "@tauri-apps/api/core";
import { log } from "../logger";

type Platform = "macos" | "linux" | "windows" | "unknown" | "";

class PlatformStore {
  #platform = $state<Platform>("");
  #initialized = false;

  // Derived states for convenience
  get isMacOS(): boolean {
    return this.#platform === "macos";
  }

  get isLinux(): boolean {
    return this.#platform === "linux";
  }

  get isWindows(): boolean {
    return this.#platform === "windows";
  }

  get platform(): Platform {
    return this.#platform;
  }

  get isInitialized(): boolean {
    return this.#initialized;
  }

  /**
   * Initialize platform detection.
   * Should be called once on app startup (e.g., in App.svelte onMount).
   */
  async init(): Promise<void> {
    if (this.#initialized) return;

    try {
      const platform = await invoke<string>("get_platform");
      this.#platform = platform as Platform;
      this.#initialized = true;

      // Apply platform-specific CSS class to html element
      this.#applyPlatformClass();

      log.info(`Platform detected: ${this.#platform}`);
    } catch (e) {
      log.error("Failed to detect platform:", e);
      // Fallback: try to detect from user agent
      this.#detectFromUserAgent();
      this.#applyPlatformClass();
    }
  }

  /**
   * Fallback platform detection using navigator.userAgent
   */
  #detectFromUserAgent(): void {
    if (typeof navigator === "undefined") {
      this.#platform = "unknown";
      return;
    }

    const userAgent = navigator.userAgent.toLowerCase();

    if (userAgent.includes("mac")) {
      this.#platform = "macos";
    } else if (userAgent.includes("linux")) {
      this.#platform = "linux";
    } else if (userAgent.includes("win")) {
      this.#platform = "windows";
    } else {
      this.#platform = "unknown";
    }

    this.#initialized = true;
  }

  /**
   * Apply platform-specific CSS class to the html element.
   * Classes: platform-macos, platform-linux, platform-windows
   */
  #applyPlatformClass(): void {
    if (typeof document === "undefined") return;

    const html = document.documentElement;

    // Remove any existing platform classes
    html.classList.remove(
      "platform-macos",
      "platform-linux",
      "platform-windows",
      "platform-unknown",
    );

    // Add the current platform class
    if (this.#platform) {
      html.classList.add(`platform-${this.#platform}`);
    }
  }
}

export const platformStore = new PlatformStore();
