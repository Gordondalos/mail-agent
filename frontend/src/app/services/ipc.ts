import { Injectable } from '@angular/core';
import { invoke } from '@tauri-apps/api/core';
import { listen, UnlistenFn } from '@tauri-apps/api/event';

@Injectable({
  providedIn: 'root'
})
export class Ipc {
  invoke<T = unknown>(cmd: string, payload?: Record<string, unknown>): Promise<T> {
    return invoke<T>(cmd, payload as any);
  }

  on<T = unknown>(eventName: string, handler: (payload: T) => void): Promise<UnlistenFn> {
    return listen<T>(eventName, (e) => handler(e.payload as T));
  }
}
