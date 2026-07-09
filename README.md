# pluto-cli

A thin launcher for the [Pluto.jl](https://plutojl.org) notebook server, so that

```bash
julia -e 'using Pluto; Pluto.run()'
```

becomes

```bash
pluto
```

## Install

Requires a Rust toolchain and a Julia install with Pluto available.

```bash
cargo install --path .
```

## Usage

```bash
pluto                                  # start the server
pluto -n notebook.jl                   # open a notebook (repeat -n for several)
pluto -t 4                             # 4 threads for notebook worker processes
pluto --base-url /pluto/               # serve under a base URL
pluto -p @science                      # julia --project=@science (env where Pluto lives)
pluto --julia ~/bin/julia-nightly      # pick the julia binary (or set PLUTO_CLI_JULIA)
```

Note that `-t/--notebook-threads` configures the notebook *worker* processes
(Pluto's `threads` compiler option), not the server process.

## Shell completions

```bash
pluto completions zsh > ~/.zfunc/_pluto      # zsh (with ~/.zfunc in fpath)
pluto completions bash > ~/.local/share/bash-completion/completions/pluto
pluto completions fish > ~/.config/fish/completions/pluto.fish
```

## How it works

The binary `exec()`s `julia --startup-file=no -e '<static snippet>' -- key=value ...`.
The Julia snippet never changes; your values travel as arguments after `--`, so paths
with spaces or special characters need no escaping. If Pluto isn't installed in the
selected project environment, it prints an install hint and exits non-zero.

`--startup-file=no` keeps the server launch fast and independent of your
`~/.julia/config/startup.jl` (e.g. a `using Revise` there would slow every launch).

## Non-goals (for now)

- Generic passthrough of arbitrary `Pluto.run` keyword arguments (`-- --foo bar`)
- Port/host flags — easy to add later with the same `key=value` protocol
- Windows support (`exec()` is Unix-only)
