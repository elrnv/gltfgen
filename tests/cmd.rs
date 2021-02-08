use assert_cmd::Command;
use gltf::{Error, Gltf};
use predicates::prelude::*;

mod utils;
use utils::*;

#[test]
fn box_triangulated() -> Result<(), Error> {
    let mut cmd = Command::cargo_bin("gltfgen").unwrap();
    let stderr = "Material ID was found but no materials were specified.";
    cmd.arg("./tests/artifacts/box_triangulated.glb")
        .arg("./assets/{box_triangulated}.vtk")
        .arg("-a")
        .arg("{\"pressure\": f32}")
        .assert()
        .stderr(predicate::str::contains(stderr)) // No errors
        .success();

    let expected = Gltf::open("./assets/box_triangulated_expected.glb")?;
    let actual = Gltf::open("./tests/artifacts/box_triangulated.glb")?;

    assert_eq_gltf(&expected, &actual);
    Ok(())
}

#[test]
fn box_rotate_simple() -> Result<(), Error> {
    let mut cmd = Command::cargo_bin("gltfgen").unwrap();
    let stderr = "Material ID was found but no materials were specified.";
    cmd.arg("./tests/artifacts/box_rotate_simple.glb")
        .arg("./assets/{box_rotate}_#.vtk")
        .assert()
        .stderr(predicate::str::contains(stderr)) // No errors
        .success();

    let expected = Gltf::open("./assets/box_rotate_simple_expected.glb")?;
    let actual = Gltf::open("./tests/artifacts/box_rotate_simple.glb")?;

    assert_eq_gltf_with_bytes(&expected, &actual);
    Ok(())
}

#[test]
fn box_rotate_obj() -> Result<(), Error> {
    let mut cmd = Command::cargo_bin("gltfgen").unwrap();
    cmd.arg("./tests/artifacts/box_rotate_obj.glb")
        .arg("./assets/{box_rotate}_#.obj")
        .arg("-x")
        .arg("(image: Embed(\"./assets/checker16.png\"))")
        .arg("-u")
        .arg("{\"uv\": f32}")
        .arg("-m")
        .arg("(name:\"checkerboard\")")
        .assert()
        .stderr(b"" as &[u8]) // No errors
        .success();

    let expected = Gltf::open("./assets/box_rotate_expected.glb")?;
    let actual = Gltf::open("./tests/artifacts/box_rotate.glb")?;

    assert_eq_gltf_with_bytes(&expected, &actual);
    Ok(())
}

#[test]
fn box_rotate() -> Result<(), Error> {
    let mut cmd = Command::cargo_bin("gltfgen").unwrap();
    cmd.arg("./tests/artifacts/box_rotate.glb")
        .arg("./assets/{box_rotate}_#.vtk")
        .arg("-x")
        .arg("(image: Embed(\"./assets/checker16.png\"))")
        .arg("-u")
        .arg("{\"uv\": f32}")
        .arg("-m")
        .arg("(name:\"checkerboard\")")
        .assert()
        .stderr(b"" as &[u8]) // No errors
        .success();

    let expected = Gltf::open("./assets/box_rotate_expected.glb")?;
    let actual = Gltf::open("./tests/artifacts/box_rotate.glb")?;

    assert_eq_gltf_with_bytes(&expected, &actual);
    Ok(())
}

#[test]
fn box_rotate_attribs() -> Result<(), Error> {
    let mut cmd = Command::cargo_bin("gltfgen").unwrap();
    cmd.arg("./tests/artifacts/box_rotate_pressure.glb")
        .arg("./assets/{box_rotate}_#.vtk")
        .arg("-x")
        .arg("(image: Embed(\"./assets/checker16.png\"))")
        .arg("-u")
        .arg("{\"uv\": f32}")
        .arg("-a")
        .arg("{\"pressure\": f32}")
        .arg("-c")
        .arg("{\"Cd\": vec3(f32)}")
        .arg("-m")
        .arg("(name:\"checkerboard\")")
        .assert()
        .stderr(b"" as &[u8]) // No errors
        .success();

    let expected = Gltf::open("./assets/box_rotate_pressure_expected.glb")?;
    let actual = Gltf::open("./tests/artifacts/box_rotate_pressure.glb")?;

    assert_eq_gltf_with_bytes(&expected, &actual);
    Ok(())
}

#[test]
fn box_rotate_attribs_gltf() -> Result<(), Error> {
    let mut cmd = Command::cargo_bin("gltfgen").unwrap();
    cmd.arg("./tests/artifacts/box_rotate_pressure.gltf")
        .arg("./assets/{box_rotate}_#.vtk")
        .arg("-x")
        .arg("(image: Embed(\"./assets/checker16.png\"))")
        .arg("-u")
        .arg("{\"uv\": f32}")
        .arg("-a")
        .arg("{\"pressure\": f32}")
        .arg("-c")
        .arg("{\"Cd\": vec3(f32)}")
        .arg("-m")
        .arg("(name:\"checkerboard\")")
        .assert()
        .stderr(b"" as &[u8]) // No errors
        .success();

    let expected = Gltf::open("./assets/box_rotate_pressure_expected.glb")?;
    let actual = Gltf::open("./tests/artifacts/box_rotate_pressure.gltf")?;

    assert_eq_gltf(&expected, &actual);
    Ok(())
}

#[test]
fn tet() -> Result<(), Error> {
    let mut cmd = Command::cargo_bin("gltfgen").unwrap();
    let warning = "Material ID was found but no materials were specified.";
    cmd.arg("./tests/artifacts/tet.glb")
        .arg("./assets/{tet}_#.vtk")
        .arg("-a")
        .arg("{\"pressure\": f32}")
        .assert()
        .stderr(predicate::str::contains(warning))
        .success();

    let expected = Gltf::open("./assets/tet_expected.glb")?;
    let actual = Gltf::open("./tests/artifacts/tet.glb")?;

    assert_eq_gltf_with_bytes(&expected, &actual);
    Ok(())
}

#[test]
fn multi() -> Result<(), Error> {
    // Capture both tet and box_rotate animations in one glb fle.
    let mut cmd = Command::cargo_bin("gltfgen").unwrap();
    let warning1 = "Material ID was found but no materials were specified.";
    let warning2 = "Path 'assets/box_triangulated.vtk' skipped since regex '^assets/([^/]*)_(?P<frame>[0-9]+)\\.vtk$' did not match.";
    cmd.arg("./tests/artifacts/multi.glb")
        .arg("./assets/{*}_#.vtk")
        .arg("-a")
        .arg("{\"pressure\": f32}")
        .assert()
        .stderr(predicate::str::contains(warning1).and(predicate::str::contains(warning2)))
        .success();

    let expected = Gltf::open("./assets/multi_expected.glb")?;
    let actual = Gltf::open("./tests/artifacts/multi.glb")?;

    assert_eq_gltf_with_bytes(&expected, &actual);
    Ok(())
}
