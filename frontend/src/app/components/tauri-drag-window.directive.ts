// src/app/shared/tauri-drag-window.directive.ts
import { Directive, HostListener } from '@angular/core';

@Directive({
  selector: '[appTauriDragWindow]',
  standalone: true,
})
export class TauriDragWindowDirective {
  @HostListener('mousedown', ['$event'])
  async onMouseDown(ev: MouseEvent) {
    // Игнорируем правую кнопку и модификаторы
    if (ev.button !== 0 || ev.altKey || ev.ctrlKey || ev.metaKey || ev.shiftKey) return;

    // Защита: если запущено в браузере (нет Tauri), просто выходим
    // @ts-ignore
    if (!(window as any).__TAURI__) return;

    const { getCurrentWindow } = await import('@tauri-apps/api/window');
    const win = getCurrentWindow();
    try {
      await win.startDragging();
    } catch (e) {
      console.error('Tauri startDragging failed:', e);
    }
  }
}
