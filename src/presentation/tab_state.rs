//! Tab State Management
//!
//! Each open file gets a `TabState` holding its own camera, layer navigation,
//! display toggles, and vector playback state — fully independent of other tabs.
//!
//! `TabManager` tracks the ordered list of open tabs, the active tab, and the
//! split-mode partner.

use std::collections::HashMap;

use crate::application::ports::ViewState;
use crate::application::use_cases::NavigateLayersUseCase;
use crate::domain::entities::SliceStack;

// ── Per-tab state ────────────────────────────────────────────────────────

/// Fully independent state for one tab (one file).
pub struct TabState {
    /// File ID this tab represents
    pub file_id: usize,

    /// Camera state (center, zoom, rotation)
    pub view_state: ViewState,

    /// Layer navigation (current index, z-heights, helpers)
    pub navigation: NavigateLayersUseCase,

    // ── Display toggles ──
    pub show_slices: bool,
    pub show_contours: bool,
    pub show_hatches: bool,
    pub show_arrows: bool,
    pub show_wait_markers: bool,

    // ── Vector playback ──
    pub vector_view_enabled: bool,
    pub current_vector_index: usize,
    pub total_vectors_in_layer: usize,
    pub vector_view_playing: bool,
    pub playback_time_accumulator: f64,
    pub playback_speed: f32,
}

impl TabState {
    /// Create a fresh tab for a file, initialising navigation from its layers.
    pub fn new(file_id: usize, stack: &SliceStack) -> Self {
        let mut navigation = NavigateLayersUseCase::new();
        navigation.initialize(stack);

        Self {
            file_id,
            view_state: ViewState::default(),
            navigation,
            show_slices: true,
            show_contours: true,
            show_hatches: true,
            show_arrows: false,
            show_wait_markers: false,
            vector_view_enabled: false,
            current_vector_index: 0,
            total_vectors_in_layer: 0,
            vector_view_playing: false,
            playback_time_accumulator: 0.0,
            playback_speed: 1.0,
        }
    }
}

// ── Tab manager ──────────────────────────────────────────────────────────

/// Manages open tabs and their states.
pub struct TabManager {
    tabs: HashMap<usize, TabState>,
    /// Ordered list of open tab file-IDs (tab-bar display order).
    pub open_tab_ids: Vec<usize>,
    /// Currently active (focused) tab.
    pub active_tab_id: Option<usize>,
    /// Second file shown in Split mode.
    pub split_partner_id: Option<usize>,
}

impl TabManager {
    pub fn new() -> Self {
        Self {
            tabs: HashMap::new(),
            open_tab_ids: Vec::new(),
            active_tab_id: None,
            split_partner_id: None,
        }
    }

    /// Open (or reopen) a tab for the given file.
    /// If a TabState already exists it is reused; otherwise a fresh one is created.
    pub fn open_tab(&mut self, file_id: usize, stack: &SliceStack) {
        if !self.tabs.contains_key(&file_id) {
            self.tabs.insert(file_id, TabState::new(file_id, stack));
        }
        if !self.open_tab_ids.contains(&file_id) {
            self.open_tab_ids.push(file_id);
        }
        // Auto-activate the first tab
        if self.active_tab_id.is_none() {
            self.active_tab_id = Some(file_id);
        }
    }

    /// Close a tab (hide file). The `TabState` is kept so it can be reopened.
    pub fn close_tab(&mut self, file_id: usize) {
        self.open_tab_ids.retain(|&id| id != file_id);
        if self.active_tab_id == Some(file_id) {
            self.active_tab_id = self.open_tab_ids.first().copied();
        }
        if self.split_partner_id == Some(file_id) {
            self.split_partner_id = self.open_tab_ids.iter()
                .find(|&&id| Some(id) != self.active_tab_id)
                .copied();
        }
    }

    /// Reopen a previously closed tab (no-op if already open or unknown).
    pub fn reopen_tab(&mut self, file_id: usize) {
        if self.tabs.contains_key(&file_id) && !self.open_tab_ids.contains(&file_id) {
            self.open_tab_ids.push(file_id);
        }
    }

    /// Set the active tab.
    pub fn set_active(&mut self, file_id: usize) {
        if self.open_tab_ids.contains(&file_id) {
            self.active_tab_id = Some(file_id);
        }
    }

    // ── Accessors ──

    pub fn active_tab(&self) -> Option<&TabState> {
        self.active_tab_id.and_then(|id| self.tabs.get(&id))
    }

    pub fn active_tab_mut(&mut self) -> Option<&mut TabState> {
        self.active_tab_id.and_then(|id| self.tabs.get_mut(&id))
    }

    pub fn split_partner(&self) -> Option<&TabState> {
        self.split_partner_id.and_then(|id| self.tabs.get(&id))
    }

    pub fn tab(&self, file_id: usize) -> Option<&TabState> {
        self.tabs.get(&file_id)
    }

    pub fn tab_mut(&mut self, file_id: usize) -> Option<&mut TabState> {
        self.tabs.get_mut(&file_id)
    }

    /// Permanently remove a file's tab state (when the file is unloaded).
    pub fn remove_tab(&mut self, file_id: usize) {
        self.tabs.remove(&file_id);
        self.open_tab_ids.retain(|&id| id != file_id);
        if self.active_tab_id == Some(file_id) {
            self.active_tab_id = self.open_tab_ids.first().copied();
        }
        if self.split_partner_id == Some(file_id) {
            self.split_partner_id = None;
        }
    }

    pub fn has_tabs(&self) -> bool {
        !self.open_tab_ids.is_empty()
    }

    pub fn tab_count(&self) -> usize {
        self.open_tab_ids.len()
    }
}
