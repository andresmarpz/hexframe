use greyscreen_core::library::storyboard::{Storyboard, Tile};
use rune::{El, col, el, text};
use rune_kit::{Kit, Placement, popover};

use super::tabular;
use super::timeline::PREVIEW_ANCHOR;
use crate::ui::{ReviewState, Ui, format_clock};
use crate::view::thumb::ready_image;

const ANCHOR_GAP: f32 = 8.0;
const PADDING: f32 = 6.0;
const TILE_RADIUS: f32 = 4.0;

pub(crate) fn seek_preview(s: &Ui, k: &Kit) -> Option<El> {
    let review = s.review.as_ref()?;
    if review.hovered_event().is_some() {
        return None;
    }
    let secs = review.seek_preview()?;
    let dark = Kit::neutral(true);
    let k = if review.theater { &dark } else { k };
    let surface = popover::raised(k, body(review, secs, k), k.radius.md);
    Some(popover::anchored(
        k,
        surface,
        PREVIEW_ANCHOR,
        Placement::Top,
        ANCHOR_GAP,
    ))
}

fn body(review: &ReviewState, secs: f64, k: &Kit) -> El {
    let clock = text(format_clock(secs.max(0.0) as u32))
        .font(tabular(k.ty.small))
        .color(k.c.text_base);
    let mut card = col()
        .key("seek-preview")
        .padding(PADDING)
        .gap(4.0)
        .items_center()
        .a11y_hidden();
    if let (Some(board), Some((sheet, tile))) =
        (review.storyboard.as_ref(), review.preview_tile(secs))
    {
        card = card.child(tile_view(board, &sheet, tile, k));
    }
    card.child(clock)
}

fn tile_view(board: &Storyboard, sheet: &std::path::Path, tile: Tile, k: &Kit) -> El {
    let frame = el()
        .key("seek-preview-tile")
        .size(board.tile_width as f32, board.tile_height as f32)
        .radius(TILE_RADIUS)
        .clip_rounded(TILE_RADIUS)
        .fill(k.c.layer_02);
    let Some(image) = ready_image(sheet) else {
        return frame;
    };
    let (width, height) = board.sheet_size();
    frame.child(
        el().size(width as f32, height as f32)
            .pin(Some(-(tile.x as f32)), Some(-(tile.y as f32)), None, None)
            .pixels(image.pixels()),
    )
}
