/**
 * Structured logging for fuckupRSS frontend
 *
 * Provides consistent logging with levels that can be filtered.
 * In development, all logs are shown. In production, only info and above.
 *
 * Uses tauri-plugin-log to send logs to both console and Rust backend.
 */

import { invoke } from '@tauri-apps/api/core';

export type LogLevel = 'error' | 'warn' | 'info' | 'debug' | 'trace';

const LOG_LEVELS: Record<LogLevel, number> = {
  error: 0,
  warn: 1,
  info: 2,
  debug: 3,
  trace: 4,
};

// ANSI color codes for terminal output (works in Tauri dev console)
const COLORS = {
  error: '\x1b[31m', // red
  warn: '\x1b[33m', // yellow
  info: '\x1b[36m', // cyan
  debug: '\x1b[35m', // magenta
  trace: '\x1b[90m', // gray
  reset: '\x1b[0m',
};

// Browser console styles
const BROWSER_STYLES = {
  error: 'color: #f38ba8; font-weight: bold',
  warn: 'color: #f9e2af; font-weight: bold',
  info: 'color: #89dceb',
  debug: 'color: #cba6f7',
  trace: 'color: #6c7086',
};

class Logger {
  #currentLevel: LogLevel = import.meta.env.DEV ? 'debug' : 'info';
  #context: string = 'app';

  /**
   * Create a logger instance
   * @param context - Optional context/module name for log messages
   */
  constructor(context?: string) {
    if (context) {
      this.#context = context;
    }
  }

  /**
   * Set the current log level
   * Messages below this level will not be shown
   */
  setLevel(level: LogLevel) {
    this.#currentLevel = level;
    // Also notify backend
    invoke('set_log_level', { level }).catch(() => {
      // Ignore errors if command not available
    });
  }

  /**
   * Get the current log level
   */
  getLevel(): LogLevel {
    return this.#currentLevel;
  }

  /**
   * Check if a log level should be shown
   */
  #shouldLog(level: LogLevel): boolean {
    return LOG_LEVELS[level] <= LOG_LEVELS[this.#currentLevel];
  }

  /**
   * Format and output a log message
   */
  #log(level: LogLevel, message: string, ...args: unknown[]) {
    if (!this.#shouldLog(level)) return;

    const timestamp = new Date().toISOString().slice(11, 23);
    const prefix = `[${timestamp}] [${level.toUpperCase().padEnd(5)}] [${this.#context}]`;

    // Check if we're in a browser environment with styled console
    if (typeof window !== 'undefined' && 'chrome' in window) {
      // Browser with styled console support
      const style = BROWSER_STYLES[level];
      console[level === 'trace' ? 'debug' : level](
        `%c${prefix}%c ${message}`,
        style,
        '',
        ...args
      );
    } else {
      // Terminal output with ANSI colors
      const color = COLORS[level];
      const reset = COLORS.reset;
      console[level === 'trace' ? 'debug' : level](
        `${color}${prefix}${reset} ${message}`,
        ...args
      );
    }
  }

  /**
   * Log an error message
   * Use for errors that prevent normal operation
   */
  error(message: string, ...args: unknown[]) {
    this.#log('error', message, ...args);
  }

  /**
   * Log a warning message
   * Use for potential issues that don't prevent operation
   */
  warn(message: string, ...args: unknown[]) {
    this.#log('warn', message, ...args);
  }

  /**
   * Log an info message
   * Use for general information about app operation
   */
  info(message: string, ...args: unknown[]) {
    this.#log('info', message, ...args);
  }

  /**
   * Log a debug message
   * Use for detailed debugging information
   */
  debug(message: string, ...args: unknown[]) {
    this.#log('debug', message, ...args);
  }

  /**
   * Log a trace message
   * Use for very detailed tracing information
   */
  trace(message: string, ...args: unknown[]) {
    this.#log('trace', message, ...args);
  }

  /**
   * Create a child logger with a specific context
   */
  child(context: string): Logger {
    const child = new Logger(`${this.#context}:${context}`);
    child.#currentLevel = this.#currentLevel;
    return child;
  }

  /**
   * Time a function and log the duration
   */
  async time<T>(label: string, fn: () => Promise<T>): Promise<T> {
    const start = performance.now();
    try {
      const result = await fn();
      const duration = (performance.now() - start).toFixed(2);
      this.debug(`${label} completed in ${duration}ms`);
      return result;
    } catch (error) {
      const duration = (performance.now() - start).toFixed(2);
      this.error(`${label} failed after ${duration}ms`, error);
      throw error;
    }
  }

  /**
   * Log a group of related messages
   */
  group(label: string, fn: () => void) {
    if (!this.#shouldLog('debug')) return;
    console.group(label);
    fn();
    console.groupEnd();
  }
}

// Default logger instance
export const log = new Logger();

// Factory function to create loggers for specific modules
export function createLogger(context: string): Logger {
  return new Logger(context);
}

// Re-export for convenience
export { Logger };
