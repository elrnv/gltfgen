//!
//! This module tests the command line tool directly by invoking it on assests stored in the `assets` directory.
//!
//! What to look for in test results when evaluating regressions:
//!   - Animated attributes are not supported yet, expect textures to stretch and color to remain constant.
//!
use assert_cmd::Command;
use gltf::{Error, Gltf};
use predicates::prelude::*;

mod utils;
use utils::*;

#[test]
fn box_triangulated() -> Result<(), Error> {
    let mut cmd = Command::cargo_bin("gltfgen").unwrap();
    let stderr = "Material ID was found but no materials were specified.";
    let artifact = "./tests/artifacts/box_triangulated.glb";
    cmd.arg(artifact)
        .arg("./assets/{box_triangulated}.vtk")
        .arg("-r") // reverse polygon orientation
        .arg("-a")
        .arg("{\"pressure\": f32}")
        .assert()
        .stderr(predicate::str::contains(stderr))
        .success();

    let expected = Gltf::open("./assets/box_triangulated_expected.glb")?;
    let actual = Gltf::open(artifact)?;

    assert_eq_gltf(&expected, &actual);
    Ok(())
}

#[test]
fn box_rotate_simple() -> Result<(), Error> {
    let mut cmd = Command::cargo_bin("gltfgen").unwrap();
    let stderr = "Material ID was found but no materials were specified.";
    let artifact = "./tests/artifacts/box_rotate_simple.glb";
    cmd.arg(artifact)
        .arg("./assets/{box_rotate}_#.vtk")
        .arg("-r") // reverse polygon orientation
        .assert()
        .stderr(predicate::str::contains(stderr))
        .success();

    let expected = Gltf::open("./assets/box_rotate_simple_expected.glb")?;
    let actual = Gltf::open(artifact)?;

    assert_eq_gltf_with_bytes(&expected, &actual);
    Ok(())
}

#[test]
fn box_rotate_obj() -> Result<(), Error> {
    let mut cmd = Command::cargo_bin("gltfgen").unwrap();
    let artifact = "./tests/artifacts/box_rotate_obj.glb";
    cmd.arg(artifact)
        .arg("./assets/{box_rotate}_#.obj")
        .arg("-r") // reverse polygon orientation
        .arg("-x")
        .arg("(image: Embed(\"./assets/checker16.png\"))")
        .arg("-u")
        .arg("{\"uv\": f32}")
        .arg("-m")
        .arg("(name:\"checkerboard\", base_texture:(index:0,texcoord:0))")
        .assert()
        .stderr(b"" as &[u8])
        .success();

    // The reason this has a different result than box_rotate is because in this case
    // the saved objs have 2D uv coordinates instead of 3D.
    let expected = Gltf::open("./assets/box_rotate_2Duv_expected.glb")?;
    let actual = Gltf::open(artifact)?;

    assert_eq_gltf_with_bytes(&expected, &actual);
    Ok(())
}

#[test]
fn box_rotate() -> Result<(), Error> {
    let mut cmd = Command::cargo_bin("gltfgen").unwrap();
    let artifact = "./tests/artifacts/box_rotate.glb";
    cmd.arg(artifact)
        .arg("./assets/{box_rotate}_#.vtk")
        .arg("-r") // reverse polygon orientation
        .arg("-x")
        .arg("(image: Embed(\"./assets/checker16.png\"))")
        .arg("-u")
        .arg("{\"uv\": f32}")
        .arg("-m")
        .arg("(name:\"checkerboard\", base_texture:(index:0,texcoord:0))")
        .assert()
        .stderr(b"" as &[u8]) // No errors
        .success();

    let expected = Gltf::open("./assets/box_rotate_expected.glb")?;
    let actual = Gltf::open(artifact)?;

    assert_eq_gltf_with_bytes(&expected, &actual);
    Ok(())
}

#[test]
fn box_rotate_attribs() -> Result<(), Error> {
    let mut cmd = Command::cargo_bin("gltfgen").unwrap();
    let artifact = "./tests/artifacts/box_rotate_pressure.glb";
    cmd.arg(artifact)
        .arg("./assets/{box_rotate}_#.vtk")
        .arg("-r") // reverse polygon orientation
        .arg("-x")
        .arg("(image: Embed(\"./assets/checker16.png\"))")
        .arg("-u")
        .arg("{\"uv\": f32}")
        .arg("-a")
        .arg("{\"pressure\": f32}")
        .arg("-c")
        .arg("{\"Cd\": vec3(f32)}")
        .arg("-m")
        .arg("(name:\"checkerboard\", base_texture:(index:0,texcoord:0))")
        .assert()
        .stderr(b"" as &[u8]) // No errors
        .success();

    let expected = Gltf::open("./assets/box_rotate_pressure_expected.glb")?;
    let actual = Gltf::open(artifact)?;

    assert_eq_gltf_with_bytes(&expected, &actual);
    Ok(())
}

#[test]
fn box_rotate_attribs_gltf() -> Result<(), Error> {
    let mut cmd = Command::cargo_bin("gltfgen").unwrap();
    let artifact = "./tests/artifacts/box_rotate_pressure.gltf";
    cmd.arg(artifact)
        .arg("./assets/{box_rotate}_#.vtk")
        .arg("-r") // reverse polygon orientation
        .arg("-x")
        .arg("(image: Embed(\"./assets/checker16.png\"))")
        .arg("-u")
        .arg("{\"uv\": f32}")
        .arg("-a")
        .arg("{\"pressure\": f32}")
        .arg("-c")
        .arg("{\"Cd\": vec3(f32)}")
        .arg("-m")
        .arg("(name:\"checkerboard\", base_texture:(index:0,texcoord:0))")
        .assert()
        .stderr(b"" as &[u8]) // No errors
        .success();

    let expected = Gltf::open("./assets/box_rotate_pressure_expected.glb")?;
    let actual = Gltf::open(artifact)?;

    assert_eq_gltf(&expected, &actual);
    Ok(())
}

#[test]
fn tet() -> Result<(), Error> {
    let mut cmd = Command::cargo_bin("gltfgen").unwrap();
    let warning = "Material ID was found but no materials were specified.";
    let artifact = "./tests/artifacts/tet.glb";
    cmd.arg(artifact)
        .arg("./assets/{tet}_#.vtk")
        .arg("-a")
        .arg("{\"pressure\": f32}")
        .assert()
        .stderr(predicate::str::contains(warning))
        .success();

    let expected = Gltf::open("./assets/tet_expected.glb")?;
    let actual = Gltf::open(artifact)?;

    assert_eq_gltf_with_bytes(&expected, &actual);
    Ok(())
}

#[test]
fn tet_and_tri() -> Result<(), Error> {
    let mut cmd = Command::cargo_bin("gltfgen").unwrap();
    let warning = "Material ID was found but no materials were specified.";
    let artifact = "./tests/artifacts/tet_and_tri.glb";
    cmd.arg(artifact)
        .arg("./assets/{tet_and_tri}_#.vtk")
        .arg("-a")
        .arg("{\"pressure\": f32}")
        .assert()
        .stderr(predicate::str::contains(warning))
        .success();

    let expected = Gltf::open("./assets/tet_and_tri_expected.glb")?;
    let actual = Gltf::open(artifact)?;

    assert_eq_gltf_with_bytes(&expected, &actual);
    Ok(())
}

#[test]
fn multi() -> Result<(), Error> {
    // Capture both tet and box_rotate animations in one glb file.
    let mut cmd = Command::cargo_bin("gltfgen").unwrap();
    let warning1 = "Material ID was found but no materials were specified.";
    let warning2 = "Path 'assets/box_triangulated.vtk' skipped since regex '^assets/([^/]*)_(?P<frame>[0-9]+)\\.vtk$' did not match.";
    let artifact = "./tests/artifacts/multi.glb";
    cmd.arg(artifact)
        .arg("./assets/{*}_#.vtk")
        .arg("-r") // reverse polygon orientation
        .arg("-a")
        .arg("{\"pressure\": f32}")
        .assert()
        .stderr(predicate::str::contains(warning1).and(predicate::str::contains(warning2)))
        .success();

    let expected = Gltf::open("./assets/multi_expected.glb")?;
    let actual = Gltf::open(artifact)?;

    assert_eq_gltf_with_bytes(&expected, &actual);
    Ok(())
}
