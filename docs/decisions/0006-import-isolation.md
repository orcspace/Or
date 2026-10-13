# ADR-0006: Импорт вне процесса UI

Статус: **принято как прямое требование SPEC-AUDIT §3.C.5**; способ OS sandbox ещё не утверждён.

Контекст: safe XML/zip limits не защищают основной процесс от падения native libraries/LO и resource exhaustion. Требование дополняет изолированные временные каталоги.

Варианты: библиотека внутри UI / worker process / OS sandboxed worker. Первый отвергнут заданием. Обязательный минимум: worker, bounded IPC, input limits, timeout/cancellation, memory limits, whole process-tree termination, no macro/OLE/DDE/network refresh. ОС-песочница — отдельный исследуемый усиленный вариант.

На Windows исследовать Job Objects и restricted token; не объявлять это полной sandbox без tests. Проверки: corrupt/zip bomb/crash/hang/cancel, app survives; partial model never committed; resources released. Performance overhead измерить в спайке. Unknown OOXML parts preserve bytes but never execute. Licensing/header parser audit до включения.

