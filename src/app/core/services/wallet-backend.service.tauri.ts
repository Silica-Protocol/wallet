import { Provider } from '@angular/core';
import { WALLET_BACKEND } from './wallet-backend.interface';
import { TauriService } from './tauri.service';

export function provideWalletBackend(): Provider[] {
  return [
    {
      provide: WALLET_BACKEND,
      useExisting: TauriService
    }
  ];
}
