//! Color Value Object
//!
//! RGBA color representation with utilities.

/// RGBA color with components in 0.0-1.0 range
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Color {
    /// Create a new color
    pub const fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    /// Create an opaque RGB color
    pub const fn rgb(r: f32, g: f32, b: f32) -> Self {
        Self { r, g, b, a: 1.0 }
    }

    /// Create from hex string (e.g., "#FF5500" or "FF5500")
    pub fn from_hex(hex: &str) -> Option<Self> {
        let hex = hex.trim_start_matches('#');
        
        if hex.len() == 6 {
            let r = u8::from_str_radix(&hex[0..2], 16).ok()? as f32 / 255.0;
            let g = u8::from_str_radix(&hex[2..4], 16).ok()? as f32 / 255.0;
            let b = u8::from_str_radix(&hex[4..6], 16).ok()? as f32 / 255.0;
            Some(Self::rgb(r, g, b))
        } else if hex.len() == 8 {
            let r = u8::from_str_radix(&hex[0..2], 16).ok()? as f32 / 255.0;
            let g = u8::from_str_radix(&hex[2..4], 16).ok()? as f32 / 255.0;
            let b = u8::from_str_radix(&hex[4..6], 16).ok()? as f32 / 255.0;
            let a = u8::from_str_radix(&hex[6..8], 16).ok()? as f32 / 255.0;
            Some(Self::new(r, g, b, a))
        } else {
            None
        }
    }

    /// Convert to hex string
    pub fn to_hex(&self) -> String {
        format!(
            "#{:02X}{:02X}{:02X}",
            (self.r * 255.0) as u8,
            (self.g * 255.0) as u8,
            (self.b * 255.0) as u8
        )
    }

    /// Convert to RGBA array
    pub fn to_array(&self) -> [f32; 4] {
        [self.r, self.g, self.b, self.a]
    }

    /// Blend with another color
    pub fn blend(&self, other: &Color, t: f32) -> Color {
        Color {
            r: self.r + (other.r - self.r) * t,
            g: self.g + (other.g - self.g) * t,
            b: self.b + (other.b - self.b) * t,
            a: self.a + (other.a - self.a) * t,
        }
    }

    /// With different alpha
    pub fn with_alpha(self, a: f32) -> Self {
        Self { a, ..self }
    }

    /// Viridis colormap: perceptually uniform, colorblind-safe gradient
    /// purple (#440154) -> teal (#31688E) -> green (#35B779) -> yellow (#FDE725)
    pub fn viridis_gradient(t: f32) -> Color {
        let t = t.clamp(0.0, 1.0);
        // Viridis color stops
        const C0: (f32, f32, f32) = (0.267, 0.004, 0.329); // #440154
        const C1: (f32, f32, f32) = (0.192, 0.408, 0.557); // #31688E
        const C2: (f32, f32, f32) = (0.208, 0.718, 0.475); // #35B779
        const C3: (f32, f32, f32) = (0.992, 0.906, 0.145); // #FDE725

        if t < 0.333 {
            let s = t / 0.333;
            Color::rgb(
                C0.0 + (C1.0 - C0.0) * s,
                C0.1 + (C1.1 - C0.1) * s,
                C0.2 + (C1.2 - C0.2) * s,
            )
        } else if t < 0.667 {
            let s = (t - 0.333) / 0.334;
            Color::rgb(
                C1.0 + (C2.0 - C1.0) * s,
                C1.1 + (C2.1 - C1.1) * s,
                C1.2 + (C2.2 - C1.2) * s,
            )
        } else {
            let s = (t - 0.667) / 0.333;
            Color::rgb(
                C2.0 + (C3.0 - C2.0) * s,
                C2.1 + (C3.1 - C2.1) * s,
                C2.2 + (C3.2 - C2.2) * s,
            )
        }
    }

    // Common colors
    pub const WHITE: Color = Color::rgb(1.0, 1.0, 1.0);
    pub const BLACK: Color = Color::rgb(0.0, 0.0, 0.0);
    pub const RED: Color = Color::rgb(1.0, 0.0, 0.0);
    pub const GREEN: Color = Color::rgb(0.0, 1.0, 0.0);
    pub const BLUE: Color = Color::rgb(0.0, 0.0, 1.0);
    pub const YELLOW: Color = Color::rgb(1.0, 1.0, 0.0);
    pub const CYAN: Color = Color::rgb(0.0, 1.0, 1.0);
    pub const MAGENTA: Color = Color::rgb(1.0, 0.0, 1.0);
    pub const GRAY: Color = Color::rgb(0.5, 0.5, 0.5);
    pub const TRANSPARENT: Color = Color::new(0.0, 0.0, 0.0, 0.0);
}

impl Default for Color {
    fn default() -> Self {
        Self::WHITE
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_hex() {
        let c = Color::from_hex("#FF5500").unwrap();
        assert!((c.r - 1.0).abs() < 1e-2);
        assert!((c.g - 0.333).abs() < 1e-2);
        assert!((c.b - 0.0).abs() < 1e-2);
    }

    #[test]
    fn test_to_hex() {
        let c = Color::rgb(1.0, 0.5, 0.0);
        assert!(c.to_hex().starts_with("#FF"));
    }
}
