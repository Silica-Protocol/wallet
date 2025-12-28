import { ApplicationConfig, provideZoneChangeDetection, ErrorHandler } from '@angular/core';
import { provideRouter } from '@angular/router';
import { provideAnimationsAsync } from '@angular/platform-browser/animations/async';
import { provideHttpClient } from '@angular/common/http';

import { routes } from './app.routes';
import { GlobalErrorHandler } from './core/services/global-error-handler.service';
import { provideWalletBackend } from './core/services/wallet-backend.service';
import { provideWalletBackend as provideTauriWalletBackend } from './core/services/wallet-backend.service.tauri';

export const appConfig: ApplicationConfig = {
  providers: [
    provideZoneChangeDetection({ eventCoalescing: true }),
    provideRouter(routes),
    provideAnimationsAsync(),
    provideHttpClient(),
    { provide: ErrorHandler, useClass: GlobalErrorHandler },
    // Choose backend based on environment
    ...(isTauri() ? provideTauriWalletBackend() : provideWalletBackend())
  ]
};

// Detect if running in Tauri
function isTauri(): boolean {
  return typeof window !== 'undefined' && (window as any).__TAURI__ !== undefined;
}
