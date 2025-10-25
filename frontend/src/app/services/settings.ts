import { Injectable } from '@angular/core';
import { Ipc } from './ipc';

export interface VoicePreset {
  id: string;
  label: string;
  file_name: string;
  path: string;
}

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

  async voicePresets(): Promise<VoicePreset[]> {
    return this.ipc.invoke('list_voice_tracks');
  }
}
