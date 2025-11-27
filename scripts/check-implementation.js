#!/usr/bin/env node

/**
 * Быстрая проверка: Реализация отображения тела письма
 *
 * Проверяет наличие всех необходимых изменений для функционала
 * развёрнутого просмотра писем.
 */

const fs = require('fs');
const path = require('path');

console.log('🔍 Проверка реализации отображения тела письма...\n');

let allOk = true;
let errors = [];
let warnings = [];

// Функция проверки наличия текста в файле
function checkFileContains(filePath, searchText, description) {
  try {
    const content = fs.readFileSync(filePath, 'utf8');
    if (content.includes(searchText)) {
      console.log(`✅ ${description}`);
      return true;
    } else {
      console.log(`❌ ${description}`);
      errors.push(`${description} - не найдено: "${searchText}"`);
      allOk = false;
      return false;
    }
  } catch (error) {
    console.log(`❌ ${description} - ошибка чтения файла`);
    errors.push(`${description} - ошибка: ${error.message}`);
    allOk = false;
    return false;
  }
}

// Функция проверки существования файла
function checkFileExists(filePath, description) {
  if (fs.existsSync(filePath)) {
    console.log(`✅ ${description}`);
    return true;
  } else {
    console.log(`❌ ${description}`);
    errors.push(`${description} - файл не найден`);
    allOk = false;
    return false;
  }
}

console.log('📦 Backend (Rust):');
console.log('─────────────────────────────────────');

// Проверка config.rs
checkFileContains(
  'src-tauri/src/config.rs',
  'notification_expanded_width',
  'config.rs: поле notification_expanded_width'
);

checkFileContains(
  'src-tauri/src/config.rs',
  'notification_expanded_height',
  'config.rs: поле notification_expanded_height'
);

// Проверка gmail.rs
checkFileContains(
  'src-tauri/src/gmail.rs',
  'pub body: Option<String>',
  'gmail.rs: поле body в GmailNotification'
);

checkFileContains(
  'src-tauri/src/gmail.rs',
  'fn extract_body',
  'gmail.rs: функция extract_body'
);

checkFileContains(
  'src-tauri/src/gmail.rs',
  'fn decode_body',
  'gmail.rs: функция decode_body'
);

checkFileContains(
  'src-tauri/src/gmail.rs',
  'format", "full"',
  'gmail.rs: запрос полного содержимого (full format)'
);

console.log('\n💻 Frontend (Angular):');
console.log('─────────────────────────────────────');

// Проверка notification-overlay.ts
checkFileContains(
  'frontend/src/app/components/notification-overlay/notification-overlay.ts',
  'body?: string | null',
  'notification-overlay.ts: поле body в NotificationPayload'
);

checkFileContains(
  'frontend/src/app/components/notification-overlay/notification-overlay.ts',
  'isExpanded = signal<boolean>(false)',
  'notification-overlay.ts: signal isExpanded'
);

checkFileContains(
  'frontend/src/app/components/notification-overlay/notification-overlay.ts',
  'async toggleExpand()',
  'notification-overlay.ts: метод toggleExpand'
);

checkFileContains(
  'frontend/src/app/components/notification-overlay/notification-overlay.ts',
  'LogicalSize',
  'notification-overlay.ts: импорт LogicalSize'
);

// Проверка HTML template
checkFileContains(
  'frontend/src/app/components/notification-overlay/notification-overlay.component.html',
  '(dblclick)="toggleExpand()"',
  'HTML: обработчик двойного клика'
);

checkFileContains(
  'frontend/src/app/components/notification-overlay/notification-overlay.component.html',
  '[innerHTML]="n.body"',
  'HTML: рендеринг тела через innerHTML'
);

checkFileContains(
  'frontend/src/app/components/notification-overlay/notification-overlay.component.html',
  '*ngIf="isExpanded()',
  'HTML: условное отображение развёрнутого вида'
);

// Проверка SCSS
checkFileContains(
  'frontend/src/app/components/notification-overlay/notification-overlay.component.scss',
  '.alert-body',
  'SCSS: стили для alert-body'
);

const scssContent = fs.readFileSync('frontend/src/app/components/notification-overlay/notification-overlay.component.scss', 'utf8');
if (scssContent.includes('.alert-recipient') && !scssContent.match(/\.alert-recipient[^}]*margin-left:\s*12px/)) {
  console.log('✅ SCSS: отступ у .alert-recipient убран');
} else {
  console.log('⚠️  SCSS: проверьте отступ у .alert-recipient');
  warnings.push('Возможно, отступ margin-left всё ещё присутствует у .alert-recipient');
}

// Проверка settings-page
checkFileContains(
  'frontend/src/app/components/settings-page/settings-page.ts',
  'notification_expanded_width',
  'settings-page.ts: поле notification_expanded_width в модели'
);

checkFileContains(
  'frontend/src/app/components/settings-page/settings-page.component.html',
  'Ширина развёрнутого окна',
  'settings-page.html: UI для настройки размеров'
);

console.log('\n📚 Документация:');
console.log('─────────────────────────────────────');

checkFileExists('docs/EXPANDED_VIEW.md', 'Техническая документация');
checkFileExists('docs/USER_GUIDE_EXPANDED_VIEW.md', 'Руководство пользователя');
checkFileExists('CHANGELOG_EXPANDED_VIEW.md', 'Changelog');

console.log('\n' + '═'.repeat(50));

if (allOk && warnings.length === 0) {
  console.log('✅ ВСЁ ГОТОВО! Все компоненты на месте.');
  console.log('\n📋 Следующие шаги:');
  console.log('   1. Запустить приложение: npm start');
  console.log('   2. Получить тестовое письмо');
  console.log('   3. Дважды кликнуть по уведомлению');
  console.log('   4. Проверить отображение тела письма');
} else {
  console.log('❌ НАЙДЕНЫ ПРОБЛЕМЫ!');

  if (errors.length > 0) {
    console.log('\n🔴 Ошибки:');
    errors.forEach((err, i) => {
      console.log(`   ${i + 1}. ${err}`);
    });
  }

  if (warnings.length > 0) {
    console.log('\n⚠️  Предупреждения:');
    warnings.forEach((warn, i) => {
      console.log(`   ${i + 1}. ${warn}`);
    });
  }

  process.exit(1);
}

console.log('\n' + '═'.repeat(50));
console.log('🚀 Готово к тестированию!\n');

