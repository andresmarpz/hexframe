use serde::Serialize;

#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub(crate) struct LiveObjects {
    pub(crate) count: u32,
    pub(crate) lines: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub(crate) struct GpuReport {
    pub(crate) debug_layer: bool,
    pub(crate) device_removed: u32,
    pub(crate) live_objects: Vec<LiveObjects>,
}

impl GpuReport {
    pub(crate) fn notes(&self) -> Vec<String> {
        if !self.debug_layer {
            return Vec::new();
        }
        if self.live_objects.is_empty() {
            return vec![
                "d3d debug layer: no live object report; is the Graphics Tools feature installed?"
                    .to_owned(),
            ];
        }
        let counts: Vec<String> = self
            .live_objects
            .iter()
            .map(|r| r.count.to_string())
            .collect();
        vec![format!(
            "d3d live objects at the end of each capture session: {} (lines in the report; the device itself is expected)",
            counts.join(", ")
        )]
    }
}

#[cfg(windows)]
pub(crate) fn enable_debug_layer() {
    greyscreen_capture::debug_layer::enable();
}

#[cfg(not(windows))]
pub(crate) fn enable_debug_layer() {
    eprintln!("note: --d3d-debug needs Windows and has no effect here");
}

#[cfg(windows)]
pub(crate) fn collect(debug_layer: bool) -> GpuReport {
    GpuReport {
        debug_layer,
        device_removed: greyscreen_capture::debug_layer::device_removed_count(),
        live_objects: greyscreen_capture::debug_layer::take_live_reports()
            .into_iter()
            .map(|r| LiveObjects {
                count: r.count,
                lines: r.lines,
            })
            .collect(),
    }
}

#[cfg(not(windows))]
pub(crate) fn collect(debug_layer: bool) -> GpuReport {
    GpuReport {
        debug_layer,
        ..GpuReport::default()
    }
}
