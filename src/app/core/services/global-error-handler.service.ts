import { Injectable, ErrorHandler } from '@angular/core';

@Injectable()
export class GlobalErrorHandler implements ErrorHandler {
  handleError(error: unknown): void {
     console.error('Global error caught:', error);

     // In development, show detailed error
     if (!this.isProduction()) {
       const errorObj = error as Error;
       console.error('Error details:', {
         message: errorObj.message,
         stack: errorObj.stack,
         timestamp: new Date().toISOString()
       });
     }

     // Send to logging service
     this.logError(error);
   }

  private isProduction(): boolean {
    // This will be replaced with proper environment detection
    return false;
  }

  private logError(error: unknown): void {
     // Log errors using the logging service
     const errorObj = error as Error;

     // Use the logging service for structured logging
     // For now, we can't inject services in error handlers, so we'll use console for now
     // In a production app, we'd have a global logging instance
     console.error('Global error logged:', {
       message: errorObj.message || 'Unknown error',
       stack: errorObj.stack || 'No stack trace',
       timestamp: new Date().toISOString(),
       userAgent: navigator.userAgent,
       url: window.location.href
     });
   }
}