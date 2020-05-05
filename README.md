# Heightmap2BRS

![Example output](https://i.imgur.com/QdPLN09.png)

### Compiling

You need [rust](https://www.rust-lang.org/).

### Usage

Compile or download from releases.

`heightmap.exe --help` for flags:

    USAGE:
        heightmap [FLAGS] [OPTIONS] <INPUT>

    FLAGS:
        -z               Cull 0 layer bricks (I didn't really test this lol)
        -h, --help       Prints help information
        -V, --version    Prints version information

    OPTIONS:
        -c <colormap>    Supply colormap PNG file (same size as heightmap)
        -o <outfile>     Output file (brs)
        -x <scale>       Vertical scale of the output (default 1)
        -s <size>        Brick size of the output (default 1)

    ARGS:
        <INPUT>          Input PNG file

