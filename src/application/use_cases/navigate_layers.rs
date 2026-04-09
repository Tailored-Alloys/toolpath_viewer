//! Navigate Layers Use Case
//!
//! Handles layer navigation and state management.

use crate::domain::entities::{Layer, SliceStack};

/// State for layer navigation
#[derive(Debug, Clone)]
pub struct LayerNavigationState {
    /// Current layer index
    pub current_index: usize,
    /// Total number of layers
    pub total_layers: usize,
    /// Current Z height
    pub current_z: f32,
    /// All Z heights
    pub z_heights: Vec<f32>,
}

impl Default for LayerNavigationState {
    fn default() -> Self {
        Self {
            current_index: 0,
            total_layers: 0,
            current_z: 0.0,
            z_heights: Vec::new(),
        }
    }
}

/// Use case for navigating between layers
pub struct NavigateLayersUseCase {
    state: LayerNavigationState,
}

impl NavigateLayersUseCase {
    /// Create a new navigation use case
    pub fn new() -> Self {
        Self {
            state: LayerNavigationState::default(),
        }
    }

    /// Initialize with a slice stack
    pub fn initialize(&mut self, stack: &SliceStack) {
        self.state.total_layers = stack.layer_count();
        self.state.z_heights = stack.z_heights();
        self.state.current_index = 0;
        
        if let Some(layer) = stack.get_layer(0) {
            self.state.current_z = layer.z_height;
        }
    }

    /// Initialize from a pre-merged list of Z-heights (for multi-file mode).
    /// Preserves the current Z position by snapping to the closest Z in the new list.
    pub fn initialize_from_z_heights(&mut self, z_heights: Vec<f32>) {
        let prev_z = self.state.current_z;
        self.state.total_layers = z_heights.len();
        self.state.z_heights = z_heights;

        // Find closest Z-height to the previous position
        if let Some((idx, &z)) = self.state.z_heights.iter().enumerate().min_by(|(_, a), (_, b)| {
            let da = (*a - prev_z).abs();
            let db = (*b - prev_z).abs();
            da.partial_cmp(&db).unwrap()
        }) {
            self.state.current_index = idx;
            self.state.current_z = z;
        } else {
            self.state.current_index = 0;
            self.state.current_z = 0.0;
        }
    }

    /// Get current state
    pub fn state(&self) -> &LayerNavigationState {
        &self.state
    }

    /// Go to a specific layer by index
    pub fn go_to_layer(&mut self, index: usize) -> bool {
        if index < self.state.total_layers {
            self.state.current_index = index;
            if index < self.state.z_heights.len() {
                self.state.current_z = self.state.z_heights[index];
            }
            true
        } else {
            false
        }
    }

    /// Go to next layer
    pub fn next_layer(&mut self) -> bool {
        if self.state.current_index + 1 < self.state.total_layers {
            self.go_to_layer(self.state.current_index + 1)
        } else {
            false
        }
    }

    /// Go to previous layer
    pub fn previous_layer(&mut self) -> bool {
        if self.state.current_index > 0 {
            self.go_to_layer(self.state.current_index - 1)
        } else {
            false
        }
    }

    /// Go to first layer
    pub fn first_layer(&mut self) -> bool {
        self.go_to_layer(0)
    }

    /// Go to last layer
    pub fn last_layer(&mut self) -> bool {
        if self.state.total_layers > 0 {
            self.go_to_layer(self.state.total_layers - 1)
        } else {
            false
        }
    }

    /// Go to layer by Z height (finds closest)
    pub fn go_to_z(&mut self, z: f32) -> bool {
        if self.state.z_heights.is_empty() {
            return false;
        }

        // Find closest Z height
        let (closest_idx, _) = self.state.z_heights
            .iter()
            .enumerate()
            .min_by(|(_, a), (_, b)| {
                let da = (*a - z).abs();
                let db = (*b - z).abs();
                da.partial_cmp(&db).unwrap()
            })
            .unwrap();

        self.go_to_layer(closest_idx)
    }

    /// Jump forward by N layers
    pub fn jump_forward(&mut self, count: usize) -> bool {
        let target = (self.state.current_index + count).min(self.state.total_layers.saturating_sub(1));
        self.go_to_layer(target)
    }

    /// Jump backward by N layers
    pub fn jump_backward(&mut self, count: usize) -> bool {
        let target = self.state.current_index.saturating_sub(count);
        self.go_to_layer(target)
    }

    /// Get current layer from a slice stack
    pub fn get_current_layer<'a>(&self, stack: &'a SliceStack) -> Option<&'a Layer> {
        stack.get_layer(self.state.current_index)
    }
}

impl Default for NavigateLayersUseCase {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_stack() -> SliceStack {
        let mut stack = SliceStack::new("Test");
        for i in 0..10 {
            stack.add_layer(Layer::new(i, i as f32 * 0.1));
        }
        stack
    }

    #[test]
    fn test_navigation() {
        let stack = create_test_stack();
        let mut nav = NavigateLayersUseCase::new();
        nav.initialize(&stack);

        assert_eq!(nav.state().current_index, 0);
        assert_eq!(nav.state().total_layers, 10);

        nav.next_layer();
        assert_eq!(nav.state().current_index, 1);

        nav.last_layer();
        assert_eq!(nav.state().current_index, 9);

        nav.previous_layer();
        assert_eq!(nav.state().current_index, 8);

        nav.first_layer();
        assert_eq!(nav.state().current_index, 0);
    }
}
