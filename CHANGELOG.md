# CHANGELOG

This document outlines changes and updates in major releases of `gltfgen`.


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