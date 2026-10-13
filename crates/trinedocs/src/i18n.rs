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
}
