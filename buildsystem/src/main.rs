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

    /// build server in debug mode
    #[arg(long)]
    debug: bool,

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

    // Also for the launcher wrapper. Typescript could do this but eh.
    Command::new(NPX)
        .arg("--no-install")
        .arg("esbuild")
        .arg("--bundle")
        .arg("client/server_launcher.ts")
        .arg("--outdir=dist")
        .arg("--platform=node")
        .arg("--external:vscode")
        .status()?
        .exit_ok()?;

    Ok(())
}

/// Get the final output path depending on the current and target platforms.
fn copy_server_binary_to_dist(debug: bool) -> Result<()> {
    fs::create_dir_all("dist")?;

    let from = if debug {
        "server/target/wasm32-wasi/debug/sail_server.wasm"
    } else {
        "server/target/wasm32-wasi/release/sail_server.wasm"
    };
    let to = "dist/server.wasm";

    fs::copy(from, to)?;

    Ok(())
}

fn make_server(debug: bool) -> Result<()> {
    eprintln!("Building server...");

    let mut command = Command::new("cargo");
    command.arg("build");
    if !debug {
        command.arg("--release");
    }
    command.arg("--target").arg("wasm32-wasi").current_dir("server");

    command.status()?.exit_ok()?;

    // Copy the output to `dist`.
    copy_server_binary_to_dist(debug)?;

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

    check_build_dependencies()?;

    match opts.target {
        Target::Client => {
            make_client()?;
        }
        Target::Server => {
            make_server(opts.debug)?;
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
            make_server(opts.debug)?;
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


fn check_build_dependencies() -> Result<()> {
    eprintln!("Checking build dependencies...");

    // For now just check all dependencies, but we could skip some checks
    // depending on opts.target.

    check_command_exists(NPM, &["--version"], "You might need to install Node. I recommend this method: https://github.com/Schniz/fnm#installation")?;
    check_command_exists("cargo", &["--version"], "You might need to install Rust: https://www.rust-lang.org/tools/install")?;
    check_command_exists("rustup", &["--version"], "You might need to install Rust: https://www.rust-lang.org/tools/install")?;


    let installed_targets = rustup_installed_items("target")?;
    // let installed_components = rustup_installed_items("component")?;

    if !installed_targets.contains("wasm32-wasi") {
        bail!("The wasm32-wasi target is not installed. Try `rustup target add wasm32-wasi`");
    }
    // The WASI standard library is not precompiled yet apparently? Maybe it never will be?
    // if !installed_components.contains("rust-std-wasm32-wasi") {
    //     bail!("The rust-std-wasm32-wasi component is not installed. Try `rustup component add rust-std-wasm32-wasi`");
    // }

    Ok(())
}
