# CHANGELOG

This document outlines changes and updates in major releases of `gltfgen`.

# Release 0.8

New major release, with lots of new features and bug fixes:

- The output file path is now specified by the optional `-o` flag instead of a required positional argument. The default value is `./out.glb`.
- Added support for custom configuration files.
  - Specify a configuration file on the input with the `--config` flag.
  - Print the currently used configuration in JSON or RON format using `--print-json-config` or `--print-ron-config`. This can be piped directly to an external file and used in a subsequent call to `gltfgen`.
- Support animated **normal** and **tangent** vertex attributes in animations specified via "N" and "T" `vec3(f32)`` attributes on input meshes.
  - You can disable animated normals or tangents using `--no-animated-normals` or `--no-animated-tangents` flags to save file size.
- The pattern for input files is still positional but now optional with a default value of `./#.obj`.
- Added an option to insert vanishing frames for each node in the output glTF file that has the positions of all vertices set to the origin before and after the animation sequence. This is a hack that allows one to create animations with sequences that have varying topology. The only caveat is that the resulting animation can only be played with a fixed frame rate to avoid flickering artifacts during the transition between the different animations.
- Fixed bug in sparse accessors in output gltf files.
- Added explicit names to many accessors on the output to make it easier to inspect and debug generated glTF files.
- Enabled having different materials on different parts of the same mesh.
- Added sensible defaults making `gltfgen` more useful for common use cases. For instance, now by default, normals, texture coordinates and materials are automatically read from input meshes.
- Improved error messages.


# Release 0.5

This release most notably adds support for reading VTK files in the modern XML format.
This includes `.vtu` for tetrahedral meshes and polygon meshes, `.vtp` for polygon meshes and
`.pvtu` and `.pvtp` for their "parallel" counterparts.

Minor changes include:
- Fixed specifying `base_texture` under the `--materials` (or `-m`) flag. Previously the keyword `Some` was erroneously required.
- Clarified what `base_texture` is in the CLI docs.
- Fixed specifying a vertex based texture coordinate. In contrast to face-vertex UV coordinates, which require extra work to convert into glTF format, vertex uvs are directly compatible with glTF. They were previously incorrectly ignored.
- Added names to accessors. This makes it easier inspect glTFs and possibly provide extra feedback to the sources of data when converting to glTF.

A few maintenance updates include:
- Improved testing coverage.
- Refactored out texture building in `export.rs`.