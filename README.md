# Packwiz Parent Pack

Add a parent modpack to your packwiz modpack!  
This is an external tool which generates a combined modpack based of your own modpack and the specified parent modpack.

## Installation

Clone this repository (or download zip) and then run `cargo install --path .`. you will need [rust](https://www.rust-lang.org/tools/install) for this.
afterwards you can run it using `packwiz-parent-pack` in the command line.

## Usage

Add the following to your `pack.toml`

```toml
[options]
parent = "<link to parent pack's pack.toml>"
```

Now you can use `packwiz-parent-pack -o <your output folder>` and your new pack, made from the specified parent pack and your own pack, should be ready to use!
