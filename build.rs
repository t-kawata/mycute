use std::env;
use std::path::PathBuf;

fn main() {
    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap();

    if target_os == "windows" {
        // Windows Dynamic Link (DLL)
        // Native AOT produces a .dll and a .lib (import library).
        
        println!("cargo:rustc-link-lib=SpeechHelper");
        println!("cargo:rerun-if-changed=native/cs/SpeechHelper/SpeechHelper.cs");
        println!("cargo:rerun-if-changed=native/cs/SpeechHelper/SpeechHelper.csproj");

        let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
        
        // Potential paths for .lib (native) and .dll (publish)
        // Adjust these based on dotnet publish output structure
        let base_out_dir = manifest_dir.join("native/cs/SpeechHelper/bin/Release/net10.0-windows10.0.26100.0/win-x64");
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
            println!("cargo:warning=SpeechHelper.lib not found in expected locations. Build may fail.");
        }

        // 2. DLL Copy Logic (Runtime dependency)
        // The DLL is usually in 'publish'
        let dll_name = "SpeechHelper.dll";
        let dll_source = if publish_dir.join(dll_name).exists() {
            Some(publish_dir.join(dll_name))
        } else if native_dir.join(dll_name).exists() {
            Some(native_dir.join(dll_name))
        } else {
            None
        };

        if let Some(src) = dll_source {
            let target_path = manifest_dir.join(dll_name);
            // Always copy to ensure latest version
            match std::fs::copy(&src, &target_path) {
                Ok(_) => {}, // println!("cargo:warning=Copied {} to crate root", dll_name),
                Err(e) => println!("cargo:warning=Failed to copy DLL: {}", e),
            }
        } else {
            println!("cargo:warning={} not found. Runtime error is likely.", dll_name);
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
                            let paths_str = &stdout[list_start+1..list_end];
                            
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
