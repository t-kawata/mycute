use std::env;
use std::path::PathBuf;

fn main() {
    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap();

    if target_os == "windows" {
        // -------------------------------------------------------------------------
        // Windows Shared Link (DLL) - Native AOT
        // -------------------------------------------------------------------------
        // Native AOT の Shared モードでは、OS ローダーによって自動的に .NET ランタイム
        // (GC, Threading 等) が初期化されます。CRT も DLL 内部にカプセル化されるため安全です。
        println!("cargo:rustc-link-lib=SpeechHelper");
        println!("cargo:rerun-if-changed=native/cs/SpeechHelper/SpeechHelper.cs");
        println!("cargo:rerun-if-changed=native/cs/SpeechHelper/SpeechHelper.csproj");

        // （DLL化により、静的リンク用のランタイムライブラリ指定はすべて不要になりました）

        // Windows システムライブラリ（.NET ランタイムが内部で使用）
        println!("cargo:rustc-link-lib=ole32");
        println!("cargo:rustc-link-lib=oleaut32");
        println!("cargo:rustc-link-lib=advapi32");
        println!("cargo:rustc-link-lib=bcrypt");
        println!("cargo:rustc-link-lib=crypt32");
        println!("cargo:rustc-link-lib=iphlpapi");
        println!("cargo:rustc-link-lib=kernel32");
        println!("cargo:rustc-link-lib=mswsock");
        println!("cargo:rustc-link-lib=ntdll");
        println!("cargo:rustc-link-lib=secur32");
        println!("cargo:rustc-link-lib=user32");
        println!("cargo:rustc-link-lib=ws2_32");

        // /IGNORE:4099 は PDB が見つからない警告の抑制。
        // （注：/FORCE はLNK1319等も無視してSegment Faultを誘発するため削除）
        // println!("cargo:rustc-link-arg=/FORCE");
        println!("cargo:rustc-link-arg=/IGNORE:4099");

        let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());

        // Potential paths for .lib (native) and .dll (publish)
        // Adjust these based on dotnet publish output structure
        let base_out_dir = manifest_dir
            .join("native/cs/SpeechHelper/bin/Release/net10.0-windows10.0.26100.0/win-x64");
        let native_dir = base_out_dir.join("native");
        let publish_dir = base_out_dir.join("publish");

        // 1. Setup Library Search Path (for .lib)
        let mut lib_found = false;

        // Priority 1: Env var from Makefile
        if let Ok(lib_dir) = env::var("SPEECH_HELPER_LIB_DIR") {
            println!("cargo:rustc-link-search=native={}", lib_dir);
            lib_found = true;
        }

        // Priority 2: Standard locations
        if native_dir.exists() {
            println!("cargo:rustc-link-search=native={}", native_dir.display());
            lib_found = true;
        }
        if publish_dir.exists() {
            println!("cargo:rustc-link-search=native={}", publish_dir.display());
            lib_found = true;
        }

        if !lib_found {
            println!(
                "cargo:warning=SpeechHelper.lib not found in expected locations. Build may fail."
            );
        }

        // =========================================================
        // 実行時 (Runtime) のために DLL を Target フォルダにコピーする
        // =========================================================
        let dll_path = publish_dir.join("SpeechHelper.dll");
        if dll_path.exists() {
            let out_dir = env::var("OUT_DIR").unwrap();
            let dest_path = PathBuf::from(out_dir)
                .join("..")
                .join("..")
                .join("..")
                .join("SpeechHelper.dll");

            match std::fs::copy(&dll_path, &dest_path) {
                Ok(_) => {
                    println!(
                        "cargo:warning=SpeechHelper.dll copied to target: {}",
                        dest_path.display()
                    );
                }
                Err(e) => {
                    println!("cargo:warning=Failed to copy SpeechHelper.dll: {}", e);
                }
            }
        } else {
            println!(
                "cargo:warning=SpeechHelper.dll NOT FOUND AT: {}",
                dll_path.display()
            );
        }
    } else if target_os == "macos" {
        // macOS Static Link (Swift)
        println!("cargo:rustc-link-lib=static=SpeechHelper");
        println!("cargo:rerun-if-changed=native/swift");

        // Try to find the .a file (default is target/swift)
        if let Ok(lib_dir) = env::var("SPEECH_HELPER_LIB_DIR") {
            println!("cargo:rustc-link-search=native={}", lib_dir);
        } else {
            println!("cargo:rustc-link-search=native=target/swift");
        }

        // Add Swift runtime library paths to link search
        // This resolves "Library not loaded: @rpath/libswift_Concurrency.dylib" by finding system libs
        let output = std::process::Command::new("swiftc")
            .args(&["-print-target-info"])
            .output();

        if let Ok(output) = output {
            if let Ok(stdout) = String::from_utf8(output.stdout) {
                // Parse JSON output using simple string searching to avoid adding serde_json dependency
                // We are looking for "runtimeLibraryPaths": [ ... ]
                if let Some(paths_start) = stdout.find("\"runtimeLibraryPaths\"") {
                    if let Some(list_start) = stdout[paths_start..].find('[') {
                        let list_start = paths_start + list_start;
                        if let Some(list_end) = stdout[list_start..].find(']') {
                            let list_end = list_start + list_end;
                            let paths_str = &stdout[list_start + 1..list_end];

                            for path in paths_str.split(',') {
                                let path = path.trim().trim_matches('"').trim();
                                if !path.is_empty() {
                                    println!("cargo:rustc-link-search=native={}", path);
                                }
                            }
                        }
                    }
                }
            }
        }

        // Add RPATH to help finding Swift standard libraries AND embedded dylibs
        println!("cargo:rustc-link-arg=-Wl,-rpath,/usr/lib/swift");
        println!("cargo:rustc-link-arg=-Wl,-rpath,/usr/lib/swift/macosx");

        // CRITICAL: Add @executable_path/ to RPATH so the binary looks for dylibs next to itself,
        // even when elevated (where DYLD_LIBRARY_PATH is stripped).
        println!("cargo:rustc-link-arg=-Wl,-rpath,@executable_path/");
        println!("cargo:rustc-link-arg=-Wl,-rpath,@loader_path/");

        // Tauri はバンドルリソースを Contents/Resources/ に配置するため、
        // 実行ファイル（Contents/MacOS/）からの相対パスでリソースフォルダも探索対象に含める
        println!("cargo:rustc-link-arg=-Wl,-rpath,@loader_path/../Resources");

        // Link required system frameworks for Swift
        println!("cargo:rustc-link-lib=framework=Foundation");
        println!("cargo:rustc-link-lib=framework=AVFoundation");
        println!("cargo:rustc-link-lib=framework=Speech");
        println!("cargo:rustc-link-lib=framework=CoreFoundation");
        // macOS 12+ requires ExtensionFoundation in some cases, but usually standard libs suffice
    }

    println!("cargo:rerun-if-changed=ui/dist");
    tauri_build::build();
}
