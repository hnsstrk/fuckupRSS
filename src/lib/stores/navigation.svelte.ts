import type { MainView } from "../types";
import { networkStore } from "./network.svelte";

class NavigationStore {
  currentView = $state<MainView>('articles');
  pendingKeywordId = $state<number | null>(null);

  navigateToNetwork(keywordId?: number): void {
    this.currentView = 'network';
    if (keywordId !== undefined) {
      this.pendingKeywordId = keywordId;
      networkStore.selectKeyword(keywordId);
    }
  }

  navigateToArticles(): void {
    this.currentView = 'articles';
    this.pendingKeywordId = null;
  }

  navigateToFnord(): void {
    this.currentView = 'fnord';
    this.pendingKeywordId = null;
  }

  toggleView(): void {
    if (this.currentView === 'network') {
      this.navigateToArticles();
    } else {
      this.navigateToNetwork();
    }
  }

  consumePendingKeyword(): number | null {
    const id = this.pendingKeywordId;
    this.pendingKeywordId = null;
    return id;
  }
}

export const navigationStore = new NavigationStore();
