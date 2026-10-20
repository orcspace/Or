pub const STRINGS: &[(&str, &str)] = &[
    ("Home", "Главная"),
    ("Document", "Документ"),
    ("Spreadsheet", "Таблица"),
    ("Presentation", "Презентация"),
    ("New", "Создать"),
    ("Open", "Открыть"),
    ("Save", "Сохранить"),
    ("Save as", "Сохранить как"),
    ("Save copy", "Сохранить копию"),
    ("Close", "Закрыть"),
    ("Settings", "Настройки"),
    ("Recent files", "Недавние файлы"),
    ("Search", "Поиск"),
    ("Undo", "Отменить"),
    ("Redo", "Повторить"),
    ("Export PDF", "Экспорт PDF"),
    ("Print via PDF viewer", "Печать через PDF-просмотрщик"),
    ("AI changes", "Изменения AI"),
    ("Accept", "Принять"),
    ("Reject", "Откатить"),
    ("Cancel", "Отмена"),
    ("Recover", "Восстановить"),
    ("Discard", "Не сохранять"),
    ("Ready", "Готово"),
    ("File", "Файл"),
    ("Edit", "Правка"),
    ("Language", "Язык"),
    ("Theme", "Тема"),
    ("Light", "Светлая"),
    ("Dark", "Тёмная"),
    ("System", "Системная"),
    ("Backup", "Резервная копия"),
    ("Dark paper", "Тёмная бумага"),
    ("Reduced motion", "Уменьшить движение"),
    ("Copy MCP config", "Скопировать конфиг MCP"),
    ("Unsaved changes", "Есть несохранённые изменения"),
    ("Your workspace", "Ваше рабочее пространство"),
    (
        "Documents, spreadsheets and presentations in one place.",
        "Документы, таблицы и презентации в одном месте.",
    ),
    ("Recent", "Недавние"),
    ("Pinned", "Закреплённые"),
    ("Search files", "Поиск файлов"),
    ("Open a file · Ctrl+O", "Открыть файл · Ctrl+O"),
    ("All files", "Все файлы"),
    ("Clear search", "Очистить поиск"),
    ("Nothing found", "Ничего не найдено"),
    ("No pinned files", "Нет закреплённых файлов"),
    ("Your files will appear here", "Здесь появятся ваши файлы"),
    (
        "Change the name, path or file type.",
        "Измените имя, путь или тип файла.",
    ),
    (
        "Open ··· next to a file and choose «Pin».",
        "Откройте ··· рядом с файлом и выберите «Закрепить».",
    ),
    (
        "Create a file or drag it into the window.",
        "Создайте файл или перетащите его в окно.",
    ),
    ("Name", "Имя"),
    ("Pin", "Закрепить"),
    ("Unpin", "Открепить"),
    ("Remove from list", "Убрать из списка"),
    ("File actions", "Действия с файлом"),
    (
        "Support for all Microsoft Office features is not complete yet.",
        "поддержка всех возможностей Microsoft Office пока неполная.",
    ),
    ("Russian", "Русский"),
    ("Scale", "Масштаб"),
    ("Backup .bak", "Резервная копия .bak"),
    ("MCP: local clients", "MCP: локальные клиенты"),
    (
        "MCP registration with preview:",
        "Регистрация MCP с предпросмотром:",
    ),
    ("Commands", "Команды"),
    ("Find an action…", "Найти действие…"),
    ("No commands found", "Команды не найдены"),
    ("Open · Ctrl+O", "Открыть · Ctrl+O"),
    ("Save · Ctrl+S", "Сохранить · Ctrl+S"),
    ("Undo · Ctrl+Z", "Отменить · Ctrl+Z"),
    ("Redo · Ctrl+Y", "Повторить · Ctrl+Y"),
    ("Check the document", "Проверить документ"),
    ("Find commands · Ctrl+K", "Поиск команд · Ctrl+K"),
    ("Find commands", "Поиск команд"),
    ("Light theme", "Светлая тема"),
    ("Dark theme", "Тёмная тема"),
    ("Settings and commands", "Настройки и команды"),
    ("Assistant", "Ассистент"),
    (
        "AI chat and changes · Ctrl+Shift+A",
        "AI-чат и изменения · Ctrl+Shift+A",
    ),
    ("Create a document", "Создать документ"),
    ("Create a spreadsheet", "Создать таблицу"),
    ("Create a presentation", "Создать презентацию"),
    ("Save as… · Ctrl+Shift+S", "Сохранить как… · Ctrl+Shift+S"),
    ("Save a copy…", "Сохранить копию…"),
    ("Export to PDF", "Экспорт в PDF"),
    ("Close the document · Ctrl+W", "Закрыть документ · Ctrl+W"),
    (
        "The file is locked for writing. Editing stays available; the original file is protected from being overwritten.",
        "Файл занят. Редактирование доступно; исходный файл защищён от перезаписи.",
    ),
    ("Lock…", "Блокировка…"),
    ("Document check", "Проверка документа"),
    ("Checking the document…", "Проверяем документ…"),
    (
        "The document changed or closed after the check.",
        "Документ изменён или закрыт после проверки.",
    ),
    (
        "Search by location and description",
        "Поиск по месту и описанию",
    ),
    ("Reset", "Сбросить"),
    ("Copy the report", "Копировать отчёт"),
    ("Check report copied", "Отчёт проверки скопирован"),
    (
        "No findings for this filter.",
        "По этому фильтру замечаний нет.",
    ),
    (
        "No findings for the supported checks.",
        "По поддерживаемым проверкам замечаний нет.",
    ),
    ("Check the current document", "Проверить текущий документ"),
    ("Message", "Сообщение"),
    ("Allow the MCP client?", "Разрешить MCP-клиент?"),
    (
        "Read and edit open documents. Files on disk go through the interface.",
        "Чтение и правка открытых документов. Файлы на диске — через интерфейс.",
    ),
    ("Allow", "Разрешить"),
    ("Deny", "Запретить"),
    ("Recovery", "Восстановление"),
    ("Delete snapshots", "Удалить снимки"),
    ("The file changed on disk", "Файл изменён на диске"),
    ("Reload", "Перезагрузить"),
    ("Keep the edits", "Оставить правки"),
    (
        "Wait for the file check to repeat",
        "Дождитесь повторной проверки файла",
    ),
    ("Confirmation", "Подтверждение"),
    ("Unlock saving", "Разблокировать сохранение"),
    ("Add", "Добавить"),
    ("Continue", "Продолжить"),
    ("Save changes?", "Сохранить изменения?"),
    ("Interface check", "Проверка интерфейса"),
    ("Editing", "Редактирование"),
    ("Create", "Создать"),
    ("Close the document", "Закрыть документ"),
    ("Read only", "Только чтение"),
    ("File is locked", "Файл занят"),
    (
        "Edits can be saved to a separate copy.",
        "Правки можно сохранить в отдельную копию.",
    ),
    ("Saving…", "Сохранение…"),
    ("Not saved", "Не сохранено"),
    ("New file", "Новый файл"),
    ("Saved", "Сохранено"),
    ("Save / export…", "Сохранить / экспортировать…"),
    (
        "Close the assistant panel · Ctrl+Shift+A",
        "Закрыть панель ассистента · Ctrl+Shift+A",
    ),
    ("Changes", "Изменения"),
    ("Chat", "Чат"),
    ("Preparing", "Подготовка"),
    ("Applied", "Применено"),
    ("Preview", "Предпросмотр"),
    ("Cancelled", "Отменено"),
    ("Not applied", "Не применено"),
    ("No unaccepted AI changes.", "Нет непринятых правок AI."),
    ("Accept all", "Принять все"),
    (
        "Reloading replaces the current edits. Use “Save a copy” to keep them.",
        "Перезагрузка заменит текущие правки. Для их сохранения используйте «Сохранить копию».",
    ),
    (
        "Delete the remaining lock marker? Close this file in other programs. The active TrineDocs lock stays protected.",
        "Удалить оставшийся lock-маркер? Закройте этот файл в других программах. Активная блокировка TrineDocs остаётся защищённой.",
    ),
    (
        "The lock marker is already gone. Take the lock for saving?",
        "Lock-маркер уже отсутствует. Получить блокировку для сохранения?",
    ),
    (
        "The other entries are kept. The existing trinedocs entry is not replaced.",
        "Остальные записи сохранятся. Существующая запись trinedocs не заменяется.",
    ),
    (
        "Export keeps only what the chosen format supports. Formatting and objects may change or disappear. For an editable copy choose .docx, .xlsx or .pptx; the Office export limits remain.",
        "Экспорт сохраняет только возможности выбранного формата. Форматирование и объекты могут измениться или исчезнуть. Для редактируемой копии выберите .docx, .xlsx или .pptx; ограничения Office-экспорта также остаются.",
    ),
    (
        "The document changed or is busy with a file operation",
        "Документ изменился или занят файловой операцией",
    ),
    (
        "Finish the dialog or the file operation before closing",
        "Завершите диалог или файловую операцию перед закрытием",
    ),
    ("Cancel…", "Отмена…"),
    ("Preparing the edits", "Подготовка правок"),
    ("Untitled", "Без названия"),
    (
        "The MCP client is registered. Restart the client.",
        "Клиент MCP зарегистрирован. Перезапустите клиент.",
    ),
    ("File reloaded", "Файл перезагружен"),
    ("File opened", "Файл открыт"),
    ("Copy saved", "Копия сохранена"),
    ("Export finished", "Экспорт завершён"),
    (
        "New edits appeared after saving",
        "После сохранения появились новые правки",
    ),
    ("Reloading…", "Перезагрузка…"),
    (
        "The lock was removed · the file can be saved",
        "Блокировка снята · можно сохранить файл",
    ),
    ("Saving is already running", "Сохранение уже выполняется"),
];
pub fn tr<'a>(s: &'a str, language: &str) -> &'a str {
    if language == "ru" {
        STRINGS
            .iter()
            .find(|(en, _)| *en == s)
            .map(|(_, ru)| *ru)
            .unwrap_or(s)
    } else {
        s
    }
}
#[cfg(test)]
mod tests {
    #[test]
    fn unique_complete() {
        let mut en = std::collections::HashSet::new();
        for (a, b) in super::STRINGS {
            assert!(!a.is_empty() && !b.is_empty());
            assert!(en.insert(a));
            assert_eq!(super::tr(a, "ru"), *b);
        }
    }
    /// Every string the shell asks for has to exist in the table, otherwise the
    /// English mode would show the Russian text.
    #[test]
    fn every_shell_string_has_a_translation() {
        let source = include_str!("app.rs");
        let marker = "crate::i18n::tr(\"";
        let mut checked = 0;
        for (index, _) in source.match_indices(marker) {
            let rest = &source[index + marker.len()..];
            let end = rest.find('"').expect("unterminated shell string");
            let key = &rest[..end];
            assert!(
                super::STRINGS.iter().any(|(en, _)| *en == key),
                "shell string without a translation: {key}"
            );
            checked += 1;
        }
        assert!(checked > 50, "the shell strings were not found");
    }
}
