<script lang="ts">
  import { _ } from "svelte-i18n";
  import { invoke } from "@tauri-apps/api/core";
  import { toasts } from "../../stores/state.svelte";

  interface Props {
    ollamaAvailable: boolean;
  }

  let { ollamaAvailable }: Props = $props();

  // Prompts state
  let summaryPrompt = $state("");
  let analysisPrompt = $state("");
  let defaultPrompts = $state<{
    summary_prompt: string;
    analysis_prompt: string;
  } | null>(null);
  let promptsModified = $state(false);

  export async function init() {
    await loadPrompts();
  }

  async function loadPrompts() {
    try {
      const prompts = await invoke<{
        summary_prompt: string;
        analysis_prompt: string;
      }>("get_prompts");
      summaryPrompt = prompts.summary_prompt;
      analysisPrompt = prompts.analysis_prompt;

      defaultPrompts = await invoke<{
        summary_prompt: string;
        analysis_prompt: string;
      }>("get_default_prompts");

      if (defaultPrompts) {
        promptsModified =
          summaryPrompt !== defaultPrompts.summary_prompt ||
          analysisPrompt !== defaultPrompts.analysis_prompt;
      }
    } catch (e) {
      console.error("Failed to load prompts:", e);
    }
  }

  function handlePromptChange() {
    if (defaultPrompts) {
      promptsModified =
        summaryPrompt !== defaultPrompts.summary_prompt ||
        analysisPrompt !== defaultPrompts.analysis_prompt;
    }
  }

  async function handleSavePrompts() {
    try {
      await invoke("set_prompts", {
        summaryPrompt: summaryPrompt,
        analysisPrompt: analysisPrompt,
      });
      promptsModified = false;
      toasts.success($_("settings.promptsSaved"));
    } catch (e) {
      console.error("Failed to save prompts:", e);
      toasts.error($_("settings.saveError"));
    }
  }

  async function handleResetPrompts() {
    try {
      const prompts = await invoke<{
        summary_prompt: string;
        analysis_prompt: string;
      }>("reset_prompts");
      summaryPrompt = prompts.summary_prompt;
      analysisPrompt = prompts.analysis_prompt;
      promptsModified = false;
    } catch (e) {
      console.error("Failed to reset prompts:", e);
    }
  }
</script>

<h3>{$_("settings.prompts.title")}</h3>

{#if !ollamaAvailable}
  <div class="status-unavailable">
    <i class="status-icon fa-solid fa-xmark"></i>
    {$_("settings.ollama.unavailable")}
  </div>
{:else}
  <div class="setting-group">
    <label class="label" for="summary-prompt">{$_("settings.prompts.summaryPrompt")}</label>
    <textarea
      id="summary-prompt"
      class="prompt-textarea"
      bind:value={summaryPrompt}
      oninput={handlePromptChange}
      rows="6"
    ></textarea>
  </div>

  <div class="setting-group">
    <label class="label" for="analysis-prompt">{$_("settings.prompts.analysisPrompt")}</label>
    <textarea
      id="analysis-prompt"
      class="prompt-textarea"
      bind:value={analysisPrompt}
      oninput={handlePromptChange}
      rows="8"
    ></textarea>
  </div>

  {#if promptsModified}
    <div class="prompt-actions">
      <button type="button" class="btn-save-prompts" onclick={handleSavePrompts}>
        {$_("settings.prompts.save") || "Prompts speichern"}
      </button>
      <button type="button" class="btn-reset" onclick={handleResetPrompts}>
        {$_("settings.prompts.reset")}
      </button>
    </div>
  {/if}
{/if}

<style>
  h3 {
    margin: 0 0 1rem 0;
    font-size: 1rem;
    color: var(--text-secondary);
  }

  .setting-group {
    margin-bottom: 1.25rem;
    max-width: 600px;
  }

  .setting-group > label,
  .setting-group > .label {
    display: block;
    margin-bottom: 0.375rem;
    font-weight: 500;
    color: var(--text-primary);
  }

  .status-unavailable {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
    color: var(--status-error);
    padding: 0.5rem;
    background-color: rgba(243, 139, 168, 0.1);
    border-radius: 0.375rem;
  }

  .status-unavailable .status-icon {
    display: inline;
  }

  .status-icon {
    font-weight: bold;
  }

  /* Prompts */
  .prompt-textarea {
    width: 100%;
    padding: 0.75rem;
    border: 1px solid var(--border-default);
    border-radius: 0.375rem;
    background-color: var(--bg-overlay);
    color: var(--text-primary);
    font-family: monospace;
    font-size: 0.875rem;
    resize: vertical;
    min-height: 100px;
  }

  .prompt-textarea:focus {
    outline: none;
    border-color: var(--accent-primary);
  }

  /* Prompt Actions */
  .prompt-actions {
    display: flex;
    gap: 0.5rem;
    margin-top: 0.5rem;
  }

  .btn-save-prompts {
    padding: 0.5rem 1rem;
    border: none;
    border-radius: 0.375rem;
    background-color: var(--accent-primary);
    color: var(--text-on-accent);
    font-size: 0.875rem;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.2s;
  }

  .btn-save-prompts:hover {
    filter: brightness(1.1);
  }

  .btn-reset {
    padding: 0.5rem 1rem;
    border: 1px solid var(--border-default);
    border-radius: 0.375rem;
    background: none;
    color: var(--text-secondary);
    font-size: 0.875rem;
    cursor: pointer;
    transition: all 0.2s;
  }

  .btn-reset:hover {
    border-color: var(--accent-primary);
    color: var(--accent-primary);
  }
</style>
