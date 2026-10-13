# ADR-0016: Сервер, E2EE и эксплуатация

Статус: **ждёт владельца**. Дата:2026-10-12UTC.

## Контекст

Sharing требует email/notifications/abuse controls/backups/deletion/monitoring; E2EE влияет на server search/preview/AI. Pricing/eligibility ещё не проверены.

## Варианты и рекомендация

A: TLS+at-rest encryption, trusted server; B: E2EE+client search/preview/AI; C: local-only initial stage, cloud позже. Рекомендация C до protocol/model readiness; выбор A/B принимает владелец, не самовольно.

## Критерии, данные и последствия

Nusers/load50active, budgets/read-write/S3/email/egress estimates только по official prices и workload assumptions; recovery drill, deletion asset/version audit, JWT role enforcement. GDPR applicability юридически проверить, не делать выводов.

Замеры/числовые затраты пока отсутствуют. Это не принятое ослабление требований и не результат выполненного спайка; независимую фазу0 продолжать.
