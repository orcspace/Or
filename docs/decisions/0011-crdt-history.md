# ADR-0011: Рост истории CRDT

Статус: **предложено; спайк4.2 до решения**. Дата:2026-10-12UTC.

## Контекст

Loro shallow snapshot действительно удаляет историю и ограничивает слияние старых peers (FACT-009). Требования time machine/full history/local undo/долгий offline конфликтуют с безусловным сжатием.

## Варианты и рекомендация

A: полная история + архив старых updates; B: adjustable shallow + обязательный rebootstrap старых peers; C: гибрид сохраняемых именованных версий/снапшотов. Не удалять историю до измерения1mops/undo/offline sync. Рекомендация исследовать гибрид; retention defaults/consent ещё не приняты.

## Критерии, данные и последствия

Размер истории, RSS, reopen/merge, undo после границы; spiking4.2 Windows сейчас/Mac позже. Numerical thresholds только после замеров.

Первые [замеры и raw JSON](../spikes/02-crdt.md) получены на development Linux и Windows Server CI; целевая Windows 11 приёмка ещё не проведена. Map history 1m ops слишком оптимистична для rich-text истории; thresholds не приняты.

Дополнительный спайк 2026-10-13 воспроизвёл: shallow cutoff отвергает старую офлайн-правку, full archive сохраняет её. Ошибка mixed import может сопровождаться частичными изменениями (ADR-0024). Marks сохраняются в shallow snapshot. Новый UndoManager после full snapshot reload, как и после shallow reload, не восстанавливает live undo stack: это отдельное состояние менеджера, а не доказательство потери undo именно из-за GC.

Рекомендация до принятия retention policy: хранить полную историю; не включать destructive GC и не заменять старые офлайн-правки rebootstrap, который их отбросит. Сжатие рабочего состояния допустимо исследовать только с сохраняемым архивом и доказанной доставкой всех offline edits. Машина времени/undo/отсутствие потерь не ослаблены.
