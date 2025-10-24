import { Injectable } from '@angular/core';
import { Ipc } from './ipc';

@Injectable({
  providedIn: 'root'
})
export class Settings {
  constructor(private readonly ipc: Ipc) {}

  async initialise(): Promise<{ settings: any; authorised: boolean }> {
    return this.ipc.invoke('initialise');
  }

  async update(update: Record<string, unknown>): Promise<any> {
    return this.ipc.invoke('update_settings', { update });
  }
}
