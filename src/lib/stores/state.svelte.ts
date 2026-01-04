import { invoke } from "@tauri-apps/api/core";

// Types matching Rust structs
export interface Pentacle {
  id: number;
  url: string;
  title: string | null;
  description: string | null;
  site_url: string | null;
  icon_url: string | null;
  default_quality: number;
  article_count: number;
  unread_count: number;
}

export interface Fnord {
  id: number;
  pentacle_id: number;
  pentacle_title: string | null;
  guid: string;
  url: string;
  title: string;
  author: string | null;
  content_raw: string | null;
  content_full: string | null;
  summary: string | null;
  image_url: string | null;
  published_at: string | null;
  status: "fnord" | "illuminated" | "golden_apple";
  political_bias: number | null;
  sachlichkeit: number | null;
  quality_score: number | null;
  article_type: string | null;
}

export interface FnordFilter {
  pentacle_id?: number;
  status?: string;
  limit?: number;
}

// Svelte 5 runes-based state
class AppState {
  pentacles = $state<Pentacle[]>([]);
  fnords = $state<Fnord[]>([]);
  selectedPentacleId = $state<number | null>(null);
  selectedFnordId = $state<number | null>(null);
  loading = $state(false);
  error = $state<string | null>(null);

  get selectedPentacle(): Pentacle | undefined {
    return this.pentacles.find((p) => p.id === this.selectedPentacleId);
  }

  get selectedFnord(): Fnord | undefined {
    return this.fnords.find((f) => f.id === this.selectedFnordId);
  }

  get totalUnread(): number {
    return this.pentacles.reduce((sum, p) => sum + p.unread_count, 0);
  }

  async loadPentacles(): Promise<void> {
    try {
      this.loading = true;
      this.error = null;
      this.pentacles = await invoke<Pentacle[]>("get_pentacles");
    } catch (e) {
      this.error = String(e);
      console.error("Failed to load pentacles:", e);
    } finally {
      this.loading = false;
    }
  }

  async loadFnords(filter?: FnordFilter): Promise<void> {
    try {
      this.loading = true;
      this.error = null;
      this.fnords = await invoke<Fnord[]>("get_fnords", { filter });
    } catch (e) {
      this.error = String(e);
      console.error("Failed to load fnords:", e);
    } finally {
      this.loading = false;
    }
  }

  async addPentacle(url: string, title?: string): Promise<void> {
    try {
      this.loading = true;
      this.error = null;
      const pentacle = await invoke<Pentacle>("add_pentacle", { url, title });
      this.pentacles = [...this.pentacles, pentacle];
    } catch (e) {
      this.error = String(e);
      console.error("Failed to add pentacle:", e);
    } finally {
      this.loading = false;
    }
  }

  async deletePentacle(id: number): Promise<void> {
    try {
      this.loading = true;
      this.error = null;
      await invoke("delete_pentacle", { id });
      this.pentacles = this.pentacles.filter((p) => p.id !== id);
      if (this.selectedPentacleId === id) {
        this.selectedPentacleId = null;
      }
    } catch (e) {
      this.error = String(e);
      console.error("Failed to delete pentacle:", e);
    } finally {
      this.loading = false;
    }
  }

  async updateFnordStatus(
    id: number,
    status: "fnord" | "illuminated" | "golden_apple"
  ): Promise<void> {
    try {
      await invoke("update_fnord_status", { id, status });
      // Update local state
      const fnord = this.fnords.find((f) => f.id === id);
      if (fnord) {
        fnord.status = status;
      }
      // Reload pentacles to update counts
      await this.loadPentacles();
    } catch (e) {
      this.error = String(e);
      console.error("Failed to update fnord status:", e);
    }
  }

  selectPentacle(id: number | null): void {
    this.selectedPentacleId = id;
    this.selectedFnordId = null;
    if (id !== null) {
      this.loadFnords({ pentacle_id: id });
    } else {
      this.loadFnords();
    }
  }

  selectFnord(id: number | null): void {
    this.selectedFnordId = id;
    // Auto-mark as read when selecting
    if (id !== null) {
      const fnord = this.fnords.find((f) => f.id === id);
      if (fnord && fnord.status === "fnord") {
        this.updateFnordStatus(id, "illuminated");
      }
    }
  }

  selectNextFnord(): void {
    if (this.fnords.length === 0) return;

    const currentIndex = this.fnords.findIndex(
      (f) => f.id === this.selectedFnordId
    );
    const nextIndex =
      currentIndex < this.fnords.length - 1 ? currentIndex + 1 : 0;
    this.selectFnord(this.fnords[nextIndex].id);
  }

  selectPrevFnord(): void {
    if (this.fnords.length === 0) return;

    const currentIndex = this.fnords.findIndex(
      (f) => f.id === this.selectedFnordId
    );
    const prevIndex =
      currentIndex > 0 ? currentIndex - 1 : this.fnords.length - 1;
    this.selectFnord(this.fnords[prevIndex].id);
  }

  toggleGoldenApple(id: number): void {
    const fnord = this.fnords.find((f) => f.id === id);
    if (!fnord) return;

    const newStatus =
      fnord.status === "golden_apple" ? "illuminated" : "golden_apple";
    this.updateFnordStatus(id, newStatus);
  }
}

export const appState = new AppState();

// Export selected state for components
export const selectedPentacle = {
  get current() {
    return appState.selectedPentacle;
  },
};

export const selectedFnord = {
  get current() {
    return appState.selectedFnord;
  },
};
