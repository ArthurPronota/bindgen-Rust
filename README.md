# Разбор проекта с `bindgen`: C → Rust

У вас получился **канонический пример** автоматической генерации Rust-привязок из C-заголовков. Разберём всё по порядку.

---

## Что такое `bindgen`

**`bindgen`** — инструмент, который читает **C/C++ заголовочный файл** (`.h`) и **автоматически генерирует** Rust-код с объявлениями функций, структур, констант, типов.

Направление преобразования:

```
C header (.h)  ──►  bindgen  ──►  Rust bindings (.rs)
```

Это избавляет от ручного написания `extern "C"`-объявлений и снижает риск ошибок в сигнатурах.

---

## Структура проекта

```
o_070_binding/
├── Cargo.toml
├── build.rs                    ← сборочный скрипт
├── native/
│   ├── add.c                   ← реализация на C
│   └── wrapper.h               ← заголовок для bindgen
└── src/
    └── main.rs                 ← Rust-код, использующий привязки
```

---

## Разбор `Cargo.toml`

```toml
[package]
name = "o_070_binding"
version = "0.1.0"
edition = "2024"

[dependencies]

[build-dependencies]
bindgen = "0.73.2"
cc = "1.6.0"
```

Ключевое — **`[build-dependencies]`**, а не `[dependencies]`:

| Секция | Когда используется | Для чего |
|---|---|---|
| `[dependencies]` | во время компиляции крейта | библиотеки, которые нужны коду |
| `[build-dependencies]` | во время выполнения `build.rs` | инструменты сборки (cc, bindgen) |

`cc` и `bindgen` **не попадают** в финальный бинарник — они нужны только на этапе сборки.

---

## Разбор `build.rs`

```rust
use std::{env, error::Error, path::PathBuf};
use cc;

fn main() -> Result<(), Box<dyn Error>> {
    // 1. Компиляция C в статическую библиотеку
    cc::Build::new()
        .file("native/add.c")
        .compile("add");

    // 2. Генерация Rust-привязок из заголовка
    let my_bind = bindgen::Builder::default()
        .header("native/wrapper.h")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .generate()?;

    // 3. Запись привязок в OUT_DIR/bindings.rs
    let out_path = PathBuf::from(env::var("OUT_DIR")?);

    my_bind.write_to_file(out_path.join("bindings.rs"))?;

    Ok(())
}
```

### Шаг 1: `cc::Build` — компиляция C

```rust
cc::Build::new()
    .file("native/add.c")
    .compile("add");
```

- `cc::Build::new()` — создаёт билдер C-компилятора;
- `.file("native/add.c")` — добавляет исходник;
- `.compile("add")` — компилирует в **статическую библиотеку** `add.lib` (Windows) или `libadd.a` (Linux/macOS);
- `cc` автоматически сообщает Cargo: `cargo:rustc-link-lib=static=add`, `cargo:rustc-link-search=...`.

### Шаг 2: `bindgen::Builder` — генерация привязок

```rust
bindgen::Builder::default()
    .header("native/wrapper.h")
    .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
    .generate()?
```

| Метод | Назначение |
|---|---|
| `Builder::default()` | создаёт билдер с настройками по умолчанию |
| `.header("native/wrapper.h")` | задаёт заголовок для парсинга |
| `.parse_callbacks(...)` | настраивает поведение при парсинге |
| `.generate()` | запускает генерацию, возвращает `Result<Bindings>` |

**`CargoCallbacks`** — специальный callback, который:
- сообщает Cargo о зависимостях от заголовков (`cargo:rerun-if-changed=...`);
- при изменении `.h` пересборка запускается автоматически.

Без него Cargo не знал бы, что `wrapper.h` влияет на сборку.

### Шаг 3: запись результата

```rust
let out_path = PathBuf::from(env::var("OUT_DIR")?);
my_bind.write_to_file(out_path.join("bindings.rs"))?;
```

- **`OUT_DIR`** — переменная окружения, которую Cargo устанавливает для каждого крейта. Это временная папка вида `target/debug/build/<pkg>-<hash>/out/`.
- `bindings.rs` — сгенерированный файл с привязками.

### Почему `Result` вместо `panic!`

```rust
fn main() -> Result<(), Box<dyn Error>>
```

- ошибки передаются через `?`;
- Cargo покажет чистое сообщение без backtrace;
- нет шумного `thread 'main' panicked at ...`.

---

## Разбор C-кода

**`native/add.c`:**

```c
int add(int a, int b) {
    return a + b;
}
```

**`native/wrapper.h`:**

```c
int add(int a, int b);
```

Зачем нужен `wrapper.h`, если есть `add.c`?

`bindgen` **не парсит `.c`** — он работает только с **заголовками** (`.h`). Заголовок — это «публичный интерфейс» C-модуля. `add.c` — реализация.

Часто `wrapper.h` — не просто копия объявления, а **сборный заголовок**, который включает несколько `.h` и настраивает макросы:

```c
// wrapper.h
#define SOME_FEATURE 1
#include "add.h"
#include "other.h"
#include "third_party/library.h"
```

В вашем случае — минимальный пример.

---

## Разбор `main.rs`

```rust
include!(
    concat!(
        env!("OUT_DIR"),
        "/bindings.rs"
    )
);

fn main() {
    let x = 10;
    let y = 20;
    unsafe {
        println!("{x} + {y} = {}", add(x, y));
    }
}
```

### `include!` — вставка файла

`include!` — макрос, который **вставляет содержимое файла** в код на этапе компиляции:

```rust
include!("path/to/file.rs");
```

Здесь путь формируется динамически:

```rust
concat!(
    env!("OUT_DIR"),     // "/path/to/target/debug/build/.../out"
    "/bindings.rs"       // → "/path/to/.../out/bindings.rs"
)
```

- `env!("OUT_DIR")` — макрос, подставляющий **значение переменной окружения** как строковый литерал (на этапе компиляции);
- `concat!` — склеивает литералы в один;
- `include!` — вставляет файл.

Итог: сгенерированный `bindings.rs` как будто написан прямо в `main.rs`.

### Почему `unsafe`

`bindgen` генерирует объявления с `extern "C"`:

```rust
unsafe extern "C" {
    pub fn add(a: c_int, b: c_int) -> c_int;
}
```

В edition 2024 вызов внешней функции требует `unsafe` — компилятор не может проверить её корректность.

---

## Что генерирует `bindgen`

Для вашего `wrapper.h` `bindings.rs` будет выглядеть примерно так:

```rust
/* automatically generated by rust-bindgen 0.73.2 */

pub type __int32_t = ::std::os::raw::c_int;

extern "C" {
    pub fn add(a: ::std::os::raw::c_int, b: ::std::os::raw::c_int) -> ::std::os::raw::c_int;
}
```

Или (в зависимости от версии и настроек):

```rust
unsafe extern "C" {
    pub fn add(a: c_int, b: c_int) -> c_int;
}
```

Здесь:
- `c_int` = `i32` на большинстве платформ;
- функция объявлена как `extern "C"` — C ABI;
- имя `add` без манглинга (bindgen сам добавляет нужные атрибуты).

---

## Полный цикл сборки

```
1. cargo build
        │
        ▼
2. Cargo компилирует build.rs (с build-dependencies)
        │
        ▼
3. build.rs выполняется:
   ├── cc компилирует native/add.c → libadd.a
   └── bindgen парсит native/wrapper.h → bindings.rs (в OUT_DIR)
        │
        ▼
4. Cargo компилирует src/main.rs
   └── include! вставляет bindings.rs
        │
        ▼
5. Линкер соединяет main.o + libadd.a → бинарник
        │
        ▼
6. ./target/debug/o_070_binding
   → 10 + 20 = 30
```

---

## Преимущества `bindgen`

| Без bindgen | С bindgen |
|---|---|
| Вручную писать `extern "C"` | Автоматическая генерация |
| Ошибки в сигнатурах | Точное соответствие `.h` |
| Нужно знать типы C | Автоматический маппинг (`int` → `c_int`) |
| Сложно для больших API | Масштабируется на тысячи функций |
| Забыли обновить при изменении `.h` | Пересборка автоматически |

---

## Частые настройки `bindgen`

```rust
bindgen::Builder::default()
    .header("native/wrapper.h")
    .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
    // белый список — генерировать только эти символы
    .allowlist_function("add")
    .allowlist_type("Point")
    // запретить генерацию для ненужного
    .blocklist_function("internal_.*")
    // стиль: типы, константы
    .rustified_enum("MyEnum")
    // путь к clang
    .clang_arg("-I/usr/include")
    .clang_arg("-DFEATURE=1")
    .generate()?;
```

Полезные методы:

| Метод | Назначение |
|---|---|
| `.allowlist_function(...)` | генерировать только указанные функции |
| `.blocklist_function(...)` | исключить функции |
| `.allowlist_type(...)` | только указанные типы |
| `.rustified_enum(...)` | enum как Rust-enum |
| `.clang_arg(...)` | передать флаг clang (`-I`, `-D`) |
| `.layout_tests(true)` | генерировать тесты layout |
| `.derive_default(true)` | добавлять `#[derive(Default)]` |

---

## Требования

- **`libclang`** должен быть установлен и доступен (см. ваш предыдущий вопрос);
- **C-компилятор** для `cc` (`gcc`, `clang`, MSVC);
- **переменная `LIBCLANG_PATH`** — если `libclang` не в стандартном месте.

---

## Типичные ошибки

| Ошибка | Причина |
|---|---|
| `Unable to find libclang` | не установлен или не задан `LIBCLANG_PATH` |
| `error: Linking with cc failed` | нет C-компилятора |
| `undefined reference to add` | `cc` не скомпилировал или не сообщил Cargo |
| `include!: couldn't read .../bindings.rs` | `build.rs` не выполнился или упал |
| `main function not found in crate build_script_build` | нет `fn main()` в `build.rs` |

---

## Итог

| Элемент | Роль |
|---|---|
| `bindgen` | генерирует Rust-привязки из C-заголовков |
| `cc` | компилирует C-код в статическую библиотеку |
| `build.rs` | запускает оба инструмента до сборки крейта |
| `wrapper.h` | заголовок — вход для bindgen |
| `add.c` | реализация на C |
| `OUT_DIR` | временная папка сборки (Cargo) |
| `bindings.rs` | сгенерированные привязки |
| `include!` | вставляет `bindings.rs` в код |
| `unsafe` | обязателен для вызова внешних функций |

**Короткий ответ:** `bindgen` — инструмент автоматической генерации Rust-привязок из C-заголовков. В вашем проекте `build.rs` делает две вещи: (1) через `cc` компилирует `native/add.c` в статическую библиотеку, (2) через `bindgen` парсит `native/wrapper.h` и генерирует `bindings.rs` в `OUT_DIR`. Затем `main.rs` через `include!` встраивает сгенерированный файл и вызывает `add` в блоке `unsafe`. Это стандартный способ интеграции C-кода в Rust-проект без ручного написания `extern "C"`-объявлений. Требуется установленный `libclang` и C-компилятор.
