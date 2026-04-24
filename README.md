# pulse

Pulse is a command-line system monitor that collects Linux host metrics and prints them as JSON.

It runs a scheduler that loads these modules regularly:

- `cpu`: CPU brand, architecture, per-core and global usage, frequency, core counts, and temperature
- `mem`: RAM and swap totals, used values, and usage percentages
- `disk`: mounted disk totals, free/used space, and usage percentages
- `net`: network interfaces, default route, link type, IP address, flags, and byte counters
- `gpu`: AMD GPU memory and device info via libdrm

## Usage

Build and run with Cargo:

```sh
cargo run --release -- --help
```

Options:

- `--modules` prints available module names
- `--refresh module:duration` sets refresh intervals for modules, e.g. `cpu:2s` or `net:500ms`
- `--only [modules]` applies a filter on what system modules to run

The program outputs a JSON object keyed by module name.

## Supported platforms

- Linux only
- AMD GPU support only

## TODO

- Add support for non-Linux platforms
- Add Nvidia/Intel GPU support
- Add more robust error handling and retries
