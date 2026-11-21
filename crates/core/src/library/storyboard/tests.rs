use super::*;

fn board(duration: f64) -> Storyboard {
    Storyboard::plan(duration).expect("plan")
}

#[test]
fn short_clips_get_a_frame_every_second() {
    assert_eq!(interval_for(10.0), 1);
    assert_eq!(interval_for(500.0), 1);
    let plan = board(30.0);
    assert_eq!((plan.interval_secs, plan.frames), (1, 31));
}

#[test]
fn a_twenty_minute_game_gets_about_four_hundred_frames() {
    let plan = board(1200.0);
    assert_eq!((plan.interval_secs, plan.frames), (3, 401));
    assert_eq!(plan.sheets.len(), 5);
}

#[test]
fn long_games_stop_at_six_seconds_a_frame() {
    assert_eq!(interval_for(3600.0), 6);
    assert_eq!(interval_for(10_000.0), 6);
    assert_eq!(board(3600.0).frames, 601);
}

#[test]
fn recordings_under_a_second_get_no_plan() {
    assert_eq!(Storyboard::plan(0.0), None);
    assert_eq!(Storyboard::plan(0.9), None);
    assert_eq!(Storyboard::plan(f64::NAN), None);
}

#[test]
fn frame_times_step_by_the_interval_from_zero() {
    let plan = board(1200.0);
    let times: Vec<f64> = plan.times().collect();
    assert_eq!(times.len(), plan.frames as usize);
    assert_eq!(&times[..3], &[0.0, 3.0, 6.0]);
    assert_eq!(times.last(), Some(&1200.0));
}

#[test]
fn tiles_fill_a_sheet_row_by_row_then_start_the_next_sheet() {
    let plan = board(1200.0);
    let tile = |secs| plan.tile_at(secs).expect("tile");
    assert_eq!(
        tile(0.0),
        Tile {
            sheet: 0,
            x: 0,
            y: 0
        }
    );
    assert_eq!(
        tile(9.0),
        Tile {
            sheet: 0,
            x: 3 * 160,
            y: 0
        }
    );
    assert_eq!(
        tile(30.0),
        Tile {
            sheet: 0,
            x: 0,
            y: 90
        }
    );
    assert_eq!(
        tile(297.0),
        Tile {
            sheet: 0,
            x: 9 * 160,
            y: 9 * 90
        }
    );
    assert_eq!(
        tile(300.0),
        Tile {
            sheet: 1,
            x: 0,
            y: 0
        }
    );
    assert_eq!(
        tile(1200.0),
        Tile {
            sheet: 4,
            x: 0,
            y: 0
        }
    );
}

#[test]
fn a_time_picks_the_nearest_frame_and_clamps_to_the_ends() {
    let plan = board(1200.0);
    assert_eq!(plan.tile_at(1.4), plan.tile_at(0.0));
    assert_eq!(plan.tile_at(1.6), plan.tile_at(3.0));
    assert_eq!(plan.tile_at(-5.0), plan.tile_at(0.0));
    assert_eq!(plan.tile_at(99_999.0), plan.tile_at(1200.0));
}

#[test]
fn a_damaged_index_gives_no_tile() {
    let mut plan = board(60.0);
    plan.sheets.clear();
    assert_eq!(plan.tile_at(5.0), None);
    let mut plan = board(60.0);
    plan.columns = 0;
    assert_eq!(plan.tile_at(5.0), None);
}

#[test]
fn the_index_round_trips_through_json() {
    let plan = board(1200.0);
    let json = serde_json::to_string(&plan).unwrap();
    assert_eq!(serde_json::from_str::<Storyboard>(&json).unwrap(), plan);
    assert!(json.contains("\"interval_secs\":3"));
    assert!(json.contains("\"storyboard-0.jpg\""));
}

fn folder_with_sheets(plan: &Storyboard) -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    for sheet in &plan.sheets {
        fs::write(dir.path().join(sheet), b"jpeg").unwrap();
    }
    dir
}

#[test]
fn a_written_index_reads_back_when_its_sheets_exist() {
    let plan = board(120.0);
    let dir = folder_with_sheets(&plan);
    assert_eq!(read(dir.path()), None);
    write(dir.path(), &plan).unwrap();
    assert_eq!(read(dir.path()), Some(plan));
}

#[test]
fn an_index_with_a_missing_sheet_does_not_read() {
    let plan = board(1200.0);
    let dir = folder_with_sheets(&plan);
    write(dir.path(), &plan).unwrap();
    fs::remove_file(dir.path().join(&plan.sheets[2])).unwrap();
    assert_eq!(read(dir.path()), None);
}

#[test]
fn an_index_naming_a_path_outside_the_folder_does_not_read() {
    let mut plan = board(60.0);
    plan.sheets = vec!["../game.mp4".into()];
    let dir = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join("storyboard.json"),
        serde_json::to_vec(&plan).unwrap(),
    )
    .unwrap();
    assert_eq!(read(dir.path()), None);
}

#[test]
fn unreadable_or_future_indexes_do_not_read() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join(STORYBOARD_FILE), b"{ nope").unwrap();
    assert_eq!(read(dir.path()), None);
    let mut plan = board(60.0);
    plan.version = 9;
    let dir = folder_with_sheets(&plan);
    write(dir.path(), &plan).unwrap();
    assert_eq!(read(dir.path()), None);
}

#[test]
fn removing_a_storyboard_leaves_the_other_files() {
    let plan = board(1200.0);
    let dir = folder_with_sheets(&plan);
    write(dir.path(), &plan).unwrap();
    fs::write(dir.path().join("storyboard-9.jpg.part"), b"x").unwrap();
    fs::write(dir.path().join("game.mp4"), b"video").unwrap();
    fs::write(dir.path().join("thumb.jpg"), b"thumb").unwrap();
    assert!(bytes(dir.path()) > 0);
    remove(dir.path());
    let mut left: Vec<String> = fs::read_dir(dir.path())
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    left.sort();
    assert_eq!(left, ["game.mp4", "thumb.jpg"]);
    assert_eq!(bytes(dir.path()), 0);
}
