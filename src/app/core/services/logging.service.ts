import { Injectable } from '@angular/core';

export enum LogLevel {
  Debug = 0,
  Info = 1,
  Warn = 2,
  Error = 3
}

@Injectable({
  providedIn: 'root'
})
export class LoggingService {
  private currentLogLevel = LogLevel.Debug; // Will be configured based on environment

  debug(message: string, ...args: unknown[]): void {
    this.log(LogLevel.Debug, message, ...args);
  }

  info(message: string, ...args: unknown[]): void {
    this.log(LogLevel.Info, message, ...args);
  }

  warn(message: string, ...args: unknown[]): void {
    this.log(LogLevel.Warn, message, ...args);
  }

  error(message: string, error?: unknown, ...args: unknown[]): void {
    this.log(LogLevel.Error, message, error, ...args);
  }

  private log(level: LogLevel, message: string, ...args: unknown[]): void {
    if (level < this.currentLogLevel) {
      return;
    }

    const timestamp = new Date().toISOString();
    const logEntry = {
      timestamp,
      level: LogLevel[level],
      message,
      args
    };

    switch (level) {
      case LogLevel.Debug:
        console.debug(`[${timestamp}] DEBUG:`, message, ...args);
        break;
      case LogLevel.Info:
        console.info(`[${timestamp}] INFO:`, message, ...args);
        break;
      case LogLevel.Warn:
        console.warn(`[${timestamp}] WARN:`, message, ...args);
        break;
      case LogLevel.Error:
        console.error(`[${timestamp}] ERROR:`, message, ...args);
        break;
    }

    // Persist log entry (in production, this could send to external service)
     this.persistLog(logEntry);
  }

  private persistLog(logEntry: Record<string, unknown>): void {
     // Store logs in sessionStorage for debugging
     // In production, this could be replaced with external logging service
     try {
       const logs = JSON.parse(sessionStorage.getItem('wallet-logs') || '[]') as Record<string, unknown>[];
       logs.push(logEntry);

       // Keep only last 100 log entries to prevent storage bloat
       if (logs.length > 100) {
         logs.splice(0, logs.length - 100);
       }

       sessionStorage.setItem('wallet-logs', JSON.stringify(logs));
     } catch (error) {
       console.error('Failed to persist log:', error);
     }
   }

  getLogs(): Record<string, unknown>[] {
    try {
      return JSON.parse(sessionStorage.getItem('wallet-logs') || '[]') as Record<string, unknown>[];
    } catch {
      return [];
    }
  }

  clearLogs(): void {
    sessionStorage.removeItem('wallet-logs');
  }
}