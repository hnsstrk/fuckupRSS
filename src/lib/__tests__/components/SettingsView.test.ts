import { describe, it, expect, vi } from "vitest";
import { appState } from "../../stores/state.svelte";

vi.mock("../../stores/state.svelte", async () => {
  const actual = await vi.importActual<typeof import("../../stores/state.svelte")>(
    "../../stores/state.svelte",
  );

  return {
    ...actual,
    appState: {
      ...actual.appState,
      checkOllama: vi.fn().mockResolvedValue({
        available: true,
        models: ["model"],
        recommended_main: "model",
        recommended_embedding: "embed",
        has_recommended_main: true,
        has_recommended_embedding: true,
      }),
    },
  };
});

describe("SettingsView", () => {
  it("refreshes Ollama status from appState", async () => {
    await appState.checkOllama();
    expect(appState.checkOllama).toHaveBeenCalledTimes(1);
  });
});
