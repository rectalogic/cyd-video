use std::{
    env,
    path::{Path, PathBuf},
};

fn main() {
    linker_be_nice();

    esp_new_jpeg();

    embed_video();

    // make sure linkall.x is the last linker script (otherwise might cause problems with flip-link)
    println!("cargo:rustc-link-arg=-Tlinkall.x");
}

fn embed_video() {
    println!("cargo:rerun-if-env-changed=EMBED_VIDEO");
    let embed_enabled = std::env::var("CARGO_FEATURE_EMBED_VIDEO").is_ok();
    let embed_env_var_set = std::env::var("EMBED_VIDEO").is_ok();

    if embed_enabled && !embed_env_var_set {
        panic!(
            "'embed_video' feature is enabled, but EMBED_VIDEO environment variable is not set."
        );
    }
    if !embed_enabled && embed_env_var_set {
        panic!(
            "EMBED_VIDEO environment variable is set, but 'embed_video' feature is not enabled."
        );
    }

    if embed_enabled && embed_env_var_set {
        let path = Path::new(&std::env::var("CARGO_MANIFEST_DIR").unwrap())
            .join(std::env::var("EMBED_VIDEO").unwrap());
        if !path.exists() {
            panic!(
                "The file specified by EMBED_VIDEO does not exist: {}",
                path.display()
            );
        }

        println!("cargo:rustc-env=EMBED_VIDEO={}", path.display());
        println!("cargo:rerun-if-changed={}", path.display());
    }
}

fn esp_new_jpeg() {
    // Link the esp_new_jpeg C library for JPEG decoding
    println!(
        "cargo:rustc-link-search=native={}/esp-adf-libs/esp_new_jpeg/lib/esp32s3",
        std::env::var("CARGO_MANIFEST_DIR").unwrap(),
    );
    println!("cargo:rustc-link-lib=static=esp_new_jpeg");

    let bindings = bindgen::Builder::default()
        .header("esp-adf-libs/esp_new_jpeg/include/esp_jpeg_dec.h")
        // Tell cargo to invalidate the built crate whenever any of the
        // included header files changed.
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .use_core()
        .derive_default(true)
        .constified_enum_module("jpeg_.*_t")
        .generate()
        .expect("Unable to generate esp_new_jpeg bindings");

    // Write the bindings to the $OUT_DIR/bindings.rs file.
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write esp_new_jpeg bindings!");
}

fn linker_be_nice() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() > 1 {
        let kind = &args[1];
        let what = &args[2];

        match kind.as_str() {
            "undefined-symbol" => match what.as_str() {
                what if what.starts_with("_defmt_") => {
                    eprintln!();
                    eprintln!(
                        "💡 `defmt` not found - make sure `defmt.x` is added as a linker script and you have included `use defmt_rtt as _;`"
                    );
                    eprintln!();
                }
                "_stack_start" => {
                    eprintln!();
                    eprintln!("💡 Is the linker script `linkall.x` missing?");
                    eprintln!();
                }
                what if what.starts_with("esp_rtos_") => {
                    eprintln!();
                    eprintln!(
                        "💡 `esp-radio` has no scheduler enabled. Make sure you have initialized `esp-rtos` or provided an external scheduler."
                    );
                    eprintln!();
                }
                "embedded_test_linker_file_not_added_to_rustflags" => {
                    eprintln!();
                    eprintln!(
                        "💡 `embedded-test` not found - make sure `embedded-test.x` is added as a linker script for tests"
                    );
                    eprintln!();
                }
                "free"
                | "malloc"
                | "calloc"
                | "get_free_internal_heap_size"
                | "malloc_internal"
                | "realloc_internal"
                | "calloc_internal"
                | "free_internal" => {
                    eprintln!();
                    eprintln!(
                        "💡 Did you forget the `esp-alloc` dependency or didn't enable the `compat` feature on it?"
                    );
                    eprintln!();
                }
                _ => (),
            },
            // we don't have anything helpful for "missing-lib" yet
            _ => {
                std::process::exit(1);
            }
        }

        std::process::exit(0);
    }

    println!(
        "cargo:rustc-link-arg=-Wl,--error-handling-script={}",
        std::env::current_exe().unwrap().display()
    );
}
