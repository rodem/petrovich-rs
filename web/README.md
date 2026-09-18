# petrovich-web

Minimal dependency-free HTTP service over `petrovich-core`, aimed at
1C / VBScript / `MSXML2.XMLHTTP` clients that prefer `text/plain` over JSON.

```sh
cargo run -p petrovich-web -- --bind 127.0.0.1:8080
```

## Хост и порт

```sh
petrovich-web --bind 127.0.0.1:8080   # по умолчанию: только локальные подключения
petrovich-web --bind 127.0.0.1:51999  # любой свободный порт 1–65535
petrovich-web --bind 0.0.0.0:8080     # слушать все интерфейсы (для 1С/VBS с другой машины)
```

Если порт занят, сервер сразу завершается с ошибкой `cannot bind …` (код 1).
При доступе извне не забудьте правило firewall на выбранный порт.

## Routes (all `GET`, UTF-8)

| Route | Example | Response |
|---|---|---|
| `/health` | `/health` | `ok` |
| `/decline` | `/decline?lastname=Иванов&firstname=Иван&case=dative&sex=male` | `Иванову Ивану` (`text/plain`) |
| `/gender` | `/gender?firstname=Саша` | `male\|female\|androgynous` |
| `/cases` | `/cases?firstname=Иван&sex=male` | 6 cases, tab-separated, nominative…prepositional |
| `/api/v1/decline` | same params as `/decline` | `{"lastname":"…","firstname":"…","middlename":"…"}` (`application/json`) |
| `/api/v1/gender` | same params as `/gender` | `{"gender":"…"}` |

Params:

- `case`: `nominative|genitive|dative|accusative|instrumental|prepositional`
  or `1`–`6` (padeg-style; also `им|рд|дт|вн|тв|пр`). Default: nominative
  (identity) for `/decline`; omitted for `/cases` (returns all six anyway).
- `sex`: `male|female|auto` (also `m|f|м|ж`, empty = `auto` — detected
  from the given parts).

Errors are `400 text/plain` with a message; unknown routes are `404`.
At least one of `lastname|firstname|middlename` is required.

## 1C example

```bsl
HTTP = Новый HTTPСоединение("127.0.0.1", 8080);
Запрос = Новый HTTPЗапрос("/decline?lastname=Иванов&firstname=Иван&case=3&sex=male");
Ответ = HTTP.Получить(Запрос);
// Ответ.ПолучитьТелоКакСтроку() -> "Иванову Ивану"
```

(URL-encode Cyrillic params; `case=3` is dative.)

## MSXML2.XMLHTTP example (VBScript)

```vbscript
' cscript decline.vbs — склонение через petrovich-web
Option Explicit

Function UrlEncodeUtf8(s)
    ' Минимальное percent-кодирование для кириллицы в UTF-8.
    Dim i, code, b1, b2, out
    out = ""
    For i = 1 To Len(s)
        code = AscW(Mid(s, i, 1))
        If (code >= 48 And code <= 57) Or (code >= 65 And code <= 90) _
                Or (code >= 97 And code <= 122) Then
            out = out & Mid(s, i, 1)
        ElseIf code < 128 Then
            out = out & "%" & Right("0" & Hex(code), 2)
        Else
            ' U+0400..U+07FF -> два байта UTF-8 (покрывает кириллицу)
            b1 = &HC0 Or ((code \ 64) And &H1F)
            b2 = &H80 Or (code And &H3F)
            out = out & "%" & Hex(b1) & "%" & Hex(b2)
        End If
    Next
    UrlEncodeUtf8 = out
End Function

Function Decline(hostPort, lastName, firstName, padeg)
    Dim http, url
    url = "http://" & hostPort & "/decline" _
        & "?lastname=" & UrlEncodeUtf8(lastName) _
        & "&firstname=" & UrlEncodeUtf8(firstName) _
        & "&case=" & padeg & "&sex=auto"
    Set http = CreateObject("MSXML2.XMLHTTP")
    http.open "GET", url, False
    http.send
    If http.Status = 200 Then
        Decline = http.responseText  ' UTF-8: сервер отдаёт charset=utf-8
    Else
        Err.Raise 1, "petrovich-web", "HTTP " & http.Status & ": " & http.responseText
    End If
End Function

WScript.Echo Decline("127.0.0.1:8080", "Иванов", "Иван", "3")
' -> Иванову Ивану
```

Замечания:

- `MSXML2.XMLHTTP` декодирует `responseText` по `charset` из `Content-Type` —
  сервер всегда отдаёт `charset=utf-8`, отдельно перекодировать не нужно.
- Для построчного склонения файла удобнее CLI batch-режим
  (`petrovich decline --batch`), а не по одному HTTP-запросу на строку.
- Альтернатива без COM: `WinHTTP.WinHTTPRequest.5.1` — тот же `open/send`
  интерфейс, но `responseText` там всегда в UTF-16 без учёта `charset`;
  кириллицу надёжнее читать через `responseBody` + `ADODB.Stream` с
  `Charset = "utf-8"`.

## Docker

```sh
docker build -f web/Dockerfile -t petrovich-web .
docker run --rm -p 8080:8080 petrovich-web
```
