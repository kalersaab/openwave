fn main() {
    // Tell Cargo where to find VLC libraries on macOS
    #[cfg(target_os = "macos")]
    {
        println!("cargo:rustc-link-search=/Applications/VLC.app/Contents/MacOS/lib");
        println!("cargo:rustc-link-lib=dylib=vlc");
        println!("cargo:rustc-link-lib=dylib=vlccore");
    }
    
    println!("cargo:rerun-if-changed=build.rs");
}
