# Heightmap2BRS

![Example output](https://i.imgur.com/QdPLN09.png)
![GTAV Map](https://i.imgur.com/J9XpmT3.png)

### Compiling

You need [rust](https://www.rust-lang.org/).

### Usage

Compile or download from releases.

`heightmap.exe --help` for usage instructions:

    USAGE:
        heightmap.exe [FLAGS] [OPTIONS] <INPUT>

    FLAGS:
            --cull       Automatically remove bottom level bricks
        -h, --help       Prints help information
            --old        Use old unoptimized heightmap code
            --snap       Snap bricks to the brick grid
            --tile       Render bricks as tiles
        -V, --version    Prints version information

    OPTIONS:
        -c, --colormap <colormap>    Input colormap PNG image
        -o, --output <output>        Output BRS file
        -s, --size <size>            Brick stud size (default 1)
        -v, --vertical <vertical>    Vertical scale multiplier (default 1)

    ARGS:
        <INPUT>    Input heightmap PNG image

An example command for generating the GTA V map would be:

`heightmap example_maps/gta5_fixed2_height.png -c example_maps/gta5_fixed2_color.png -s 4 -v 20 --tile -o gta5.brs`