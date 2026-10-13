# ADR-0020: BYOK, AI proxy и приватность

Статус: **BYOK принято заданием; proxy ждёт владельца**. Дата:2026-10-12UTC.

## Контекст

Default только user API key, OS keychain. Consumer subscription нельзя выдавать за API billing; конкретные provider policies пока не проверены.

## Варианты и рекомендация

A: direct BYOK providers; B: own paid proxy/quota service with costs/retention; C: local endpoint. Выбраны A/C по заданию, B требует владельца/ADR.

## Критерии, данные и последствия

UI first-use provider+data notice/context chips/cost units where verified; token limits/stop/clear logs/AIoff honored. API billing and retention terms official-source audit before onboarding. Keys/content never telemetry. EULA/privacy drafts review owner/legal before release.

Замеры/числовые затраты пока отсутствуют. Это не принятое ослабление требований и не результат выполненного спайка; независимую фазу0 продолжать.
