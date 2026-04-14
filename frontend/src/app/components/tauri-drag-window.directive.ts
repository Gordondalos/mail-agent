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

    const target = ev.target as HTMLElement | null;
    if (this.shouldSkipDrag(target)) {
      return;
    }

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

  private shouldSkipDrag(target: HTMLElement | null): boolean {
    if (!target) {
      return false;
    }

    // Уважаем Tauri no-drag область и интерактивные элементы.
    if (target.closest('[data-tauri-drag-region="false"]')) {
      return true;
    }
    if (target.closest('.no-drag')) {
      return true;
    }
    if (target.closest('button, a, input, textarea, select, option, summary, details, [contenteditable="true"]')) {
      return true;
    }
    return false;
  }
}
