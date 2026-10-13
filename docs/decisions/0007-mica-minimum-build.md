# ADR-0007: Mica API и Windows 11 build 22000

Статус: **ждёт владельца**.

[DWM_SYSTEMBACKDROP_TYPE](https://learn.microsoft.com/en-us/windows/win32/api/dwmapi/ne-dwmapi-dwm_systembackdrop_type), проверено 2026-10-12: documented minimum build 22621. AGENTS требует build 22000+ и конкретный backdrop API. Это реальный конфликт, не разрешается игнорированием HRESULT.

| Вариант | Плюсы | Минусы/стоимость |
|---|---|---|
| На 22000 solid surface, с 22621 documented Mica | сохраняет платформенный минимум, documented API | исключение из обязательной Mica на части поддержанных ОС — только владелец |
| Минимум 22621 | одинаковый API | исключает часть Windows 11, меняет жёсткую платформу — только владелец |
| Поддержанный альтернативный способ на 22000 | потенциально весь объём | не проверен; undocumented attributes не принимать за production solution |

Рекомендация: первый, **не принят**. В core только чистая проверка capability; Mica не реализована. Нужны data о Qt/Win11 old build и стоимости workaround; числа неизвестны.

