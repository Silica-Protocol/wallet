import { Injectable } from '@angular/core';
import { BehaviorSubject, Observable } from 'rxjs';

export function MeasurePerformance(label: string, thresholdMs = 200) {
  return (_target: unknown, propertyKey: string, descriptor: PropertyDescriptor) => {
    const originalMethod = descriptor.value;

    descriptor.value = async function (...args: any[]) {
      if (!performance || !performance.now) {
        return originalMethod.apply(this, args);
      }

      const start = performance.now();
      try {
        const result = await originalMethod.apply(this, args);
        const end = performance.now();
        const duration = end - start;

        if (duration > thresholdMs) {
          console.warn(`Performance warning for ${label}: ${duration.toFixed(2)}ms`);
        } else {
          console.log(`Performance for ${label}: ${duration.toFixed(2)}ms`);
        }

        return result;
      } catch (error) {
        const end = performance.now();
        const duration = end - start;
        console.error(`Error during ${label} after ${duration.toFixed(2)}ms:`, error);
        throw error;
      }
    };

    return descriptor;
  };
}

@Injectable({
  providedIn: 'root'
})
export class WasmLoaderService {
  private wasmReady$ = new BehaviorSubject<boolean>(false);
  private wasmInstance: any = null;
  private wasmConfig: any = null;

  constructor() {
    void this.loadWasmModule();
  }

  private async loadWasmModule(): Promise<void> {
    try {
  this.wasmInstance = await import('../../../../assets/wasm/chert_wallet_wasm');
      this.wasmReady$.next(true);
      console.log('Wallet WASM module loaded successfully');
    } catch (error) {
      console.error('Failed to load wallet WASM module:', error);
      this.wasmReady$.next(false);
    }
  }

  @MeasurePerformance('wasm_execution', 1000)
  async executeWasmFunction(functionName: string, ...args: any[]): Promise<any> {
    await this.ensureWasmReady();

    if (!this.wasmInstance || !this.wasmInstance[functionName]) {
      throw new Error(`WASM function not available: ${functionName}`);
    }

    try {
      const result = await this.wasmInstance[functionName](...args);
      return result;
    } catch (error) {
      console.error(`Error executing WASM function ${functionName}:`, error);
      throw error;
    }
  }

  async configure(config: any): Promise<void> {
    this.wasmConfig = config;
    await this.executeWasmFunction('set_config', config);
  }

  async ensureWasmReady(): Promise<void> {
    if (this.wasmInstance) {
      return;
    }

    const loaded = await new Promise<boolean>(resolve => {
      const subscription = this.wasmReady$.subscribe(isReady => {
        if (isReady) {
          subscription.unsubscribe();
          resolve(true);
        }
      });

      setTimeout(() => {
        subscription.unsubscribe();
        resolve(false);
      }, 10000);
    });

    if (!loaded) {
      throw new Error('WASM module not ready after timeout');
    }
  }

  isWasmLoaded(): Observable<boolean> {
    return this.wasmReady$.asObservable();
  }

  getConfiguredNetwork(): string {
    return this.wasmConfig?.networkName ?? 'unknown';
  }
}
