use fs_err as fs;

use std::{
    path::Path,
    process::{Command, ExitStatus},
    str::FromStr, collections::HashSet,
};

use anyhow::{bail, Result};
use clap::Parser;

// NPM command name. On Windows we need .cmd otherwise it can't find it (at least
// when using FNM).
#[cfg(windows)]
const NPM: &'static str = "npm.cmd";
#[cfg(not(windows))]
const NPM: &'static str = "npm";

#[cfg(windows)]
const NPX: &'static str = "npx.cmd";
#[cfg(not(windows))]
const NPX: &'static str = "npx";

#[derive(PartialEq, Eq, Copy, Clone, Debug)]
enum Target {
    Clean,
    Client,
    NpmInstall,
    Package,
    Release,
    Server,
}

impl FromStr for Target {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        Ok(match s {
            "clean" => Self::Clean,
            "client" => Self::Client,
            "npm_install" => Self::NpmInstall,
            "package" => Self::Package,
            "release" => Self::Release,
            "server" => Self::Server,
            x => bail!("Invalid target: {}", x),
        })
    }
}

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
/// Extension build system.
struct Opts {
    /// the target to build. One of:
    ///
    /// * clean         Clean everything (including node_modules)
    /// * client        Build the Typescript extension client
    /// * npm_install   Run NPM install
    /// * package       Generate the VSIX package
    /// * release       Clean and rebuild everything and make a
    ///                 package
    /// * server        Build the Rust language server
    target: Target,

    /// don't cross-compile the server (useful for development). Ignored for the release target.
    #[arg(long)]
    no_cross: bool,

    /// don't clean when making a release (only for `make release`)
    #[arg(long)]
    no_clean: bool,
}

// Simple version of the "real" ExitStatus::exit_ok() which is currently unstable.
trait ExitOk {
    fn exit_ok(self) -> Result<()>;
}

impl ExitOk for ExitStatus {
    fn exit_ok(self) -> Result<()> {
        if self.success() {
            Ok(())
        } else {
            bail!("Command failed with exit code: {:?}", self.code());
        }
    }
}

fn make_client() -> Result<()> {
    eprintln!("Building client...");

    // Type check with the Typescript compiler.
    Command::new(NPX)
        .arg("--no-install")
        .arg("tsc")
        .arg("-p")
        .arg("tsconfig.json")
        .arg("--noEmit")
        .status()?
        .exit_ok()?;

    // Then bundle using esbuild which ignores Typescript types.
    // This is necessary so we don't have to ship `node_modules` which includes
    // a load of dev dependencies.
    Command::new(NPX)
        .arg("--no-install")
        .arg("esbuild")
        .arg("--bundle")
        .arg("client/extension.ts")
        .arg("--outdir=dist")
        .arg("--platform=node")
        .arg("--external:vscode")
        .status()?
        .exit_ok()?;

    Ok(())
}

#[derive(PartialEq, Eq, Debug)]
enum Platform {
    LinuxX86,
    MacX86,
    MacArm,
    WinX86,
    Other(String),
}

impl Platform {
    fn native() -> Self {
        use std::env::consts::{ARCH, OS};
        match (OS, ARCH) {
            ("linux", "x86_64") => Self::LinuxX86,
            ("macos", "x86_64") => Self::MacX86,
            ("macos", "aarch64") => Self::MacArm,
            ("windows", "x86_64") => Self::WinX86,
            _ => Self::Other(format!("{} {}", OS, ARCH)),
        }
    }

    fn rust_target(&self) -> Option<&'static str> {
        match self {
            Platform::LinuxX86 => Some("x86_64-unknown-linux-musl"),
            Platform::MacArm => Some("aarch64-apple-darwin"),
            Platform::MacX86 => Some("x86_64-apple-darwin"),
            Platform::WinX86 => Some("x86_64-pc-windows-gnu"),
            Platform::Other(_) => None,
        }
    }
}

/// Get the value passed to `--target` for Rust compilation, if any.
/// It is not used for native builds except on Linux where we always use
/// it to get musl.
fn target_flag_value(target_platform: &Platform) -> Result<Option<&'static str>> {
    // Both platforms must be known.
    if Platform::native().rust_target().is_none() || target_platform.rust_target().is_none() {
        bail!("Unsupported platform: {host_platform:?} -> {target_platform:?}", host_platform=Platform::native())
    }
    // Always use the --target flag on Linux to get musl.
    if *target_platform == Platform::LinuxX86 {
        return Ok(target_platform.rust_target());
    }
    // If we're running on native don't use --target.
    if *target_platform == Platform::native() {
        return Ok(None)
    }
    // Return the target flag.
    Ok(target_platform.rust_target())
}

/// Set flags on the command for cross-compiling, depending on the current
/// and target platforms.
fn set_cargo_flags(command: &mut Command, target_platform: &Platform) -> Result<()> {
    if let Some(flag) = target_flag_value(target_platform)? {
        command.arg("--target").arg(flag);
    }
    Ok(())
}

/// Get the final output path depending on the current and target platforms.
/// TODO: Probably should use `cargo metadata` strictly.
fn copy_server_binary_to_dist(target_platform: &Platform) -> Result<()> {
    fs::create_dir_all("dist")?;

    let target = target_flag_value(target_platform)?;

    let mut from = if let Some(target) = target {
        format!("server/target/{target}/release/sail_server")
    } else {
        "server/target/release/sail_server".to_owned()
    };

    if matches!(target_platform, Platform::Other(_)) {
        bail!("Unsupported target platform: {:?}", target_platform);
    }

    let mut to = format!("dist/server_{}", target_platform.rust_target().unwrap());

    if *target_platform == Platform::WinX86 {
        from.push_str(".exe");
        to.push_str(".exe");
    }

    fs::copy(from, to)?;

    Ok(())
}

fn make_server(no_cross: bool) -> Result<()> {
    eprintln!("Building server...");

    for target_platform in [Platform::LinuxX86, Platform::MacX86, /*Platform::MacArm, */Platform::WinX86] {
        if no_cross && target_platform != Platform::native() {
            continue;
        }

        eprintln!("  Platform: {:?}", target_platform);

        let mut command = Command::new("cargo");
        command.arg("build").arg("--release").current_dir("server");

        set_cargo_flags(&mut command, &target_platform)?;

        command.status()?.exit_ok()?;

        // Copy the output to `dist`.
        copy_server_binary_to_dist(&target_platform)?;
    }

    Ok(())
}

fn make_package() -> Result<()> {
    eprintln!("Building VSIX package...");

    Command::new(NPX)
        .arg("--no-install")
        .arg("vsce")
        .arg("package")
        .status()?
        .exit_ok()?;

    Ok(())
}

fn npm_install() -> Result<()> {
    eprintln!("Running npm install...");

    Command::new(NPM).arg("install").status()?.exit_ok()?;

    Ok(())
}

fn clean() -> Result<()> {
    for cargo_dir in ["server"] {
        eprintln!("Cleaning {}...", cargo_dir);

        Command::new("cargo")
            .arg("clean")
            .current_dir(cargo_dir)
            .status()?
            .exit_ok()?;
    }

    for dir in [
        "client_out",
        "node_modules",
    ] {
        eprintln!("Removing {}", dir);
        let path = Path::new(dir);
        if path.exists() {
            fs::remove_dir_all(path)?;
        }
    }

    Ok(())
}

fn main() -> Result<()> {
    let opts = Opts::parse();

    // `cd ..` so that this works when run from ../make.
    let cwd = std::env::current_dir()?;
    if cwd.ends_with("buildsystem") {
        if let Some(parent) = std::env::current_dir()?.parent() {
            std::env::set_current_dir(parent)?;
        }
    }

    check_build_dependencies(&opts)?;

    match opts.target {
        Target::Client => {
            make_client()?;
        }
        Target::Server => {
            make_server(opts.no_cross)?;
        }
        Target::Package => {
            make_package()?;
        }
        Target::Clean => {
            clean()?;
        }
        Target::Release => {
            if !opts.no_clean {
                clean()?;
            }
            npm_install()?;
            make_client()?;
            make_server(opts.no_cross)?;
            make_package()?;
        }
        Target::NpmInstall => {
            npm_install()?;
        }
    }

    Ok(())
}

fn check_command_exists(program: &str, args: &[&str], message: &str) -> Result<()> {
    let result = Command::new(program).args(args).output();

    match result {
        Ok(o) => {
            if !o.status.success() {
                bail!("Executed `{program}` but it returned an error. {message}");
            }
        }
        Err(e) => bail!("Could not execute `{program}` ({e:?}). {message}"),
    }
    Ok(())
}

/// Pass "component" or "target" to get the installed Rustup components or targets.
fn rustup_installed_items(item_type: &str) -> Result<HashSet<String>> {
    let rustup_result = Command::new("rustup")
        .arg(item_type)
        .arg("list")
        .arg("--installed")
        .output()?;
    rustup_result.status.exit_ok()?;
    Ok(String::from_utf8(rustup_result.stdout)?.lines().map(|x| x.to_owned()).collect())
}


fn check_build_dependencies(opts: &Opts) -> Result<()> {
    eprintln!("Checking build dependencies...");

    // For now just check all dependencies, but we could skip some checks
    // depending on opts.target.

    check_command_exists(NPM, &["--version"], "You might need to install Node. I recommend this method: https://github.com/Schniz/fnm#installation")?;
    check_command_exists("cargo", &["--version"], "You might need to install Rust: https://www.rust-lang.org/tools/install")?;
    check_command_exists("rustup", &["--version"], "You might need to install Rust: https://www.rust-lang.org/tools/install")?;

    if opts.no_cross {
        if Platform::native() == Platform::LinuxX86 {
            // We always use musl on Linux.
            check_command_exists("x86_64-linux-musl-gcc", &["--version"], "You might need to install a compiler for Musl Linux. Try 'sudo apt install musl-tools'.")?;
        }
        return Ok(());
    }

    // Check cross-compilers.
    match Platform::native() {
        Platform::LinuxX86 => {
            check_command_exists("x86_64-linux-musl-gcc", &["--version"], "You might need to install a compiler for Musl Linux. Try 'sudo apt install musl-tools'.")?;
            check_command_exists("x86_64-w64-mingw32-gcc", &["--version"], "You might need to install a cross-compiler for Windows. Try 'sudo apt install gcc-mingw-w64-x86-64-win32'.")?;
            // check_command_exists("x86_64-apple-darwin-gcc", &["--version"], "You might need to install a cross-compiler for Mac.")?;
            // check_command_exists("aarch64-apple-darwin-gcc", &["--version"], "You might need to install a cross-compiler for Mac.")?;
        }
        Platform::MacX86 => {
            check_command_exists("x86_64-linux-musl-gcc", &["--version"], "You might need to install a cross-compiler for Linux. Try `brew install FiloSottile/musl-cross/musl-cross`.")?;
            check_command_exists("x86_64-w64-mingw32-gcc", &["--version"], "You might need to install a cross-compiler for Windows. Try `brew install mingw-w64`.")?;
            check_command_exists("aarch64-apple-darwin-gcc", &["--version"], "You might need to install a cross-compiler for Mac.")?;
        }
        Platform::MacArm => {
            check_command_exists("x86_64-linux-musl-gcc", &["--version"], "You might need to install a cross-compiler for Linux. Try `brew install FiloSottile/musl-cross/musl-cross`.")?;
            check_command_exists("x86_64-w64-mingw32-gcc", &["--version"], "You might need to install a cross-compiler for Windows. Try `brew install mingw-w64`.")?;
            check_command_exists("x86_64-apple-darwin-gcc", &["--version"], "You might need to install a cross-compiler for Mac.")?;
        }
        _ => {}
    }

    // Check non-native Rustup targets and components.
    let target_platforms: &[Platform] = match Platform::native() {
        Platform::LinuxX86 => &[
            Platform::MacX86,
            Platform::MacArm,
            Platform::WinX86,
        ],
        Platform::MacX86 => &[
            Platform::LinuxX86,
            Platform::MacArm,
            Platform::WinX86,
        ],
        Platform::MacArm => &[
            Platform::LinuxX86,
            Platform::MacX86,
            Platform::WinX86,
        ],
        _ => &[],
    };

    let installed_targets = rustup_installed_items("target")?;
    let installed_components = rustup_installed_items("component")?;

    for target_platform in target_platforms {
        let target = target_platform.rust_target().unwrap();
        if !installed_targets.contains(target) {
            bail!("{target} is not installed. Try `rustup target add {target}`");
        }
        let component = format!("rust-std-{target}");
        if !installed_components.contains(&component) {
            bail!("{component} is not installed. Try `rustup component add {component}`");
        }
    }

    Ok(())
}
