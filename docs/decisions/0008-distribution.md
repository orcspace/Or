# ADR-0008: Модель распространения и подпись

Статус: **ждёт владельца**; юридические вопросы передать владельцу/юристу.

Варианты: бесплатный closed-source desktop / платная подписка с BYOK и cloud / open-source desktop + платный cloud. Для каждого Qt dynamic LGPL requirement сохраняется; LGPL не означает автоматическое разрешение всего dependency tree. Server/подпись/OAuth имеют собственные условия.

Критерии: business model, Qt compliance/replacement, исходная лицензия проекта, signing identity/MSIX sideload, cloud budget, Apple account. Все суммы неизвестны, проверяются официальными pricing/eligibility под страну/организацию владельца; сертификатов/аккаунтов сейчас не проверяли.

Рекомендация на разработку: локальный offline/BYOK без оплачиваемого proxy; релизную модель не выбирать. LICENSE проекта пока не назначен, никакой MIT/AGPL по умолчанию. Unsigned dev output никогда не обозначать production-signed.

