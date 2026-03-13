//! CLI (Common Layer Interface) Parser
//!
//! Parses the CLI file format used in additive manufacturing.
//! CLI is a simple ASCII format with commands for layer data.
//!
//! Format specification:
//! - $$HEADERSTART / $$HEADEREND: Header section
//! - $$GEOMETRYSTART / $$GEOMETRYEND: Geometry section
//! - $$LAYER/z: Start a new layer at height z
//! - $$POLYLINE/id,dir,n,x1,y1,x2,y2,...: Polyline with n points
//! - $$HATCHES/id,n,x1,y1,x2,y2,...: Hatch lines (pairs of points)
//! - $$POWERS/id,value: Laser power for the layer
//! - $$SPEEDS/id,value: Laser speed for the layer
//! - $$WAIT/idx,time,...: Wait times between hatch vectors

use crate::application::ports::{FileError, FileResult};
use crate::domain::entities::{Layer, LayerParameters, SliceStack, Toolpath, ToolpathMetadata, Vector, VectorType};
use crate::domain::value_objects::Point2D;
use log::{debug, trace, warn};
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read};

/// CLI file parser
pub struct CliParser {
    /// Scale factor for coordinates (to convert to mm)
    scale: f32,
    /// Override the VectorType for hatches (e.g., Contour for VK files)
    hatch_type_override: Option<VectorType>,
    /// Source label (e.g., "vk" or "vs") attached to layer parameters
    source_label: Option<String>,
}

impl CliParser {
    /// Create a new CLI parser
    pub fn new() -> Self {
        Self { scale: 1.0, hatch_type_override: None, source_label: None }
    }

    /// Create a parser with a specific scale
    pub fn with_scale(scale: f32) -> Self {
        Self { scale, hatch_type_override: None, source_label: None }
    }

    /// Set the vector type override for hatches
    pub fn set_hatch_type_override(&mut self, vtype: VectorType) {
        self.hatch_type_override = Some(vtype);
    }

    /// Set the source label for this parse (e.g., "vk" or "vs")
    pub fn set_source_label(&mut self, label: impl Into<String>) {
        self.source_label = Some(label.into());
    }

    /// Parse CLI data from a reader
    /// 
    /// CLI files use $$ as command delimiters anywhere in the file (not just line starts).
    /// Commands can span multiple lines with coordinates continuing until the next $$ marker.
    pub fn parse<R: Read>(&self, reader: R) -> FileResult<Toolpath> {
        // Read entire content
        let mut content = String::new();
        let mut buf_reader = BufReader::new(reader);
        buf_reader.read_to_string(&mut content)
            .map_err(|e| FileError::IoError(e))?;
        
        let mut parser_state = CliParserState::new(self.scale);
        parser_state.hatch_type_override = self.hatch_type_override;
        parser_state.source_label = self.source_label.clone();
        
        // Split by $$ and process each command
        // The first split part (before first $$) might be empty or header data
        let parts: Vec<&str> = content.split("$$").collect();
        
        for (idx, part) in parts.iter().enumerate() {
            if idx == 0 && !part.trim().is_empty() {
                // Data before first $$ - likely header or continuation
                continue;
            }
            
            let trimmed = part.trim();
            if trimmed.is_empty() {
                continue;
            }
            
            // Remove all newlines and extra whitespace, normalize to single content
            let normalized: String = trimmed
                .chars()
                .map(|c| if c.is_whitespace() { ' ' } else { c })
                .collect::<String>()
                .split_whitespace()
                .collect::<Vec<&str>>()
                .join("");
            
            if !normalized.is_empty() {
                // Prepend $$ for process_line compatibility
                let command = format!("$${}", normalized);
                if let Err(e) = parser_state.process_line(&command, idx + 1) {
                    trace!("Parse issue at command {}: {:?}", idx, e);
                }
            }
        }

        parser_state.finalize()
    }
}

impl Default for CliParser {
    fn default() -> Self {
        Self::new()
    }
}

/// Internal parser state
struct CliParserState {
    scale: f32,
    in_header: bool,
    in_geometry: bool,
    current_layer: Option<Layer>,
    layers: Vec<Layer>,
    metadata: ToolpathMetadata,
    header_data: HashMap<String, String>,
    vector_id_counter: u64,
    /// Override vector type for hatches (e.g., Contour for VK files)
    hatch_type_override: Option<VectorType>,
    /// Source label for layer parameters
    source_label: Option<String>,
    /// Power commands: (start_vector_index, value) - sticky until next index
    power_commands: Vec<(u32, f32)>,
    /// Speed commands: (start_vector_index, value) - sticky until next index  
    speed_commands: Vec<(u32, f32)>,
    /// Current layer wait times: (vector_index, wait_time_us) - non-sticky
    current_wait_times: Vec<(u32, u32)>,
}

impl CliParserState {
    fn new(scale: f32) -> Self {
        Self {
            scale,
            in_header: false,
            in_geometry: false,
            current_layer: None,
            layers: Vec::new(),
            metadata: ToolpathMetadata::default(),
            header_data: HashMap::new(),
            vector_id_counter: 0,
            hatch_type_override: None,
            source_label: None,
            power_commands: Vec::new(),
            speed_commands: Vec::new(),
            current_wait_times: Vec::new(),
        }
    }

    fn process_line(&mut self, line: &str, line_num: usize) -> FileResult<()> {
        // Check for section markers
        if line.starts_with("$$HEADERSTART") {
            self.in_header = true;
            return Ok(());
        }
        if line.starts_with("$$HEADEREND") {
            self.in_header = false;
            self.process_header();
            return Ok(());
        }
        if line.starts_with("$$GEOMETRYSTART") {
            self.in_geometry = true;
            return Ok(());
        }
        if line.starts_with("$$GEOMETRYEND") {
            self.in_geometry = false;
            return Ok(());
        }

        // Process header data
        if self.in_header {
            self.process_header_line(line);
            return Ok(());
        }

        // Process geometry commands
        if line.starts_with("$$") {
            self.process_command(line, line_num)?;
        }

        Ok(())
    }

    fn process_header_line(&mut self, line: &str) {
        // Skip $$ prefix if present
        let line = line.trim_start_matches("$$");
        
        // Parse KEY/VALUE format
        if let Some(idx) = line.find('/') {
            let key = line[..idx].trim().to_uppercase();
            let value = line[idx + 1..].trim().to_string();
            self.header_data.insert(key, value);
        }
    }

    fn process_header(&mut self) {
        // Extract metadata from header
        if let Some(units) = self.header_data.get("UNITS") {
            // Parse units and set scale accordingly
            if units.to_uppercase().contains("MM") {
                self.scale = 1.0;
            } else if units.to_uppercase().contains("INCH") {
                self.scale = 25.4; // Convert inches to mm
            }
        }

        self.metadata.format = "CLI".to_string();
        
        if let Some(label) = self.header_data.get("LABEL") {
            self.metadata.properties.insert("label".to_string(), label.clone());
        }
    }

    fn process_command(&mut self, line: &str, line_num: usize) -> FileResult<()> {
        let line = line.trim_start_matches("$$");
        
        // Find command and parameters
        let (command, params) = if let Some(idx) = line.find('/') {
            (&line[..idx], &line[idx + 1..])
        } else {
            (line, "")
        };

        match command.to_uppercase().as_str() {
            "LAYER" => self.process_layer_command(params, line_num),
            "POLYLINE" => self.process_polyline_command(params, line_num),
            "HATCHES" => self.process_hatches_command(params, line_num),
            "HATCH" => self.process_hatches_command(params, line_num),
            "CONTOUR" => self.process_contour_command(params, line_num),
            "BOUNDARY" => self.process_boundary_command(params, line_num),
            "POWERS" => self.process_powers_command(params, line_num),
            "SPEEDS" => self.process_speeds_command(params, line_num),
            "WAIT" => self.process_wait_command(params, line_num),
            _ => {
                trace!("Ignoring unknown command: {}", command);
                Ok(())
            }
        }
    }

    fn process_layer_command(&mut self, params: &str, _line_num: usize) -> FileResult<()> {
        // Save current layer if exists, attaching accumulated parameters
        if let Some(mut layer) = self.current_layer.take() {
            self.attach_params_to_layer(&mut layer);
            self.layers.push(layer);
        }
        // Reset per-layer state
        self.power_commands.clear();
        self.speed_commands.clear();
        self.current_wait_times.clear();

        // Parse Z height
        let z: f32 = params
            .trim()
            .parse()
            .map_err(|_| FileError::ParseError {
                line: _line_num,
                message: format!("Invalid layer height: {}", params),
            })?;

        let layer_index = self.layers.len();
        self.current_layer = Some(Layer::new(layer_index, z * self.scale));
        
        debug!("Starting layer {} at z={}", layer_index, z * self.scale);
        Ok(())
    }

    fn process_polyline_command(&mut self, params: &str, line_num: usize) -> FileResult<()> {
        // Format: id,dir,n,x1,y1,x2,y2,...
        let parts: Vec<&str> = params.split(',').collect();
        
        if parts.len() < 4 {
            return Err(FileError::ParseError {
                line: line_num,
                message: "Polyline requires at least 4 parameters".to_string(),
            });
        }

        let _id: i32 = parts[0].trim().parse().unwrap_or(0);
        let _dir: i32 = parts[1].trim().parse().unwrap_or(0);
        let n: usize = parts[2].trim().parse().map_err(|_| FileError::ParseError {
            line: line_num,
            message: "Invalid point count".to_string(),
        })?;

        let expected_coords = n * 2;
        if parts.len() < 3 + expected_coords {
            return Err(FileError::ParseError {
                line: line_num,
                message: format!(
                    "Polyline expects {} coordinates but got {}",
                    expected_coords,
                    parts.len() - 3
                ),
            });
        }

        let points = self.parse_points(&parts[3..], n, line_num)?;
        
        if points.len() >= 2 {
            let mut vector = Vector::new(VectorType::Contour, points);
            vector.id = self.next_vector_id();
            // Power/speed applied later via sticky lookup in attach_params_to_layer
            
            if let Some(ref mut layer) = self.current_layer {
                layer.add_vector(vector);
            }
        }

        Ok(())
    }

    fn process_hatches_command(&mut self, params: &str, line_num: usize) -> FileResult<()> {
        // Format: id,n,x1,y1,x2,y2,... (pairs of points forming individual lines)
        let parts: Vec<&str> = params.split(',').collect();
        
        if parts.len() < 2 {
            return Err(FileError::ParseError {
                line: line_num,
                message: "Hatches requires at least 2 parameters".to_string(),
            });
        }

        let _id: i32 = parts[0].trim().parse().unwrap_or(0);
        let n: usize = parts[1].trim().parse().map_err(|_| FileError::ParseError {
            line: line_num,
            message: "Invalid hatch count".to_string(),
        })?;

        // Determine vector type: use override if set, else default to Hatch
        let vtype = self.hatch_type_override.unwrap_or(VectorType::Hatch);

        // Each hatch line is 2 points = 4 coordinates
        let expected_coords = n * 4;
        if parts.len() < 2 + expected_coords {
            // Try to parse as many as we can
            let available_coords = parts.len() - 2;
            let actual_hatches = available_coords / 4;
            
            if actual_hatches == 0 {
                return Ok(());
            }
        }

        // Parse hatch lines
        let coord_parts = &parts[2..];
        let mut idx = 0;
        
        while idx + 3 < coord_parts.len() {
            let x1: f32 = coord_parts[idx].trim().parse().unwrap_or(0.0) * self.scale;
            let y1: f32 = coord_parts[idx + 1].trim().parse().unwrap_or(0.0) * self.scale;
            let x2: f32 = coord_parts[idx + 2].trim().parse().unwrap_or(0.0) * self.scale;
            let y2: f32 = coord_parts[idx + 3].trim().parse().unwrap_or(0.0) * self.scale;
            
            let points = vec![Point2D::new(x1, y1), Point2D::new(x2, y2)];
            let mut vector = Vector::new(vtype, points);
            vector.id = self.next_vector_id();
            // Power/speed applied later via sticky lookup in attach_params_to_layer
            
            if let Some(ref mut layer) = self.current_layer {
                layer.add_vector(vector);
            }
            
            idx += 4;
        }

        Ok(())
    }

    fn process_contour_command(&mut self, params: &str, line_num: usize) -> FileResult<()> {
        // Same format as polyline
        self.process_polyline_command(params, line_num)
    }

    fn process_boundary_command(&mut self, params: &str, line_num: usize) -> FileResult<()> {
        // Same format as polyline but creates boundary type
        let parts: Vec<&str> = params.split(',').collect();
        
        if parts.len() < 4 {
            return Err(FileError::ParseError {
                line: line_num,
                message: "Boundary requires at least 4 parameters".to_string(),
            });
        }

        let _id: i32 = parts[0].trim().parse().unwrap_or(0);
        let _dir: i32 = parts[1].trim().parse().unwrap_or(0);
        let n: usize = parts[2].trim().parse().unwrap_or(0);

        if n == 0 || parts.len() < 3 + n * 2 {
            return Ok(());
        }

        let points = self.parse_points(&parts[3..], n, line_num)?;
        
        if points.len() >= 2 {
            let mut vector = Vector::polygon(VectorType::Boundary, points);
            vector.id = self.next_vector_id();
            // Power/speed applied later via sticky lookup in attach_params_to_layer
            
            if let Some(ref mut layer) = self.current_layer {
                layer.add_vector(vector);
            }
        }

        Ok(())
    }

    fn process_powers_command(&mut self, params: &str, _line_num: usize) -> FileResult<()> {
        // Format: index,value,index,value,... (sticky: value persists from index until next)
        // Example: $POWER 5,200,14,170,27,190 means vectors 5-13 have power 200, 14-26 have 170, etc.
        let parts: Vec<&str> = params.split(',').collect();
        let mut i = 0;
        while i + 1 < parts.len() {
            if let (Ok(idx), Ok(value)) = (
                parts[i].trim().parse::<u32>(),
                parts[i + 1].trim().parse::<f32>(),
            ) {
                self.power_commands.push((idx, value));
                trace!("Power at vector {} set to {}", idx, value);
            }
            i += 2;
        }
        Ok(())
    }

    fn process_speeds_command(&mut self, params: &str, _line_num: usize) -> FileResult<()> {
        // Format: index,value,index,value,... (sticky: value persists from index until next)
        // Example: $SPEED 5,1000,14,800 means vectors 5-13 have speed 1000, 14+ have 800, etc.
        let parts: Vec<&str> = params.split(',').collect();
        let mut i = 0;
        while i + 1 < parts.len() {
            if let (Ok(idx), Ok(value)) = (
                parts[i].trim().parse::<u32>(),
                parts[i + 1].trim().parse::<f32>(),
            ) {
                self.speed_commands.push((idx, value));
                trace!("Speed at vector {} set to {}", idx, value);
            }
            i += 2;
        }
        Ok(())
    }

    fn process_wait_command(&mut self, params: &str, _line_num: usize) -> FileResult<()> {
        // Format: index,time_us,index,time_us,...
        let parts: Vec<&str> = params.split(',').collect();
        let mut i = 0;
        while i + 1 < parts.len() {
            if let (Ok(idx), Ok(time)) = (
                parts[i].trim().parse::<u32>(),
                parts[i + 1].trim().parse::<u32>(),
            ) {
                self.current_wait_times.push((idx, time));
            }
            i += 2;
        }
        Ok(())
    }

    fn attach_params_to_layer(&mut self, layer: &mut Layer) {
        let label = self.source_label.clone().unwrap_or_default();
        
        // Sort commands by index for proper sticky lookup
        self.power_commands.sort_by_key(|&(idx, _)| idx);
        self.speed_commands.sort_by_key(|&(idx, _)| idx);
        
        // Apply sticky power/speed values to each vector based on its index
        for (vec_idx, vec) in layer.vectors.iter_mut().enumerate() {
            // Sticky power: find the last command with index <= vec_idx
            vec.parameters.power = self.get_sticky_value(&self.power_commands, vec_idx as u32);
            // Sticky speed: find the last command with index <= vec_idx
            vec.parameters.speed = self.get_sticky_value(&self.speed_commands, vec_idx as u32);
        }
        
        // Apply wait times to individual vectors (non-sticky: only specific indices)
        for &(idx, time_us) in &self.current_wait_times {
            if let Some(vec) = layer.vectors.get_mut(idx as usize) {
                vec.parameters.wait_time = Some(time_us as f32);
            }
        }
        
        // Store layer-level parameters for reference
        // Use the first power/speed value if available (for display purposes)
        let params = LayerParameters {
            power: self.power_commands.first().map(|&(_, v)| v),
            speed: self.speed_commands.first().map(|&(_, v)| v),
            wait_times: self.current_wait_times.clone(),
        };
        layer.params.push((label, params));
    }
    
    /// Get the sticky value for a given vector index
    /// Returns the value from the last command whose index <= vec_idx
    fn get_sticky_value(&self, commands: &[(u32, f32)], vec_idx: u32) -> Option<f32> {
        // Commands should be sorted by index
        // Find the last command with index <= vec_idx
        let mut result = None;
        for &(cmd_idx, value) in commands {
            if cmd_idx <= vec_idx {
                result = Some(value);
            } else {
                break;
            }
        }
        result
    }

    fn parse_points(&self, parts: &[&str], n: usize, line_num: usize) -> FileResult<Vec<Point2D>> {
        let mut points = Vec::with_capacity(n);
        
        for i in 0..n {
            let x_idx = i * 2;
            let y_idx = i * 2 + 1;
            
            if y_idx >= parts.len() {
                break;
            }

            let x: f32 = parts[x_idx]
                .trim()
                .parse()
                .map_err(|_| FileError::ParseError {
                    line: line_num,
                    message: format!("Invalid x coordinate: {}", parts[x_idx]),
                })?;
                
            let y: f32 = parts[y_idx]
                .trim()
                .parse()
                .map_err(|_| FileError::ParseError {
                    line: line_num,
                    message: format!("Invalid y coordinate: {}", parts[y_idx]),
                })?;

            points.push(Point2D::new(x * self.scale, y * self.scale));
        }

        Ok(points)
    }

    fn next_vector_id(&mut self) -> u64 {
        self.vector_id_counter += 1;
        self.vector_id_counter
    }

    fn finalize(mut self) -> FileResult<Toolpath> {
        // Add last layer if exists, attaching accumulated parameters
        if let Some(mut layer) = self.current_layer.take() {
            self.attach_params_to_layer(&mut layer);
            self.layers.push(layer);
        }

        // Sort layers by Z height
        self.layers.sort_by(|a, b| a.z_height.partial_cmp(&b.z_height).unwrap());
        
        // Update layer indices
        for (i, layer) in self.layers.iter_mut().enumerate() {
            layer.index = i;
        }

        let slice_stack = SliceStack::with_layers("CLI File", self.layers);
        Ok(Toolpath::with_metadata(slice_stack, self.metadata))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_parse_simple_cli() {
        let cli_data = r#"
$$HEADERSTART
$$UNITS/MM
$$HEADEREND
$$GEOMETRYSTART
$$LAYER/0.1
$$POLYLINE/1,0,3,0,0,10,0,10,10
$$LAYER/0.2
$$POLYLINE/1,0,2,0,0,5,5
$$GEOMETRYEND
"#;

        let parser = CliParser::new();
        let toolpath = parser.parse(Cursor::new(cli_data)).unwrap();
        
        assert_eq!(toolpath.slice_stack.layer_count(), 2);
        assert_eq!(toolpath.slice_stack.get_layer(0).unwrap().vectors.len(), 1);
    }

    #[test]
    fn test_parse_hatches() {
        let cli_data = r#"
$$GEOMETRYSTART
$$LAYER/0.1
$$HATCHES/1,2,0,0,10,0,0,5,10,5
$$GEOMETRYEND
"#;

        let parser = CliParser::new();
        let toolpath = parser.parse(Cursor::new(cli_data)).unwrap();
        
        let layer = toolpath.slice_stack.get_layer(0).unwrap();
        assert_eq!(layer.hatches().len(), 2);
    }

    #[test]
    fn test_sticky_power_speed() {
        // Test that POWER/SPEED values are sticky (persist until next index)
        // $POWER 2,200,5,170 means: vectors 2-4 have power 200, vectors 5+ have power 170
        let cli_data = r#"
$$GEOMETRYSTART
$$LAYER/0.1
$$HATCHES/1,8,0,0,10,0,0,5,10,5,0,10,10,10,0,15,10,15,0,20,10,20,0,25,10,25,0,30,10,30,0,35,10,35
$$POWERS/2,200,5,170
$$SPEEDS/1,1000,4,800
$$GEOMETRYEND
"#;

        let parser = CliParser::new();
        let toolpath = parser.parse(Cursor::new(cli_data)).unwrap();
        let layer = toolpath.slice_stack.get_layer(0).unwrap();
        
        assert_eq!(layer.vectors.len(), 8);
        
        // Vectors 0,1: no power (before index 2)
        assert_eq!(layer.vectors[0].parameters.power, None);
        assert_eq!(layer.vectors[1].parameters.power, None);
        
        // Vectors 2,3,4: power = 200 (sticky from index 2)
        assert_eq!(layer.vectors[2].parameters.power, Some(200.0));
        assert_eq!(layer.vectors[3].parameters.power, Some(200.0));
        assert_eq!(layer.vectors[4].parameters.power, Some(200.0));
        
        // Vectors 5,6,7: power = 170 (sticky from index 5)
        assert_eq!(layer.vectors[5].parameters.power, Some(170.0));
        assert_eq!(layer.vectors[6].parameters.power, Some(170.0));
        assert_eq!(layer.vectors[7].parameters.power, Some(170.0));
        
        // Speed: vectors 0: no speed, 1-3: 1000, 4+: 800
        assert_eq!(layer.vectors[0].parameters.speed, None);
        assert_eq!(layer.vectors[1].parameters.speed, Some(1000.0));
        assert_eq!(layer.vectors[2].parameters.speed, Some(1000.0));
        assert_eq!(layer.vectors[3].parameters.speed, Some(1000.0));
        assert_eq!(layer.vectors[4].parameters.speed, Some(800.0));
        assert_eq!(layer.vectors[5].parameters.speed, Some(800.0));
    }

    #[test]
    fn test_non_sticky_wait() {
        // Test that WAIT values are non-sticky (only apply to specific indices)
        // $WAIT 2,4000,6,8000 means: vector 2 has wait 4000, vector 6 has wait 8000
        // Vectors 3,4,5 should have NO wait time
        let cli_data = r#"
$$GEOMETRYSTART
$$LAYER/0.1
$$HATCHES/1,8,0,0,10,0,0,5,10,5,0,10,10,10,0,15,10,15,0,20,10,20,0,25,10,25,0,30,10,30,0,35,10,35
$$WAIT/2,4000,6,8000
$$GEOMETRYEND
"#;

        let parser = CliParser::new();
        let toolpath = parser.parse(Cursor::new(cli_data)).unwrap();
        let layer = toolpath.slice_stack.get_layer(0).unwrap();
        
        assert_eq!(layer.vectors.len(), 8);
        
        // Only vector 2 and 6 should have wait times
        assert_eq!(layer.vectors[0].parameters.wait_time, None);
        assert_eq!(layer.vectors[1].parameters.wait_time, None);
        assert_eq!(layer.vectors[2].parameters.wait_time, Some(4000.0));
        assert_eq!(layer.vectors[3].parameters.wait_time, None);  // Non-sticky!
        assert_eq!(layer.vectors[4].parameters.wait_time, None);
        assert_eq!(layer.vectors[5].parameters.wait_time, None);
        assert_eq!(layer.vectors[6].parameters.wait_time, Some(8000.0));
        assert_eq!(layer.vectors[7].parameters.wait_time, None);
    }
}
