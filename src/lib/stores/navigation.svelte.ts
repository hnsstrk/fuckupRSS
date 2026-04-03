import { networkStore } from "./network.svelte";

export type AppView =
  | "erisianArchives"
  | "network"
  | "fnord"
  | "mindfuck"
  | "briefings"
  | "storyClusters"
  | "settings";

class NavigationStore {
  currentView = $state<AppView>("erisianArchives");
  pendingKeywordId = $state<number | null>(null);

  navigateToNetwork(keywordId?: number): void {
    this.currentView = "network";
    if (keywordId !== undefined) {
      this.pendingKeywordId = keywordId;
      networkStore.selectKeyword(keywordId);
    }
  }

  navigateToArticles(): void {
    this.currentView = "erisianArchives";
    this.pendingKeywordId = null;
  }

  navigateTo(view: AppView): void {
    this.currentView = view;
  }

  async navigateToArticle(articleId: number): Promise<void> {
    // Lazy import to avoid circular dependency
    const { appState } = await import("./state.svelte");

    if (this.currentView !== "erisianArchives") {
      this.currentView = "erisianArchives";
    }

    await appState.ensureFnordLoaded(articleId);
    appState.selectFnord(articleId);
  }
}

export const navigationStore = new NavigationStore();
