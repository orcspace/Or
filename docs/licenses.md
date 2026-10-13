# Зависимости и лицензии

Полный production Cargo.lock: 444 registry packages. Проверка exact versions на 2026-10-13T00:00:00+00:00.
Cargo.lock SHA-256: a799a9c9cfebd0d3df604b765563b896565a43d512a5335174779b1ce809ed80.

Реестр консервативно включает все target-specific и build-time зависимости lock; это не заявление, что каждый crate связан с Windows exe. Qt/CRDT spike не входят в этот граф.
Декларации лицензий взяты из checksum-verified crates, даты/yank/checksum — из crates.io API. Отсутствующие в package LICENSE/NOTICE дополнены только с exact upstream commit из .cargo_vcs_info.json.

Выбор permissive OR альтернативы указан явно; все AND obligations сохранены. Notices сохраняют все найденные исходные LICENSE/COPYRIGHT/NOTICE тексты, включая альтернативы. Это проверка происхождения/наличия текстов, а не юридическое заключение о достаточности распространения.

Генератор: [tools/audit_dependencies.py](../tools/audit_dependencies.py). Полные проверяемые evidence и supplemental тексты: [audit/dependencies.md](audit/dependencies.md).

Offline сборка notices: python -B tools/audit_dependencies.py --offline --check --notices target/dist/THIRD-PARTY-NOTICES.txt
Online refresh: python -B tools/audit_dependencies.py --refresh

| Package | Pin | Declared SPDX | Selected | Published UTC | Source texts | Windows | Gate |
|---|---|---|---|---|---|---|---|
| [accesskit](https://crates.io/api/v1/crates/accesskit/0.24.1) | 0.24.1 | MIT OR Apache-2.0 | MIT | 2026-06-12T13:23:59.640896Z | 3; exact-upstream-commit | True | pass |
| [accesskit_atspi_common](https://crates.io/api/v1/crates/accesskit_atspi_common/0.18.1) | 0.18.1 | MIT OR Apache-2.0 | MIT | 2026-05-11T18:44:55.046063Z | 3; exact-upstream-commit |  | pass |
| [accesskit_consumer](https://crates.io/api/v1/crates/accesskit_consumer/0.35.0) | 0.35.0 | MIT OR Apache-2.0 | MIT | 2026-03-04T20:53:21.967053Z | 3; exact-upstream-commit | True | pass |
| [accesskit_consumer](https://crates.io/api/v1/crates/accesskit_consumer/0.36.0) | 0.36.0 | MIT OR Apache-2.0 | MIT | 2026-05-11T18:44:14.064261Z | 3; exact-upstream-commit |  | pass |
| [accesskit_consumer](https://crates.io/api/v1/crates/accesskit_consumer/0.38.0) | 0.38.0 | MIT OR Apache-2.0 | MIT | 2026-07-14T17:05:45.168750Z | 3; exact-upstream-commit |  | pass |
| [accesskit_macos](https://crates.io/api/v1/crates/accesskit_macos/0.26.3) | 0.26.3 | MIT OR Apache-2.0 | MIT | 2026-07-14T17:07:15.444374Z | 3; exact-upstream-commit |  | pass |
| [accesskit_unix](https://crates.io/api/v1/crates/accesskit_unix/0.21.1) | 0.21.1 | MIT OR Apache-2.0 | MIT | 2026-05-11T18:44:57.664346Z | 3; exact-upstream-commit |  | pass |
| [accesskit_windows](https://crates.io/api/v1/crates/accesskit_windows/0.32.1) | 0.32.1 | MIT OR Apache-2.0 | MIT | 2026-03-04T20:54:15.885020Z | 3; exact-upstream-commit | True | pass |
| [accesskit_winit](https://crates.io/api/v1/crates/accesskit_winit/0.32.2) | 0.32.2 | Apache-2.0 | Apache-2.0 | 2026-03-04T20:54:22.635328Z | 3; exact-upstream-commit | True | pass |
| [adler2](https://crates.io/api/v1/crates/adler2/2.0.1) | 2.0.1 | 0BSD OR MIT OR Apache-2.0 | MIT | 2025-06-09T22:31:53.608434Z | 3; checksum-verified-registry-archive | True | pass |
| [ahash](https://crates.io/api/v1/crates/ahash/0.8.12) | 0.8.12 | MIT OR Apache-2.0 | MIT | 2025-05-08T05:05:32.999746Z | 2; checksum-verified-registry-archive | True | pass |
| [android-activity](https://crates.io/api/v1/crates/android-activity/0.6.1) | 0.6.1 | MIT OR Apache-2.0 | MIT | 2026-03-24T00:59:18.755650Z | 3; checksum-verified-registry-archive |  | pass |
| [android-properties](https://crates.io/api/v1/crates/android-properties/0.2.2) | 0.2.2 | MIT | MIT | 2021-01-11T20:37:22.070209Z | 1; checksum-verified-registry-archive |  | pass |
| [android_system_properties](https://crates.io/api/v1/crates/android_system_properties/0.1.6) | 0.1.6 | MIT OR Apache-2.0 | MIT | 2026-08-05T13:44:27.717073Z | 2; checksum-verified-registry-archive |  | pass |
| [arboard](https://crates.io/api/v1/crates/arboard/3.6.1) | 3.6.1 | MIT OR Apache-2.0 | MIT | 2025-08-23T20:19:03.127566Z | 2; checksum-verified-registry-archive | True | pass |
| [arrayvec](https://crates.io/api/v1/crates/arrayvec/0.7.8) | 0.7.8 | MIT OR Apache-2.0 | MIT | 2026-07-02T12:44:35.487269Z | 2; checksum-verified-registry-archive | True | pass |
| [as-raw-xcb-connection](https://crates.io/api/v1/crates/as-raw-xcb-connection/1.0.1) | 1.0.1 | MIT OR Apache-2.0 | MIT | 2023-12-17T15:47:18.212890Z | 2; checksum-verified-registry-archive |  | pass |
| [async-broadcast](https://crates.io/api/v1/crates/async-broadcast/0.7.2) | 0.7.2 | MIT OR Apache-2.0 | MIT | 2024-12-23T21:10:23.927278Z | 2; checksum-verified-registry-archive |  | pass |
| [async-channel](https://crates.io/api/v1/crates/async-channel/2.5.0) | 2.5.0 | Apache-2.0 OR MIT | MIT | 2025-07-06T07:06:07.101546Z | 2; checksum-verified-registry-archive |  | pass |
| [async-executor](https://crates.io/api/v1/crates/async-executor/1.14.0) | 1.14.0 | Apache-2.0 OR MIT | MIT | 2026-02-15T16:45:48.706047Z | 2; checksum-verified-registry-archive |  | pass |
| [async-io](https://crates.io/api/v1/crates/async-io/2.6.0) | 2.6.0 | Apache-2.0 OR MIT | MIT | 2025-09-14T14:03:50.201510Z | 2; checksum-verified-registry-archive |  | pass |
| [async-lock](https://crates.io/api/v1/crates/async-lock/3.4.2) | 3.4.2 | Apache-2.0 OR MIT | MIT | 2025-12-21T16:24:46.954239Z | 2; checksum-verified-registry-archive |  | pass |
| [async-process](https://crates.io/api/v1/crates/async-process/2.5.0) | 2.5.0 | Apache-2.0 OR MIT | MIT | 2025-09-14T14:11:17.759362Z | 2; checksum-verified-registry-archive |  | pass |
| [async-recursion](https://crates.io/api/v1/crates/async-recursion/1.1.1) | 1.1.1 | MIT OR Apache-2.0 | MIT | 2024-04-25T16:06:43.164603Z | 2; checksum-verified-registry-archive |  | pass |
| [async-signal](https://crates.io/api/v1/crates/async-signal/0.2.14) | 0.2.14 | Apache-2.0 OR MIT | MIT | 2026-04-07T11:50:26.817406Z | 2; checksum-verified-registry-archive |  | pass |
| [async-task](https://crates.io/api/v1/crates/async-task/4.7.1) | 4.7.1 | Apache-2.0 OR MIT | MIT | 2024-04-27T23:06:20.560257Z | 2; checksum-verified-registry-archive |  | pass |
| [async-trait](https://crates.io/api/v1/crates/async-trait/0.1.92) | 0.1.92 | MIT OR Apache-2.0 | MIT | 2026-08-08T07:10:35.376083Z | 2; checksum-verified-registry-archive | True | pass |
| [atoi_simd](https://crates.io/api/v1/crates/atoi_simd/0.18.1) | 0.18.1 | MIT OR Apache-2.0 | MIT | 2026-02-09T12:24:52.257437Z | 2; checksum-verified-registry-archive | True | pass |
| [atomic-waker](https://crates.io/api/v1/crates/atomic-waker/1.1.2) | 1.1.2 | Apache-2.0 OR MIT | MIT | 2023-09-25T17:45:09.388277Z | 3; checksum-verified-registry-archive | True | pass |
| [atspi](https://crates.io/api/v1/crates/atspi/0.29.0) | 0.29.0 | Apache-2.0 OR MIT | MIT | 2025-09-18T14:15:56.499600Z | 2; checksum-verified-registry-archive |  | pass |
| [atspi-common](https://crates.io/api/v1/crates/atspi-common/0.13.0) | 0.13.0 | Apache-2.0 OR MIT | MIT | 2025-09-18T14:15:21.072701Z | 2; checksum-verified-registry-archive |  | pass |
| [atspi-proxies](https://crates.io/api/v1/crates/atspi-proxies/0.13.0) | 0.13.0 | Apache-2.0 OR MIT | MIT | 2025-09-18T14:15:33.961842Z | 2; checksum-verified-registry-archive |  | pass |
| [autocfg](https://crates.io/api/v1/crates/autocfg/1.5.1) | 1.5.1 | Apache-2.0 OR MIT | MIT | 2026-05-22T11:03:04.576867Z | 2; checksum-verified-registry-archive | True | pass |
| [axum](https://crates.io/api/v1/crates/axum/0.8.9) | 0.8.9 | MIT | MIT | 2026-04-14T07:55:20.258398Z | 1; checksum-verified-registry-archive | True | pass |
| [axum-core](https://crates.io/api/v1/crates/axum-core/0.5.6) | 0.5.6 | MIT | MIT | 2025-12-27T09:19:34.014637Z | 1; checksum-verified-registry-archive | True | pass |
| [base64](https://crates.io/api/v1/crates/base64/0.23.1) | 0.23.1 | MIT OR Apache-2.0 | MIT | 2026-08-04T12:34:31.976297Z | 2; checksum-verified-registry-archive | True | pass |
| [bitflags](https://crates.io/api/v1/crates/bitflags/1.3.2) | 1.3.2 | MIT/Apache-2.0 | MIT | 2021-08-16T04:16:02.683020Z | 2; checksum-verified-registry-archive |  | pass |
| [bitflags](https://crates.io/api/v1/crates/bitflags/2.13.2) | 2.13.2 | MIT OR Apache-2.0 | MIT | 2026-09-10T01:11:38.500809Z | 2; checksum-verified-registry-archive | True | pass |
| [block2](https://crates.io/api/v1/crates/block2/0.5.1) | 0.5.1 | MIT | MIT | 2024-05-21T07:56:48.797007Z | 1; exact-upstream-commit |  | pass |
| [block2](https://crates.io/api/v1/crates/block2/0.6.2) | 0.6.2 | MIT | MIT | 2025-10-04T16:00:20.987008Z | 1; exact-upstream-commit |  | pass |
| [blocking](https://crates.io/api/v1/crates/blocking/1.7.0) | 1.7.0 | Apache-2.0 OR MIT | MIT | 2026-08-23T09:00:11.479384Z | 2; checksum-verified-registry-archive |  | pass |
| [bumpalo](https://crates.io/api/v1/crates/bumpalo/3.20.3) | 3.20.3 | MIT OR Apache-2.0 | MIT | 2026-05-22T14:59:11.132139Z | 2; checksum-verified-registry-archive | True | pass |
| [bytemuck](https://crates.io/api/v1/crates/bytemuck/1.25.2) | 1.25.2 | Zlib OR Apache-2.0 OR MIT | MIT | 2026-07-19T19:36:16.239554Z | 3; checksum-verified-registry-archive | True | pass |
| [bytemuck_derive](https://crates.io/api/v1/crates/bytemuck_derive/1.12.1) | 1.12.1 | Zlib OR Apache-2.0 OR MIT | MIT | 2026-09-11T20:47:15.014342Z | 3; checksum-verified-registry-archive | True | pass |
| [byteorder](https://crates.io/api/v1/crates/byteorder/1.5.0) | 1.5.0 | Unlicense OR MIT | MIT | 2023-10-06T00:45:57.877036Z | 2; checksum-verified-registry-archive | True | pass |
| [byteorder-lite](https://crates.io/api/v1/crates/byteorder-lite/0.1.0) | 0.1.0 | Unlicense OR MIT | MIT | 2024-04-07T19:09:06.481843Z | 1; checksum-verified-registry-archive | True | pass |
| [bytes](https://crates.io/api/v1/crates/bytes/1.12.1) | 1.12.1 | MIT | MIT | 2026-07-08T10:01:28.616466Z | 1; checksum-verified-registry-archive | True | pass |
| [calamine](https://crates.io/api/v1/crates/calamine/0.36.1) | 0.36.1 | MIT | MIT | 2026-07-27T10:06:39.750200Z | 1; checksum-verified-registry-archive | True | pass |
| [calloop](https://crates.io/api/v1/crates/calloop/0.13.0) | 0.13.0 | MIT | MIT | 2024-03-01T02:42:00.830871Z | 1; checksum-verified-registry-archive |  | pass |
| [calloop](https://crates.io/api/v1/crates/calloop/0.14.4) | 0.14.4 | MIT | MIT | 2026-02-13T19:58:36.355210Z | 1; checksum-verified-registry-archive |  | pass |
| [calloop-wayland-source](https://crates.io/api/v1/crates/calloop-wayland-source/0.3.0) | 0.3.0 | MIT | MIT | 2024-03-01T20:07:51.694409Z | 1; checksum-verified-registry-archive |  | pass |
| [calloop-wayland-source](https://crates.io/api/v1/crates/calloop-wayland-source/0.4.1) | 0.4.1 | MIT | MIT | 2025-09-06T13:16:16.065219Z | 1; checksum-verified-registry-archive |  | pass |
| [cc](https://crates.io/api/v1/crates/cc/1.5.1) | 1.5.1 | MIT OR Apache-2.0 | MIT | 2026-09-25T13:33:11.057198Z | 2; checksum-verified-registry-archive |  | pass |
| [cfg-if](https://crates.io/api/v1/crates/cfg-if/1.0.5) | 1.0.5 | MIT OR Apache-2.0 | MIT | 2026-09-16T17:36:11.278420Z | 2; checksum-verified-registry-archive | True | pass |
| [cfg_aliases](https://crates.io/api/v1/crates/cfg_aliases/0.2.2) | 0.2.2 | MIT | MIT | 2026-07-16T14:57:16.699789Z | 2; checksum-verified-registry-archive | True | pass |
| [cgl](https://crates.io/api/v1/crates/cgl/0.3.2) | 0.3.2 | MIT / Apache-2.0 | MIT | 2019-09-11T17:54:10.322944Z | 3; checksum-verified-registry-archive |  | pass |
| [chacha20](https://crates.io/api/v1/crates/chacha20/0.10.2) | 0.10.2 | MIT OR Apache-2.0 | MIT | 2026-08-27T17:51:13.045493Z | 2; checksum-verified-registry-archive | True | pass |
| [chrono](https://crates.io/api/v1/crates/chrono/0.4.45) | 0.4.45 | MIT OR Apache-2.0 | MIT | 2026-06-04T12:16:42.063037Z | 1; checksum-verified-registry-archive | True | pass |
| [clipboard-win](https://crates.io/api/v1/crates/clipboard-win/5.4.1) | 5.4.1 | BSL-1.0 | BSL-1.0 | 2025-07-17T23:38:58.532254Z | 1; exact-upstream-commit | True | pass |
| [codepage](https://crates.io/api/v1/crates/codepage/0.1.3) | 0.1.3 | Apache-2.0 OR MIT | MIT | 2026-09-12T17:15:28.233190Z | 3; checksum-verified-registry-archive | True | pass |
| [color](https://crates.io/api/v1/crates/color/0.3.3) | 0.3.3 | Apache-2.0 OR MIT | MIT | 2026-05-05T14:20:48.490544Z | 2; checksum-verified-registry-archive | True | pass |
| [combine](https://crates.io/api/v1/crates/combine/4.6.8) | 4.6.8 | MIT | MIT | 2026-08-25T15:36:05.728682Z | 1; checksum-verified-registry-archive |  | pass |
| [concurrent-queue](https://crates.io/api/v1/crates/concurrent-queue/2.5.0) | 2.5.0 | Apache-2.0 OR MIT | MIT | 2024-04-26T14:26:18.816602Z | 2; checksum-verified-registry-archive |  | pass |
| [core-foundation](https://crates.io/api/v1/crates/core-foundation/0.9.4) | 0.9.4 | MIT OR Apache-2.0 | MIT | 2023-11-30T16:04:49.362965Z | 2; checksum-verified-registry-archive |  | pass |
| [core-foundation-sys](https://crates.io/api/v1/crates/core-foundation-sys/0.8.7) | 0.8.7 | MIT OR Apache-2.0 | MIT | 2024-08-11T00:48:27.870880Z | 2; checksum-verified-registry-archive |  | pass |
| [core-graphics](https://crates.io/api/v1/crates/core-graphics/0.23.2) | 0.23.2 | MIT OR Apache-2.0 | MIT | 2024-04-04T17:47:45.408685Z | 3; checksum-verified-registry-archive |  | pass |
| [core-graphics-types](https://crates.io/api/v1/crates/core-graphics-types/0.1.3) | 0.1.3 | MIT OR Apache-2.0 | MIT | 2023-11-30T22:08:24.071771Z | 2; checksum-verified-registry-archive |  | pass |
| [core_detect](https://crates.io/api/v1/crates/core_detect/1.0.0) | 1.0.0 | MIT/Apache-2.0 | MIT | 2021-05-09T16:36:07.524627Z | 2; checksum-verified-registry-archive | True | pass |
| [cpufeatures](https://crates.io/api/v1/crates/cpufeatures/0.3.1) | 0.3.1 | MIT OR Apache-2.0 | MIT | 2026-08-26T18:39:59.636708Z | 2; checksum-verified-registry-archive | True | pass |
| [crc32fast](https://crates.io/api/v1/crates/crc32fast/1.5.2) | 1.5.2 | MIT OR Apache-2.0 | MIT | 2026-09-12T18:41:46.923561Z | 2; checksum-verified-registry-archive | True | pass |
| [crossbeam-utils](https://crates.io/api/v1/crates/crossbeam-utils/0.8.23) | 0.8.23 | MIT OR Apache-2.0 | MIT | 2026-09-05T17:46:14.665751Z | 2; checksum-verified-registry-archive |  | pass |
| [crunchy](https://crates.io/api/v1/crates/crunchy/0.2.4) | 0.2.4 | MIT | MIT | 2025-06-25T12:54:43.146786Z | 1; checksum-verified-registry-archive |  | pass |
| [cursor-icon](https://crates.io/api/v1/crates/cursor-icon/1.2.0) | 1.2.0 | MIT OR Apache-2.0 OR Zlib | MIT | 2025-05-25T14:10:49.317221Z | 3; checksum-verified-registry-archive | True | pass |
| [darling](https://crates.io/api/v1/crates/darling/0.24.1) | 0.24.1 | MIT | MIT | 2026-08-20T15:12:01.702411Z | 1; checksum-verified-registry-archive | True | pass |
| [darling_core](https://crates.io/api/v1/crates/darling_core/0.24.1) | 0.24.1 | MIT | MIT | 2026-08-20T15:11:43.129462Z | 1; checksum-verified-registry-archive | True | pass |
| [darling_macro](https://crates.io/api/v1/crates/darling_macro/0.24.1) | 0.24.1 | MIT | MIT | 2026-08-20T15:11:50.704280Z | 1; checksum-verified-registry-archive | True | pass |
| [debug_unsafe](https://crates.io/api/v1/crates/debug_unsafe/0.1.4) | 0.1.4 | MIT OR Apache-2.0 | MIT | 2026-02-09T12:18:49.712292Z | 2; checksum-verified-registry-archive | True | pass |
| [dispatch](https://crates.io/api/v1/crates/dispatch/0.2.0) | 0.2.0 | MIT | MIT | 2020-01-19T06:24:11.514318Z | 0; MISSING |  | pass; source notices absent, not Windows |
| [dispatch2](https://crates.io/api/v1/crates/dispatch2/0.3.1) | 0.3.1 | Zlib OR Apache-2.0 OR MIT | MIT | 2026-02-26T15:35:48.969374Z | 1; exact-upstream-commit |  | pass |
| [displaydoc](https://crates.io/api/v1/crates/displaydoc/0.2.7) | 0.2.7 | MIT OR Apache-2.0 | MIT | 2026-07-28T13:58:21.461609Z | 2; checksum-verified-registry-archive | True | pass |
| [dlib](https://crates.io/api/v1/crates/dlib/0.5.3) | 0.5.3 | MIT | MIT | 2026-02-26T03:32:04.429182Z | 1; checksum-verified-registry-archive |  | pass |
| [document-features](https://crates.io/api/v1/crates/document-features/0.2.12) | 0.2.12 | MIT OR Apache-2.0 | MIT | 2025-10-24T09:43:31.356480Z | 2; checksum-verified-registry-archive | True | pass |
| [downcast-rs](https://crates.io/api/v1/crates/downcast-rs/1.2.1) | 1.2.1 | MIT/Apache-2.0 | MIT | 2024-04-07T00:40:26.467757Z | 2; checksum-verified-registry-archive |  | pass |
| [dpi](https://crates.io/api/v1/crates/dpi/0.1.2) | 0.1.2 | Apache-2.0 AND MIT | Apache-2.0 AND MIT | 2025-05-02T07:19:18.829952Z | 2; checksum-verified-registry-archive | True | pass |
| [dyn-clone](https://crates.io/api/v1/crates/dyn-clone/1.0.20) | 1.0.20 | MIT OR Apache-2.0 | MIT | 2025-07-27T19:29:46.625527Z | 2; checksum-verified-registry-archive | True | pass |
| [ecolor](https://crates.io/api/v1/crates/ecolor/0.36.2) | 0.36.2 | MIT OR Apache-2.0 | MIT | 2026-09-08T15:10:45.128880Z | 2; exact-upstream-commit | True | pass |
| [eframe](https://crates.io/api/v1/crates/eframe/0.36.2) | 0.36.2 | MIT OR Apache-2.0 | MIT | 2026-09-08T15:12:19.120290Z | 2; exact-upstream-commit | True | pass |
| [egui](https://crates.io/api/v1/crates/egui/0.36.2) | 0.36.2 | MIT OR Apache-2.0 | MIT | 2026-09-08T15:11:09.756806Z | 2; exact-upstream-commit | True | pass |
| [egui-winit](https://crates.io/api/v1/crates/egui-winit/0.36.2) | 0.36.2 | MIT OR Apache-2.0 | MIT | 2026-09-08T15:11:24.477629Z | 2; exact-upstream-commit | True | pass |
| [egui_glow](https://crates.io/api/v1/crates/egui_glow/0.36.2) | 0.36.2 | MIT OR Apache-2.0 | MIT | 2026-09-08T15:11:31.931789Z | 2; exact-upstream-commit | True | pass |
| [either](https://crates.io/api/v1/crates/either/1.18.0) | 1.18.0 | MIT OR Apache-2.0 | MIT | 2026-08-20T19:36:48.266137Z | 2; checksum-verified-registry-archive | True | pass |
| [emath](https://crates.io/api/v1/crates/emath/0.36.2) | 0.36.2 | MIT OR Apache-2.0 | MIT | 2026-09-08T15:10:41.082995Z | 2; exact-upstream-commit | True | pass |
| [encoding_rs](https://crates.io/api/v1/crates/encoding_rs/0.8.42) | 0.8.42 | (Apache-2.0 OR MIT) AND BSD-3-Clause | BSD-3-Clause AND MIT | 2026-09-24T14:45:32.296812Z | 4; checksum-verified-registry-archive | True | pass |
| [endi](https://crates.io/api/v1/crates/endi/1.1.1) | 1.1.1 | MIT | MIT | 2025-11-26T17:18:58.382799Z | 1; checksum-verified-registry-archive |  | pass |
| [enumflags2](https://crates.io/api/v1/crates/enumflags2/0.7.12) | 0.7.12 | MIT OR Apache-2.0 | MIT | 2025-06-09T22:16:25.147642Z | 2; checksum-verified-registry-archive |  | pass |
| [enumflags2_derive](https://crates.io/api/v1/crates/enumflags2_derive/0.7.12) | 0.7.12 | MIT OR Apache-2.0 | MIT | 2025-06-09T22:15:13.358677Z | 2; checksum-verified-registry-archive |  | pass |
| [enumn](https://crates.io/api/v1/crates/enumn/0.1.14) | 0.1.14 | MIT OR Apache-2.0 | MIT | 2024-07-30T05:57:57.757718Z | 2; checksum-verified-registry-archive | True | pass |
| [epaint](https://crates.io/api/v1/crates/epaint/0.36.2) | 0.36.2 | MIT OR Apache-2.0 | MIT | 2026-09-08T15:11:00.536683Z | 2; exact-upstream-commit | True | pass |
| [equivalent](https://crates.io/api/v1/crates/equivalent/1.0.2) | 1.0.2 | Apache-2.0 OR MIT | MIT | 2025-02-14T22:14:18.212755Z | 2; checksum-verified-registry-archive | True | pass |
| [errno](https://crates.io/api/v1/crates/errno/0.3.14) | 0.3.14 | MIT OR Apache-2.0 | MIT | 2025-09-09T04:35:15.362504Z | 2; checksum-verified-registry-archive |  | pass |
| [error-code](https://crates.io/api/v1/crates/error-code/3.4.0) | 3.4.0 | BSL-1.0 | BSL-1.0 | 2026-08-15T09:38:47.556114Z | 1; checksum-verified-registry-archive | True | pass |
| [euclid](https://crates.io/api/v1/crates/euclid/0.22.14) | 0.22.14 | MIT OR Apache-2.0 | MIT | 2026-03-18T20:00:26.960761Z | 3; checksum-verified-registry-archive | True | pass |
| [event-listener](https://crates.io/api/v1/crates/event-listener/5.4.2) | 5.4.2 | Apache-2.0 OR MIT | MIT | 2026-07-27T11:31:20.582548Z | 2; checksum-verified-registry-archive |  | pass |
| [event-listener-strategy](https://crates.io/api/v1/crates/event-listener-strategy/0.5.4) | 0.5.4 | Apache-2.0 OR MIT | MIT | 2025-03-27T17:31:33.741698Z | 2; checksum-verified-registry-archive |  | pass |
| [fast-float2](https://crates.io/api/v1/crates/fast-float2/0.2.4) | 0.2.4 | MIT OR Apache-2.0 | MIT | 2026-08-14T02:30:24.263507Z | 2; checksum-verified-registry-archive | True | pass |
| [fastrand](https://crates.io/api/v1/crates/fastrand/2.5.0) | 2.5.0 | Apache-2.0 OR MIT | MIT | 2026-07-19T12:40:53.683020Z | 2; checksum-verified-registry-archive |  | pass |
| [fax](https://crates.io/api/v1/crates/fax/0.2.7) | 0.2.7 | MIT | MIT | 2026-04-27T16:23:20.463012Z | 1; checksum-verified-registry-archive | True | pass |
| [fdeflate](https://crates.io/api/v1/crates/fdeflate/0.3.7) | 0.3.7 | MIT OR Apache-2.0 | MIT | 2024-12-06T03:16:12.038530Z | 2; checksum-verified-registry-archive | True | pass |
| [fearless_simd](https://crates.io/api/v1/crates/fearless_simd/0.4.1) | 0.4.1 | Apache-2.0 OR MIT | MIT | 2026-05-16T09:49:57.602219Z | 2; checksum-verified-registry-archive | True | pass |
| [find-msvc-tools](https://crates.io/api/v1/crates/find-msvc-tools/0.1.14) | 0.1.14 | MIT OR Apache-2.0 | MIT | 2026-09-25T10:09:48.951244Z | 2; checksum-verified-registry-archive |  | pass |
| [flate2](https://crates.io/api/v1/crates/flate2/1.1.10) | 1.1.10 | MIT OR Apache-2.0 | MIT | 2026-08-28T04:35:22.192325Z | 2; checksum-verified-registry-archive | True | pass |
| [foldhash](https://crates.io/api/v1/crates/foldhash/0.2.0) | 0.2.0 | Zlib | Zlib | 2025-08-23T19:14:01.103482Z | 1; checksum-verified-registry-archive | True | pass |
| [font-types](https://crates.io/api/v1/crates/font-types/0.11.3) | 0.11.3 | MIT OR Apache-2.0 | MIT | 2026-04-13T19:48:21.100332Z | 2; checksum-verified-registry-archive | True | pass |
| [font-types](https://crates.io/api/v1/crates/font-types/0.12.5) | 0.12.5 | MIT OR Apache-2.0 | MIT | 2026-09-08T19:41:32.081290Z | 2; checksum-verified-registry-archive | True | pass |
| [foreign-types](https://crates.io/api/v1/crates/foreign-types/0.5.0) | 0.5.0 | MIT/Apache-2.0 | MIT | 2019-10-13T19:24:15.900127Z | 2; checksum-verified-registry-archive |  | pass |
| [foreign-types-macros](https://crates.io/api/v1/crates/foreign-types-macros/0.2.4) | 0.2.4 | MIT/Apache-2.0 | MIT | 2026-07-22T20:33:59.720421Z | 2; checksum-verified-registry-archive |  | pass |
| [foreign-types-shared](https://crates.io/api/v1/crates/foreign-types-shared/0.3.1) | 0.3.1 | MIT/Apache-2.0 | MIT | 2022-03-21T22:06:37.413498Z | 2; checksum-verified-registry-archive |  | pass |
| [form_urlencoded](https://crates.io/api/v1/crates/form_urlencoded/1.2.2) | 1.2.2 | MIT OR Apache-2.0 | MIT | 2025-08-21T08:47:43.939560Z | 2; checksum-verified-registry-archive | True | pass |
| [futures](https://crates.io/api/v1/crates/futures/0.3.34) | 0.3.34 | MIT OR Apache-2.0 | MIT | 2026-08-11T12:13:28.064590Z | 2; checksum-verified-registry-archive | True | pass |
| [futures-channel](https://crates.io/api/v1/crates/futures-channel/0.3.34) | 0.3.34 | MIT OR Apache-2.0 | MIT | 2026-08-11T12:13:10.334104Z | 2; checksum-verified-registry-archive | True | pass |
| [futures-core](https://crates.io/api/v1/crates/futures-core/0.3.34) | 0.3.34 | MIT OR Apache-2.0 | MIT | 2026-08-11T12:13:01.547638Z | 2; checksum-verified-registry-archive | True | pass |
| [futures-executor](https://crates.io/api/v1/crates/futures-executor/0.3.34) | 0.3.34 | MIT OR Apache-2.0 | MIT | 2026-08-11T12:13:23.714779Z | 2; checksum-verified-registry-archive | True | pass |
| [futures-io](https://crates.io/api/v1/crates/futures-io/0.3.34) | 0.3.34 | MIT OR Apache-2.0 | MIT | 2026-08-11T12:13:03.992711Z | 2; checksum-verified-registry-archive | True | pass |
| [futures-lite](https://crates.io/api/v1/crates/futures-lite/2.6.1) | 2.6.1 | Apache-2.0 OR MIT | MIT | 2025-08-04T00:29:25.672150Z | 3; checksum-verified-registry-archive |  | pass |
| [futures-macro](https://crates.io/api/v1/crates/futures-macro/0.3.34) | 0.3.34 | MIT OR Apache-2.0 | MIT | 2026-08-11T12:13:14.842510Z | 2; checksum-verified-registry-archive | True | pass |
| [futures-sink](https://crates.io/api/v1/crates/futures-sink/0.3.34) | 0.3.34 | MIT OR Apache-2.0 | MIT | 2026-08-11T12:13:06.160611Z | 2; checksum-verified-registry-archive | True | pass |
| [futures-task](https://crates.io/api/v1/crates/futures-task/0.3.34) | 0.3.34 | MIT OR Apache-2.0 | MIT | 2026-08-11T12:13:07.886026Z | 2; checksum-verified-registry-archive | True | pass |
| [futures-util](https://crates.io/api/v1/crates/futures-util/0.3.34) | 0.3.34 | MIT OR Apache-2.0 | MIT | 2026-08-11T12:13:19.272525Z | 2; checksum-verified-registry-archive | True | pass |
| [gethostname](https://crates.io/api/v1/crates/gethostname/1.1.0) | 1.1.0 | Apache-2.0 | Apache-2.0 | 2025-10-14T21:50:44.962698Z | 1; checksum-verified-registry-archive |  | pass |
| [getrandom](https://crates.io/api/v1/crates/getrandom/0.3.4) | 0.3.4 | MIT OR Apache-2.0 | MIT | 2025-10-14T17:47:36.464125Z | 2; checksum-verified-registry-archive | True | pass |
| [getrandom](https://crates.io/api/v1/crates/getrandom/0.4.3) | 0.4.3 | MIT OR Apache-2.0 | MIT | 2026-06-17T18:01:59.979552Z | 2; checksum-verified-registry-archive | True | pass |
| [gl_generator](https://crates.io/api/v1/crates/gl_generator/0.14.0) | 0.14.0 | Apache-2.0 | Apache-2.0 | 2019-10-03T10:44:28.795911Z | 1; exact-upstream-commit | True | pass |
| [glifo](https://crates.io/api/v1/crates/glifo/0.2.0) | 0.2.0 | Apache-2.0 OR MIT | MIT | 2026-07-29T18:52:20.995994Z | 2; checksum-verified-registry-archive | True | pass |
| [glow](https://crates.io/api/v1/crates/glow/0.17.0) | 0.17.0 | MIT OR Apache-2.0 OR Zlib | MIT | 2026-03-07T04:35:01.036126Z | 3; checksum-verified-registry-archive | True | pass |
| [glutin](https://crates.io/api/v1/crates/glutin/0.32.3) | 0.32.3 | Apache-2.0 | Apache-2.0 | 2025-04-30T11:33:39.726207Z | 1; checksum-verified-registry-archive | True | pass |
| [glutin-winit](https://crates.io/api/v1/crates/glutin-winit/0.5.0) | 0.5.0 | MIT | MIT | 2024-06-08T10:31:13.194529Z | 1; checksum-verified-registry-archive | True | pass |
| [glutin_egl_sys](https://crates.io/api/v1/crates/glutin_egl_sys/0.7.1) | 0.7.1 | Apache-2.0 | Apache-2.0 | 2025-01-12T09:53:13.771937Z | 1; checksum-verified-registry-archive | True | pass |
| [glutin_wgl_sys](https://crates.io/api/v1/crates/glutin_wgl_sys/0.6.1) | 0.6.1 | Apache-2.0 | Apache-2.0 | 2025-01-12T09:52:21.238564Z | 1; checksum-verified-registry-archive | True | pass |
| [guillotiere](https://crates.io/api/v1/crates/guillotiere/0.7.0) | 0.7.0 | MIT/Apache-2.0 | MIT | 2026-03-18T21:58:48.041153Z | 2; checksum-verified-registry-archive | True | pass |
| [half](https://crates.io/api/v1/crates/half/2.7.1) | 2.7.1 | MIT OR Apache-2.0 | MIT | 2025-10-14T03:59:26.575229Z | 2; checksum-verified-registry-archive | True | pass |
| [harfrust](https://crates.io/api/v1/crates/harfrust/0.12.0) | 0.12.0 | MIT | MIT | 2026-07-03T19:32:00.510158Z | 1; checksum-verified-registry-archive | True | pass |
| [hashbrown](https://crates.io/api/v1/crates/hashbrown/0.16.1) | 0.16.1 | MIT OR Apache-2.0 | MIT | 2025-11-20T16:47:18.741054Z | 2; checksum-verified-registry-archive | True | pass |
| [hashbrown](https://crates.io/api/v1/crates/hashbrown/0.17.1) | 0.17.1 | MIT OR Apache-2.0 | MIT | 2026-05-09T04:35:04.251Z | 2; checksum-verified-registry-archive | True | pass |
| [hermit-abi](https://crates.io/api/v1/crates/hermit-abi/0.5.3) | 0.5.3 | MIT OR Apache-2.0 | MIT | 2026-08-31T11:45:18.991301Z | 2; checksum-verified-registry-archive |  | pass |
| [hex](https://crates.io/api/v1/crates/hex/0.4.3) | 0.4.3 | MIT OR Apache-2.0 | MIT | 2021-03-03T10:48:58.682837Z | 2; checksum-verified-registry-archive |  | pass |
| [http](https://crates.io/api/v1/crates/http/1.5.0) | 1.5.0 | MIT OR Apache-2.0 | MIT | 2026-07-29T14:57:21.755258Z | 2; checksum-verified-registry-archive | True | pass |
| [http-body](https://crates.io/api/v1/crates/http-body/1.1.0) | 1.1.0 | MIT | MIT | 2026-07-13T17:25:06.475903Z | 1; checksum-verified-registry-archive | True | pass |
| [http-body-util](https://crates.io/api/v1/crates/http-body-util/0.1.5) | 0.1.5 | MIT | MIT | 2026-08-12T15:22:21.622138Z | 1; checksum-verified-registry-archive | True | pass |
| [httparse](https://crates.io/api/v1/crates/httparse/1.10.1) | 1.10.1 | MIT OR Apache-2.0 | MIT | 2025-03-03T14:44:09.975762Z | 2; checksum-verified-registry-archive | True | pass |
| [httpdate](https://crates.io/api/v1/crates/httpdate/1.0.3) | 1.0.3 | MIT OR Apache-2.0 | MIT | 2023-08-13T10:55:54.391658Z | 2; checksum-verified-registry-archive | True | pass |
| [hyper](https://crates.io/api/v1/crates/hyper/1.11.1) | 1.11.1 | MIT | MIT | 2026-08-28T12:22:30.892284Z | 1; checksum-verified-registry-archive | True | pass |
| [hyper-util](https://crates.io/api/v1/crates/hyper-util/0.1.21) | 0.1.21 | MIT | MIT | 2026-09-24T17:47:59.783880Z | 1; checksum-verified-registry-archive | True | pass |
| [iana-time-zone](https://crates.io/api/v1/crates/iana-time-zone/0.1.65) | 0.1.65 | MIT OR Apache-2.0 | MIT | 2026-01-28T08:40:52.528531Z | 2; checksum-verified-registry-archive |  | pass |
| [iana-time-zone-haiku](https://crates.io/api/v1/crates/iana-time-zone-haiku/0.1.2) | 0.1.2 | MIT OR Apache-2.0 | MIT | 2023-05-12T08:43:45.406909Z | 2; checksum-verified-registry-archive |  | pass |
| [icu_collections](https://crates.io/api/v1/crates/icu_collections/2.3.0) | 2.3.0 | Unicode-3.0 | Unicode-3.0 | 2026-08-13T23:28:40.279341Z | 1; checksum-verified-registry-archive | True | pass |
| [icu_locale_core](https://crates.io/api/v1/crates/icu_locale_core/2.3.0) | 2.3.0 | Unicode-3.0 | Unicode-3.0 | 2026-08-13T23:28:12.641726Z | 1; checksum-verified-registry-archive | True | pass |
| [icu_normalizer](https://crates.io/api/v1/crates/icu_normalizer/2.3.0) | 2.3.0 | Unicode-3.0 | Unicode-3.0 | 2026-08-13T23:28:56.931089Z | 1; checksum-verified-registry-archive | True | pass |
| [icu_normalizer_data](https://crates.io/api/v1/crates/icu_normalizer_data/2.3.0) | 2.3.0 | Unicode-3.0 | Unicode-3.0 | 2026-08-13T23:28:53.835366Z | 1; checksum-verified-registry-archive | True | pass |
| [icu_properties](https://crates.io/api/v1/crates/icu_properties/2.3.0) | 2.3.0 | Unicode-3.0 | Unicode-3.0 | 2026-08-13T23:28:46.255153Z | 1; checksum-verified-registry-archive | True | pass |
| [icu_properties_data](https://crates.io/api/v1/crates/icu_properties_data/2.3.0) | 2.3.0 | Unicode-3.0 | Unicode-3.0 | 2026-08-13T23:28:42.576523Z | 1; checksum-verified-registry-archive | True | pass |
| [icu_provider](https://crates.io/api/v1/crates/icu_provider/2.3.1) | 2.3.1 | Unicode-3.0 | Unicode-3.0 | 2026-08-20T23:25:32.095205Z | 1; checksum-verified-registry-archive | True | pass |
| [ident_case](https://crates.io/api/v1/crates/ident_case/1.0.1) | 1.0.1 | MIT/Apache-2.0 | MIT | 2019-03-18T20:07:20.426691Z | 1; checksum-verified-registry-archive | True | pass |
| [idna](https://crates.io/api/v1/crates/idna/1.1.0) | 1.1.0 | MIT OR Apache-2.0 | MIT | 2025-08-21T08:48:20.326736Z | 2; checksum-verified-registry-archive | True | pass |
| [idna_adapter](https://crates.io/api/v1/crates/idna_adapter/1.2.2) | 1.2.2 | Apache-2.0 OR MIT | MIT | 2026-04-27T17:45:54.838641Z | 2; checksum-verified-registry-archive | True | pass |
| [image](https://crates.io/api/v1/crates/image/0.25.10) | 0.25.10 | MIT OR Apache-2.0 | MIT | 2026-03-10T16:29:18.250581Z | 2; checksum-verified-registry-archive | True | pass |
| [indexmap](https://crates.io/api/v1/crates/indexmap/2.14.2) | 2.14.2 | Apache-2.0 OR MIT | MIT | 2026-09-05T01:53:48.019655Z | 2; checksum-verified-registry-archive | True | pass |
| [ipnet](https://crates.io/api/v1/crates/ipnet/2.12.2) | 2.12.2 | MIT OR Apache-2.0 | MIT | 2026-09-06T05:45:08.121491Z | 2; checksum-verified-registry-archive | True | pass |
| [itertools](https://crates.io/api/v1/crates/itertools/0.15.0) | 0.15.0 | MIT OR Apache-2.0 | MIT | 2026-06-16T15:09:28.880416Z | 2; checksum-verified-registry-archive | True | pass |
| [itoa](https://crates.io/api/v1/crates/itoa/1.0.18) | 1.0.18 | MIT OR Apache-2.0 | MIT | 2026-03-20T00:12:37.043881Z | 2; checksum-verified-registry-archive | True | pass |
| [jni](https://crates.io/api/v1/crates/jni/0.22.4) | 0.22.4 | MIT OR Apache-2.0 | MIT | 2026-03-16T15:38:08.740007Z | 2; exact-upstream-commit |  | pass |
| [jni-macros](https://crates.io/api/v1/crates/jni-macros/0.22.4) | 0.22.4 | MIT OR Apache-2.0 | MIT | 2026-03-16T13:29:39.385205Z | 2; exact-upstream-commit |  | pass |
| [jni-sys](https://crates.io/api/v1/crates/jni-sys/0.3.1) | 0.3.1 | MIT OR Apache-2.0 | MIT | 2026-03-22T16:30:17.889789Z | 2; checksum-verified-registry-archive |  | pass |
| [jni-sys](https://crates.io/api/v1/crates/jni-sys/0.4.1) | 0.4.1 | MIT OR Apache-2.0 | MIT | 2026-01-09T00:33:31.392921Z | 2; checksum-verified-registry-archive |  | pass |
| [jni-sys-macros](https://crates.io/api/v1/crates/jni-sys-macros/0.4.1) | 0.4.1 | MIT OR Apache-2.0 | MIT | 2026-01-09T00:23:44.614075Z | 2; exact-upstream-commit |  | pass |
| [jobserver](https://crates.io/api/v1/crates/jobserver/0.1.35) | 0.1.35 | MIT OR Apache-2.0 | MIT | 2026-07-06T01:43:15.155394Z | 2; checksum-verified-registry-archive |  | pass |
| [js-sys](https://crates.io/api/v1/crates/js-sys/0.3.106) | 0.3.106 | MIT OR Apache-2.0 | MIT | 2026-09-25T07:13:42.260040Z | 2; checksum-verified-registry-archive |  | pass |
| [khronos_api](https://crates.io/api/v1/crates/khronos_api/3.1.0) | 3.1.0 | Apache-2.0 | Apache-2.0 | 2019-04-01T04:52:05.349671Z | 1; exact-upstream-commit | True | pass |
| [kurbo](https://crates.io/api/v1/crates/kurbo/0.13.1) | 0.13.1 | Apache-2.0 OR MIT | MIT | 2026-05-13T16:32:09.461806Z | 2; checksum-verified-registry-archive | True | pass |
| [libc](https://crates.io/api/v1/crates/libc/0.2.189) | 0.2.189 | MIT OR Apache-2.0 | MIT | 2026-07-21T21:33:28.018110Z | 2; checksum-verified-registry-archive | True | pass |
| [libloading](https://crates.io/api/v1/crates/libloading/0.8.9) | 0.8.9 | ISC | ISC | 2025-09-21T10:59:30.598195Z | 1; checksum-verified-registry-archive | True | pass |
| [libredox](https://crates.io/api/v1/crates/libredox/0.1.25) | 0.1.25 | MIT | MIT | 2026-09-21T13:30:01.781331Z | 1; checksum-verified-registry-archive |  | pass |
| [linebender_resource_handle](https://crates.io/api/v1/crates/linebender_resource_handle/0.1.1) | 0.1.1 | Apache-2.0 OR MIT | MIT | 2025-09-16T09:53:56.589443Z | 2; checksum-verified-registry-archive | True | pass |
| [linux-raw-sys](https://crates.io/api/v1/crates/linux-raw-sys/0.12.1) | 0.12.1 | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT | MIT | 2025-12-23T00:00:07.059646Z | 4; checksum-verified-registry-archive |  | pass |
| [linux-raw-sys](https://crates.io/api/v1/crates/linux-raw-sys/0.4.15) | 0.4.15 | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT | MIT | 2025-01-08T00:37:56.538364Z | 4; checksum-verified-registry-archive |  | pass |
| [litemap](https://crates.io/api/v1/crates/litemap/0.8.3) | 0.8.3 | Unicode-3.0 | Unicode-3.0 | 2026-08-13T23:27:57.524692Z | 1; checksum-verified-registry-archive | True | pass |
| [litrs](https://crates.io/api/v1/crates/litrs/1.0.0) | 1.0.0 | MIT OR Apache-2.0 | MIT | 2025-10-20T10:00:07.760737Z | 2; checksum-verified-registry-archive | True | pass |
| [lock_api](https://crates.io/api/v1/crates/lock_api/0.4.14) | 0.4.14 | MIT OR Apache-2.0 | MIT | 2025-10-03T23:33:12.850087Z | 2; checksum-verified-registry-archive | True | pass |
| [log](https://crates.io/api/v1/crates/log/0.4.34) | 0.4.34 | MIT OR Apache-2.0 | MIT | 2026-08-22T11:44:27.941626Z | 2; checksum-verified-registry-archive | True | pass |
| [matchit](https://crates.io/api/v1/crates/matchit/0.8.4) | 0.8.4 | MIT AND BSD-3-Clause | BSD-3-Clause AND MIT | 2024-07-11T07:22:03.735375Z | 2; checksum-verified-registry-archive | True | pass |
| [memchr](https://crates.io/api/v1/crates/memchr/2.8.3) | 2.8.3 | Unlicense OR MIT | MIT | 2026-07-08T00:49:54.408285Z | 2; checksum-verified-registry-archive | True | pass |
| [memmap2](https://crates.io/api/v1/crates/memmap2/0.9.11) | 0.9.11 | MIT OR Apache-2.0 | MIT | 2026-06-22T16:04:03.992829Z | 2; checksum-verified-registry-archive |  | pass |
| [memoffset](https://crates.io/api/v1/crates/memoffset/0.9.1) | 0.9.1 | MIT | MIT | 2024-03-26T19:58:31.849513Z | 1; checksum-verified-registry-archive |  | pass |
| [mime](https://crates.io/api/v1/crates/mime/0.3.17) | 0.3.17 | MIT OR Apache-2.0 | MIT | 2023-03-20T15:05:04.272177Z | 2; checksum-verified-registry-archive | True | pass |
| [miniz_oxide](https://crates.io/api/v1/crates/miniz_oxide/0.8.9) | 0.8.9 | MIT OR Zlib OR Apache-2.0 | MIT | 2025-06-09T22:26:12.218895Z | 4; checksum-verified-registry-archive | True | pass |
| [miniz_oxide](https://crates.io/api/v1/crates/miniz_oxide/0.9.1) | 0.9.1 | MIT OR Zlib OR Apache-2.0 | MIT | 2026-03-13T00:19:26.378812Z | 4; checksum-verified-registry-archive | True | pass |
| [mio](https://crates.io/api/v1/crates/mio/1.2.3) | 1.2.3 | MIT | MIT | 2026-09-02T13:22:12.206563Z | 1; checksum-verified-registry-archive | True | pass |
| [moxcms](https://crates.io/api/v1/crates/moxcms/0.8.1) | 0.8.1 | BSD-3-Clause OR Apache-2.0 | Apache-2.0 | 2026-03-07T15:47:14.526501Z | 2; checksum-verified-registry-archive | True | pass |
| [multiversion_no_op](https://crates.io/api/v1/crates/multiversion_no_op/1.0.0) | 1.0.0 | Apache-2.0 OR MIT | MIT | 2026-07-30T13:20:49.233526Z | 3; checksum-verified-registry-archive | True | pass |
| [ndk](https://crates.io/api/v1/crates/ndk/0.9.0) | 0.9.0 | MIT OR Apache-2.0 | MIT | 2024-04-26T15:15:21.991128Z | 2; exact-upstream-commit |  | pass |
| [ndk-context](https://crates.io/api/v1/crates/ndk-context/0.1.1) | 0.1.1 | MIT OR Apache-2.0 | MIT | 2022-04-19T14:40:07.603632Z | 2; exact-upstream-commit |  | pass |
| [ndk-sys](https://crates.io/api/v1/crates/ndk-sys/0.6.0%2B11769913) | 0.6.0+11769913 | MIT OR Apache-2.0 | MIT | 2024-04-26T15:14:59.325509Z | 2; exact-upstream-commit |  | pass |
| [nohash-hasher](https://crates.io/api/v1/crates/nohash-hasher/0.2.0) | 0.2.0 | Apache-2.0 OR MIT | MIT | 2020-02-17T11:38:13.111407Z | 2; checksum-verified-registry-archive | True | pass |
| [num-traits](https://crates.io/api/v1/crates/num-traits/0.2.19) | 0.2.19 | MIT OR Apache-2.0 | MIT | 2024-05-03T21:27:39.856244Z | 2; checksum-verified-registry-archive | True | pass |
| [num_enum](https://crates.io/api/v1/crates/num_enum/0.7.6) | 0.7.6 | BSD-3-Clause OR MIT OR Apache-2.0 | MIT | 2026-03-15T21:29:40.865699Z | 3; checksum-verified-registry-archive |  | pass |
| [num_enum_derive](https://crates.io/api/v1/crates/num_enum_derive/0.7.6) | 0.7.6 | BSD-3-Clause OR MIT OR Apache-2.0 | MIT | 2026-03-15T21:29:38.220565Z | 3; checksum-verified-registry-archive |  | pass |
| [objc-sys](https://crates.io/api/v1/crates/objc-sys/0.3.5) | 0.3.5 | MIT | MIT | 2024-05-21T07:56:24.552811Z | 1; exact-upstream-commit |  | pass |
| [objc2](https://crates.io/api/v1/crates/objc2/0.5.2) | 0.5.2 | MIT | MIT | 2024-05-21T07:56:40.563699Z | 1; exact-upstream-commit |  | pass |
| [objc2](https://crates.io/api/v1/crates/objc2/0.6.4) | 0.6.4 | MIT | MIT | 2026-02-26T15:35:34.959793Z | 1; exact-upstream-commit |  | pass |
| [objc2-app-kit](https://crates.io/api/v1/crates/objc2-app-kit/0.2.2) | 0.2.2 | MIT | MIT | 2024-05-21T08:25:08.369660Z | 1; exact-upstream-commit |  | pass |
| [objc2-app-kit](https://crates.io/api/v1/crates/objc2-app-kit/0.3.2) | 0.3.2 | Zlib OR Apache-2.0 OR MIT | MIT | 2025-10-04T16:16:50.311115Z | 1; exact-upstream-commit |  | pass |
| [objc2-cloud-kit](https://crates.io/api/v1/crates/objc2-cloud-kit/0.2.2) | 0.2.2 | MIT | MIT | 2024-05-21T08:30:35.709618Z | 1; exact-upstream-commit |  | pass |
| [objc2-contacts](https://crates.io/api/v1/crates/objc2-contacts/0.2.2) | 0.2.2 | MIT | MIT | 2024-05-21T08:30:25.248897Z | 1; exact-upstream-commit |  | pass |
| [objc2-core-data](https://crates.io/api/v1/crates/objc2-core-data/0.2.2) | 0.2.2 | MIT | MIT | 2024-05-21T08:24:20.633797Z | 1; exact-upstream-commit |  | pass |
| [objc2-core-foundation](https://crates.io/api/v1/crates/objc2-core-foundation/0.3.2) | 0.3.2 | Zlib OR Apache-2.0 OR MIT | MIT | 2025-10-04T15:46:51.264986Z | 1; exact-upstream-commit |  | pass |
| [objc2-core-graphics](https://crates.io/api/v1/crates/objc2-core-graphics/0.3.2) | 0.3.2 | Zlib OR Apache-2.0 OR MIT | MIT | 2025-10-04T15:47:47.786249Z | 1; exact-upstream-commit |  | pass |
| [objc2-core-image](https://crates.io/api/v1/crates/objc2-core-image/0.2.2) | 0.2.2 | MIT | MIT | 2024-05-21T08:24:40.012311Z | 1; exact-upstream-commit |  | pass |
| [objc2-core-location](https://crates.io/api/v1/crates/objc2-core-location/0.2.2) | 0.2.2 | MIT | MIT | 2024-05-21T08:30:29.450774Z | 1; exact-upstream-commit |  | pass |
| [objc2-encode](https://crates.io/api/v1/crates/objc2-encode/4.1.0) | 4.1.0 | MIT | MIT | 2025-01-22T23:40:12.051433Z | 1; exact-upstream-commit |  | pass |
| [objc2-foundation](https://crates.io/api/v1/crates/objc2-foundation/0.2.2) | 0.2.2 | MIT | MIT | 2024-05-21T08:23:47.661422Z | 2; checksum-verified-registry-archive |  | pass |
| [objc2-foundation](https://crates.io/api/v1/crates/objc2-foundation/0.3.2) | 0.3.2 | MIT | MIT | 2025-10-04T15:47:21.149969Z | 2; checksum-verified-registry-archive |  | pass |
| [objc2-io-surface](https://crates.io/api/v1/crates/objc2-io-surface/0.3.2) | 0.3.2 | Zlib OR Apache-2.0 OR MIT | MIT | 2025-10-04T15:47:28.561974Z | 1; exact-upstream-commit |  | pass |
| [objc2-link-presentation](https://crates.io/api/v1/crates/objc2-link-presentation/0.2.2) | 0.2.2 | MIT | MIT | 2024-05-21T08:32:05.308271Z | 1; exact-upstream-commit |  | pass |
| [objc2-metal](https://crates.io/api/v1/crates/objc2-metal/0.2.2) | 0.2.2 | MIT | MIT | 2024-05-21T08:24:30.712409Z | 1; exact-upstream-commit |  | pass |
| [objc2-quartz-core](https://crates.io/api/v1/crates/objc2-quartz-core/0.2.2) | 0.2.2 | MIT | MIT | 2024-05-21T08:24:47.012901Z | 1; exact-upstream-commit |  | pass |
| [objc2-symbols](https://crates.io/api/v1/crates/objc2-symbols/0.2.2) | 0.2.2 | MIT | MIT | 2024-05-21T08:33:40.249073Z | 1; exact-upstream-commit |  | pass |
| [objc2-ui-kit](https://crates.io/api/v1/crates/objc2-ui-kit/0.2.2) | 0.2.2 | MIT | MIT | 2024-05-21T20:00:46.262148Z | 1; exact-upstream-commit |  | pass |
| [objc2-ui-kit](https://crates.io/api/v1/crates/objc2-ui-kit/0.3.2) | 0.3.2 | Zlib OR Apache-2.0 OR MIT | MIT | 2025-10-04T16:16:02.465579Z | 1; exact-upstream-commit |  | pass |
| [objc2-uniform-type-identifiers](https://crates.io/api/v1/crates/objc2-uniform-type-identifiers/0.2.2) | 0.2.2 | MIT | MIT | 2024-05-21T08:31:27.449256Z | 1; exact-upstream-commit |  | pass |
| [objc2-user-notifications](https://crates.io/api/v1/crates/objc2-user-notifications/0.2.2) | 0.2.2 | MIT | MIT | 2024-05-21T08:33:43.504686Z | 1; exact-upstream-commit |  | pass |
| [once_cell](https://crates.io/api/v1/crates/once_cell/1.21.4) | 1.21.4 | MIT OR Apache-2.0 | MIT | 2026-03-12T10:59:52.512992Z | 2; checksum-verified-registry-archive | True | pass |
| [orbclient](https://crates.io/api/v1/crates/orbclient/0.3.55) | 0.3.55 | MIT | MIT | 2026-05-24T15:16:32.462236Z | 1; checksum-verified-registry-archive |  | pass |
| [ordered-stream](https://crates.io/api/v1/crates/ordered-stream/0.2.0) | 0.2.0 | MIT OR Apache-2.0 | MIT | 2023-02-11T15:42:09.415977Z | 2; checksum-verified-registry-archive |  | pass |
| [parking](https://crates.io/api/v1/crates/parking/2.2.1) | 2.2.1 | Apache-2.0 OR MIT | MIT | 2024-09-07T17:39:03.956479Z | 3; checksum-verified-registry-archive |  | pass |
| [parking_lot](https://crates.io/api/v1/crates/parking_lot/0.12.5) | 0.12.5 | MIT OR Apache-2.0 | MIT | 2025-10-03T23:34:14.078935Z | 2; checksum-verified-registry-archive | True | pass |
| [parking_lot_core](https://crates.io/api/v1/crates/parking_lot_core/0.9.12) | 0.9.12 | MIT OR Apache-2.0 | MIT | 2025-10-03T23:33:43.164426Z | 2; checksum-verified-registry-archive | True | pass |
| [pastey](https://crates.io/api/v1/crates/pastey/0.2.3) | 0.2.3 | MIT OR Apache-2.0 | MIT | 2026-05-20T05:43:36.527579Z | 2; checksum-verified-registry-archive | True | pass |
| [pdf-writer](https://crates.io/api/v1/crates/pdf-writer/0.15.0) | 0.15.0 | MIT OR Apache-2.0 | MIT | 2026-05-27T16:12:00.835379Z | 2; checksum-verified-registry-archive | True | pass |
| [peniko](https://crates.io/api/v1/crates/peniko/0.6.1) | 0.6.1 | Apache-2.0 OR MIT | MIT | 2026-05-15T04:59:45.348256Z | 2; checksum-verified-registry-archive | True | pass |
| [percent-encoding](https://crates.io/api/v1/crates/percent-encoding/2.3.2) | 2.3.2 | MIT OR Apache-2.0 | MIT | 2025-08-21T08:46:50.131801Z | 2; checksum-verified-registry-archive | True | pass |
| [phf](https://crates.io/api/v1/crates/phf/0.13.1) | 0.13.1 | MIT | MIT | 2025-08-23T11:06:34.224289Z | 1; checksum-verified-registry-archive |  | pass |
| [phf_generator](https://crates.io/api/v1/crates/phf_generator/0.13.1) | 0.13.1 | MIT | MIT | 2025-08-23T11:06:08.136816Z | 1; checksum-verified-registry-archive |  | pass |
| [phf_macros](https://crates.io/api/v1/crates/phf_macros/0.13.1) | 0.13.1 | MIT | MIT | 2025-08-23T11:06:20.808422Z | 1; checksum-verified-registry-archive |  | pass |
| [phf_shared](https://crates.io/api/v1/crates/phf_shared/0.13.1) | 0.13.1 | MIT | MIT | 2025-08-23T11:05:56.874671Z | 1; checksum-verified-registry-archive |  | pass |
| [pin-project](https://crates.io/api/v1/crates/pin-project/1.1.13) | 1.1.13 | Apache-2.0 OR MIT | MIT | 2026-05-13T18:32:09.030433Z | 2; checksum-verified-registry-archive |  | pass |
| [pin-project-internal](https://crates.io/api/v1/crates/pin-project-internal/1.1.13) | 1.1.13 | Apache-2.0 OR MIT | MIT | 2026-05-13T18:32:07.416866Z | 2; checksum-verified-registry-archive |  | pass |
| [pin-project-lite](https://crates.io/api/v1/crates/pin-project-lite/0.2.17) | 0.2.17 | Apache-2.0 OR MIT | MIT | 2026-02-27T15:19:08.132543Z | 2; checksum-verified-registry-archive | True | pass |
| [piper](https://crates.io/api/v1/crates/piper/0.2.5) | 0.2.5 | MIT OR Apache-2.0 | MIT | 2026-02-28T06:00:20.764458Z | 2; checksum-verified-registry-archive |  | pass |
| [pkg-config](https://crates.io/api/v1/crates/pkg-config/0.3.34) | 0.3.34 | MIT OR Apache-2.0 | MIT | 2026-08-14T08:27:51.724149Z | 2; checksum-verified-registry-archive |  | pass |
| [plain](https://crates.io/api/v1/crates/plain/0.2.3) | 0.2.3 | MIT/Apache-2.0 | MIT | 2017-11-24T15:28:33.966750Z | 2; checksum-verified-registry-archive |  | pass |
| [png](https://crates.io/api/v1/crates/png/0.18.1) | 0.18.1 | MIT OR Apache-2.0 | MIT | 2026-02-14T16:24:10.825565Z | 2; checksum-verified-registry-archive | True | pass |
| [polling](https://crates.io/api/v1/crates/polling/3.11.0) | 3.11.0 | Apache-2.0 OR MIT | MIT | 2025-09-14T14:00:51.946852Z | 2; checksum-verified-registry-archive |  | pass |
| [polycool](https://crates.io/api/v1/crates/polycool/0.4.0) | 0.4.0 | MIT OR Apache-2.0 | MIT | 2025-10-23T22:48:11.442086Z | 2; checksum-verified-registry-archive | True | pass |
| [potential_utf](https://crates.io/api/v1/crates/potential_utf/0.1.6) | 0.1.6 | Unicode-3.0 | Unicode-3.0 | 2026-08-13T23:28:14.578806Z | 1; checksum-verified-registry-archive | True | pass |
| [proc-macro-crate](https://crates.io/api/v1/crates/proc-macro-crate/3.5.0) | 3.5.0 | MIT OR Apache-2.0 | MIT | 2026-03-04T11:12:33.029865Z | 2; checksum-verified-registry-archive |  | pass |
| [proc-macro2](https://crates.io/api/v1/crates/proc-macro2/1.0.107) | 1.0.107 | MIT OR Apache-2.0 | MIT | 2026-07-19T00:18:25.189787Z | 2; checksum-verified-registry-archive | True | pass |
| [profiling](https://crates.io/api/v1/crates/profiling/1.0.18) | 1.0.18 | MIT OR Apache-2.0 | MIT | 2026-05-04T03:33:15.842050Z | 2; exact-upstream-commit | True | pass |
| [pxfm](https://crates.io/api/v1/crates/pxfm/0.1.30) | 0.1.30 | BSD-3-Clause OR Apache-2.0 | Apache-2.0 | 2026-07-06T06:43:55.517978Z | 2; checksum-verified-registry-archive | True | pass |
| [quick-error](https://crates.io/api/v1/crates/quick-error/2.0.1) | 2.0.1 | MIT/Apache-2.0 | MIT | 2021-05-07T12:10:10.154641Z | 2; checksum-verified-registry-archive | True | pass |
| [quick-xml](https://crates.io/api/v1/crates/quick-xml/0.41.0) | 0.41.0 | MIT | MIT | 2026-06-29T18:04:43.720155Z | 1; checksum-verified-registry-archive | True | pass |
| [quick-xml](https://crates.io/api/v1/crates/quick-xml/0.42.0) | 0.42.0 | MIT | MIT | 2026-08-22T19:22:01.795943Z | 1; checksum-verified-registry-archive | True | pass |
| [quote](https://crates.io/api/v1/crates/quote/1.0.47) | 1.0.47 | MIT OR Apache-2.0 | MIT | 2026-07-19T00:16:57.083778Z | 2; checksum-verified-registry-archive | True | pass |
| [r-efi](https://crates.io/api/v1/crates/r-efi/5.3.0) | 5.3.0 | MIT OR Apache-2.0 OR LGPL-2.1-or-later | MIT | 2025-06-17T09:12:38.227277Z | 1; checksum-verified-registry-archive |  | pass |
| [r-efi](https://crates.io/api/v1/crates/r-efi/6.0.0) | 6.0.0 | MIT OR Apache-2.0 OR LGPL-2.1-or-later | MIT | 2026-02-03T09:53:46.251122Z | 1; checksum-verified-registry-archive |  | pass |
| [rand](https://crates.io/api/v1/crates/rand/0.10.3) | 0.10.3 | MIT OR Apache-2.0 | MIT | 2026-09-20T06:37:47.850624Z | 3; checksum-verified-registry-archive | True | pass |
| [rand_core](https://crates.io/api/v1/crates/rand_core/0.10.1) | 0.10.1 | MIT OR Apache-2.0 | MIT | 2026-04-13T15:26:43.049921Z | 3; checksum-verified-registry-archive | True | pass |
| [raw-window-handle](https://crates.io/api/v1/crates/raw-window-handle/0.6.2) | 0.6.2 | MIT OR Apache-2.0 OR Zlib | MIT | 2024-05-17T19:41:38.796080Z | 3; checksum-verified-registry-archive | True | pass |
| [read-fonts](https://crates.io/api/v1/crates/read-fonts/0.39.2) | 0.39.2 | MIT OR Apache-2.0 | MIT | 2026-04-22T19:17:22.303807Z | 2; checksum-verified-registry-archive | True | pass |
| [read-fonts](https://crates.io/api/v1/crates/read-fonts/0.41.0) | 0.41.0 | MIT OR Apache-2.0 | MIT | 2026-07-03T18:33:14.197122Z | 2; checksum-verified-registry-archive | True | pass |
| [redox_syscall](https://crates.io/api/v1/crates/redox_syscall/0.4.1) | 0.4.1 | MIT | MIT | 2023-09-08T17:14:57.408432Z | 1; checksum-verified-registry-archive |  | pass |
| [redox_syscall](https://crates.io/api/v1/crates/redox_syscall/0.5.18) | 0.5.18 | MIT | MIT | 2025-10-04T14:57:04.747959Z | 1; checksum-verified-registry-archive |  | pass |
| [redox_syscall](https://crates.io/api/v1/crates/redox_syscall/0.9.4) | 0.9.4 | MIT | MIT | 2026-09-05T08:52:15.805729Z | 1; checksum-verified-registry-archive |  | pass |
| [ref-cast](https://crates.io/api/v1/crates/ref-cast/1.0.27) | 1.0.27 | MIT OR Apache-2.0 | MIT | 2026-08-19T04:27:28.040119Z | 2; checksum-verified-registry-archive | True | pass |
| [ref-cast-impl](https://crates.io/api/v1/crates/ref-cast-impl/1.0.27) | 1.0.27 | MIT OR Apache-2.0 | MIT | 2026-08-19T04:27:24.118760Z | 2; checksum-verified-registry-archive | True | pass |
| [reqwest](https://crates.io/api/v1/crates/reqwest/0.13.5) | 0.13.5 | MIT OR Apache-2.0 | MIT | 2026-09-08T20:55:26.479356Z | 2; checksum-verified-registry-archive | True | pass |
| [rfd](https://crates.io/api/v1/crates/rfd/0.17.2) | 0.17.2 | MIT | MIT | 2026-01-12T20:57:19.255609Z | 2; checksum-verified-registry-archive | True | pass |
| [rmcp](https://crates.io/api/v1/crates/rmcp/3.5.0) | 3.5.0 | Apache-2.0 | Apache-2.0 | 2026-09-28T00:16:06.911025Z | 1; exact-upstream-commit | True | pass |
| [rmcp-macros](https://crates.io/api/v1/crates/rmcp-macros/3.5.0) | 3.5.0 | Apache-2.0 | Apache-2.0 | 2026-09-28T00:15:44.354500Z | 1; exact-upstream-commit | True | pass |
| [rust_xlsxwriter](https://crates.io/api/v1/crates/rust_xlsxwriter/0.99.1) | 0.99.1 | MIT OR Apache-2.0 | MIT | 2026-09-14T07:22:55.962652Z | 2; checksum-verified-registry-archive | True | pass |
| [rustc-hash](https://crates.io/api/v1/crates/rustc-hash/2.1.3) | 2.1.3 | Apache-2.0 OR MIT | MIT | 2026-07-02T19:13:03.150587Z | 2; checksum-verified-registry-archive | True | pass |
| [rustc_version](https://crates.io/api/v1/crates/rustc_version/0.4.1) | 0.4.1 | MIT OR Apache-2.0 | MIT | 2024-08-28T09:21:22.033328Z | 2; checksum-verified-registry-archive |  | pass |
| [rustix](https://crates.io/api/v1/crates/rustix/0.38.44) | 0.38.44 | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT | MIT | 2025-01-21T23:46:00.129474Z | 4; checksum-verified-registry-archive |  | pass |
| [rustix](https://crates.io/api/v1/crates/rustix/1.1.5) | 1.1.5 | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT | MIT | 2026-09-16T23:05:02.319690Z | 4; checksum-verified-registry-archive |  | pass |
| [rustversion](https://crates.io/api/v1/crates/rustversion/1.0.23) | 1.0.23 | MIT OR Apache-2.0 | MIT | 2026-07-07T02:10:26.015429Z | 2; checksum-verified-registry-archive | True | pass |
| [ryu](https://crates.io/api/v1/crates/ryu/1.0.23) | 1.0.23 | Apache-2.0 OR BSL-1.0 | Apache-2.0 | 2026-02-08T20:34:03.456199Z | 2; checksum-verified-registry-archive | True | pass |
| [same-file](https://crates.io/api/v1/crates/same-file/1.0.6) | 1.0.6 | Unlicense/MIT | MIT | 2020-01-11T17:02:30.960838Z | 2; checksum-verified-registry-archive |  | pass |
| [schemars](https://crates.io/api/v1/crates/schemars/1.2.2) | 1.2.2 | MIT | MIT | 2026-07-27T14:35:28.246709Z | 1; checksum-verified-registry-archive | True | pass |
| [schemars_derive](https://crates.io/api/v1/crates/schemars_derive/1.2.2) | 1.2.2 | MIT | MIT | 2026-07-27T14:35:25.339689Z | 1; checksum-verified-registry-archive | True | pass |
| [scoped-tls](https://crates.io/api/v1/crates/scoped-tls/1.0.1) | 1.0.1 | MIT/Apache-2.0 | MIT | 2022-10-31T15:01:56.394137Z | 2; checksum-verified-registry-archive |  | pass |
| [scopeguard](https://crates.io/api/v1/crates/scopeguard/1.2.0) | 1.2.0 | MIT OR Apache-2.0 | MIT | 2023-07-17T13:47:53.232324Z | 2; checksum-verified-registry-archive | True | pass |
| [self_cell](https://crates.io/api/v1/crates/self_cell/1.3.0) | 1.3.0 | Apache-2.0 OR GPL-2.0-only | Apache-2.0 | 2026-07-16T07:47:15.429408Z | 2; checksum-verified-registry-archive | True | pass |
| [semver](https://crates.io/api/v1/crates/semver/1.0.28) | 1.0.28 | MIT OR Apache-2.0 | MIT | 2026-04-04T00:25:14.379075Z | 2; checksum-verified-registry-archive |  | pass |
| [serde](https://crates.io/api/v1/crates/serde/1.0.229) | 1.0.229 | MIT OR Apache-2.0 | MIT | 2026-07-18T23:05:13.266456Z | 2; checksum-verified-registry-archive | True | pass |
| [serde_core](https://crates.io/api/v1/crates/serde_core/1.0.229) | 1.0.229 | MIT OR Apache-2.0 | MIT | 2026-07-18T23:05:10.896830Z | 2; checksum-verified-registry-archive | True | pass |
| [serde_derive](https://crates.io/api/v1/crates/serde_derive/1.0.229) | 1.0.229 | MIT OR Apache-2.0 | MIT | 2026-07-18T23:05:07.504375Z | 2; checksum-verified-registry-archive | True | pass |
| [serde_derive_internals](https://crates.io/api/v1/crates/serde_derive_internals/0.30.0) | 0.30.0 | MIT OR Apache-2.0 | MIT | 2026-07-25T14:58:00.367023Z | 2; checksum-verified-registry-archive | True | pass |
| [serde_json](https://crates.io/api/v1/crates/serde_json/1.0.151) | 1.0.151 | MIT OR Apache-2.0 | MIT | 2026-07-20T05:54:44.078083Z | 2; checksum-verified-registry-archive | True | pass |
| [serde_path_to_error](https://crates.io/api/v1/crates/serde_path_to_error/0.1.20) | 0.1.20 | MIT OR Apache-2.0 | MIT | 2025-09-15T15:05:54.817744Z | 2; checksum-verified-registry-archive | True | pass |
| [serde_repr](https://crates.io/api/v1/crates/serde_repr/0.1.21) | 0.1.21 | MIT OR Apache-2.0 | MIT | 2026-07-18T21:51:01.774317Z | 2; checksum-verified-registry-archive |  | pass |
| [serde_urlencoded](https://crates.io/api/v1/crates/serde_urlencoded/0.7.1) | 0.7.1 | MIT/Apache-2.0 | MIT | 2022-01-17T11:26:21.628923Z | 2; checksum-verified-registry-archive | True | pass |
| [shlex](https://crates.io/api/v1/crates/shlex/2.0.1) | 2.0.1 | MIT OR Apache-2.0 | MIT | 2026-05-17T18:56:12.775983Z | 2; checksum-verified-registry-archive |  | pass |
| [signal-hook-registry](https://crates.io/api/v1/crates/signal-hook-registry/1.4.8) | 1.4.8 | MIT OR Apache-2.0 | MIT | 2025-12-25T17:35:43.742246Z | 2; checksum-verified-registry-archive |  | pass |
| [simd-adler32](https://crates.io/api/v1/crates/simd-adler32/0.3.10) | 0.3.10 | MIT | MIT | 2026-07-14T17:42:45.047242Z | 1; checksum-verified-registry-archive | True | pass |
| [simd_cesu8](https://crates.io/api/v1/crates/simd_cesu8/1.2.0) | 1.2.0 | Apache-2.0 OR MIT | MIT | 2026-07-13T21:51:49.803302Z | 2; checksum-verified-registry-archive |  | pass |
| [simdutf8](https://crates.io/api/v1/crates/simdutf8/0.1.5) | 0.1.5 | MIT OR Apache-2.0 | MIT | 2024-09-22T09:18:22.610907Z | 2; checksum-verified-registry-archive | True | pass |
| [siphasher](https://crates.io/api/v1/crates/siphasher/1.0.4) | 1.0.4 | MIT OR Apache-2.0 | MIT | 2026-09-25T08:51:05.944322Z | 3; checksum-verified-registry-archive |  | pass |
| [skrifa](https://crates.io/api/v1/crates/skrifa/0.42.1) | 0.42.1 | MIT OR Apache-2.0 | MIT | 2026-04-22T19:17:42.287843Z | 2; checksum-verified-registry-archive | True | pass |
| [skrifa](https://crates.io/api/v1/crates/skrifa/0.44.0) | 0.44.0 | MIT OR Apache-2.0 | MIT | 2026-07-04T01:17:33.247998Z | 2; checksum-verified-registry-archive | True | pass |
| [slab](https://crates.io/api/v1/crates/slab/0.4.12) | 0.4.12 | MIT | MIT | 2026-01-31T12:59:22.609198Z | 1; checksum-verified-registry-archive | True | pass |
| [slotmap](https://crates.io/api/v1/crates/slotmap/1.1.1) | 1.1.1 | Zlib | Zlib | 2025-12-06T12:30:22.308199Z | 1; checksum-verified-registry-archive |  | pass |
| [smallvec](https://crates.io/api/v1/crates/smallvec/1.16.2) | 1.16.2 | MIT OR Apache-2.0 | MIT | 2026-09-25T05:54:44.970885Z | 2; checksum-verified-registry-archive | True | pass |
| [smithay-client-toolkit](https://crates.io/api/v1/crates/smithay-client-toolkit/0.19.2) | 0.19.2 | MIT | MIT | 2024-07-15T21:49:23.606060Z | 1; checksum-verified-registry-archive |  | pass |
| [smithay-client-toolkit](https://crates.io/api/v1/crates/smithay-client-toolkit/0.20.0) | 0.20.0 | MIT | MIT | 2025-08-08T20:51:28.245952Z | 1; checksum-verified-registry-archive |  | pass |
| [smithay-clipboard](https://crates.io/api/v1/crates/smithay-clipboard/0.7.3) | 0.7.3 | MIT | MIT | 2025-11-02T20:12:35.216592Z | 1; checksum-verified-registry-archive |  | pass |
| [smol_str](https://crates.io/api/v1/crates/smol_str/0.2.2) | 0.2.2 | MIT OR Apache-2.0 | MIT | 2024-05-14T10:24:31.374376Z | 2; checksum-verified-registry-archive | True | pass |
| [socket2](https://crates.io/api/v1/crates/socket2/0.6.5) | 0.6.5 | MIT OR Apache-2.0 | MIT | 2026-07-13T19:45:58.436336Z | 2; checksum-verified-registry-archive | True | pass |
| [sse-stream](https://crates.io/api/v1/crates/sse-stream/0.2.6) | 0.2.6 | MIT OR Apache-2.0 | MIT | 2026-09-03T14:01:17.516741Z | 2; checksum-verified-registry-archive | True | pass |
| [stable_deref_trait](https://crates.io/api/v1/crates/stable_deref_trait/1.2.1) | 1.2.1 | MIT OR Apache-2.0 | MIT | 2025-10-09T05:24:54.643083Z | 2; checksum-verified-registry-archive | True | pass |
| [static_assertions](https://crates.io/api/v1/crates/static_assertions/1.1.0) | 1.1.0 | MIT OR Apache-2.0 | MIT | 2019-11-03T17:02:11.618786Z | 2; checksum-verified-registry-archive | True | pass |
| [strsim](https://crates.io/api/v1/crates/strsim/0.11.1) | 0.11.1 | MIT | MIT | 2024-04-02T22:18:09.406790Z | 1; checksum-verified-registry-archive | True | pass |
| [subsetter](https://crates.io/api/v1/crates/subsetter/0.2.6) | 0.2.6 | MIT OR Apache-2.0 | MIT | 2026-06-04T11:19:53.251775Z | 3; checksum-verified-registry-archive | True | pass |
| [syn](https://crates.io/api/v1/crates/syn/2.0.119) | 2.0.119 | MIT OR Apache-2.0 | MIT | 2026-07-15T00:23:48.828241Z | 2; checksum-verified-registry-archive | True | pass |
| [syn](https://crates.io/api/v1/crates/syn/3.0.6) | 3.0.6 | MIT OR Apache-2.0 | MIT | 2026-09-16T22:31:12.314477Z | 2; checksum-verified-registry-archive | True | pass |
| [sync_wrapper](https://crates.io/api/v1/crates/sync_wrapper/1.0.2) | 1.0.2 | Apache-2.0 | Apache-2.0 | 2024-11-20T16:29:59.416129Z | 1; checksum-verified-registry-archive | True | pass |
| [synstructure](https://crates.io/api/v1/crates/synstructure/0.13.2) | 0.13.2 | MIT | MIT | 2025-04-30T16:37:46.407388Z | 1; checksum-verified-registry-archive | True | pass |
| [synstructure](https://crates.io/api/v1/crates/synstructure/0.14.0) | 0.14.0 | MIT | MIT | 2026-07-30T19:43:04.804397Z | 1; checksum-verified-registry-archive | True | pass |
| [tempfile](https://crates.io/api/v1/crates/tempfile/3.27.0) | 3.27.0 | MIT OR Apache-2.0 | MIT | 2026-03-11T00:20:04.812840Z | 2; checksum-verified-registry-archive |  | pass |
| [thiserror](https://crates.io/api/v1/crates/thiserror/1.0.69) | 1.0.69 | MIT OR Apache-2.0 | MIT | 2024-11-10T06:24:10.762680Z | 2; checksum-verified-registry-archive |  | pass |
| [thiserror](https://crates.io/api/v1/crates/thiserror/2.0.21) | 2.0.21 | MIT OR Apache-2.0 | MIT | 2026-09-23T20:24:07.771104Z | 2; checksum-verified-registry-archive | True | pass |
| [thiserror-impl](https://crates.io/api/v1/crates/thiserror-impl/1.0.69) | 1.0.69 | MIT OR Apache-2.0 | MIT | 2024-11-10T06:24:04.631987Z | 2; checksum-verified-registry-archive |  | pass |
| [thiserror-impl](https://crates.io/api/v1/crates/thiserror-impl/2.0.21) | 2.0.21 | MIT OR Apache-2.0 | MIT | 2026-09-23T20:24:05.038819Z | 2; checksum-verified-registry-archive | True | pass |
| [tiff](https://crates.io/api/v1/crates/tiff/0.11.3) | 0.11.3 | MIT | MIT | 2026-02-10T04:18:15.326938Z | 2; checksum-verified-registry-archive | True | pass |
| [tinystr](https://crates.io/api/v1/crates/tinystr/0.8.4) | 0.8.4 | Unicode-3.0 | Unicode-3.0 | 2026-08-13T23:28:07.485828Z | 1; checksum-verified-registry-archive | True | pass |
| [tokio](https://crates.io/api/v1/crates/tokio/1.53.1) | 1.53.1 | MIT | MIT | 2026-07-20T17:06:09.996426Z | 1; checksum-verified-registry-archive | True | pass |
| [tokio-macros](https://crates.io/api/v1/crates/tokio-macros/2.7.2) | 2.7.2 | MIT | MIT | 2026-07-29T13:15:27.042096Z | 1; checksum-verified-registry-archive | True | pass |
| [tokio-stream](https://crates.io/api/v1/crates/tokio-stream/0.1.19) | 0.1.19 | MIT | MIT | 2026-07-22T08:19:01.447272Z | 1; checksum-verified-registry-archive | True | pass |
| [tokio-util](https://crates.io/api/v1/crates/tokio-util/0.7.19) | 0.7.19 | MIT | MIT | 2026-07-21T12:10:46.915710Z | 1; checksum-verified-registry-archive | True | pass |
| [toml_datetime](https://crates.io/api/v1/crates/toml_datetime/1.1.1%2Bspec-1.1.0) | 1.1.1+spec-1.1.0 | MIT OR Apache-2.0 | MIT | 2026-03-31T19:06:07.594537Z | 2; checksum-verified-registry-archive | True | pass |
| [toml_edit](https://crates.io/api/v1/crates/toml_edit/0.25.15%2Bspec-1.1.0) | 0.25.15+spec-1.1.0 | MIT OR Apache-2.0 | MIT | 2026-09-11T01:44:08.521316Z | 2; checksum-verified-registry-archive | True | pass |
| [toml_parser](https://crates.io/api/v1/crates/toml_parser/1.1.3%2Bspec-1.1.0) | 1.1.3+spec-1.1.0 | MIT OR Apache-2.0 | MIT | 2026-07-27T14:53:02.089853Z | 2; checksum-verified-registry-archive | True | pass |
| [toml_writer](https://crates.io/api/v1/crates/toml_writer/1.1.2%2Bspec-1.1.0) | 1.1.2+spec-1.1.0 | MIT OR Apache-2.0 | MIT | 2026-07-14T13:09:46.208101Z | 2; checksum-verified-registry-archive | True | pass |
| [tower](https://crates.io/api/v1/crates/tower/0.5.3) | 0.5.3 | MIT | MIT | 2026-01-12T18:43:05.227461Z | 1; checksum-verified-registry-archive | True | pass |
| [tower-http](https://crates.io/api/v1/crates/tower-http/0.6.11) | 0.6.11 | MIT | MIT | 2026-05-18T12:51:03.996149Z | 1; checksum-verified-registry-archive | True | pass |
| [tower-layer](https://crates.io/api/v1/crates/tower-layer/0.3.3) | 0.3.3 | MIT | MIT | 2024-08-13T16:53:54.096608Z | 1; checksum-verified-registry-archive | True | pass |
| [tower-service](https://crates.io/api/v1/crates/tower-service/0.3.3) | 0.3.3 | MIT | MIT | 2024-08-13T16:23:33.548845Z | 1; checksum-verified-registry-archive | True | pass |
| [tracing](https://crates.io/api/v1/crates/tracing/0.1.44) | 0.1.44 | MIT | MIT | 2025-12-18T14:26:12.360139Z | 1; checksum-verified-registry-archive | True | pass |
| [tracing-attributes](https://crates.io/api/v1/crates/tracing-attributes/0.1.31) | 0.1.31 | MIT | MIT | 2025-11-26T11:23:01.809940Z | 1; checksum-verified-registry-archive | True | pass |
| [tracing-core](https://crates.io/api/v1/crates/tracing-core/0.1.36) | 0.1.36 | MIT | MIT | 2025-12-18T14:13:04.171045Z | 2; checksum-verified-registry-archive | True | pass |
| [try-lock](https://crates.io/api/v1/crates/try-lock/0.2.5) | 0.2.5 | MIT | MIT | 2023-12-07T16:13:16.960779Z | 1; checksum-verified-registry-archive | True | pass |
| [ttf-parser](https://crates.io/api/v1/crates/ttf-parser/0.25.1) | 0.25.1 | MIT OR Apache-2.0 | MIT | 2024-11-29T19:59:37.395141Z | 2; checksum-verified-registry-archive | True | pass |
| [typed-path](https://crates.io/api/v1/crates/typed-path/0.12.3) | 0.12.3 | MIT OR Apache-2.0 | MIT | 2026-02-11T02:30:37.067886Z | 2; checksum-verified-registry-archive | True | pass |
| [uds_windows](https://crates.io/api/v1/crates/uds_windows/1.2.1) | 1.2.1 | MIT | MIT | 2026-03-14T15:31:37.094401Z | 1; checksum-verified-registry-archive |  | pass |
| [unicode-general-category](https://crates.io/api/v1/crates/unicode-general-category/1.1.0) | 1.1.0 | Apache-2.0 | Apache-2.0 | 2025-09-16T00:44:45.093075Z | 1; checksum-verified-registry-archive | True | pass |
| [unicode-ident](https://crates.io/api/v1/crates/unicode-ident/1.0.26) | 1.0.26 | (MIT OR Apache-2.0) AND Unicode-3.0 | MIT AND Unicode-3.0 | 2026-09-17T06:18:03.751388Z | 3; checksum-verified-registry-archive | True | pass |
| [unicode-segmentation](https://crates.io/api/v1/crates/unicode-segmentation/1.13.3) | 1.13.3 | MIT OR Apache-2.0 | MIT | 2026-06-01T15:58:43.979474Z | 3; checksum-verified-registry-archive | True | pass |
| [url](https://crates.io/api/v1/crates/url/2.5.8) | 2.5.8 | MIT OR Apache-2.0 | MIT | 2026-01-05T22:14:31.414927Z | 2; checksum-verified-registry-archive | True | pass |
| [utf8_iter](https://crates.io/api/v1/crates/utf8_iter/1.0.4) | 1.0.4 | Apache-2.0 OR MIT | MIT | 2023-12-01T17:26:51.542538Z | 3; checksum-verified-registry-archive | True | pass |
| [uuid](https://crates.io/api/v1/crates/uuid/1.26.1) | 1.26.1 | Apache-2.0 OR MIT | MIT | 2026-09-10T00:30:44.938530Z | 2; checksum-verified-registry-archive | True | pass |
| [vello_common](https://crates.io/api/v1/crates/vello_common/0.1.0) | 0.1.0 | Apache-2.0 OR MIT | MIT | 2026-07-29T18:52:16.897032Z | 2; checksum-verified-registry-archive | True | pass |
| [vello_cpu](https://crates.io/api/v1/crates/vello_cpu/0.1.0) | 0.1.0 | Apache-2.0 OR MIT | MIT | 2026-07-29T18:52:23.045883Z | 2; checksum-verified-registry-archive | True | pass |
| [version_check](https://crates.io/api/v1/crates/version_check/0.9.5) | 0.9.5 | MIT/Apache-2.0 | MIT | 2024-07-25T23:38:14.142737Z | 2; checksum-verified-registry-archive | True | pass |
| [walkdir](https://crates.io/api/v1/crates/walkdir/2.5.0) | 2.5.0 | Unlicense/MIT | MIT | 2024-03-01T17:55:09.221032Z | 2; checksum-verified-registry-archive |  | pass |
| [want](https://crates.io/api/v1/crates/want/0.3.1) | 0.3.1 | MIT | MIT | 2023-06-14T21:50:52.862160Z | 1; checksum-verified-registry-archive | True | pass |
| [wasi](https://crates.io/api/v1/crates/wasi/0.11.1%2Bwasi-snapshot-preview1) | 0.11.1+wasi-snapshot-preview1 | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT | MIT | 2025-06-10T14:23:45.376914Z | 3; checksum-verified-registry-archive |  | pass |
| [wasip2](https://crates.io/api/v1/crates/wasip2/1.0.4%2Bwasi-0.2.12) | 1.0.4+wasi-0.2.12 | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT | MIT | 2026-06-12T17:42:43.116313Z | 3; checksum-verified-registry-archive |  | pass |
| [wasm-bindgen](https://crates.io/api/v1/crates/wasm-bindgen/0.2.129) | 0.2.129 | MIT OR Apache-2.0 | MIT | 2026-09-25T07:13:40.367031Z | 2; checksum-verified-registry-archive |  | pass |
| [wasm-bindgen-futures](https://crates.io/api/v1/crates/wasm-bindgen-futures/0.4.79) | 0.4.79 | MIT OR Apache-2.0 | MIT | 2026-09-25T07:13:46.762707Z | 2; checksum-verified-registry-archive |  | pass |
| [wasm-bindgen-macro](https://crates.io/api/v1/crates/wasm-bindgen-macro/0.2.129) | 0.2.129 | MIT OR Apache-2.0 | MIT | 2026-09-25T07:13:38.603121Z | 2; checksum-verified-registry-archive |  | pass |
| [wasm-bindgen-macro-support](https://crates.io/api/v1/crates/wasm-bindgen-macro-support/0.2.129) | 0.2.129 | MIT OR Apache-2.0 | MIT | 2026-09-25T07:13:36.815330Z | 2; checksum-verified-registry-archive |  | pass |
| [wasm-bindgen-shared](https://crates.io/api/v1/crates/wasm-bindgen-shared/0.2.129) | 0.2.129 | MIT OR Apache-2.0 | MIT | 2026-09-25T07:13:33.159722Z | 2; checksum-verified-registry-archive |  | pass |
| [wasm-streams](https://crates.io/api/v1/crates/wasm-streams/0.5.0) | 0.5.0 | MIT OR Apache-2.0 | MIT | 2026-01-22T08:14:59.195606Z | 2; checksum-verified-registry-archive |  | pass |
| [wayland-backend](https://crates.io/api/v1/crates/wayland-backend/0.3.17) | 0.3.17 | MIT | MIT | 2026-08-14T21:50:33.143344Z | 1; checksum-verified-registry-archive |  | pass |
| [wayland-client](https://crates.io/api/v1/crates/wayland-client/0.31.15) | 0.31.15 | MIT | MIT | 2026-07-22T20:35:54.669482Z | 1; checksum-verified-registry-archive |  | pass |
| [wayland-csd-frame](https://crates.io/api/v1/crates/wayland-csd-frame/0.3.0) | 0.3.0 | MIT | MIT | 2023-09-16T13:27:30.087305Z | 1; checksum-verified-registry-archive |  | pass |
| [wayland-cursor](https://crates.io/api/v1/crates/wayland-cursor/0.31.14) | 0.31.14 | MIT | MIT | 2026-03-31T02:22:00.214711Z | 1; checksum-verified-registry-archive |  | pass |
| [wayland-protocols](https://crates.io/api/v1/crates/wayland-protocols/0.32.13) | 0.32.13 | MIT | MIT | 2026-06-19T15:32:40.359346Z | 2; checksum-verified-registry-archive |  | pass |
| [wayland-protocols-experimental](https://crates.io/api/v1/crates/wayland-protocols-experimental/20250721.0.1) | 20250721.0.1 | MIT | MIT | 2025-07-29T14:45:13.294296Z | 1; checksum-verified-registry-archive |  | pass |
| [wayland-protocols-misc](https://crates.io/api/v1/crates/wayland-protocols-misc/0.3.12) | 0.3.12 | MIT | MIT | 2026-03-31T02:22:31.310136Z | 1; checksum-verified-registry-archive |  | pass |
| [wayland-protocols-plasma](https://crates.io/api/v1/crates/wayland-protocols-plasma/0.3.12) | 0.3.12 | MIT | MIT | 2026-03-31T02:22:39.323045Z | 2; checksum-verified-registry-archive |  | pass |
| [wayland-protocols-wlr](https://crates.io/api/v1/crates/wayland-protocols-wlr/0.3.12) | 0.3.12 | MIT | MIT | 2026-03-31T02:22:47.529355Z | 1; checksum-verified-registry-archive |  | pass |
| [wayland-scanner](https://crates.io/api/v1/crates/wayland-scanner/0.31.11) | 0.31.11 | MIT | MIT | 2026-07-22T20:35:37.798276Z | 1; checksum-verified-registry-archive |  | pass |
| [wayland-sys](https://crates.io/api/v1/crates/wayland-sys/0.31.11) | 0.31.11 | MIT | MIT | 2026-03-31T02:20:48.724050Z | 1; checksum-verified-registry-archive |  | pass |
| [web-sys](https://crates.io/api/v1/crates/web-sys/0.3.106) | 0.3.106 | MIT OR Apache-2.0 | MIT | 2026-09-25T07:13:44.661972Z | 2; checksum-verified-registry-archive |  | pass |
| [web-time](https://crates.io/api/v1/crates/web-time/1.1.0) | 1.1.0 | MIT OR Apache-2.0 | MIT | 2024-03-01T10:51:46.373917Z | 2; checksum-verified-registry-archive | True | pass |
| [weezl](https://crates.io/api/v1/crates/weezl/0.1.12) | 0.1.12 | MIT OR Apache-2.0 | MIT | 2025-11-11T20:13:29.888463Z | 2; checksum-verified-registry-archive | True | pass |
| [winapi-util](https://crates.io/api/v1/crates/winapi-util/0.1.11) | 0.1.11 | Unlicense OR MIT | MIT | 2025-09-07T12:24:31.232695Z | 2; checksum-verified-registry-archive |  | pass |
| [windows](https://crates.io/api/v1/crates/windows/0.62.2) | 0.62.2 | MIT OR Apache-2.0 | MIT | 2025-10-06T19:19:49.704461Z | 2; checksum-verified-registry-archive | True | pass |
| [windows-collections](https://crates.io/api/v1/crates/windows-collections/0.3.2) | 0.3.2 | MIT OR Apache-2.0 | MIT | 2025-10-06T19:16:09.553894Z | 2; checksum-verified-registry-archive | True | pass |
| [windows-core](https://crates.io/api/v1/crates/windows-core/0.62.2) | 0.62.2 | MIT OR Apache-2.0 | MIT | 2025-10-06T19:15:14.207850Z | 2; checksum-verified-registry-archive | True | pass |
| [windows-future](https://crates.io/api/v1/crates/windows-future/0.3.2) | 0.3.2 | MIT OR Apache-2.0 | MIT | 2025-10-06T19:15:33.355015Z | 2; checksum-verified-registry-archive | True | pass |
| [windows-implement](https://crates.io/api/v1/crates/windows-implement/0.60.2) | 0.60.2 | MIT OR Apache-2.0 | MIT | 2025-10-06T19:13:31.467507Z | 2; checksum-verified-registry-archive | True | pass |
| [windows-interface](https://crates.io/api/v1/crates/windows-interface/0.59.3) | 0.59.3 | MIT OR Apache-2.0 | MIT | 2025-10-06T19:14:18.053188Z | 2; checksum-verified-registry-archive | True | pass |
| [windows-link](https://crates.io/api/v1/crates/windows-link/0.2.1) | 0.2.1 | MIT OR Apache-2.0 | MIT | 2025-10-06T17:35:08.351469Z | 2; checksum-verified-registry-archive | True | pass |
| [windows-numerics](https://crates.io/api/v1/crates/windows-numerics/0.3.1) | 0.3.1 | MIT OR Apache-2.0 | MIT | 2025-10-06T19:17:19.909774Z | 2; checksum-verified-registry-archive | True | pass |
| [windows-result](https://crates.io/api/v1/crates/windows-result/0.4.1) | 0.4.1 | MIT OR Apache-2.0 | MIT | 2025-10-06T19:14:39.441465Z | 2; checksum-verified-registry-archive | True | pass |
| [windows-strings](https://crates.io/api/v1/crates/windows-strings/0.5.1) | 0.5.1 | MIT OR Apache-2.0 | MIT | 2025-10-06T19:15:01.410695Z | 2; checksum-verified-registry-archive | True | pass |
| [windows-sys](https://crates.io/api/v1/crates/windows-sys/0.52.0) | 0.52.0 | MIT OR Apache-2.0 | MIT | 2023-11-15T18:55:55.748059Z | 2; checksum-verified-registry-archive | True | pass |
| [windows-sys](https://crates.io/api/v1/crates/windows-sys/0.61.2) | 0.61.2 | MIT OR Apache-2.0 | MIT | 2025-10-06T19:18:30.163595Z | 2; checksum-verified-registry-archive | True | pass |
| [windows-targets](https://crates.io/api/v1/crates/windows-targets/0.52.6) | 0.52.6 | MIT OR Apache-2.0 | MIT | 2024-07-03T19:44:18.100049Z | 2; checksum-verified-registry-archive | True | pass |
| [windows-threading](https://crates.io/api/v1/crates/windows-threading/0.2.1) | 0.2.1 | MIT OR Apache-2.0 | MIT | 2025-10-06T17:35:22.550255Z | 2; checksum-verified-registry-archive | True | pass |
| [windows_aarch64_gnullvm](https://crates.io/api/v1/crates/windows_aarch64_gnullvm/0.52.6) | 0.52.6 | MIT OR Apache-2.0 | MIT | 2024-07-03T19:43:36.557711Z | 2; checksum-verified-registry-archive |  | pass |
| [windows_aarch64_msvc](https://crates.io/api/v1/crates/windows_aarch64_msvc/0.52.6) | 0.52.6 | MIT OR Apache-2.0 | MIT | 2024-07-03T19:43:40.566034Z | 2; checksum-verified-registry-archive |  | pass |
| [windows_i686_gnu](https://crates.io/api/v1/crates/windows_i686_gnu/0.52.6) | 0.52.6 | MIT OR Apache-2.0 | MIT | 2024-07-03T19:43:46.438870Z | 2; checksum-verified-registry-archive |  | pass |
| [windows_i686_gnullvm](https://crates.io/api/v1/crates/windows_i686_gnullvm/0.52.6) | 0.52.6 | MIT OR Apache-2.0 | MIT | 2024-07-03T19:43:50.092560Z | 2; checksum-verified-registry-archive |  | pass |
| [windows_i686_msvc](https://crates.io/api/v1/crates/windows_i686_msvc/0.52.6) | 0.52.6 | MIT OR Apache-2.0 | MIT | 2024-07-03T19:43:54.897517Z | 2; checksum-verified-registry-archive |  | pass |
| [windows_x86_64_gnu](https://crates.io/api/v1/crates/windows_x86_64_gnu/0.52.6) | 0.52.6 | MIT OR Apache-2.0 | MIT | 2024-07-03T19:43:58.189149Z | 2; checksum-verified-registry-archive |  | pass |
| [windows_x86_64_gnullvm](https://crates.io/api/v1/crates/windows_x86_64_gnullvm/0.52.6) | 0.52.6 | MIT OR Apache-2.0 | MIT | 2024-07-03T19:44:03.184321Z | 2; checksum-verified-registry-archive |  | pass |
| [windows_x86_64_msvc](https://crates.io/api/v1/crates/windows_x86_64_msvc/0.52.6) | 0.52.6 | MIT OR Apache-2.0 | MIT | 2024-07-03T19:44:07.225369Z | 2; checksum-verified-registry-archive | True | pass |
| [winit](https://crates.io/api/v1/crates/winit/0.30.13) | 0.30.13 | Apache-2.0 | Apache-2.0 | 2026-03-02T13:50:47.664884Z | 1; checksum-verified-registry-archive | True | pass |
| [winnow](https://crates.io/api/v1/crates/winnow/1.0.4) | 1.0.4 | MIT | MIT | 2026-07-13T14:31:41.405838Z | 1; checksum-verified-registry-archive | True | pass |
| [wit-bindgen](https://crates.io/api/v1/crates/wit-bindgen/0.57.1) | 0.57.1 | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT | MIT | 2026-04-17T17:49:55.029832Z | 3; checksum-verified-registry-archive |  | pass |
| [write-fonts](https://crates.io/api/v1/crates/write-fonts/0.48.1) | 0.48.1 | MIT OR Apache-2.0 | MIT | 2026-04-23T17:20:22.233903Z | 2; checksum-verified-registry-archive | True | pass |
| [writeable](https://crates.io/api/v1/crates/writeable/0.6.4) | 0.6.4 | Unicode-3.0 | Unicode-3.0 | 2026-08-13T23:28:09.637125Z | 1; checksum-verified-registry-archive | True | pass |
| [x11-dl](https://crates.io/api/v1/crates/x11-dl/2.21.0) | 2.21.0 | MIT | MIT | 2023-01-18T20:43:48.234556Z | 1; checksum-verified-registry-archive |  | pass |
| [x11rb](https://crates.io/api/v1/crates/x11rb/0.13.2) | 0.13.2 | MIT OR Apache-2.0 | MIT | 2025-08-29T16:34:47.956210Z | 2; checksum-verified-registry-archive |  | pass |
| [x11rb-protocol](https://crates.io/api/v1/crates/x11rb-protocol/0.13.2) | 0.13.2 | MIT OR Apache-2.0 | MIT | 2025-08-29T16:34:14.895086Z | 2; checksum-verified-registry-archive |  | pass |
| [xcursor](https://crates.io/api/v1/crates/xcursor/0.3.11) | 0.3.11 | MIT | MIT | 2026-07-31T11:53:43.571238Z | 1; checksum-verified-registry-archive |  | pass |
| [xkbcommon-dl](https://crates.io/api/v1/crates/xkbcommon-dl/0.4.2) | 0.4.2 | MIT | MIT | 2024-02-11T16:59:18.037924Z | 1; checksum-verified-registry-archive |  | pass |
| [xkeysym](https://crates.io/api/v1/crates/xkeysym/0.2.1) | 0.2.1 | MIT OR Apache-2.0 OR Zlib | MIT | 2024-06-07T01:37:51.789727Z | 3; checksum-verified-registry-archive |  | pass |
| [xml-rs](https://crates.io/api/v1/crates/xml-rs/0.8.29) | 0.8.29 | MIT | MIT | 2026-08-06T16:08:01.866097Z | 1; checksum-verified-registry-archive | True | pass |
| [yoke](https://crates.io/api/v1/crates/yoke/0.8.3) | 0.8.3 | Unicode-3.0 | Unicode-3.0 | 2026-06-03T20:59:17.678438Z | 1; checksum-verified-registry-archive | True | pass |
| [yoke-derive](https://crates.io/api/v1/crates/yoke-derive/0.8.2) | 0.8.2 | Unicode-3.0 | Unicode-3.0 | 2026-04-01T16:46:42.411874Z | 1; checksum-verified-registry-archive | True | pass |
| [zbus](https://crates.io/api/v1/crates/zbus/5.19.0) | 5.19.0 | MIT | MIT | 2026-08-09T17:48:33.130343Z | 1; checksum-verified-registry-archive |  | pass |
| [zbus-lockstep](https://crates.io/api/v1/crates/zbus-lockstep/0.5.2) | 0.5.2 | MIT | MIT | 2025-11-29T16:54:49.988824Z | 1; checksum-verified-registry-archive |  | pass |
| [zbus-lockstep-macros](https://crates.io/api/v1/crates/zbus-lockstep-macros/0.5.2) | 0.5.2 | MIT | MIT | 2025-11-29T16:54:54.749209Z | 1; checksum-verified-registry-archive |  | pass |
| [zbus_macros](https://crates.io/api/v1/crates/zbus_macros/5.19.0) | 5.19.0 | MIT | MIT | 2026-08-09T17:48:14.994171Z | 1; checksum-verified-registry-archive |  | pass |
| [zbus_names](https://crates.io/api/v1/crates/zbus_names/4.3.4) | 4.3.4 | MIT | MIT | 2026-07-17T20:50:13.159832Z | 1; checksum-verified-registry-archive |  | pass |
| [zbus_xml](https://crates.io/api/v1/crates/zbus_xml/5.2.1) | 5.2.1 | MIT | MIT | 2026-07-17T20:50:50.345061Z | 1; checksum-verified-registry-archive |  | pass |
| [zcheapstr](https://crates.io/api/v1/crates/zcheapstr/1.1.0) | 1.1.0 | MIT | MIT | 2026-08-10T12:59:25.547866Z | 1; checksum-verified-registry-archive |  | pass |
| [zerocopy](https://crates.io/api/v1/crates/zerocopy/0.8.59) | 0.8.59 | BSD-2-Clause OR Apache-2.0 OR MIT | MIT | 2026-09-25T17:03:33.335429Z | 3; checksum-verified-registry-archive | True | pass |
| [zerocopy-derive](https://crates.io/api/v1/crates/zerocopy-derive/0.8.59) | 0.8.59 | BSD-2-Clause OR Apache-2.0 OR MIT | MIT | 2026-09-25T17:02:26.306915Z | 3; checksum-verified-registry-archive | True | pass |
| [zerofrom](https://crates.io/api/v1/crates/zerofrom/0.1.8) | 0.1.8 | Unicode-3.0 | Unicode-3.0 | 2026-05-12T09:08:18.380489Z | 1; checksum-verified-registry-archive | True | pass |
| [zerofrom-derive](https://crates.io/api/v1/crates/zerofrom-derive/0.1.8) | 0.1.8 | Unicode-3.0 | Unicode-3.0 | 2026-09-15T21:39:20.888453Z | 1; checksum-verified-registry-archive | True | pass |
| [zerotrie](https://crates.io/api/v1/crates/zerotrie/0.2.5) | 0.2.5 | Unicode-3.0 | Unicode-3.0 | 2026-08-13T23:28:19.996564Z | 1; checksum-verified-registry-archive | True | pass |
| [zerovec](https://crates.io/api/v1/crates/zerovec/0.11.8) | 0.11.8 | Unicode-3.0 | Unicode-3.0 | 2026-08-19T22:32:16.933982Z | 1; checksum-verified-registry-archive | True | pass |
| [zerovec-derive](https://crates.io/api/v1/crates/zerovec-derive/0.11.6) | 0.11.6 | Unicode-3.0 | Unicode-3.0 | 2026-08-20T23:25:28.777419Z | 1; checksum-verified-registry-archive | True | pass |
| [zip](https://crates.io/api/v1/crates/zip/8.6.0) | 8.6.0 | MIT | MIT | 2026-04-25T19:11:11.854239Z | 1; checksum-verified-registry-archive | True | pass |
| [zlib-rs](https://crates.io/api/v1/crates/zlib-rs/0.6.8) | 0.6.8 | Zlib | Zlib | 2026-09-15T13:10:00.283903Z | 1; checksum-verified-registry-archive | True | pass |
| [zmij](https://crates.io/api/v1/crates/zmij/1.0.23) | 1.0.23 | MIT | MIT | 2026-07-13T12:58:53.705886Z | 1; checksum-verified-registry-archive | True | pass |
| [zopfli](https://crates.io/api/v1/crates/zopfli/0.8.3) | 0.8.3 | Apache-2.0 | Apache-2.0 | 2025-10-30T14:31:25.858107Z | 1; checksum-verified-registry-archive | True | pass |
| [zune-core](https://crates.io/api/v1/crates/zune-core/0.5.3) | 0.5.3 | MIT OR Apache-2.0 OR Zlib | MIT | 2026-08-07T20:55:44.361773Z | 3; checksum-verified-registry-archive | True | pass |
| [zune-jpeg](https://crates.io/api/v1/crates/zune-jpeg/0.5.15) | 0.5.15 | MIT OR Apache-2.0 OR Zlib | MIT | 2026-03-26T21:57:53.722533Z | 3; checksum-verified-registry-archive | True | pass |
| [zvariant](https://crates.io/api/v1/crates/zvariant/5.15.0) | 5.15.0 | MIT | MIT | 2026-08-18T11:46:20.343901Z | 1; checksum-verified-registry-archive |  | pass |
| [zvariant_derive](https://crates.io/api/v1/crates/zvariant_derive/5.15.0) | 5.15.0 | MIT | MIT | 2026-08-18T11:46:04.962442Z | 1; checksum-verified-registry-archive |  | pass |
| [zvariant_utils](https://crates.io/api/v1/crates/zvariant_utils/4.2.0) | 4.2.0 | MIT | MIT | 2026-08-18T11:45:57.255393Z | 1; checksum-verified-registry-archive |  | pass |

## Assets, toolchain и остальные компоненты

- Системные Windows Arial/Segoe загружаются из ОС и не входят в installer. PDF subsets нуждаются в embedding flags/rights audit системного шрифта; это отдельная проверка.
- Inter 4.1 / JetBrains Mono 2.304 (OFL-1.1), Lucide 1.48.0 (ISC) находятся в source assets. Их прежние exact SHA/notices — в assets/manifests; текущие Rust exe не встраивают эти fonts/icons.
- Loro CRDT spike изолирован: docs/dependencies/crdt-spike.{md,json}. Он не включён в production Cargo.lock, и его notices не подменяют этот граф.
- Rust 1.98.1, Microsoft Windows SDK/MSVC CRT и NSIS — build/platform components. Для static CRT installer применяются отдельно подтверждённые redistribution terms; этот Cargo реестр не является их аудитом.
- CI actions и NSIS script проверяются отдельным release review. AGPL запрещён. Age/yank/prerelease/checksum gate применяется ко всему lock; license/source-text gate — к Windows target graph. Non-Windows source gaps сохраняются честно в audit report.
