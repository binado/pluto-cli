use std::ffi::OsString;
use std::os::unix::process::CommandExt;
use std::path::PathBuf;
use std::process::Command;

use clap::{CommandFactory, Parser, Subcommand, ValueHint};
use clap_complete::Shell;

/// Julia code executed by `julia -e`. This string is constant: user-provided
/// values are never spliced into it. They arrive as `key=value` pairs in ARGS,
/// so paths and URLs need no quoting or escaping on the Rust side.
const PLUTO_SNIPPET: &str = r#"
try
    @eval import Pluto
catch
    println(stderr, "error: Pluto not found in project: ", Base.active_project())
    println(stderr, "install it with: julia -e 'using Pkg; Pkg.add(\"Pluto\")'")
    exit(1)
end
kw = Dict{Symbol,Any}()
nbs = String[]
for a in ARGS
    k, rest = split(a, '='; limit=2)
    v = String(rest)
    if k == "notebook"
        push!(nbs, v)
    elseif k == "threads"
        kw[:threads] = something(tryparse(Int, v), v)
    else
        kw[Symbol(k)] = v
    end
end
isempty(nbs) || (kw[:notebook] = length(nbs) == 1 ? only(nbs) : nbs)
Pluto.run(; kw...)
"#;

/// Launch the Pluto.jl notebook server
#[derive(Parser, Debug)]
#[command(name = "pluto", version)]
struct Cli {
    /// Notebook file(s) to open on launch (repeatable)
    #[arg(short, long, value_hint = ValueHint::FilePath)]
    notebook: Vec<PathBuf>,

    /// Base URL on which the server responds
    #[arg(long)]
    base_url: Option<String>,

    /// Threads for the notebook worker processes ("auto" or a number)
    #[arg(short = 't', long, value_name = "N")]
    notebook_threads: Option<String>,

    /// Julia project environment for the server process (where Pluto is installed)
    #[arg(short, long, value_name = "ENV", value_hint = ValueHint::DirPath)]
    project: Option<String>,

    /// Julia executable to run
    #[arg(long, env = "PLUTO_CLI_JULIA", default_value = "julia", value_hint = ValueHint::FilePath)]
    julia: PathBuf,

    #[command(subcommand)]
    command: Option<Cmd>,
}

#[derive(Subcommand, Debug)]
enum Cmd {
    /// Generate a shell completion script (e.g. `pluto completions zsh`)
    Completions { shell: Shell },
}

/// Arguments passed to the julia executable. Everything after `--` follows the
/// `key=value` protocol consumed by PLUTO_SNIPPET.
fn julia_args(cli: &Cli) -> Vec<OsString> {
    // --startup-file=no: launch faster and don't let a broken/heavy
    // ~/.julia/config/startup.jl interfere with the server process.
    let mut args: Vec<OsString> = vec!["--startup-file=no".into()];
    if let Some(project) = &cli.project {
        args.push(format!("--project={project}").into());
    }
    args.push("-e".into());
    args.push(PLUTO_SNIPPET.into());
    args.push("--".into());
    for nb in &cli.notebook {
        let mut pair = OsString::from("notebook=");
        pair.push(nb);
        args.push(pair);
    }
    if let Some(url) = &cli.base_url {
        args.push(format!("base_url={url}").into());
    }
    if let Some(threads) = &cli.notebook_threads {
        args.push(format!("threads={threads}").into());
    }
    args
}

fn main() {
    let cli = Cli::parse();

    if let Some(Cmd::Completions { shell }) = cli.command {
        let mut cmd = Cli::command();
        let name = cmd.get_name().to_string();
        clap_complete::generate(shell, &mut cmd, name, &mut std::io::stdout());
        return;
    }

    // exec() replaces this process with julia, so signals (Ctrl-C) and the
    // exit code belong to the server. It only returns on failure to launch.
    let err = Command::new(&cli.julia).args(julia_args(&cli)).exec();
    if err.kind() == std::io::ErrorKind::NotFound {
        eprintln!("error: julia executable not found: {}", cli.julia.display());
        eprintln!("install Julia from https://julialang.org/downloads/ or set PLUTO_CLI_JULIA");
    } else {
        eprintln!("error: failed to launch {}: {err}", cli.julia.display());
    }
    std::process::exit(1);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args_for(cli_args: &[&str]) -> Vec<OsString> {
        let cli = Cli::try_parse_from(std::iter::once("pluto").chain(cli_args.iter().copied()))
            .expect("args should parse");
        julia_args(&cli)
    }

    /// The `key=value` pairs after the `--` separator.
    fn kv_pairs(args: &[OsString]) -> &[OsString] {
        let sep = args.iter().position(|a| a == "--").expect("-- separator present");
        &args[sep + 1..]
    }

    #[test]
    fn bare_invocation_runs_static_snippet() {
        let args = args_for(&[]);
        assert_eq!(
            args,
            vec![
                OsString::from("--startup-file=no"),
                "-e".into(),
                PLUTO_SNIPPET.into(),
                "--".into(),
            ]
        );
    }

    #[test]
    fn flags_map_to_key_value_pairs() {
        let args = args_for(&["-n", "nb.jl", "--base-url", "/pluto/", "-t", "auto"]);
        assert_eq!(kv_pairs(&args), &[
            OsString::from("notebook=nb.jl"),
            "base_url=/pluto/".into(),
            "threads=auto".into(),
        ]);
    }

    #[test]
    fn multiple_notebooks_are_repeated_in_order() {
        let args = args_for(&["-n", "a.jl", "-n", "b.jl"]);
        assert_eq!(kv_pairs(&args), &[OsString::from("notebook=a.jl"), "notebook=b.jl".into()]);
    }

    #[test]
    fn project_goes_to_julia_argv_not_args() {
        let args = args_for(&["--project", "@science"]);
        assert_eq!(args[1], OsString::from("--project=@science"));
        assert!(kv_pairs(&args).is_empty());
    }

    #[test]
    fn path_with_spaces_stays_one_argument() {
        let args = args_for(&["-n", "/tmp/my notebooks/nb 1.jl"]);
        assert_eq!(kv_pairs(&args), &[OsString::from("notebook=/tmp/my notebooks/nb 1.jl")]);
    }
}
