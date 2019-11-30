extern crate lazy_static;

use lazy_static::lazy_static;
use std::collections::HashMap;
use std::env;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

lazy_static! {
    static ref TRIPLE_TRANSLATIONS: HashMap<&'static str, &'static str> = {
        let mut hm = HashMap::new();
        hm.insert("armv7-unknown-linux-gnueabihf", "arm-linux-gnueabihf");
        hm
    };
}

fn main() {
    let manifest_dir = PathBuf::from(cargo_env("CARGO_MANIFEST_DIR"));
    let out_dir = PathBuf::from(cargo_env("OUT_DIR"));
    let alsa_dir = manifest_dir.join("alsa-lib");

    let build_dir = out_dir.join("build");
    let install_dir = out_dir.join("install");
    fs::create_dir_all(&build_dir).expect("Could not create build dir");
    fs::create_dir_all(&install_dir).expect("Could not create install dir");

    let host = cargo_env("HOST").into_string().expect("Could not convert HOST into a string");
    let target = cargo_env("TARGET").into_string().expect("Could not convert TARGET into a string");

    if host == target {
        // Option 1: pkg-config.
        let mut pc = pkg_config::Config::new();
        match pc.atleast_version("1.2").statik(true).probe("alsa") {
            Ok(..) => return,
            Err(pkg_config::Error::Failure { .. }) => println!("cargo:warning=Could not find alsa at least v1.2 with pkg-config. Falling back on built-in version. If you wanted to link to the system alsa-lib, you might need to install pkg-config and alsa-lib-devel or libasound2-dev."),
            Err(e) => panic!("Unknown error: {}", e),
        }
    }

    let mut target2: Option<&str> = None;
    if host != target {
        if let Some(translated) = TRIPLE_TRANSLATIONS.get(target.as_str()) {
            target2 = Some(translated.as_ref());
        } else {
            target2 = Some(&target);
        }
    }

    // Option 2&3: Build from the built-in copy for a normal or cross compile.
    build_alsa(&alsa_dir, &build_dir, &install_dir, target2);
    let lib_dir = install_dir.join("usr").join("lib")
        .to_str()
        .expect("Could not convert lib dir to a string")
        .to_string();
    println!("cargo:rustc-link-search={}", lib_dir);
    println!("cargo:rustc-link-lib=asound");
    println!("cargo:rustc-link-lib=atopology");
}

fn cargo_env(name: &str) -> OsString {
    env::var_os(name).expect(&format!("Environment variable not found: {}", name))
}

fn build_alsa(alsa_dir: &Path, build_dir: &Path, install_dir: &Path, target: Option<&str>) {
    prebuild_alsa(&alsa_dir);
    configure_alsa(&alsa_dir, &build_dir, target);
    compile_alsa(&build_dir);
    install_alsa(&build_dir, &install_dir);
}

fn prebuild_alsa(alsa_dir: &Path) {
    let mut cmd = Command::new("libtoolize");
    cmd.current_dir(alsa_dir).args(&["--force", "--copy", "--automake"]);
    execute(cmd);

    let mut cmd = Command::new("aclocal");
    cmd.current_dir(alsa_dir);
    execute(cmd);

    let mut cmd = Command::new("autoheader");
    cmd.current_dir(alsa_dir);
    execute(cmd);

    let mut cmd = Command::new("automake");
    cmd.current_dir(alsa_dir).args(&["--foreign", "--copy", "--add-missing"]);
    execute(cmd);

    let mut cmd = Command::new("autoconf");
    cmd.current_dir(alsa_dir);
    execute(cmd);
}

fn configure_alsa(alsa_dir: &Path, build_dir: &Path, cross_host: Option<&str>) {
    let mut cmd = Command::new("sh");
    let configure = alsa_dir.join("configure").to_string_lossy().to_owned().to_string();
    cmd.current_dir(build_dir).arg("-c");

    match cross_host {
        Some(host) => {
            let config = format!("{} --enable-shared=no --enable-static=yes --host={}", configure, host);
            cmd.arg(config);
        },
        None => {
            let config = format!("{} --enable-shared=no --enable-static=yes", configure);
            cmd.arg(config);
        }
    }

    execute(cmd);
}

fn compile_alsa(build_dir: &Path) {
    let mut cmd = Command::new("make");
    cmd.current_dir(build_dir);
    execute(cmd);
}

fn install_alsa(build_dir: &Path, install_dir: &Path) {
    let mut cmd = Command::new("make");
    cmd.current_dir(build_dir)
        .arg("install")
        .env("DESTDIR", install_dir);
    execute(cmd);
}

fn execute(mut command: Command) {
    println!("$ {:?}", command);
    command.status().expect(&format!("Could not execute {:?}", command));
}
