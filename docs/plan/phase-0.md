# Фаза 0: план и мини-аудит

Статус: **в процессе, не принята**. Windows-first по ADR-0001.

## Последовательность

1. Сохранить три обязательных спецификации, facts, initial audit, requirements IDs, parity и ADR.
2. Dependency-free Rust workspace: stable aged toolchain pin, platform/theme policies, meaningful tests, registry checks.
3. Дождаться age gate Qt; разрешить QML/JS конфликт владельцем; получить Windows Mica22000 решение. Нельзя молча выбрать Qt6.11 или разрешить JS.
4. Проверить exact Qt packages/licenses; CXX-Qt0.10.0 bridge compatibility; pin Cargo.lock/transitives и CMake tooling по первоисточникам.
5. Тонкая QML shell: static home, mode accents; disabled future create/search/chat actions с меткой «в разработке»; real theme switching через Rust QObject.
6. Полный Theme.qml tokens, native system color/high contrast/reduced motion; fonts/icons only audited assets; ru/en qsTr.
7. Windows native adapter: actual platform probing, title drag/Snap hit-test, corners, Mica по ADR, keyboard focus/a11y.
8. Windows CI build/qmllint/colors/rustfmt/clippy/unit/visual themes; clean Windows11x64 actual launch and screen reader. No signing claim without certificate.
9. Отчёт/phase audit; только после приемки переход к фазе1 и отдельно обязательные layout/CRDT spikes.

## Мини-аудит §§1–5/20

- release-age requirement blocks Qt6.12 today.
- Absolute JS ban collides with QML bindings/qsTr.
- Mica API22621 vs supported22000.
- «Стабильные» не значит latest immediately; every pinned dependency needs license/age evidence.
- Hosted Windows runner не доказательство Win11 GUI/120fps; tests classified.
- Signing stub — только явно unsigned dev artifact, не fake signature.
- Scope reordering pending owner, phase0 independent.

## Приёмка Windows (план, не результаты)

- Окно на real Win11x64 builds22000 иcurrent; light/dark/system switching без restart, no UI stalls.
- Keyboard-only navigation, Narrator, RU/EN, high contrast/reduced motion, DPI100/150/200%, Snap/resize.
- Dynamic Qt DLLs, no prohibited modules/JS until owner decision, pin/license audit.
- Application CI green (не только spec/core CI).
- Mica state по принятому ADR, documented API/HRESULT handled.
- Signing preparation explicitly unsigned until identity ready.

macOS acceptance/spikes отложены, глобальную phase0 Done не ставить. Даже Windows provisional Done не ставить до окна/GUI tests.

