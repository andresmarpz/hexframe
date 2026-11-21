use super::*;

fn solid(width: u32, height: u32, rgb: [u8; 3]) -> Rgba {
    let pixels = (0..width * height)
        .flat_map(|_| [rgb[0], rgb[1], rgb[2], 255])
        .collect();
    Rgba {
        width,
        height,
        pixels,
    }
}

#[test]
fn small_differences_stay_within_tolerance() {
    let diff = compare(
        &solid(4, 4, [100, 100, 100]),
        &solid(4, 4, [110, 95, 100]),
        16,
    );
    assert_eq!(diff.changed, 0);
    assert!(diff.within(0.0));
}

#[test]
fn changed_pixels_are_counted_and_painted() {
    let golden = solid(4, 4, [100, 100, 100]);
    let mut actual = golden.clone();
    actual.pixels[0] = 250;
    let diff = compare(&golden, &actual, 16);
    assert_eq!((diff.changed, diff.total), (1, 16));
    assert!(!diff.within(0.05));
    assert!(diff.within(0.1));
    assert_eq!(&diff.image.expect("image").pixels[..4], &[255, 32, 64, 255]);
}

#[test]
fn a_size_change_always_fails() {
    let diff = compare(&solid(4, 4, [0, 0, 0]), &solid(4, 5, [0, 0, 0]), 255);
    assert!(!diff.within(1.0));
}

#[test]
fn images_round_trip_through_png() {
    let dir = tempfile::tempdir().expect("dir");
    let path = dir.path().join("a.png");
    let image = solid(3, 2, [1, 2, 3]);
    image.write(&path).expect("write");
    assert_eq!(Rgba::read(&path).expect("read"), image);
}
