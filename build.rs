use std::{env, error::Error, path::PathBuf};

use cc ;

fn main() ->Result<(), Box<dyn Error>>{
    // компилирует файл native/add.c и создаёт статическую библиотеку 
    cc::Build::new() // Создаёт новый экземпляр пустого набора конфигурации
        .file("native/add.c")   // Добовляет файл который будет скомпилирован
        .compile("add") // Запускает компилятор генерирующий файл add.lib
        ;

    // создание binding (привязку) для RUST
    let my_bind = 
            // match
             bindgen::Builder::default() // создаёт билдер по умолчанию для генерации Rust-привязок.
                .header("native/wrapper.h") // Добавьте заголовочный файл C/C++ для ввода данных, чтобы сгенерировать привязки на RUST.
                .parse_callbacks(   // Добавьте новый экземпляр ParseCallbacks для настройки типов в различных ситуациях.
                    Box::new(
                        bindgen::CargoCallbacks::new()  // Создаёт новое CargoCallbacks значение.
                    )
                )
                .generate() // Сгенерируйте привязки Rust, используя уже созданные параметры.
                ? ;

    // создать PathBuf из переменной окружения OUT_DIR
    let out_path = 
            PathBuf::from(
                env::var(
                    // Переменная окружения OUT_DIR, которую Cargo устанавливает для 
                    //  временной директории сборки.                    
                    "OUT_DIR"
                )?
            ) ;

    Ok(
        my_bind
            .write_to_file( // Записывает этот binding как источник текста для файла.
            out_path
                    .join(  // Создаёт влажеющий PathBuf с path присоединённым к себе
                        "bindings.rs"
                    )
            )?
    )
}
