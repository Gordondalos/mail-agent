# Тестирование отображения тела письма

## Что было сделано

Я добавил детальное логирование в фронтенд для отладки отображения тела письма:

### Изменения в `notification-overlay.ts`:

1. **В `safeBody` computed**:
   - Логирование состояния: есть ли уведомление, развернуто ли окно, есть ли тело
   - Вывод длины тела и первых 100 символов

2. **В `toggleExpand`**:
   - Логирование состояния до и после разворачивания/сворачивания
   - Вывод информации об уведомлении

## Как проверить

1. **Запустите приложение** (уже запущено)

2. **Откройте DevTools для окна уведомлений**:
   - Нажмите правой кнопкой на окно уведомления
   - Выберите "Inspect Element" или нажмите `F12`
   - Перейдите на вкладку "Console"

3. **Дождитесь уведомления** или нажмите "Check Now" в меню трея

4. **Сделайте двойной клик** по окну уведомления для разворачивания

5. **Изучите логи в Console**:
   ```
   safeBody computed: current= exists isExpanded= false hasBody= true
   safeBody: body length= 12345 first 100 chars= <html...
   toggleExpand: current state= false notification= {...}
   toggleExpand: expanding window
   toggleExpand: window expanded, isExpanded= true
   safeBody computed: current= exists isExpanded= true hasBody= true
   safeBody: body length= 12345 first 100 chars= <html...
   ```

## Возможные проблемы и решения

### Проблема 1: safeBody возвращает null при развертывании

**Признак**: В логах `safeBody: no body found` при `isExpanded= true`

**Причина**: `current()` computed не видит изменений в `isExpanded`

**Решение**: Проблема в реактивности Angular signals

### Проблема 2: Тело есть, но не отображается

**Признак**: В логах `safeBody: body length= 12345`, но на экране ничего нет

**Причина**: Проблема с CSS или структурой HTML

**Решение**: 
1. Проверить в DevTools Elements вкладку, есть ли элемент `.alert-body`
2. Проверить его стили в Computed
3. Проверить, не скрыт ли он через `display: none` или `visibility: hidden`

### Проблема 3: HTML не парсится

**Признак**: В логах видно тело, но в Elements видны символы `&lt;` вместо `<`

**Причина**: Angular не применяет `[innerHTML]`

**Решение**: Проверить, правильно ли используется `SafeHtml`

## Что проверить в DevTools

1. **В Console**: Логи `safeBody computed` и `toggleExpand`
2. **В Elements**: 
   - Наличие элемента `<div class="alert-body">`
   - Содержимое этого элемента
   - Computed стили (flex, overflow, размеры)
3. **В Network**: Загрузились ли внешние ресурсы из письма (картинки, CSS)

## Следующие шаги

Если проблема не решится с помощью логов, возможно нужно:
1. Проверить, правильно ли работает `*ngIf="isExpanded()"`
2. Добавить `ChangeDetectorRef.detectChanges()` после `this.isExpanded.set(true)`
3. Попробовать использовать `@if` вместо `*ngIf` (новый Angular синтаксис)

