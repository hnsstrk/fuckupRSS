/**
 * Consistently format unknown error values into readable strings.
 * Replaces inconsistent patterns like `String(e)`, `e instanceof Error ? e.message : String(e)`.
 */
export function formatError(error: unknown): string {
  if (error instanceof Error) return error.message;
  if (typeof error === "string") return error;
  return String(error);
}
