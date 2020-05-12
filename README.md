# `gltfgen`

A command line tool for generating glTF 2.0 animations from numbered sequences of mesh files.

[![On crates.io](https://img.shields.io/crates/v/gltfgen.svg)](https://crates.io/crates/gltfgen)
[![Travis Build status](https://travis-ci.org/elrnv/gltfgen.svg?branch=master)](https://travis-ci.org/elrnv/gltfgen)
[![GHA Build status](https://github.com/elrnv/gltfgen/workflows/CI/badge.svg)](https://github.com/elrnv/gltfgen/actions?query=workflow%3ACI)


# Usage

The latest `gltfgen` builds are available via

```
> cargo install gltfgen
```

For special builds see [Releases](https://github.com/elrnv/gltfgen/releases).

The following is the most basic usage pattern:

```
> gltfgen [FLAGS] [OPTIONS] <OUTPUT> <PATTERN>
```

  - `<OUTPUT>`     Output glTF file

  - `<PATTERN>`    A glob pattern matching files to be included in the generated glTF document. Use `#` to match a frame number. Use '{' and '}' to select parts of the pattern to be used to name meshes in the output glTF.

Run `gltfgen -h` for more options and `gltfgen --help` for full details.


# Examples

The following example assumes that there is a sequence of meshes located at
`./meshes/animation_#.vtk` where `#` represents the frame number.
To generate an animated binary glTF file named `output.glb` in the current directory, run:

```
> gltfgen output.glb "./meshes/animation_#.vtk"
```

This will assume 24 frames per second (FPS). You can specify FPS manually with the `-f` option as
follows:

```
> gltfgen -f 100 output.glb "./meshes/animation_#.vtk"
```

Alternatively, to specify a time step like 0.01 seconds between frames, use the `-t` option:

```
> gltfgen -t 0.01 output.glb "./meshes/animation_#.vtk"
```


# Features

## Input Types

 - Unstructured Legacy VTK polygon and tetrahedral meshes in double or float format.
   Tetrahedral VTK meshes are converted to triangle meshes on the fly.
 - Basic wavefront obj files containing polygon meshes (no .mtl support yet).
 - JPEG and PNG image textures are supported.

## Output Types

 - glTF 2.0 in binary and standard formats.

## Other Features

 - Multiple mesh file sequences can be embedded into a single glTF file
   automatically.
 - Non-numbered mesh files will be placed at frame 0 if captured by the glob
   pattern.
 - Skip frames with `-s` flag to reduce file size and improve performance.
 - Images textures can be referenced or embedded directly into the glTF file.
 - Full support for
    - color attributes,
    - texture attributes,
    - custom attributes,
 - Full support for textures.
 - Material attribute on vtk primitives is used to reference specific materials
   provided on the command line.

# License

This repository is licensed under the [Mozilla Public License, v. 2.0](https://mozilla.org/MPL/2.0/).

# Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for details.
